use crate::compartment::Compartment;
use crate::step::Step;
use crate::global_types::{Depth, Seconds, Pressure};
use crate::zhl_16_values::ZHLParams;
use crate::gas::Gas;

pub struct ZHLModel {
    compartments: Vec<Compartment>,
    depth: Depth,
}

impl ZHLModel {
    pub fn new(zhl_values: Vec<ZHLParams>) -> ZHLModel {
        let mut model = ZHLModel {
            compartments: vec![],
            depth: 0.,
        };
        model.create_compartments(zhl_values);

        model
    }

    pub fn step(&mut self, depth: &Depth, time: &Seconds, gas: &Gas) {
        let step = Step { depth, time, gas };
        self.depth = *step.depth;
        self.recalculate_compartments(step);

        println!("gf {:?}", self.gfs_current());
    }

    pub fn ceiling(&self) -> Depth {
        let leading_cpt: &Compartment = self.leading_cpt();
        let mut ceil = (leading_cpt.min_tolerable_amb_pressure - 1.) * 10.;
        if ceil < 0. {
            ceil = 0.;
        }

        ceil
    }

    pub fn gfs_current(&self) -> (Pressure, Pressure) {
        let leading_cpt = self.leading_cpt();
        let p_surf = 1.;
        let p_amb = p_surf + (&self.depth / 10.);
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

    fn create_compartments(&mut self, zhl_values: Vec<ZHLParams>) {
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
    use crate::gas::Gas;
    use crate::zhl_16_values::*;

    #[test]
    fn test_ceiling() {
        let mut model = ZHLModel::new(zhl_16_values().to_vec());
        let air = Gas::new(0.21);
        model.step(&40., &(30 * 60), &air);
        model.step(&30., &(30 * 60), &air);
        let calculated_ceiling = model.ceiling();
        assert_eq!(calculated_ceiling, 8.207311225723817);
    }

    #[test]
    fn test_gfs() {
        let mut model = ZHLModel::new(zhl_16_values().to_vec());
        let air = Gas::new(0.21);

        // model.step(&Step { depth: &50., time: &20., gas: &air });
        model.step(&50., &(20 * 60), &air);
        assert_eq!(model.gfs_current(), (-46.50440176081318, 198.13842597008946));

        // model.step(&Step { depth: &40., time: &10., gas: &air });
        model.step(&40., &(10 * 60), &air);
        assert_eq!(model.gfs_current(), (-48.28027926904754, 213.03171209358845));
    }

    #[test]
    fn test_model_steps_equality() {
        let mut model1 = ZHLModel::new(zhl_16_values().to_vec());
        let mut model2 = ZHLModel::new(zhl_16_values().to_vec());

        let air = Gas::new(0.21);
        let test_depth = 50.;
        let test_time_minutes = 100;

        model1.step(&test_depth, &(test_time_minutes * 60), &air);

        for _i in 1..=test_time_minutes {
            model2.step(&test_depth, &(1 * 60), &air);
        }

        assert_eq!(model1.ceiling().floor(), model2.ceiling().floor());

        let (model1_gf_now, model1_gf_surf) = model1.gfs_current();
        let (model2_gf_now, model2_gf_surf) = model1.gfs_current();
        assert_eq!(model1_gf_now.floor(), model2_gf_now.floor());
        assert_eq!(model1_gf_surf.floor(), model2_gf_surf.floor());
    }
}
