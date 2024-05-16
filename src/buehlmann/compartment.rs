use crate::{common::{Depth, GradientFactor, MbarPressure, PartialPressures, Pressure, StepData}, BuehlmannConfig, Gas};
use super::zhl_values::{ZHLParam, ZHLParams};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Compartment {
    pub no: usize,
    // pub min_tolerable_inert_pressure: Pressure,
    pub min_tolerable_amb_pressure: Pressure,
    pub inert_pressure: Pressure,
    pub params: ZHLParams,
    model_config: BuehlmannConfig,
}

impl Compartment {
    pub fn new(
        no: usize,
        params: ZHLParams,
        model_config: BuehlmannConfig,
    ) -> Self {
        let init_gas = Gas::air();
        let init_gas_compound_pressures = init_gas.gas_pressures_compound(1.);

        let mut compartment = Self {
            no,
            params,
            inert_pressure: init_gas_compound_pressures.n2 + init_gas_compound_pressures.he,
            min_tolerable_amb_pressure: -0.,
            model_config,
        };

        // calculate initial minimal tolerable ambient pressure
        let (_, gf_high) = model_config.gf;
        compartment.min_tolerable_amb_pressure = compartment.min_tolerable_amb_pressure(gf_high, init_gas);

        compartment
    }

    pub fn recalculate(&mut self, step: &StepData, max_gf: GradientFactor, surface_pressure: MbarPressure) {
        self.inert_pressure = self.compartment_inert_pressure(step, surface_pressure);
        self.min_tolerable_amb_pressure = self.min_tolerable_amb_pressure(max_gf, *step.gas);
    }

    pub fn ceiling(&self) -> Depth {
        let mut ceil = (self.min_tolerable_amb_pressure - (self.model_config.surface_pressure as f64 / 1000.)) * 10.;
        // cap ceiling at 0 if min tolerable leading compartment pressure depth equivalent negative
        if ceil < 0. {
            ceil = 0.;
        }

        ceil
    }

    pub fn gfs(&self, surface_pressure: MbarPressure, depth: Depth, gas: Gas) -> (Pressure, Pressure) {
        let p_surf = (surface_pressure as f64) / 1000.;
        let p_amb = p_surf + (depth / 10.);
        // ZHL params coefficients
        let (_, a_coeff, b_coeff) = self.weighted_zhl_params(gas);
        let m_value = a_coeff + (p_amb / b_coeff);
        let m_value_surf = a_coeff + (p_surf / b_coeff);
        let gf_now = ((self.inert_pressure - p_amb) / (m_value - p_amb)) * 100.;
        let gf_surf = ((self.inert_pressure - p_surf) / (m_value_surf - p_surf)) * 100.;

        (gf_now, gf_surf)
    }

    fn compartment_inert_pressure(&self, step: &StepData, surface_pressure: MbarPressure) -> Pressure {
        let StepData { depth, time, gas  } = step;
        let PartialPressures { n2: n2_pp, he: he_pp, .. } = gas.inspired_partial_pressures(depth, surface_pressure);
        let inert_gases_pressure = n2_pp + he_pp;
        let (half_time, ..) = self.weighted_zhl_params(**gas);
        let p_comp_delta = (inert_gases_pressure - self.inert_pressure) * (1. - (2_f64.powf(-(**time as f64 / 60.) / half_time)));

        self.inert_pressure + p_comp_delta
    }

    fn min_tolerable_amb_pressure(&self, max_gf: GradientFactor, gas: Gas) -> Pressure {
        let (_, a_coefficient, b_coefficient,) = self.weighted_zhl_params(gas);
        let max_gf_fraction = max_gf as f64 / 100.;
        let a_coefficient_adjusted = a_coefficient * max_gf_fraction;
        let b_coefficient_adjusted = b_coefficient / (max_gf_fraction - (max_gf_fraction * b_coefficient) + b_coefficient);

        (self.inert_pressure - a_coefficient_adjusted) * b_coefficient_adjusted
    }

    fn weighted_zhl_params(&self, gas: Gas) -> (ZHLParam, ZHLParam, ZHLParam) {
        fn weighted_param(he_param: ZHLParam, he_pp: Pressure, n2_param: ZHLParam, n2_pp: Pressure) -> ZHLParam {
            ((he_param * he_pp) + (n2_param * n2_pp)) / (he_pp + n2_pp)
        }
        let (
            n2_half_time,
            n2_a_coeff,
            n2_b_coeff,
            he_half_time,
            he_a_coeff,
            he_b_coeff,
        ) = self.params;
        let PartialPressures {
            o2: _o2_pp,
            he: he_pp,
            n2: n2_pp,
        } = gas.gas_pressures_compound(1.);

        (
            weighted_param(he_half_time, he_pp, n2_half_time, n2_pp),
            weighted_param(he_a_coeff, he_pp, n2_a_coeff, n2_pp),
            weighted_param(he_b_coeff, he_pp, n2_b_coeff, n2_pp),
        )
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Gas;

    fn comp_1() -> Compartment {
        let comp_1_params = (4., 1.2599, 0.5050, 1.51, 01.7424, 0.4245);
        Compartment::new(1, comp_1_params, BuehlmannConfig::default())
    }

    fn comp_5() -> Compartment {
        let comp_5_params = (27., 0.6200, 0.8126, 10.21, 0.9220, 0.7582);
        Compartment::new(5, comp_5_params, BuehlmannConfig::default())
    }

    #[test]
    fn test_constructor() {
        let comp = comp_1();
        assert_eq!(
            comp,
            Compartment {
                no: 1,
                params: (4., 1.2599, 0.5050, 1.51, 01.7424, 0.4245),
                inert_pressure: 0.79,
                min_tolerable_amb_pressure: -0.2372995,
                // mocked config and state
                model_config: BuehlmannConfig::default(),
            }
        );
    }

    #[test]
    fn test_recalculation_ongassing() {
        let mut comp = comp_5();
        let air = Gas::new(0.21, 0.);
        let step = StepData { depth: &30., time: &(10 * 60), gas: &air };
        comp.recalculate(&step, 100, 1000);
        assert_eq!(comp.inert_pressure, 1.315391144211091);
    }

    #[test]
    fn test_weighted_params_trimix() {
        let comp = comp_1();
        let tmx = Gas::new(0.18, 0.50);
        let weighted_params = comp.weighted_zhl_params(tmx);
        assert_eq!(weighted_params, (2.481707317073171, 1.5541073170731705, 0.4559146341463414));
    }

    #[test]
    fn test_min_pressure_calculation() {
        let mut comp = comp_5();
        let air = Gas::new(0.21, 0.);
        let step = StepData { depth: &30., time: &(10 * 60), gas: &air };
        comp.recalculate(&step, 100, 100);
        let min_tolerable_pressure = comp.min_tolerable_amb_pressure;
        assert_eq!(min_tolerable_pressure, 0.4342609809161748);
    }

}
