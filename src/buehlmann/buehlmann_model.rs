use crate::buehlmann::compartment::Compartment;
use crate::common::{DecoModel, DecoModelConfig, Depth, Gas, Minutes, Pressure, Seconds, Step};
use crate::buehlmann::zhl_values::{ZHL16C_VALUES, ZHLParams};
use crate::buehlmann::buehlmann_config::BuehlmannConfig;

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
        let air = Gas::new(0.21, 0.);

        let initial_model_state = ModelState {
            depth: 0.,
            time: 0,
            gas: air,
        };

        let mut model = Self {
            config,
            compartments: vec![],
            state: initial_model_state
        };

        model.create_compartments(ZHL16C_VALUES);

        model
    }

    /// model step: depth (meters), time (seconds), gas
    fn step(&mut self, depth: &Depth, time: &Seconds, gas: &Gas) {
        self.state.depth = *depth;
        self.state.gas = *gas;
        self.state.time += time;
        let step = Step { depth, time, gas };
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
        let mut ceil = (leading_cpt.min_tolerable_amb_pressure - 1.) * 10.;
        // cap ceiling at 0 if min tolerable leading compartment pressure depth equivalent negative
        if ceil < 0. {
            ceil = 0.;
        }

        ceil
    }
}

impl BuehlmannModel {
    /// set of current gradient factors (GF now, GF surface)
    pub fn gfs_current(&self) -> (Pressure, Pressure) {
        let mut gf_now = 0.;
        let mut gf_surf = 0.;

        for cpt in self.compartments.iter() {
            let (cpt_gf_now, cpt_gf_surf) = self.gfs_for_compartment(cpt);
            if cpt_gf_now > gf_now {
                gf_now = cpt_gf_now;
            }
            if cpt_gf_surf > gf_surf {
                gf_surf = cpt_gf_surf;
            }
        }

        (gf_now, gf_surf)
    }

    fn gfs_for_compartment(&self, cpt: &Compartment) -> (Pressure, Pressure) {
        // surface pressure assumed 1ATA
        let p_surf = 1.;
        let p_amb = p_surf + (&self.state.depth / 10.);
        // ZHL params coefficients
        let (_, a_coeff, b_coeff) = cpt.params;
        let m_value = a_coeff + (p_amb / b_coeff);
        let m_value_surf = a_coeff + (p_surf / b_coeff);
        let gf_now = ((cpt.inert_pressure - p_amb) / (m_value - p_amb)) * 100.;
        let gf_surf = ((cpt.inert_pressure - p_surf) / (m_value_surf - p_surf)) * 100.;

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

    fn create_compartments(&mut self, zhl_values: [ZHLParams; 16]) {
        let mut compartments: Vec<Compartment> = vec![];
        for (i, comp_values) in zhl_values.into_iter().enumerate() {
            let compartment = Compartment::new(i + 1, comp_values, self.config.gf);
            compartments.push(compartment);
        }
        self.compartments = compartments;
    }

    fn recalculate_compartments(&mut self, step: Step) {
        for compartment in self.compartments.iter_mut() {
            compartment.recalculate(&step, self.config.gf);
        }
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
        assert_eq!(model.state, ModelState { depth: 15., time: (25 * 60), gas: nx32 });
    }
}
