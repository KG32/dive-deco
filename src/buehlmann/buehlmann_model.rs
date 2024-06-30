use std::ops::RangeBounds;

use crate::buehlmann::compartment::{Compartment, Supersaturation};
use crate::common::{cns_coefficients, AscentRatePerMinute, CNSCoeffRow, CNSPercent, Deco, DecoModel, DecoModelConfig, Depth, DiveState, Gas, GradientFactor, Minutes, Pressure, Seconds, StepData};
use crate::buehlmann::zhl_values::{ZHL_16C_N2_16A_HE_VALUES, ZHLParams};
use crate::buehlmann::buehlmann_config::BuehlmannConfig;
use crate::GradientFactors;

const NDL_CUT_OFF_MINS: Minutes = 99;

#[derive(Clone, Debug)]
pub struct BuehlmannModel {
    config: BuehlmannConfig,
    compartments: Vec<Compartment>,
    state: BuehlmannState,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BuehlmannState {
    depth: Depth,
    time: Seconds,
    gas: Gas,
    gf_low_depth: Option<Depth>,
    cns: CNSPercent,
}

impl BuehlmannState {
    pub fn initial() -> Self {
        Self {
            depth: 0.,
            time: 0,
            gas: Gas::air(),
            gf_low_depth: None,
            cns: 0,
        }
    }
}

impl DecoModel for BuehlmannModel {
    type ConfigType = BuehlmannConfig;

    /// initialize new Buehlmann (ZH-L16C) model with gradient factors
    fn new(config: BuehlmannConfig) -> Self {
        // validate config
        if let Err(e) = config.validate() {
            panic!("Config error [{}]: {}", e.field, e.reason);
        }

        // air as a default init gas
        let initial_model_state = BuehlmannState::initial();

        let mut model = Self {
            config,
            compartments: vec![],
            state: initial_model_state,
        };

        model.create_compartments(ZHL_16C_N2_16A_HE_VALUES, config);

        model
    }

    /// model step: depth (meters), time (seconds), gas
    fn step(&mut self, depth: &Depth, time: &Seconds, gas: &Gas) {
        self.validate_depth(depth);
        self.state.depth = *depth;
        self.state.gas = *gas;
        self.state.time += time;
        let step = StepData { depth, time, gas };
        self.recalculate(step);
    }

    /// model travel between depths in 1s intervals
    // @todo: Schreiner equation instead of Haldane to avoid imprecise intervals
    fn step_travel(&mut self, target_depth: &Depth, time: &Seconds, gas: &Gas) {
        self.validate_depth(target_depth);
        self.state.gas = *gas;
        let mut current_depth = self.state.depth;
        let distance = target_depth - current_depth;
        let travel_time = *time as f64;
        let dist_rate = distance / travel_time;
        let mut i = 0;
        while i < travel_time as usize {
            self.state.time += 1;
            current_depth += dist_rate;
            let step = StepData { depth: &current_depth, time: &1, gas };
            self.recalculate(step);
            i += 1;
        }

        // align with target depth with lost precision @todo: round / bignumber?
        self.state.depth = *target_depth;
    }

    fn step_travel_with_rate(&mut self, target_depth: &Depth, rate: &AscentRatePerMinute, gas: &Gas) {
        self.validate_depth(target_depth);
        let distance = (target_depth - self.state.depth).abs();
        let travel_time_seconds = (distance / rate * 60.) as usize;
        self.step_travel(target_depth, &travel_time_seconds, gas);
    }

    fn ndl(&self) -> Minutes {
        let mut ndl: Minutes = Minutes::MAX;

        // create a simulation model based on current model's state
        let mut sim_model = self.fork();

        // iterate simulation model over 1min steps until NDL cut-off or in deco
        for i in 0..NDL_CUT_OFF_MINS {
            sim_model.step(&self.state.depth, &60, &self.state.gas);
            if sim_model.in_deco() {
                ndl = i;
                break;
            }
        }

        ndl
    }

    fn ceiling(&self) -> Depth {
        let leading_comp: &Compartment = self.leading_comp();
        leading_comp.ceiling()
    }

    fn deco(&self, gas_mixes: Vec<Gas>) -> Deco {
        let mut deco = Deco::default();
        deco.calc(self.fork(), gas_mixes);

        deco
    }

    fn config(&self) -> BuehlmannConfig {
        self.config
    }

    fn dive_state(&self) -> DiveState {
        let BuehlmannState { depth, time, gas, cns, .. } = self.state;
        DiveState {
            depth,
            time,
            gas,
            cns
        }
    }

    fn cns(&self) -> CNSPercent {
        self.state.cns
    }
}

impl BuehlmannModel {
    /// set of current gradient factors (GF now, GF surface)
    pub fn supersaturation(&self) -> Supersaturation {
        let mut acc_gf_99 = 0.;
        let mut acc_gf_surf = 0.;
        for comp in self.compartments.iter() {
            let Supersaturation { gf_99, gf_surf } = comp.supersaturation(self.config.surface_pressure, self.state.depth);
            if gf_99 > acc_gf_99 {
                acc_gf_99 = gf_99;
            }
            if gf_surf > acc_gf_surf {
                acc_gf_surf = gf_surf;
            }
        }

        Supersaturation { gf_99: acc_gf_99, gf_surf: acc_gf_surf }
    }

    #[deprecated(since="1.2.0", note="use `supersaturation` method instead")]
    pub fn gfs_current(&self) -> (Pressure, Pressure) {
        let Supersaturation { gf_99, gf_surf } = self.supersaturation();
        (gf_99, gf_surf)
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
            if compartment.min_tolerable_amb_pressure > comps[leading_comp_index].min_tolerable_amb_pressure {
                leading_comp_index = i;
            }
        }

        &mut comps[leading_comp_index]
    }

    fn create_compartments(&mut self, zhl_values: [ZHLParams; 16], config: BuehlmannConfig) {
        let mut compartments: Vec<Compartment> = vec![];
        for (i, comp_values) in zhl_values.into_iter().enumerate() {
            let compartment = Compartment::new(i + 1, comp_values, config);
            compartments.push(compartment);
        }
        self.compartments = compartments;
    }

    fn recalculate(&mut self, step: StepData) {
        self.recalculate_compartments(&step);
        self.recalculate_cns(&step);
    }

    fn recalculate_compartments(&mut self, step: &StepData) {
        for compartment in self.compartments.iter_mut() {
            compartment.recalculate(&step, self.config.gf.1, self.config.surface_pressure);
        }
        self.recalculate_leading_compartment_with_gf(step);
    }

    fn recalculate_cns(&mut self, step: &StepData) {
        let current_gas = self.state.gas;
        let pp_o2 = current_gas
            .partial_pressures(step.depth, self.config().surface_pressure)
            .o2;
        // only calculate CNS change if o2 partial pressure higher than 0.5
        if pp_o2 > 0.5 {
            // find coefficients for PO2 range
            let mut coeffs_for_range: Option<CNSCoeffRow> = None;
            for row in cns_coefficients.into_iter() {
                let row_range = row.0.clone();
                let in_range = row_range.contains(&(pp_o2 as f32));
                if in_range {
                    coeffs_for_range = Some(row);
                    break;
                }
            }
            // if CNS coefficients found for PO2 range
            if let Some((.., slope, intercept)) = coeffs_for_range {
                // time limit for given P02
                let t_lim = (slope as f64) * pp_o2 + (intercept as f64);
                let cns_fraction = (*step.time as f64) / t_lim;
                self.state.cns += cns_fraction as u8;
            } else {
                // PO2 out of cns table range
                todo!("handle out range PO2");
            }
        }
    }

    // fn recalculate_cns(&mut self, step: &StepData) {
    //     let current_gas = self.state.gas;
    //     let pp_o2 = current_gas
    //         .partial_pressures(step.depth, self.config().surface_pressure)
    //         .o2;
    //     // only calculate CNS change if o2 partial pressure higher than 0.5
    //     if pp_o2 > 0.5 {
    //         // find coefficients for PO2 range
    //         let mut coeffs_for_range: Option<CNSCoeffRow> = None;
    //         for row in cns_coefficients.into_iter() {
    //             let row_range = row.0.clone();
    //             let in_range = row_range.contains(&(pp_o2 as f32));
    //             if in_range {
    //                 coeffs_for_range = Some(row);
    //                 break;
    //             }
    //         }
    //         // if CNS coefficients found for PO2 range
    //         if let Some((.., slope, intercept)) = coeffs_for_range {
    //             // time limit for given P02
    //             let t_lim = (slope as f64) * pp_o2 + (intercept as f64);
    //             let cns_fraction = (*step.time as f64) / t_lim;
    //             self.state.cns += cns_fraction as u8;
    //         } else {
    //             // PO2 out of cns table range
    //             todo!("handle out range PO2");
    //         }
    //     }
    // }

    fn recalculate_leading_compartment_with_gf(&mut self, step: &StepData) {
        let BuehlmannConfig { gf, surface_pressure } = self.config;
        let max_gf = self.max_gf(gf, *step.depth);
        let leading = self.leading_comp_mut();
        let recalc_step = StepData { depth: step.depth,  time: &0, gas: step.gas };
        leading.recalculate(&recalc_step, max_gf, surface_pressure);
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
                    let mut sim_step_depth = target_depth - 1.;
                    if sim_step_depth < 0. {
                        sim_step_depth = 0.;
                    }
                    sim_model.step(&sim_step_depth, &0, &sim_gas);
                    let Supersaturation { gf_99, .. } = sim_model.supersaturation();
                    if gf_99 >= gf_low.into() {
                        break;
                    }
                    target_depth = sim_step_depth;
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

    fn gf_slope_point(&self, gf: GradientFactors, gf_low_depth: Depth, depth: Depth) -> GradientFactor {
        let (gf_low, gf_high) = gf;
        let slope_point: f64 = gf_high as f64 - (((gf_high - gf_low) as f64) / gf_low_depth ) * depth;

        slope_point as u8
    }

    fn fork(&self) -> BuehlmannModel {
        BuehlmannModel {
            config: self.config,
            compartments: self.compartments.clone(),
            state: self.state
        }
    }

    fn validate_depth(&self, depth: &Depth) {
        if *depth < 0. {
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
        model.step(&10., &(10 * 60), &air);
        model.step(&15., &(15 * 60), &nx32);
        assert_eq!(
            model.state,
            BuehlmannState {
                depth: 15.,
                time: (25 * 60),
                gas: nx32,
                gf_low_depth: None,
                cns: 0,
            }
        );
    }

    #[test]
    fn test_max_gf_within_ndl() {
        let gf = (50, 100);
        let mut model = BuehlmannModel::new(BuehlmannConfig::new().gradient_factors(gf.0, gf.1));
        let air = Gas::air();
        let step = StepData { depth: &0., time: &0, gas: &air };
        model.step(&step.depth, &step.time, &step.gas);
        assert_eq!(model.max_gf(gf, *step.depth), 100);
    }

    #[test]
    fn test_max_gf_below_first_stop() {
        let gf = (50, 100);

        let mut model = BuehlmannModel::new(BuehlmannConfig::new().gradient_factors(gf.0, gf.1));
        let air = Gas::air();
        let step = StepData { depth: &40., time: &(10 * 60), gas: &air };
        model.step(&step.depth, &step.time, &step.gas);
        assert_eq!(model.max_gf(gf, *step.depth), 50);
    }

    #[test]
    fn test_max_gf_during_deco() {
        let gf = (30, 70);
        let mut model = BuehlmannModel::new(BuehlmannConfig::new().gradient_factors(gf.0, gf.1));
        let air = Gas::air();

        model.step(&40., &(30 * 60), &air);
        model.step(&21., &(5 * 60), &air);
        model.step(&14., &(0 * 60), &air);
        assert_eq!(model.max_gf(gf, 14.), 40);
    }


    #[test]
    fn test_gf_slope_point() {
        let gf = (30, 85);
        let model = BuehlmannModel::new(BuehlmannConfig::new().gradient_factors(gf.0, gf.1));
        let slope_point = model.gf_slope_point(gf, 33.528, 30.48);
        assert_eq!(slope_point, 35);
    }
}
