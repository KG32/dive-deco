use crate::buhlmann::buhlmann_config::BuhlmannConfig;
use crate::buhlmann::compartment::{Compartment, Supersaturation};
use crate::buhlmann::zhl_values::{ZHLParams, ZHL_16C_N2_16A_HE_VALUES};
use crate::common::BreathingSource;
use crate::common::GasMix;
use crate::common::{abs, ceil, ln};
use crate::common::{
    AscentRatePerMinute, ConfigValidationErr, Deco, DecoModel, DecoModelConfig, Depth, DiveState,
    GradientFactor, OxTox, RecordData,
};
use crate::{CeilingType, DecoCalculationError, DecoRuntime, GradientFactors, Sim, Time};
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

const NDL_CUT_OFF_MINS: u8 = 99;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BuhlmannModel {
    config: BuhlmannConfig,
    compartments: Vec<Compartment>,
    state: BuhlmannState,
    sim: bool,
}
pub type BuehlmannModel = BuhlmannModel;

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BuhlmannState {
    depth: Depth,
    time: Time,
    gas: BreathingSource,
    gf_low_depth: Option<Depth>,
    ox_tox: OxTox,
}

impl Default for BuhlmannState {
    fn default() -> Self {
        Self {
            depth: Depth::zero(),
            time: Time::zero(),
            gas: BreathingSource::OpenCircuit(GasMix::air()),
            gf_low_depth: None,
            ox_tox: OxTox::default(),
        }
    }
}

impl DecoModel for BuhlmannModel {
    type ConfigType = BuhlmannConfig;

    // initialize with the default config
    fn default() -> Self {
        Self::new(BuhlmannConfig::default())
    }

    /// initialize new Buhlmann (ZH-L16C) model with gradient factors
    fn new(config: BuhlmannConfig) -> Self {
        // validate config
        if let Err(e) = config.validate() {
            panic!("Config error [{}]: {}", e.field, e.reason);
        }
        // air as a default init gas
        let initial_model_state = BuhlmannState::default();
        let mut model = Self {
            config,
            compartments: vec![],
            state: initial_model_state,
            sim: false,
        };
        model.create_compartments(ZHL_16C_N2_16A_HE_VALUES, config);

        model
    }

    fn config(&self) -> BuhlmannConfig {
        self.config
    }

    fn dive_state(&self) -> DiveState {
        let BuhlmannState {
            depth,
            time,
            gas,
            ox_tox,
            ..
        } = self.state;
        DiveState {
            depth,
            time,
            gas,
            ox_tox,
        }
    }

    /// record data: depth (meters), time (seconds), gas
    fn record(&mut self, depth: Depth, time: Time, gas: &BreathingSource) {
        self.validate_depth(depth);
        self.state.depth = depth;
        self.state.gas = *gas;
        self.state.time += time;
        let record = RecordData { depth, time, gas };
        self.recalculate(record);
    }

    /// model travel between depths in 1s intervals
    // @todo: Schreiner equation instead of Haldane to avoid imprecise intervals
    fn record_travel(&mut self, target_depth: Depth, time: Time, gas: &BreathingSource) {
        self.validate_depth(target_depth);
        self.state.gas = *gas;

        let start_depth = self.state.depth;

        // Update time and depth in state
        self.state.time += time;
        self.state.depth = target_depth;

        let travel_time_mins = time.as_minutes();
        if travel_time_mins <= 0.0 {
            return;
        }

        // Calculate inspired partial pressures at start and end
        // Note: We use the water vapor adjusted calculation inside the model
        use crate::common::physics::depth_to_pressure;
        let start_p_amb = depth_to_pressure(
            start_depth,
            self.config.surface_pressure,
            self.config.water_density,
        );
        let end_p_amb = depth_to_pressure(
            target_depth,
            self.config.surface_pressure,
            self.config.water_density,
        );

        let start_pp = gas.inspired_partial_pressures(start_p_amb);
        let end_pp = gas.inspired_partial_pressures(end_p_amb);

        // Recalculate all compartments using Schreiner equation
        // (This does the heavy lifting analytically instead of iterating 1s steps)
        for compartment in self.compartments.iter_mut() {
            compartment.recalculate_schreiner(
                start_pp.n2,
                end_pp.n2,
                start_pp.he,
                end_pp.he,
                travel_time_mins,
            );
        }

        // After updating compartments, we calculate CNS toxicity.
        // We use the 'record' struct which implies a specific depth.
        // This is consistent with standard implementations for short segments.

        if !self.is_sim() {
            // CNS accumulation is non-linear (exponential at high PO2).
            // We iterate in 1-second intervals (or coarser if needed) to integrate CNS.
            // OxTox calc is cheap (table lookup), making this acceptable for travel segments.
            let steps = time.as_seconds() as usize;
            if steps > 0 {
                let depth_delta =
                    (target_depth.as_meters() - start_depth.as_meters()) / steps as f64;
                let mut current_depth_m = start_depth.as_meters();
                // We use the gas set in state

                for _ in 0..steps {
                    current_depth_m += depth_delta;
                    let step_record = RecordData {
                        depth: Depth::from_meters(current_depth_m),
                        time: Time::from_seconds(1.),
                        gas,
                    };
                    self.recalculate_ox_tox(&step_record);
                }
            } else {
                // fractional second travel? Use average as fallback or just 1 step ending at target
                let record = RecordData {
                    depth: target_depth,
                    time,
                    gas,
                };
                self.recalculate_ox_tox(&record);
            }
        }

        // Finally, trigger a standard recalculate_compartments to ensure M-values, GF-factors, etc
        // are consistent with the FINAL depth (target_depth).
        let final_record = RecordData {
            depth: target_depth,
            time: Time::zero(), // Time already accounted for
            gas,
        };

        // Update derived values (M-values, ceilings) without adding more pressure/time.
        self.recalculate_compartments(&final_record);
    }

    fn record_travel_with_rate(
        &mut self,
        target_depth: Depth,
        // @todo ascent rate units
        rate: AscentRatePerMinute,
        gas: &BreathingSource,
    ) {
        self.validate_depth(target_depth);

        let travel_distance = abs((target_depth - self.state.depth).as_meters());

        self.record_travel(
            target_depth,
            Time::from_seconds(travel_distance / rate * 60.),
            gas,
        );
    }

    fn ndl(&self) -> Time {
        // Early return if already in deco
        if self.in_deco() {
            return Time::zero();
        }
        // Binary search for NDL between 0 and NDL_CUT_OFF_MINS using minute intervals
        let mut low: u8 = 0;
        let mut high: u8 = NDL_CUT_OFF_MINS;
        // Binary search until we narrow down to adjacent minutes
        while high - low > 1 {
            let mid = (low + high) / 2;
            // Check if staying for 'mid' minutes keeps us within NDL
            if self.check_ndl_for(Time::from_minutes(mid)) {
                // Still within NDL at mid-point, so NDL is at least this high
                low = mid;
            } else {
                // In deco at mid-point, so NDL is lower
                high = mid;
            }
        }
        // Check if we can stay for the full cut-off time
        if self.check_ndl_for(Time::from_minutes(high)) {
            Time::from_minutes(high)
        }
        // At this point, low is safe and high is in deco (or high == low + 1)
        // Verify that 'low' minutes keeps us within NDL
        else if self.check_ndl_for(Time::from_minutes(low)) {
            Time::from_minutes(low)
        } else {
            // Edge case: even 'low' puts us in deco
            Time::from_minutes(0)
        }
    }

    fn ceiling(&self) -> Depth {
        let BuhlmannConfig {
            deco_ascent_rate,
            mut ceiling_type,
            ..
        } = self.config();
        if self.sim {
            ceiling_type = CeilingType::Actual;
        }

        let leading_comp: &Compartment = self.leading_comp();
        let mut ceiling = match ceiling_type {
            CeilingType::Actual => leading_comp.ceiling(),
            CeilingType::Adaptive => {
                let mut sim_model = self.fork();
                let sim_gas = sim_model.dive_state().gas;
                let mut calculated_ceiling = sim_model.ceiling();
                loop {
                    let sim_depth = sim_model.dive_state().depth;
                    let sim_depth_cmp = sim_depth.partial_cmp(&Depth::zero());
                    let sim_depth_at_surface = match sim_depth_cmp {
                        Some(Ordering::Equal | Ordering::Less) => true,
                        Some(Ordering::Greater) => false,
                        None => panic!("Simulation depth incomparable to surface"),
                    };
                    if sim_depth_at_surface || sim_depth <= calculated_ceiling {
                        break;
                    }
                    sim_model.record_travel_with_rate(
                        calculated_ceiling,
                        deco_ascent_rate,
                        &sim_gas,
                    );
                    calculated_ceiling = sim_model.ceiling();
                }
                calculated_ceiling
            }
        };

        if self.config().round_ceiling() {
            ceiling = Depth::from_meters(ceil(ceiling.as_meters()));
        }

        ceiling
    }

    fn deco(&self, gas_mixes: Vec<BreathingSource>) -> Result<DecoRuntime, DecoCalculationError> {
        let mut deco = Deco::default();
        deco.calc(self.fork(), gas_mixes)
    }
}

impl Sim for BuhlmannModel {
    fn fork(&self) -> Self {
        Self {
            sim: true,
            ..self.clone()
        }
    }
    fn is_sim(&self) -> bool {
        self.sim
    }
}

impl BuhlmannModel {
    /// Calculate time to desaturate to 105% of surface pressure (No-Dive Time)
    pub fn desaturation_time(&self) -> Time {
        let surface_pressure = self.config.surface_pressure;
        let air = BreathingSource::OpenCircuit(GasMix::air());
        use crate::common::physics::depth_to_pressure;
        let surface_p_amb =
            depth_to_pressure(Depth::zero(), surface_pressure, self.config.water_density);
        let surface_pp = air.inspired_partial_pressures(surface_p_amb);

        // Target is 1.05 * surface partial pressure (standard safety margin)
        let target_n2 = surface_pp.n2 * 1.05;
        let target_he = surface_pp.he * 1.05; // Usually 0 for air but good for completeness

        let mut max_minutes = 0.0;

        for comp in &self.compartments {
            // Time for N2 to decay to target
            // t = -1/k * ln( (P_target - P_inspired) / (P_current - P_inspired) )
            // If P_current <= P_target, time is 0.

            // N2
            if comp.n2_ip > target_n2 {
                let numerator = target_n2 - surface_pp.n2;
                let denominator = comp.n2_ip - surface_pp.n2;
                if denominator != 0.0 && numerator > 0.0 {
                    let ratio = numerator / denominator;
                    if ratio > 0.0 {
                        let t = -(ln(ratio)) / comp.n2_k;
                        if t > max_minutes {
                            max_minutes = t;
                        }
                    }
                }
            }

            // He
            if comp.he_ip > target_he {
                let numerator = target_he - surface_pp.he;
                let denominator = comp.he_ip - surface_pp.he;
                if denominator != 0.0 && numerator > 0.0 {
                    let ratio = numerator / denominator;
                    if ratio > 0.0 {
                        let t = -(ln(ratio)) / comp.he_k;
                        if t > max_minutes {
                            max_minutes = t;
                        }
                    }
                }
            }
        }

        Time::from_minutes(max_minutes)
    }

    /// set of current gradient factors (GF now, GF surface)
    pub fn supersaturation(&self) -> Supersaturation {
        let mut acc_gf_99 = 0.;
        let mut acc_gf_surf = 0.;
        use crate::common::physics::depth_to_pressure;
        let p_amb = depth_to_pressure(
            self.state.depth,
            self.config.surface_pressure,
            self.config.water_density,
        );
        let p_surf = self.config.surface_pressure as f64 / 1000.0;

        for comp in self.compartments.iter() {
            let Supersaturation { gf_99, gf_surf } = comp.supersaturation(p_amb, p_surf);
            if gf_99 > acc_gf_99 {
                acc_gf_99 = gf_99;
            }
            if gf_surf > acc_gf_surf {
                acc_gf_surf = gf_surf;
            }
        }

        Supersaturation {
            gf_99: acc_gf_99,
            gf_surf: acc_gf_surf,
        }
    }

    pub fn tissues(&self) -> Vec<Compartment> {
        self.compartments.clone()
    }

    pub fn update_config(&mut self, new_config: BuhlmannConfig) -> Result<(), ConfigValidationErr> {
        new_config.validate()?;
        self.config = new_config;
        Ok(())
    }

    fn check_ndl_for(&self, time: Time) -> bool {
        let mut sim_model = self.fork();
        sim_model.record(self.state.depth, time, &self.state.gas);
        !sim_model.in_deco()
    }

    fn leading_comp(&self) -> &Compartment {
        let mut leading_comp: &Compartment = &self.compartments[0];
        for compartment in &self.compartments[1..] {
            if compartment.min_tolerable_amb_pressure > leading_comp.min_tolerable_amb_pressure {
                leading_comp = compartment;
            }
        }

        leading_comp
    }

    fn leading_comp_mut(&mut self) -> &mut Compartment {
        let comps = &mut self.compartments;
        let mut leading_comp_index = 0;
        for (i, compartment) in comps.iter().enumerate().skip(1) {
            if compartment.min_tolerable_amb_pressure
                > comps[leading_comp_index].min_tolerable_amb_pressure
            {
                leading_comp_index = i;
            }
        }

        &mut comps[leading_comp_index]
    }

    fn create_compartments(&mut self, zhl_values: [ZHLParams; 16], config: BuhlmannConfig) {
        let mut compartments: Vec<Compartment> = vec![];
        for (i, comp_values) in zhl_values.into_iter().enumerate() {
            let compartment = Compartment::new(i as u8 + 1, comp_values, config);
            compartments.push(compartment);
        }
        self.compartments = compartments;
    }

    fn recalculate(&mut self, record: RecordData) {
        self.recalculate_compartments(&record);
        if !self.is_sim() {
            self.recalculate_ox_tox(&record);
        }
    }

    fn recalculate_compartments(&mut self, record: &RecordData) {
        use crate::common::physics::depth_to_pressure;
        let p_amb = depth_to_pressure(
            record.depth,
            self.config.surface_pressure,
            self.config.water_density,
        );
        let inspired_pp = record.gas.inspired_partial_pressures(p_amb);
        let (gf_low, gf_high) = self.config.gf;
        for compartment in self.compartments.iter_mut() {
            compartment.recalculate(p_amb, inspired_pp, record.time, gf_high);
        }

        // recalc
        if gf_high != gf_low {
            let max_gf = self.calc_max_sloped_gf(self.config.gf, record.depth);

            let should_recalc_all_tissues =
                !self.is_sim() && self.config.recalc_all_tissues_m_values;
            match should_recalc_all_tissues {
                true => self.recalculate_all_tissues_with_gf(record, max_gf),
                false => self.recalculate_leading_compartment_with_gf(record, max_gf),
            }
        }
    }

    fn recalculate_all_tissues_with_gf(&mut self, record: &RecordData, max_gf: GradientFactor) {
        let recalc_record = RecordData {
            depth: record.depth,
            time: Time::zero(),
            gas: record.gas,
        };
        use crate::common::physics::depth_to_pressure;
        let p_amb = depth_to_pressure(
            record.depth,
            self.config.surface_pressure,
            self.config.water_density,
        );
        let inspired_pp = record.gas.inspired_partial_pressures(p_amb);
        for compartment in self.compartments.iter_mut() {
            compartment.recalculate(p_amb, inspired_pp, recalc_record.time, max_gf);
        }
    }

    fn recalculate_leading_compartment_with_gf(
        &mut self,
        record: &RecordData,
        max_gf: GradientFactor,
    ) {
        use crate::common::physics::depth_to_pressure;
        let p_amb = depth_to_pressure(
            record.depth,
            self.config.surface_pressure,
            self.config.water_density,
        );
        let inspired_pp = record.gas.inspired_partial_pressures(p_amb);
        let leading = self.leading_comp_mut();

        // recalculate leading tissue with max gf
        let leading_tissue_recalc_record = RecordData {
            depth: record.depth,
            time: Time::zero(),
            gas: record.gas,
        };
        leading.recalculate(
            p_amb,
            inspired_pp,
            leading_tissue_recalc_record.time,
            max_gf,
        );
    }

    fn recalculate_ox_tox(&mut self, record: &RecordData) {
        self.state.ox_tox.recalculate(
            record,
            self.config().surface_pressure,
            self.config().water_density,
        );
    }

    /// Calculate the maximum gradient factor (GF) for a given depth and gradient factors.
    /// This is the maximum supersaturation on a slope between GF_low and GF_high for a given depth.
    /// Side effect: updates self.state.gf_low_depth
    fn calc_max_sloped_gf(&mut self, gf: GradientFactors, depth: Depth) -> GradientFactor {
        let (gf_low, gf_high) = gf;
        let in_deco = self.ceiling() > Depth::zero();
        if !in_deco {
            return gf_high;
        }

        let gf_low_depth = match self.state.gf_low_depth {
            Some(gf_low_depth) => gf_low_depth,
            None => {
                // Direct calculation for gf_low_depth
                let surface_pressure_bar = self.config.surface_pressure as f64 / 1000.0;
                let gf_low_fraction = gf.0 as f64 / 100.0; // gf.0 is gf_low

                let mut max_calculated_depth_m = 0.0f64;

                for comp in self.compartments.iter() {
                    let total_ip = comp.total_ip;
                    let (_, a_weighted, b_weighted) =
                        comp.weighted_zhl_params(comp.he_ip, comp.n2_ip);

                    // General case: P_amb = (P_ip - G*a) / (1 - G + G/b)
                    let max_amb_p = (total_ip - gf_low_fraction * a_weighted)
                        / (1.0 - gf_low_fraction + gf_low_fraction / b_weighted);

                    let max_depth = (10.0 * (max_amb_p - surface_pressure_bar)).max(0.0);
                    max_calculated_depth_m = max_calculated_depth_m.max(max_depth);
                }

                let calculated_gf_low_depth = Depth::from_meters(max_calculated_depth_m);
                self.state.gf_low_depth = Some(calculated_gf_low_depth);
                calculated_gf_low_depth
            }
        };

        if depth > gf_low_depth {
            return gf_low;
        }

        self.gf_slope_point(gf, gf_low_depth, depth)
    }

    fn gf_slope_point(
        &self,
        gf: GradientFactors,
        gf_low_depth: Depth,
        depth: Depth,
    ) -> GradientFactor {
        let (gf_low, gf_high) = gf;
        let slope_point: f64 = gf_high as f64
            - (((gf_high - gf_low) as f64) / gf_low_depth.as_meters()) * depth.as_meters();

        slope_point as u8
    }

    fn validate_depth(&self, depth: Depth) {
        if depth < Depth::zero() {
            panic!("Invalid depth [{depth}]");
        }
    }
}
