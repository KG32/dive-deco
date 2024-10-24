use std::cmp::Ordering;

use crate::buehlmann::buehlmann_config::BuehlmannConfig;
use crate::buehlmann::compartment::{Compartment, Supersaturation};
use crate::buehlmann::zhl_values::{ZHLParams, ZHL_16C_N2_16A_HE_VALUES};
use crate::common::{
    AscentRatePerMinute, Cns, Deco, DecoModel, DecoModelConfig, Depth, DiveState, Gas,
    GradientFactor, Minutes, OxTox, RecordData, Seconds,
};
use crate::{CeilingType, DecoCalculationError, DecoRuntime, GradientFactors, Sim};

const NDL_CUT_OFF_MINS: Minutes = 99;

#[derive(Clone, Debug)]
pub struct BuehlmannModel {
    config: BuehlmannConfig,
    compartments: Vec<Compartment>,
    state: BuehlmannState,
    sim: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BuehlmannState {
    depth: Depth,
    time: Seconds,
    gas: Gas,
    gf_low_depth: Option<Depth>,
    ox_tox: OxTox,
}

impl Default for BuehlmannState {
    fn default() -> Self {
        Self {
            depth: 0.,
            time: 0,
            gas: Gas::air(),
            gf_low_depth: None,
            ox_tox: OxTox::default(),
        }
    }
}

impl DecoModel for BuehlmannModel {
    type ConfigType = BuehlmannConfig;

    // initialize with default config
    fn default() -> Self {
        Self::new(BuehlmannConfig::default())
    }

    /// initialize new Buehlmann (ZH-L16C) model with gradient factors
    fn new(config: BuehlmannConfig) -> Self {
        // validate config
        if let Err(e) = config.validate() {
            panic!("Config error [{}]: {}", e.field, e.reason);
        }
        // air as a default init gas
        let initial_model_state = BuehlmannState::default();
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
    fn record(&mut self, depth: Depth, time: Seconds, gas: &Gas) {
        self.validate_depth(depth);
        self.state.depth = depth;
        self.state.gas = *gas;
        self.state.time += time;
        let record = RecordData { depth, time, gas };
        self.recalculate(record);
    }

    /// model travel between depths in 1s intervals
    // @todo: Schreiner equation instead of Haldane to avoid imprecise intervals
    fn record_travel(&mut self, target_depth: Depth, time: Seconds, gas: &Gas) {
        self.validate_depth(target_depth);
        self.state.gas = *gas;
        let mut current_depth = self.state.depth;
        let distance = target_depth - current_depth;
        let travel_time = time as f64;
        let dist_rate = distance / travel_time;
        let mut i = 0;
        while i < travel_time as usize {
            self.state.time += 1;
            current_depth += dist_rate;
            let record = RecordData {
                depth: current_depth,
                time: 1,
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
        rate: AscentRatePerMinute,
        gas: &Gas,
    ) {
        self.validate_depth(target_depth);
        let distance = (target_depth - self.state.depth).abs();
        let travel_time_seconds = (distance / rate * 60.) as Seconds;
        self.record_travel(target_depth, travel_time_seconds, gas);
    }

    fn ndl(&self) -> Minutes {
        let mut ndl: Minutes = NDL_CUT_OFF_MINS;

        // create a simulation model based on current model's state
        let mut sim_model = self.fork();

        // iterate simulation model over 1min records until NDL cut-off or in deco
        for i in 0..NDL_CUT_OFF_MINS {
            sim_model.record(self.state.depth, 60, &self.state.gas);
            if sim_model.in_deco() {
                ndl = i;
                break;
            }
        }
        ndl
    }

    fn ceiling(&self) -> Depth {
        let BuehlmannConfig {
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
                    let sim_depth_cmp = sim_depth.partial_cmp(&0.);
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
            ceiling = ceiling.ceil();
        }

        ceiling
    }

    fn deco(&self, gas_mixes: Vec<Gas>) -> Result<DecoRuntime, DecoCalculationError> {
        let mut deco = Deco::default();
        deco.calc(self.fork(), gas_mixes)
    }

    fn config(&self) -> BuehlmannConfig {
        self.config
    }

    fn dive_state(&self) -> DiveState {
        let BuehlmannState {
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

    // deprecated

    fn step(&mut self, depth: Depth, time: Seconds, gas: &Gas) {
        self.record(depth, time, gas)
    }

    fn step_travel(&mut self, target_depth: Depth, time: Seconds, gas: &Gas) {
        self.record_travel(target_depth, time, gas)
    }

    fn step_travel_with_rate(&mut self, target_depth: Depth, rate: AscentRatePerMinute, gas: &Gas) {
        self.record_travel_with_rate(target_depth, rate, gas)
    }
}

impl Sim for BuehlmannModel {
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

impl BuehlmannModel {
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

    fn create_compartments(&mut self, zhl_values: [ZHLParams; 16], config: BuehlmannConfig) {
        let mut compartments: Vec<Compartment> = vec![];
        for (i, comp_values) in zhl_values.into_iter().enumerate() {
            let compartment = Compartment::new(i as u8 + 1, comp_values, config);
            compartments.push(compartment);
        }
        self.compartments = compartments;
    }

    fn recalculate(&mut self, record: RecordData) {
        self.recalculate_compartments(&record);
        // todo skip on sim
        self.recalculate_ox_tox(&record);
    }

    fn recalculate_compartments(&mut self, record: &RecordData) {
        let (gf_low, gf_high) = self.config.gf;
        for compartment in self.compartments.iter_mut() {
            compartment.recalculate(record, gf_high, self.config.surface_pressure);
        }

        // recalc
        if gf_high != gf_low {
            let max_gf = self.max_gf(self.config.gf, record.depth);
            match self.config.recalc_all_tissues_m_values {
                true => self.recalculate_all_tisues_with_gf(record, max_gf),
                false => self.recalculate_leading_compartment_with_gf(record, max_gf),
            }
        }
    }

    fn recalculate_all_tisues_with_gf(&mut self, record: &RecordData, max_gf: GradientFactor) {
        let recalc_record = RecordData {
            depth: record.depth,
            time: 0,
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
            time: 0,
            gas: record.gas,
        };
        leading.recalculate(&leading_tissue_recalc_record, max_gf, surface_pressure);
    }

    fn recalculate_ox_tox(&mut self, record: &RecordData) {
        self.state
            .ox_tox
            .recalculate(record, self.config().surface_pressure);
    }

    fn max_gf(&mut self, gf: GradientFactors, depth: Depth) -> GradientFactor {
        let (gf_low, gf_high) = gf;
        let in_deco = self.ceiling() > 0.;
        if !in_deco {
            return gf_high;
        }

        let gf_low_depth = match self.state.gf_low_depth {
            Some(gf_low_depth) => gf_low_depth,
            None => {
                // find GF low depth
                let mut sim_model = self.fork();
                let sim_gas = sim_model.state.gas;
                let mut target_depth = sim_model.state.depth;
                while target_depth > 0. {
                    let mut sim_record_depth = target_depth - 1.;
                    if sim_record_depth < 0. {
                        sim_record_depth = 0.;
                    }
                    sim_model.record(sim_record_depth, 0, &sim_gas);
                    let Supersaturation { gf_99, .. } = sim_model.supersaturation();
                    if gf_99 >= gf_low.into() {
                        break;
                    }
                    target_depth = sim_record_depth;
                }
                self.state.gf_low_depth = Some(target_depth);
                target_depth
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
        let slope_point: f64 =
            gf_high as f64 - (((gf_high - gf_low) as f64) / gf_low_depth) * depth;

        slope_point as u8
    }

    fn validate_depth(&self, depth: Depth) {
        if depth < 0. {
            panic!("Invalid depth [{}]", depth);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state() {
        let mut model = BuehlmannModel::new(BuehlmannConfig::default());
        let air = Gas::new(0.21, 0.);
        let nx32 = Gas::new(0.32, 0.);
        model.record(10., 10 * 60, &air);
        model.record(15., 15 * 60, &nx32);
        assert_eq!(model.state.depth, 15.);
        assert_eq!(model.state.time, (25 * 60));
        assert_eq!(model.state.gas, nx32);
        assert_eq!(model.state.gf_low_depth, None);
        assert_ne!(model.state.ox_tox, OxTox::default());
    }

    #[test]
    fn test_max_gf_within_ndl() {
        let gf = (50, 100);
        let mut model =
            BuehlmannModel::new(BuehlmannConfig::new().with_gradient_factors(gf.0, gf.1));
        let air = Gas::air();
        let record = RecordData {
            depth: 0.,
            time: 0,
            gas: &air,
        };
        model.record(record.depth, record.time, record.gas);
        assert_eq!(model.max_gf(gf, record.depth), 100);
    }

    #[test]
    fn test_max_gf_below_first_stop() {
        let gf = (50, 100);

        let mut model =
            BuehlmannModel::new(BuehlmannConfig::new().with_gradient_factors(gf.0, gf.1));
        let air = Gas::air();
        let record = RecordData {
            depth: 40.,
            time: (12 * 60),
            gas: &air,
        };
        model.record(record.depth, record.time, record.gas);
        assert_eq!(model.max_gf(gf, record.depth), 50);
    }

    #[test]
    fn test_max_gf_during_deco() {
        let gf = (30, 70);
        let mut model =
            BuehlmannModel::new(BuehlmannConfig::new().with_gradient_factors(gf.0, gf.1));
        let air = Gas::air();

        model.record(40., 30 * 60, &air);
        model.record(21., 5 * 60, &air);
        model.record(14., 0, &air);
        assert_eq!(model.max_gf(gf, 14.), 40);
    }

    #[test]
    fn test_gf_slope_point() {
        let gf = (30, 85);
        let model = BuehlmannModel::new(BuehlmannConfig::new().with_gradient_factors(gf.0, gf.1));
        let slope_point = model.gf_slope_point(gf, 33.528, 30.48);
        assert_eq!(slope_point, 35);
    }

    #[test]
    fn test_initial_supersaturation() {
        fn extract_supersaturations(model: BuehlmannModel) -> Vec<Supersaturation> {
            model
                .compartments
                .clone()
                .into_iter()
                .map(|comp| comp.supersaturation(model.config().surface_pressure, 0.))
                .collect::<Vec<Supersaturation>>()
        }

        let model_initial = BuehlmannModel::default();

        let mut model_with_surface_interval = BuehlmannModel::default();
        model_with_surface_interval.record(0., 999999, &Gas::air());

        let initial_gfs = extract_supersaturations(model_initial);
        let surface_interval_gfs = extract_supersaturations(model_with_surface_interval);

        assert_eq!(initial_gfs, surface_interval_gfs);
    }
}
