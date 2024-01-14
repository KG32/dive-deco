use crate::global_types::PartialPressure;
use crate::zhl_16_values::ZHLParams;
use crate::gas::{Gas, GasPP};
use crate::step::Step;

#[derive(Debug, PartialEq)]
pub struct Compartment {
    pub no: u8,
    pressure: PartialPressure,
    min_tolerable_pressure: PartialPressure,
    params: ZHLParams,
}

impl Compartment {
    pub fn new(
        no: u8,
        params: ZHLParams
    ) -> Compartment {
        let mut compartment = Compartment {
            no,
            params,
            pressure: 0.79,
            min_tolerable_pressure: 0.,
        };
        compartment.min_tolerable_pressure = compartment.calc_min_tolerable_pressure();

        compartment
    }

    pub fn recalculate(&mut self, step: Step) -> () {
        self.pressure = self.calc_compartment_pressure(step);
        self.min_tolerable_pressure = self.calc_min_tolerable_pressure();
    }

    fn calc_min_tolerable_pressure(&self) -> PartialPressure {
        let (_, a_coefficient, b_coefficient) = &self.params;
        (&self.pressure - a_coefficient) * b_coefficient
    }


    fn calc_compartment_pressure(&self, step: Step) -> PartialPressure {
        let Step { depth, time, gas  } = step;
        let GasPP { n2, .. } = gas.partial_pressures(depth);
        let (half_time, ..) = &self.params;
        let p_comp_delta = (n2 - &self.pressure) * (1. - (2_f32.powf(-time / half_time)));
        &self.pressure + p_comp_delta
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor() {
        let cpt_1_params = (4., 1.2599, 0.5050);
        let cpt_1 = Compartment::new(1, cpt_1_params);
        assert_eq!(
            cpt_1,
            Compartment {
                no: 1,
                params: cpt_1_params,
                pressure: 0.79,
                min_tolerable_pressure: -0.23729947
            }
        );
    }

    #[test]
    fn test_recalculation_ongassing() {
        let cpt_5_params = (27., 0.6200, 0.8126);
        let mut cpt_5 = Compartment::new(5, cpt_5_params);
        let air = Gas::new(0.21);
        let step = Step { depth: 30., time: 10., gas: air };
        cpt_5.recalculate(step);
        assert_eq!(cpt_5.pressure, 1.3266063);
    }

    #[test]
    fn test_min_pressure_calculation() {
        let cpt_5_params = (27., 0.6200, 0.8126);
        let mut cpt_5 = Compartment::new(5, cpt_5_params);
        let air = Gas::new(0.21);
        let step = Step { depth: 30., time: 10., gas: air };
        cpt_5.recalculate(step);
        let min_tolerable_pressure = cpt_5.min_tolerable_pressure;
        assert_eq!(min_tolerable_pressure, 0.5741883);
    }
}
