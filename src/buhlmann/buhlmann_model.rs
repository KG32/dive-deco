use crate::buhlmann::buhlmann_config::BuhlmannConfig;
use crate::buhlmann::compartment::{Compartment, Supersaturation};
use crate::buhlmann::zhl_values::{ZHLParams, ZHL_16C_N2_16A_HE_VALUES};
use crate::common::{
    AscentRatePerMinute, Cns, ConfigValidationErr, Deco, DecoModel, DecoModelConfig, Depth,
    DiveState, Gas, GradientFactor, OxTox, RecordData,
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
    gas: Gas,
    gf_low_depth: Option<Depth>,
    ox_tox: OxTox,
}

impl Default for BuhlmannState {
    fn default() -> Self {
        Self {
            depth: Depth::zero(),
            time: Time::zero(),
            gas: Gas::air(),
            gf_low_depth: None,
            ox_tox: OxTox::default(),
        }
    }
}

impl DecoModel for BuhlmannModel {
    type ConfigType = BuhlmannConfig;

    // initialize with default config
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

    /// record data: depth (meters), time (seconds), gas
    fn record(&mut self, depth: Depth, time: Time, gas: &Gas) {
        self.validate_depth(depth);
        self.state.depth = depth;
        self.state.gas = *gas;
        self.state.time += time;
        let record = RecordData { depth, time, gas };
        self.recalculate(record);
    }

    /// model travel between depths in 1s intervals
    // @todo: Schreiner equation instead of Haldane to avoid imprecise intervals
    fn record_travel(&mut self, target_depth: Depth, time: Time, gas: &Gas) {
        self.validate_depth(target_depth);
        self.state.gas = *gas;
        let mut current_depth = self.state.depth;
        let distance = target_depth - current_depth;
        let travel_time = time;
        let dist_rate = distance.as_meters() / travel_time.as_seconds();
        let mut i = 0;
        while i < travel_time.as_seconds() as i32 {
            self.state.time += Time::from_seconds(1.);
            current_depth += Depth::from_meters(dist_rate);
            let record = RecordData {
                depth: current_depth,
                time: Time::from_seconds(1.),
                gas,
            };
            self.recalculate(record);
            i += 1;
        }

        // align with target depth with lost precision @todo: round / bignumber?
        self.state.depth = target_depth;
    }

    fn record_travel_with_rate(
        &mut self,
        target_depth: Depth,
        // @todo ascent rate units
        rate: AscentRatePerMinute,
        gas: &Gas,
    ) {
        self.validate_depth(target_depth);
        let distance = libm::fabs((target_depth - self.state.depth).as_meters());
        self.record_travel(target_depth, Time::from_seconds(distance / rate * 60.), gas);
    }

    fn ndl(&self) -> Time {
        let mut ndl = Time::from_minutes(NDL_CUT_OFF_MINS);

        if self.in_deco() {
            return Time::zero();
        }

        // create a simulation model based on current model's state
        let mut sim_model = self.fork();

        // iterate simulation model over 1min records until NDL cut-off or in deco
        let interval = Time::from_minutes(1.);
        for i in 0..NDL_CUT_OFF_MINS {
            // @todo
            sim_model.record(self.state.depth, interval, &self.state.gas);
            if sim_model.in_deco() {
                ndl = interval * i;
                break;
            }
        }
        ndl
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
            ceiling = Depth::from_meters(libm::ceil(ceiling.as_meters()));
        }

        ceiling
    }

    fn deco(&self, gas_mixes: Vec<Gas>) -> Result<DecoRuntime, DecoCalculationError> {
        let mut deco = Deco::default();
        deco.calc(self.fork(), gas_mixes)
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

    fn cns(&self) -> Cns {
        self.state.ox_tox.cns()
    }

    fn otu(&self) -> Cns {
        self.state.ox_tox.otu()
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
    /// set of current gradient factors (GF now, GF surface)
    pub fn supersaturation(&self) -> Supersaturation {
        let mut acc_gf_99 = 0.;
        let mut acc_gf_surf = 0.;
        for comp in self.compartments.iter() {
            let Supersaturation { gf_99, gf_surf } =
                comp.supersaturation(self.config.surface_pressure, self.state.depth);
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
        let (gf_low, gf_high) = self.config.gf;
        for compartment in self.compartments.iter_mut() {
            compartment.recalculate(record, gf_high, self.config.surface_pressure);
        }

        // recalc
        if gf_high != gf_low {
            let max_gf = self.calc_max_sloped_gf(self.config.gf, record.depth);

            let should_recalc_all_tissues =
                !self.is_sim() && self.config.recalc_all_tissues_m_values;
            match should_recalc_all_tissues {
                true => self.recalculate_all_tisues_with_gf(record, max_gf),
                false => self.recalculate_leading_compartment_with_gf(record, max_gf),
            }
        }
    }

    fn recalculate_all_tisues_with_gf(&mut self, record: &RecordData, max_gf: GradientFactor) {
        let recalc_record = RecordData {
            depth: record.depth,
            time: Time::zero(),
            gas: record.gas,
        };
        for compartment in self.compartments.iter_mut() {
            compartment.recalculate(&recalc_record, max_gf, self.config.surface_pressure);
        }
    }

    fn recalculate_leading_compartment_with_gf(
        &mut self,
        record: &RecordData,
        max_gf: GradientFactor,
    ) {
        let surface_pressure = self.config.surface_pressure;
        let leading = self.leading_comp_mut();

        // recalculate leading tissue with max gf
        let leading_tissue_recalc_record = RecordData {
            depth: record.depth,
            time: Time::zero(),
            gas: record.gas,
        };
        leading.recalculate(&leading_tissue_recalc_record, max_gf, surface_pressure);
    }

    fn recalculate_ox_tox(&mut self, record: &RecordData) {
        self.state
            .ox_tox
            .recalculate(record, self.config().surface_pressure);
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
            panic!("Invalid depth [{}]", depth);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    #[test]
    fn test_state() {
        let mut model = BuhlmannModel::new(BuhlmannConfig::default());
        let air = Gas::new(0.21, 0.);
        let nx32 = Gas::new(0.32, 0.);
        model.record(Depth::from_meters(10.), Time::from_minutes(10.), &air);
        model.record(Depth::from_meters(15.), Time::from_minutes(15.), &nx32);
        assert_eq!(model.state.depth.as_meters(), 15.);
        assert_eq!(model.state.time, Time::from_minutes(25.));
        assert_eq!(model.state.gas, nx32);
        assert_eq!(model.state.gf_low_depth, None);
        assert_ne!(model.state.ox_tox, OxTox::default());
    }

    #[test]
    fn test_max_gf_within_ndl() {
        let gf = (50, 100);
        let mut model = BuhlmannModel::new(BuhlmannConfig::new().with_gradient_factors(gf.0, gf.1));
        let air = Gas::air();
        let record = RecordData {
            depth: Depth::from_meters(0.),
            time: Time::zero(),
            gas: &air,
        };
        model.record(record.depth, record.time, record.gas);
        assert_eq!(model.calc_max_sloped_gf(gf, record.depth), 100);
    }

    #[test]
    fn test_max_gf_below_first_stop() {
        let gf = (50, 100);

        let mut model = BuhlmannModel::new(BuhlmannConfig::new().with_gradient_factors(gf.0, gf.1));
        let air = Gas::air();
        let record = RecordData {
            depth: Depth::from_meters(40.),
            time: Time::from_minutes(12.),
            gas: &air,
        };
        model.record(record.depth, record.time, record.gas);
        assert_eq!(model.calc_max_sloped_gf(gf, record.depth), 50);
    }

    #[test]
    fn test_max_gf_during_deco() {
        let gf = (30, 70);
        let mut model = BuhlmannModel::new(BuhlmannConfig::new().with_gradient_factors(gf.0, gf.1));
        let air = Gas::air();

        model.record(Depth::from_meters(40.), Time::from_minutes(30.), &air);
        model.record(Depth::from_meters(21.), Time::from_minutes(5.), &air);
        model.record(Depth::from_meters(14.), Time::zero(), &air);
        assert_eq!(model.calc_max_sloped_gf(gf, Depth::from_meters(14.)), 40);
    }

    #[test]
    fn test_gf_slope_point() {
        let gf = (30, 85);
        let model = BuhlmannModel::new(BuhlmannConfig::new().with_gradient_factors(gf.0, gf.1));
        let slope_point =
            model.gf_slope_point(gf, Depth::from_meters(33.528), Depth::from_meters(30.48));
        assert_eq!(slope_point, 35);
    }

    #[test]
    fn test_initial_supersaturation() {
        fn extract_supersaturations(model: BuhlmannModel) -> Vec<Supersaturation> {
            model
                .compartments
                .clone()
                .into_iter()
                .map(|comp| comp.supersaturation(model.config().surface_pressure, Depth::zero()))
                .collect::<Vec<Supersaturation>>()
        }

        let model_initial = BuhlmannModel::default();

        let mut model_with_surface_interval = BuhlmannModel::default();
        model_with_surface_interval.record(Depth::zero(), Time::from_seconds(999999.), &Gas::air());

        let initial_gfs = extract_supersaturations(model_initial);
        let surface_interval_gfs = extract_supersaturations(model_with_surface_interval);

        assert_eq!(initial_gfs, surface_interval_gfs);
    }

    #[test]
    fn test_updating_config() {
        let mut model = BuhlmannModel::default();
        let initial_config = model.config();
        let new_config = BuhlmannConfig::new()
            .with_gradient_factors(30, 70)
            .with_round_ceiling(true)
            .with_ceiling_type(CeilingType::Adaptive)
            .with_round_ceiling(true);
        assert_ne!(initial_config, new_config, "given configs aren't identical");

        model.update_config(new_config).unwrap();
        let updated_config = model.config();
        assert_eq!(updated_config, new_config, "new config saved");

        let invalid_config = new_config.with_gradient_factors(0, 150);
        let update_res = model.update_config(invalid_config);
        assert_eq!(
            update_res,
            Err(ConfigValidationErr {
                field: String::from("gf"),
                reason: String::from("GF values have to be in 1-100 range"),
            }),
            "invalid config update results in Err"
        );
    }

    #[test]
    fn test_ndl_0_if_in_deco() {
        let mut model = BuhlmannModel::new(
            BuhlmannConfig::default()
                .with_gradient_factors(30, 70)
                .with_ceiling_type(CeilingType::Actual),
        );
        let air = Gas::air();
        model.record(Depth::from_meters(40.), Time::from_minutes(6.), &air);
        model.record(Depth::from_meters(9.), Time::zero(), &air);
        let ndl = model.ndl();
        assert_eq!(ndl, Time::zero());
    }
}
