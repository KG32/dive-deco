use crate::{common::{Depth, GradientFactor, GradientFactors, MbarPressure, PartialPressures, Pressure, StepData}, BuehlmannConfig};
use super::{buehlmann_model::ModelState, zhl_values::ZHLParams};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Compartment {
    pub no: usize,
    // pub min_tolerable_inert_pressure: Pressure,
    pub min_tolerable_amb_pressure: Pressure,
    pub inert_pressure: Pressure,
    pub params: ZHLParams,
    model_config: BuehlmannConfig,
    model_state: ModelState,
}

impl Compartment {
    pub fn new(
        no: usize,
        params: ZHLParams,
        gf_config: GradientFactors,
        model_config: BuehlmannConfig,
        model_state: ModelState,
    ) -> Self {
        let mut compartment = Self {
            no,
            params,
            inert_pressure: 0.79,
            // min_tolerable_inert_pressure: -0.,
            min_tolerable_amb_pressure: -0.,
            model_config,
            model_state,
        };

        // calculate initial minimal tolerable ambient pressure
        let (gf_low, gf_high) = gf_config;
        compartment.min_tolerable_amb_pressure = compartment.calc_min_tolerable_amb_pressure((gf_low, gf_high));

        compartment
    }

    pub fn recalculate(&mut self, step: &StepData, gf: GradientFactors, surface_pressure: MbarPressure) {
        self.inert_pressure = self.calc_compartment_inert_pressure(step, surface_pressure);
        self.min_tolerable_amb_pressure = self.calc_min_tolerable_amb_pressure(gf);
    }

    pub fn ceiling(&self) -> Depth {
        let mut ceil = (self.min_tolerable_amb_pressure - (self.model_config.surface_pressure as f64 / 1000.)) * 10.;
        // cap ceiling at 0 if min tolerable leading compartment pressure depth equivalent negative
        if ceil < 0. {
            ceil = 0.;
        }

        ceil
    }

    pub fn calc_gfs(&self, surface_pressure: MbarPressure, depth: Depth) -> (Pressure, Pressure) {
        let p_surf = (surface_pressure as f64) / 1000.;
        let p_amb = p_surf + (depth / 10.);
        // ZHL params coefficients
        let (_, a_coeff, b_coeff) = self.params;
        let m_value = a_coeff + (p_amb / b_coeff);
        let m_value_surf = a_coeff + (p_surf / b_coeff);
        let gf_now = ((self.inert_pressure - p_amb) / (m_value - p_amb)) * 100.;
        let gf_surf = ((self.inert_pressure - p_surf) / (m_value_surf - p_surf)) * 100.;

        (gf_now, gf_surf)
    }

    fn calc_compartment_inert_pressure(&self, step: &StepData, surface_pressure: MbarPressure) -> Pressure {
        let StepData { depth, time, gas  } = step;
        let PartialPressures { n2, .. } = gas.inspired_partial_pressures(depth, surface_pressure);
        let (half_time, ..) = self.params;
        let p_comp_delta = (n2 - self.inert_pressure) * (1. - (2_f64.powf(-(**time as f64 / 60.) / half_time)));

        self.inert_pressure + p_comp_delta
    }

    fn calc_min_tolerable_amb_pressure(&self, gf: GradientFactors) -> Pressure {
        let (_, a_coefficient, b_coefficient) = &self.params;
        let (_gf_lo, gf_hi) = gf;
        let gf_hi_fraction = gf_hi as f64 / 100.;
        let a_coefficient_adjusted = a_coefficient * gf_hi_fraction;
        let b_coefficient_adjusted = b_coefficient / (gf_hi_fraction - (gf_hi_fraction * b_coefficient) + b_coefficient);

        (self.inert_pressure - a_coefficient_adjusted) * b_coefficient_adjusted
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Gas;

    fn cpt_1() -> Compartment {
        let cpt_1_params = (4., 1.2599, 0.5050);
        Compartment::new(1, cpt_1_params, (100, 100), BuehlmannConfig::default(), ModelState::initial())
    }

    fn cpt_5() -> Compartment {
        let cpt_5_params = (27., 0.6200, 0.8126);
        Compartment::new(5, cpt_5_params, (100, 100), BuehlmannConfig::default(), ModelState::initial())
    }

    #[test]
    fn test_constructor() {
        let cpt = cpt_1();
        assert_eq!(
            cpt,
            Compartment {
                no: 1,
                params: (4., 1.2599, 0.5050),
                inert_pressure: 0.79,
                min_tolerable_amb_pressure: -0.2372995,
                // mocked config and state
                model_config: BuehlmannConfig::default(),
                model_state: ModelState::initial()
            }
        );
    }

    #[test]
    fn test_recalculation_ongassing() {
        let mut cpt = cpt_5();
        let air = Gas::new(0.21, 0.);
        let step = StepData { depth: &30., time: &(10 * 60), gas: &air };
        cpt.recalculate(&step, (100, 100), 1000);
        assert_eq!(cpt.inert_pressure, 1.315391144211091);
    }

    #[test]
    fn test_min_pressure_calculation() {
        let mut cpt = cpt_5();
        let air = Gas::new(0.21, 0.);
        let step = StepData { depth: &30., time: &(10 * 60), gas: &air };
        cpt.recalculate(&step, (100, 100), 100);
        let min_tolerable_pressure = cpt.min_tolerable_amb_pressure;
        assert_eq!(min_tolerable_pressure, 0.4342609809161748);
    }

}
