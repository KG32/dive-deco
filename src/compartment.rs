use crate::global_types::PartialPressure;
use crate::zhl_16_values::ZHLParams;
use crate::gas::Gas;
use crate::step::Step;

#[derive(Debug, PartialEq)]
pub struct Compartment {
    no: u8,
    params: ZHLParams,
    pressure: PartialPressure,
    min_tolerable_pressure: PartialPressure,
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
            min_tolerable_pressure: 0.0,
        };
        compartment.min_tolerable_pressure = compartment.calc_min_tolerable_pressure();

        compartment
    }

    fn calc_min_tolerable_pressure(&self) -> PartialPressure {
        let (_, a_coefficient, b_coefficient) = &self.params;
        (&self.pressure - a_coefficient) * b_coefficient
    }

    pub fn recalculate(&self, step: Step) -> () {

    }

    fn calc_compartment_pressure(step: Step) {


    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor() {
        let cpt_1_params = (4.0, 1.2599, 0.5050);
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
        let cpt_5_params = (27.0, 0.6200, 0.8126);
        let cpt_5 = Compartment::new(5, cpt_5_params);
        let air = Gas::new(0.21);
        let step = Step { depth: 30.0, time: 10.0, gas: air };
        cpt_5.recalculate(step);
        assert_eq!(cpt_5.pressure, 1.33);
    }
}
