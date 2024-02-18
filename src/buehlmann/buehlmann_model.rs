use crate::buehlmann::compartment::Compartment;
use crate::common::{DecoModel, Depth, Gas, Pressure, Seconds, Step};
use crate::buehlmann::zhl_values::{ZHL16C_VALUES, ZHLParams};

pub struct BuehlmannModel {
    compartments: Vec<Compartment>,
    depth: Depth,
}

impl DecoModel for BuehlmannModel {
    /// initialize new Buehlmann (ZH-L16C) model with GF 100/100
    fn new() -> BuehlmannModel {
        let mut model = BuehlmannModel {
            compartments: vec![],
            depth: 0.,
        };
        model.create_compartments(ZHL16C_VALUES);

        model
    }

    /// model step: depth (meters), time (seconds), gas
    fn step(&mut self, depth: &Depth, time: &Seconds, gas: &Gas) {
        let step = Step { depth, time, gas };
        self.depth = *step.depth;
        self.recalculate_compartments(step);
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
        let leading_cpt = self.leading_cpt();
        // surface pressure assumed 1ATA
        let p_surf = 1.;
        let p_amb = p_surf + (&self.depth / 10.);
        // ZHL params coefficients
        let (_, a_coeff, b_coeff) = leading_cpt.params;
        let m_value = a_coeff + (p_amb / b_coeff);
        let m_value_surf = a_coeff + (p_surf / b_coeff);
        let gf_now = ((leading_cpt.inert_pressure - p_amb) / (m_value - p_amb)) * 100.;
        let gf_surf = ((leading_cpt.inert_pressure - p_surf) / (m_value_surf - p_surf)) * 100.;

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
            let compartment = Compartment::new(i + 1, comp_values);
            compartments.push(compartment);
        }
        self.compartments = compartments;
    }

    fn recalculate_compartments(&mut self, step: Step) {
        for compartment in self.compartments.iter_mut() {
            compartment.recalculate(&step);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Gas;

    #[test]
    fn test_ceiling() {
        let mut model = BuehlmannModel::new();
        let air = Gas::new(0.21, 0.);
        model.step(&40., &(30 * 60), &air);
        model.step(&30., &(30 * 60), &air);
        let calculated_ceiling = model.ceiling();
        assert_eq!(calculated_ceiling, 8.207311225723817);
    }

    #[test]
    fn test_gfs() {
        let mut model = BuehlmannModel::new();
        let air = Gas::new(0.21, 0.);

        model.step(&50., &(20 * 60), &air);
        assert_eq!(model.gfs_current(), (-46.50440176081318, 198.13842597008946));

        model.step(&40., &(10 * 60), &air);
        assert_eq!(model.gfs_current(), (-48.28027926904754, 213.03171209358845));
    }

    #[test]
    fn test_model_steps_equality() {
        let mut model1 = BuehlmannModel::new();
        let mut model2 = BuehlmannModel::new();

        let air = Gas::new(0.21, 0.);
        let test_depth = 50.;
        let test_time_minutes: usize = 100;

        model1.step(&test_depth, &(test_time_minutes * 60), &air);

        // step every second
        for _i in 1..=(test_time_minutes * 60) {
            model2.step(&test_depth, &1, &air);
        }

        assert_eq!(model1.ceiling().floor(), model2.ceiling().floor());

        let (model1_gf_now, model1_gf_surf) = model1.gfs_current();
        let (model2_gf_now, model2_gf_surf) = model1.gfs_current();
        assert_eq!(model1_gf_now.floor(), model2_gf_now.floor());
        assert_eq!(model1_gf_surf.floor(), model2_gf_surf.floor());
    }
}
