use super::zhl_values::{ZHLParam, ZHLParams};
use crate::{
    common::{
        Depth, GradientFactor, InertGas, MbarPressure, PartialPressures, Pressure, RecordData, Unit,
    },
    BuehlmannConfig, Gas, Seconds,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Compartment {
    // tissue number
    pub no: u8,
    // tolerable tissue ambient pressure
    pub min_tolerable_amb_pressure: Pressure,
    // helium saturation pressure
    pub he_ip: Pressure,
    // nitrogen saturation pressure
    pub n2_ip: Pressure,
    // total inert gas pressure (He + N2)
    pub total_ip: Pressure,
    // M-value (original)
    pub m_value_raw: Pressure,
    // M-value (calculated considering gradient factors)
    pub m_value_calc: Pressure,
    // compartment'a Buehlmann params (N2 half time, n2 'a' coefficient, n2 'b' coefficient, He half time, ..)
    pub params: ZHLParams,
    // Buehlmann model config (gradient factors, surface pressure)
    model_config: BuehlmannConfig,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Supersaturation {
    pub gf_99: f64,
    pub gf_surf: f64,
}

impl Compartment {
    pub fn new(no: u8, params: ZHLParams, model_config: BuehlmannConfig) -> Self {
        let init_gas = Gas::air();
        let init_gas_compound_pressures =
            init_gas.inspired_partial_pressures(Depth::zero(), model_config.surface_pressure);
        let n2_ip = init_gas_compound_pressures.n2;
        let he_ip = init_gas_compound_pressures.he;

        let mut compartment = Self {
            no,
            params,
            n2_ip,
            he_ip,
            total_ip: he_ip + n2_ip,
            m_value_raw: 0.,  // initial, recalculated later
            m_value_calc: 0., // initial, recalculated later
            min_tolerable_amb_pressure: 0.,
            model_config,
        };

        // calculate initial minimal tolerable ambient pressure
        let (_, gf_high) = model_config.gf;
        compartment.m_value_raw =
            compartment.m_value(Depth::zero(), model_config.surface_pressure, 100);
        compartment.m_value_calc = compartment.m_value_raw;
        compartment.min_tolerable_amb_pressure = compartment.min_tolerable_amb_pressure(gf_high);

        compartment
    }

    // recalculate tissue inert gasses saturation and tolerable pressure
    pub fn recalculate(
        &mut self,
        record: &RecordData,
        max_gf: GradientFactor,
        surface_pressure: MbarPressure,
    ) {
        let (he_inert_pressure, n2_inert_pressure) =
            self.compartment_inert_pressure(record, surface_pressure);

        self.he_ip = he_inert_pressure;
        self.n2_ip = n2_inert_pressure;
        self.total_ip = he_inert_pressure + n2_inert_pressure;

        // @todo m_value tuple
        self.m_value_raw = self.m_value(record.depth, surface_pressure, 100);
        self.m_value_calc = self.m_value(record.depth, surface_pressure, max_gf);

        self.min_tolerable_amb_pressure = self.min_tolerable_amb_pressure(max_gf);
    }

    // tissue ceiling as depth
    pub fn ceiling(&self) -> Depth {
        let mut ceil = (self.min_tolerable_amb_pressure
            - (self.model_config.surface_pressure as f64 / 1000.))
            * 10.;
        // cap ceiling at 0 if min tolerable leading compartment pressure depth equivalent negative
        if ceil < 0. {
            ceil = 0.;
        }

        Depth::m(ceil)
    }

    // tissue supersaturation (gf99, surface gf)
    pub fn supersaturation(&self, surface_pressure: MbarPressure, depth: Depth) -> Supersaturation {
        let p_surf = (surface_pressure as f64) / 1000.;
        let p_amb = p_surf + (depth.meters() / 10.);
        let m_value = self.m_value_raw;
        let m_value_surf = self.m_value(Depth::zero(), surface_pressure, 100);
        let gf_99 = ((self.total_ip - p_amb) / (m_value - p_amb)) * 100.;
        let gf_surf = ((self.total_ip - p_surf) / (m_value_surf - p_surf)) * 100.;

        Supersaturation { gf_99, gf_surf }
    }

    fn m_value(
        &self,
        depth: Depth,
        surface_pressure: MbarPressure,
        max_gf: GradientFactor,
    ) -> Pressure {
        let weighted_zhl_params = self.weighted_zhl_params(self.he_ip, self.n2_ip);
        let (_, a_coeff_adjusted, b_coeff_adjusted) =
            self.max_gf_adjusted_zhl_params(weighted_zhl_params, max_gf);
        let p_surf = (surface_pressure as f64) / 1000.;
        let p_amb = p_surf + (depth.meters() / 10.);

        a_coeff_adjusted + (p_amb / b_coeff_adjusted)
    }

    // tissue inert gasses pressure after record
    fn compartment_inert_pressure(
        &self,
        record: &RecordData,
        surface_pressure: MbarPressure,
    ) -> (Pressure, Pressure) {
        // (he, n2)
        let RecordData { depth, time, gas } = record;
        let PartialPressures {
            n2: n2_pp,
            he: he_pp,
            ..
        } = gas.inspired_partial_pressures(*depth, surface_pressure);

        // partial pressure of inert gases in inspired gas (adjusted alveoli water vapor pressure)
        let he_inspired_pp = he_pp;
        let n2_inspired = n2_pp;

        // tissue saturation pressure change for inert gasses
        let (n2_half_time, _, _, he_half_time, ..) = self.params;
        let he_p_comp_delta = self.compartment_pressure_delta_haldane(
            InertGas::Helium,
            he_inspired_pp,
            *time,
            he_half_time,
        );
        let n2_p_comp_delta = self.compartment_pressure_delta_haldane(
            InertGas::Nitrogen,
            n2_inspired,
            *time,
            n2_half_time,
        );

        // inert gasses pressures after applying delta P
        let he_final = self.he_ip + he_p_comp_delta;
        let n2_final = self.n2_ip + n2_p_comp_delta;

        (he_final, n2_final)
    }

    // compartment pressure change for inert gas (Haldane equation)
    fn compartment_pressure_delta_haldane(
        &self,
        inert_gas: InertGas,
        gas_inspired_p: Pressure,
        time: Seconds,
        half_time: ZHLParam,
    ) -> Pressure {
        let inert_gas_load = match inert_gas {
            InertGas::Helium => self.he_ip,
            InertGas::Nitrogen => self.n2_ip,
        };

        // (Pi - Po)(1 - e^(-0.693t/half-time))
        (gas_inspired_p - inert_gas_load) * (1. - (2_f64.powf(-(time as f64 / 60.) / half_time)))
    }

    // tissue tolerable ambient pressure using GF slope, weighted Buehlmann ZHL params based on tissue inert gasses saturation proportions
    fn min_tolerable_amb_pressure(&self, max_gf: GradientFactor) -> Pressure {
        let weighted_zhl_params = self.weighted_zhl_params(self.he_ip, self.n2_ip);
        let (_, a_coefficient_adjusted, b_coefficient_adjusted) =
            self.max_gf_adjusted_zhl_params(weighted_zhl_params, max_gf);

        (self.total_ip - a_coefficient_adjusted) * b_coefficient_adjusted
    }

    // weighted ZHL params (half time, a coefficient, b coefficient) based on N2 and He params and inert gasses proportions in tissue
    fn weighted_zhl_params(
        &self,
        he_pp: Pressure,
        n2_pp: Pressure,
    ) -> (ZHLParam, ZHLParam, ZHLParam) {
        fn weighted_param(
            he_param: ZHLParam,
            he_pp: Pressure,
            n2_param: ZHLParam,
            n2_pp: Pressure,
        ) -> ZHLParam {
            ((he_param * he_pp) + (n2_param * n2_pp)) / (he_pp + n2_pp)
        }
        let (n2_half_time, n2_a_coeff, n2_b_coeff, he_half_time, he_a_coeff, he_b_coeff) =
            self.params;
        (
            weighted_param(he_half_time, he_pp, n2_half_time, n2_pp),
            weighted_param(he_a_coeff, he_pp, n2_a_coeff, n2_pp),
            weighted_param(he_b_coeff, he_pp, n2_b_coeff, n2_pp),
        )
    }

    // adjust zhl params based on max gf
    fn max_gf_adjusted_zhl_params(
        &self,
        params: (ZHLParam, ZHLParam, ZHLParam),
        max_gf: GradientFactor,
    ) -> (ZHLParam, ZHLParam, ZHLParam) {
        let (half_time, a_coeff, b_coeff) = params;
        let max_gf_fraction = max_gf as f64 / 100.;
        let a_coefficient_adjusted = a_coeff * max_gf_fraction;
        let b_coefficient_adjusted =
            b_coeff / (max_gf_fraction - (max_gf_fraction * b_coeff) + b_coeff);

        (half_time, a_coefficient_adjusted, b_coefficient_adjusted)
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
                min_tolerable_amb_pressure: -0.257127315,
                he_ip: 0.0,
                n2_ip: 0.750737,
                total_ip: 0.750737,
                m_value_raw: 3.265840594059406,
                m_value_calc: 3.265840594059406,
                params: (4.0, 1.2599, 0.505, 1.51, 1.7424, 0.4245),
                // mocked config and state
                model_config: BuehlmannConfig::default(),
            }
        );
    }

    #[test]
    fn test_m_value_raw() {
        let mut comp_1 = comp_1();
        let mut comp_5 = comp_5();
        let air = Gas::new(0.21, 0.);
        let record = RecordData {
            depth: Depth::zero(),
            time: 1,
            gas: &air,
        };
        comp_1.recalculate(&record, 100, 1000);
        comp_5.recalculate(&record, 100, 1000);
        assert_eq!(comp_1.m_value_raw, 3.24009801980198);
        assert_eq!(comp_5.m_value_raw, 1.8506177701206004);
    }

    #[test]
    fn test_m_value_calc() {
        let mut comp_1 = comp_1();
        let mut comp_5 = comp_5();
        let air = Gas::new(0.21, 0.);
        let record = RecordData {
            depth: Depth::zero(),
            time: 1,
            gas: &air,
        };
        comp_1.recalculate(&record, 70, 1000);
        comp_5.recalculate(&record, 70, 1000);
        assert_eq!(comp_1.m_value_calc, 2.568068613861386);
        assert_eq!(comp_5.m_value_calc, 1.5954324390844203);
    }

    #[test]
    fn test_recalculation_ongassing() {
        let mut comp = comp_5();
        let air = Gas::new(0.21, 0.);
        let record = RecordData {
            depth: Depth::m(30.),
            time: (10 * 60),
            gas: &air,
        };
        comp.recalculate(&record, 100, 1000);
        assert_eq!(comp.total_ip, 1.2850179204911072);
    }

    #[test]
    fn test_weighted_params_trimix() {
        let comp = comp_1();
        let weighted_params = comp.weighted_zhl_params(0.5, 1. - (0.18 + 0.5));
        assert_eq!(
            weighted_params,
            (2.481707317073171, 1.5541073170731705, 0.4559146341463414)
        );
    }

    #[test]
    fn test_min_pressure_calculation() {
        let mut comp = comp_5();
        let air = Gas::new(0.21, 0.);
        let recprd = RecordData {
            depth: Depth::m(30.),
            time: (10 * 60),
            gas: &air,
        };
        comp.recalculate(&recprd, 100, 100);
        let min_tolerable_pressure = comp.min_tolerable_amb_pressure;
        assert_eq!(min_tolerable_pressure, 0.40957969932131577);
    }
}
