use crate::{common::{Gas, GradientFactors, PartialPressures, Pressure, Step}, BuehlmannConfig};
use super::zhl_values::ZHLParams;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Compartment {
    pub no: usize,
    pub min_tolerable_inert_pressure: Pressure,
    pub min_tolerable_amb_pressure: Pressure,
    pub inert_pressure: Pressure,
    pub params: ZHLParams,
}

impl Compartment {
    pub fn new(
        no: usize,
        params: ZHLParams,
    ) -> Self {
        let mut compartment = Self {
            no,
            params,
            inert_pressure: 0.79,
            min_tolerable_inert_pressure: -0.,
            min_tolerable_amb_pressure: -0.,
        };
        // @todo initial calc with GF
        compartment.min_tolerable_amb_pressure = compartment.calc_min_tolerable_amb_pressure((100, 100));

        compartment
    }

    pub fn recalculate(&mut self, step: &Step, gf: GradientFactors) {
        self.inert_pressure = self.calc_compartment_inert_pressure(step);
        self.min_tolerable_amb_pressure = self.calc_min_tolerable_amb_pressure(gf);
        // self.min_tolerable_amb_pressure = self.calc_min_tolerable_inert_pressure(step);
    }

    pub fn calc_compartment_inert_pressure(&self, step: &Step) -> Pressure {
        let Step { depth, time, gas  } = step;
        let PartialPressures { n2, .. } = gas.inspired_partial_pressures(depth);
        let (half_time, ..) = self.params;
        let p_comp_delta = (n2 - self.inert_pressure) * (1. - (2_f64.powf(-(**time as f64 / 60.) / half_time)));

        self.inert_pressure + p_comp_delta
    }

    fn calc_min_tolerable_amb_pressure(&self, gf: GradientFactors) -> Pressure {
        let (_, a_coefficient, b_coefficient) = &self.params;
        let (_gf_lo, gf_hi) = gf;
        let gf_hi_fraction = gf_hi as f64 / 100.;
        let a2 = a_coefficient * gf_hi_fraction;
        let b2 = b_coefficient / (gf_hi_fraction - (gf_hi_fraction * b_coefficient) + b_coefficient);
        let m_value = (self.inert_pressure - a2) * b2;

        m_value
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Gas;

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
                min_tolerable_inert_pressure: -0.,
                min_tolerable_amb_pressure: -0.2372995
            }
        );
    }

    #[test]
    fn test_recalculation_ongassing() {
        let cpt_5_params = (27., 0.6200, 0.8126);
        let mut cpt_5 = Compartment::new(5, cpt_5_params);
        let air = Gas::new(0.21, 0.);
        let step = Step { depth: &30., time: &(10 * 60), gas: &air };
        cpt_5.recalculate(&step, (100, 100));
        assert_eq!(cpt_5.inert_pressure, 1.315391144211091);
    }

    #[test]
    fn test_min_pressure_calculation() {
        let cpt_5_params = (27., 0.6200, 0.8126);
        let mut cpt_5 = Compartment::new(5, cpt_5_params);
        let air = Gas::new(0.21, 0.);
        let step = Step { depth: &30., time: &(10 * 60), gas: &air };
        cpt_5.recalculate(&step, (100, 100));
        let min_tolerable_pressure = cpt_5.min_tolerable_amb_pressure;
        assert_eq!(min_tolerable_pressure, 0.5650748437859325);
    }
}
