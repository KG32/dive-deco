use crate::global_types::Pressure;
use crate::zhl_values::ZHLParams;
use crate::gas::GasPP;
use crate::step::Step;

#[derive(Debug, PartialEq)]
pub struct Compartment {
    pub no: usize,
    pub min_tolerable_amb_pressure: Pressure,
    pub inert_pressure: Pressure,
    pub params: ZHLParams,
}

impl Compartment {
    pub fn new(
        no: usize,
        params: ZHLParams
    ) -> Compartment {
        let mut compartment = Compartment {
            no,
            params,
            inert_pressure: 0.79,
            min_tolerable_amb_pressure: -0.,
        };
        compartment.min_tolerable_amb_pressure = compartment.calc_min_tolerable_pressure();

        compartment
    }

    pub fn recalculate(&mut self, step: &Step) {
        self.inert_pressure = self.calc_compartment_inert_pressure(&step);
        self.min_tolerable_amb_pressure = self.calc_min_tolerable_pressure();
    }

    pub fn calc_compartment_inert_pressure(&self, step: &Step) -> Pressure {
        let Step { depth, time, gas  } = *step;
        let GasPP { n2, .. } = gas.partial_pressures(depth);
        let (half_time, ..) = self.params;
        let p_comp_delta = (n2 - self.inert_pressure) * (1. - (2_f64.powf(-((*time as f64 / 60.)) / half_time)));
        self.inert_pressure + p_comp_delta
    }

    fn calc_min_tolerable_pressure(&self) -> Pressure {
        let (_, a_coefficient, b_coefficient) = &self.params;
        (self.inert_pressure - a_coefficient) * b_coefficient
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::gas::*;

    #[test]
    fn test_constructor() {
        let cpt_1_params = (4., 1.2599, 0.5050);
        let cpt_1 = Compartment::new(1, cpt_1_params);
        assert_eq!(
            cpt_1,
            Compartment {
                no: 1,
                params: cpt_1_params,
                inert_pressure: 0.79,
                min_tolerable_amb_pressure: -0.2372995
            }
        );
    }

    #[test]
    fn test_recalculation_ongassing() {
        let cpt_5_params = (27., 0.6200, 0.8126);
        let mut cpt_5 = Compartment::new(5, cpt_5_params);
        let air = Gas::new(0.21);
        let step = Step { depth: &30., time: &(10 * 60), gas: &air };
        cpt_5.recalculate(&step);
        assert_eq!(cpt_5.inert_pressure, 1.3266062140854773);
    }

    #[test]
    fn test_min_pressure_calculation() {
        let cpt_5_params = (27., 0.6200, 0.8126);
        let mut cpt_5 = Compartment::new(5, cpt_5_params);
        let air = Gas::new(0.21);
        let step = Step { depth: &30., time: &(10 * 60), gas: &air };
        cpt_5.recalculate(&step);
        let min_tolerable_pressure = cpt_5.min_tolerable_amb_pressure;
        assert_eq!(min_tolerable_pressure, 0.5741882095658588);
    }
}
