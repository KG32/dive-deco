use crate::buehlmann::compartment::Compartment;
use crate::common::{DecoModel, Depth, Gas, Pressure, Seconds, Step, Minutes};
use crate::buehlmann::zhl_values::{ZHL16C_VALUES, ZHLParams};

const NDL_CUT_OFF_MINS: Minutes = 99;

pub struct BuehlmannModel {
    compartments: Vec<Compartment>,
    depth: Depth,
    time: Seconds,
    gas: Gas,
}

impl DecoModel for BuehlmannModel {
    /// initialize new Buehlmann (ZH-L16C) model with GF 100/100
    fn new() -> Self {
        // air as a default init gas
        let air = Gas::new(0.21, 0.);

        let mut model = Self {
            compartments: vec![],
            depth: 0.,
            time: 0,
            gas: air,
        };
        model.create_compartments(ZHL16C_VALUES);

        model
    }

    /// model step: depth (meters), time (seconds), gas
    fn step(&mut self, depth: &Depth, time: &Seconds, gas: &Gas) {
        self.depth = *depth;
        self.gas = *gas;
        let step = Step { depth, time, gas };
        self.recalculate_compartments(step);
    }

    fn ndl(&self) -> Minutes {
        let mut ndl: Minutes = Minutes::MAX;

        // create a simulation model based on current model's state
        let mut sim_model = self.fork();

        // iterate simulation model over 1min steps until NDL cut-off or in deco
        for i in 0..NDL_CUT_OFF_MINS {
            sim_model.step(&self.depth, &60, &self.gas);
            if sim_model.is_deco() {
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
            let (cpt_gf_now, cpt_gf_surf) = self.gfs_for_compartment(&cpt);
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
        let p_amb = p_surf + (&self.depth / 10.);
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

    fn fork(&self) -> BuehlmannModel {
        BuehlmannModel {
            compartments: self.compartments.clone(),
            depth: self.depth,
            time: self.time,
            gas: self.gas,
        }
    }

    fn is_deco(&self) -> bool {
        self.ceiling() > 0.
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
        assert_eq!(calculated_ceiling, 7.860647737614171);
    }

    #[test]
    fn test_gfs() {
        let mut model = BuehlmannModel::new();
        let air = Gas::new(0.21, 0.);

        model.step(&50., &(20 * 60), &air);
        assert_eq!(model.gfs_current(), (0., 195.48223043242453));

        model.step(&40., &(10 * 60), &air);
        assert_eq!(model.gfs_current(), (0., 210.41983141337982));
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

    #[test]
    fn test_ndl_calculation() {
        let mut model = BuehlmannModel::new();
        let air = Gas::new(0.21, 0.);
        let depth = 30.;

        // with 21/00 at 30m expect NDL 16
        model.step(&depth, &0, &air);
        assert_eq!(model.ndl(), 16);

        // expect NDL 15 after 1 min
        model.step(&depth, &(1*60), &air);
        assert_eq!(model.ndl(), 15);
    }

    #[test]
    fn test_ndl_cut_off() {
        let mut model = BuehlmannModel::new();
        let air = Gas::new(0.21, 0.);

        model.step(&0., &0, &air);
        assert_eq!(model.ndl(), Minutes::MAX);

        model.step(&10., &(10*60), &air);
        assert_eq!(model.ndl(), Minutes::MAX);
    }

    #[test]
    fn test_multi_gas_ndl() {
        let mut model = BuehlmannModel::new();
        let air = Gas::new(0.21, 0.);
        let ean_28 = Gas::new(0.28, 0.);

        model.step(&30., &(0 * 60), &air);
        assert_eq!(model.ndl(), 16);

        model.step(&30., &(10 * 60), &air);
        assert_eq!(model.ndl(), 6);

        model.step(&30., &(0 * 60), &ean_28);
        assert_eq!(model.ndl(), 9);
    }
}
