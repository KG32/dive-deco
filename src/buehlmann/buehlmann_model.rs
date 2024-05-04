use crate::buehlmann::compartment::Compartment;
use crate::common::{DecoModel, DecoModelConfig, Depth, Gas, GradientFactor, Minutes, Pressure, Seconds, StepData};
use crate::buehlmann::zhl_values::{ZHL16C_VALUES, ZHLParams};
use crate::buehlmann::buehlmann_config::BuehlmannConfig;
use crate::GradientFactors;

const NDL_CUT_OFF_MINS: Minutes = 99;

#[derive(Clone, Debug)]
pub struct BuehlmannModel {
    config: BuehlmannConfig,
    compartments: Vec<Compartment>,
    state: ModelState,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelState {
    depth: Depth,
    time: Seconds,
    gas: Gas,
    gf_low_depth: Option<Depth>,
}

impl ModelState {
    pub fn initial() -> Self {
        Self {
            depth: 0.,
            time: 0,
            gas: Gas::air(),
            gf_low_depth: None,
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
        let initial_model_state = ModelState::initial();

        let mut model = Self {
            config,
            compartments: vec![],
            state: initial_model_state
        };

        model.create_compartments(ZHL16C_VALUES, config, initial_model_state);

        model
    }

    /// model step: depth (meters), time (seconds), gas
    fn step(&mut self, depth: &Depth, time: &Seconds, gas: &Gas) {
        self.state.depth = *depth;
        self.state.gas = *gas;
        self.state.time += time;
        let step = StepData { depth, time, gas };
        self.recalculate_compartments(step);
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
        let leading_cpt: &Compartment = self.leading_cpt();
        leading_cpt.ceiling()
    }

    fn config(&self) -> BuehlmannConfig {
        self.config
    }
}

impl BuehlmannModel {
    /// set of current gradient factors (GF now, GF surface)
    pub fn gfs_current(&self) -> (Pressure, Pressure) {
        let mut gf_now = 0.;
        let mut gf_surf = 0.;
        for cpt in self.compartments.iter() {
            let (cpt_gf_now, cpt_gf_surf) = cpt.calc_gfs(self.config.surface_pressure, self.state.depth);
            if cpt_gf_now > gf_now {
                gf_now = cpt_gf_now;
            }
            if cpt_gf_surf > gf_surf {
                gf_surf = cpt_gf_surf;
            }
        }

        (gf_now, gf_surf)
    }

    fn leading_cpt(&self) -> &Compartment {
        let mut leading_cpt: &Compartment = &self.compartments[0];
        for compartment in &self.compartments[1..] {
            if compartment.min_tolerable_amb_pressure > leading_cpt.min_tolerable_amb_pressure {
                leading_cpt = compartment;
            }
        }

        leading_cpt
    }

    fn create_compartments(&mut self, zhl_values: [ZHLParams; 16], config: BuehlmannConfig, state: ModelState) {
        let mut compartments: Vec<Compartment> = vec![];
        for (i, comp_values) in zhl_values.into_iter().enumerate() {
            let compartment = Compartment::new(i + 1, comp_values, self.config.gf, config, state);
            compartments.push(compartment);
        }
        self.compartments = compartments;
    }

    fn recalculate_compartments(&mut self, step: StepData) {
        for compartment in self.compartments.iter_mut() {
            compartment.recalculate(&step, self.config.gf, self.config.surface_pressure);
        }
    }

    fn max_gf(&self, gf: GradientFactors, depth: Depth) -> GradientFactor {
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
                    let sim_step_depth = target_depth - 0.1;
                    sim_model.step(&sim_step_depth, &0, &sim_gas);
                    let (gf99, ..) = sim_model.gfs_current();
                    if gf99 >= gf_low.into() {
                        break;
                    }
                    target_depth = sim_step_depth;
                }
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
        let slope_point: f64 = gf_high as f64 - (((gf_high- gf_low) as f64) / gf_low_depth ) * depth;
        slope_point as u8
    }



    fn fork(&self) -> BuehlmannModel {
        BuehlmannModel {
            config: self.config,
            compartments: self.compartments.clone(),
            state: self.state,
        }
    }

    fn in_deco(&self) -> bool {
        self.ceiling() > 0.
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
        assert_eq!(model.state, ModelState { depth: 15., time: (25 * 60), gas: nx32, gf_low_depth: None });
    }

    #[test]
    fn test_max_gf() {
        let mut model = BuehlmannModel::new(BuehlmannConfig::default());
        let air = Gas::air();
        let gf = (50, 100);
        let step_1 = StepData { depth: &0., time: &0, gas: &air };
        model.step(&step_1.depth, &step_1.time, &step_1.gas);
        assert_eq!(model.max_gf(gf, *step_1.depth), 100);

        let step_2 = StepData { depth: &40., time: &(10 * 60), gas: &air };
        model.step(&step_2.depth, &step_2.time, &step_2.gas);
        assert_eq!(model.max_gf(gf, *step_2.depth), 50);
    }

    #[test]
    fn test_gf_slope_point() {
        let gf = (30, 85);
        let model = BuehlmannModel::new(BuehlmannConfig::new().gradient_factors(gf.0, gf.1));
        let slope_point = model.gf_slope_point(gf, 33.528, 30.48);
        assert_eq!(slope_point, 35);
    }
}
