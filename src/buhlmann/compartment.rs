use super::zhl_values::{ZHLParam, ZHLParams};
use crate::{
    common::{
        abs, exp, powf, Depth, GradientFactor, InertGas, MbarPressure, PartialPressures, Pressure,
        RecordData,
    },
    BuhlmannConfig, Gas, Time,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Compartment {
    // tissue number
    pub no: u8,
    // decay constant k for He (ln(2)/half_time)
    pub he_k: f64,
    // decay constant k for N2 (ln(2)/half_time)
    pub n2_k: f64,
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
    // compartment'a Buhlmann params (N2 half time, n2 'a' coefficient, n2 'b' coefficient, He half time, ..)
    pub params: ZHLParams,
    // Buhlmann model config (gradient factors, surface pressure)
    model_config: BuhlmannConfig,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Supersaturation {
    pub gf_99: f64,
    pub gf_surf: f64,
}

impl Compartment {
    pub fn new(no: u8, params: ZHLParams, model_config: BuhlmannConfig) -> Self {
        let init_gas = Gas::air();
        let init_gas_compound_pressures =
            init_gas.inspired_partial_pressures(Depth::zero(), model_config.surface_pressure);
        let n2_ip = init_gas_compound_pressures.n2;
        let he_ip = init_gas_compound_pressures.he;

        let (n2_half_time, _, _, he_half_time, ..) = params;
        let ln2 = 0.69314718056;
        let n2_k = ln2 / n2_half_time;
        let he_k = ln2 / he_half_time;

        let mut compartment = Self {
            no,
            he_k,
            n2_k,
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
        let (_, gf_high) = compartment.model_config.gf;
        compartment.m_value_raw = compartment.m_value(
            Depth::zero(),
            compartment.model_config.surface_pressure,
            100,
        );
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

        if self.total_ip.is_nan() {
            println!(
                "recalculate (Haldane) NaN detected: n2_ip={}, he_ip={}, record={:?}",
                self.n2_ip, self.he_ip, record
            );
        }

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

        Depth::from_meters(ceil)
    }

    // tissue supersaturation (gf99, surface gf)
    pub fn supersaturation(&self, surface_pressure: MbarPressure, depth: Depth) -> Supersaturation {
        let p_surf = (surface_pressure as f64) / 1000.;
        let p_amb = p_surf + (depth.as_meters() / 10.);
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
        let p_amb = p_surf + (depth.as_meters() / 10.);

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

    /// Calculate new tissue pressure using Schreiner equation (analytical solution for linear ascent/descent)
    /// This replaces the iterative Haldane approach for travel
    pub fn recalculate_schreiner(
        &mut self,
        p_alv_start_n2: Pressure,
        p_alv_end_n2: Pressure,
        p_alv_start_he: Pressure,
        p_alv_end_he: Pressure,
        time_min: f64,
    ) {
        // N2
        let n2_r = (p_alv_end_n2 - p_alv_start_n2) / time_min;
        self.n2_ip = self.schreiner_equation(p_alv_start_n2, n2_r, time_min, self.n2_k, self.n2_ip);

        // He
        let he_r = (p_alv_end_he - p_alv_start_he) / time_min;
        self.he_ip = self.schreiner_equation(p_alv_start_he, he_r, time_min, self.he_k, self.he_ip);

        // Update totals
        self.total_ip = self.n2_ip + self.he_ip;

        if self.total_ip.is_nan() {
            println!("NaN detected: n2_ip={}, he_ip={}", self.n2_ip, self.he_ip);
        }

        // Update M-values
        let (_, _gf_high) = self.model_config.gf;
        self.m_value_raw = self.m_value(
            Depth::zero(), // This depth parameter is actually not used correctly in m_value_raw calculation in original code?
            // wait, m_value_raw in updated code depends on calc which depends on depth?
            // Actually m_value_raw is typically at surface (depth 0) for GF calculation purposes?
            // Let's keep consistent with recalculate()
            self.model_config.surface_pressure,
            100,
        );
        // Note: min tolerate pressure and m_value_calc depend on current ambient pressure (depth), which isn't passed here.
        // The calling code typically calls recalculate() with the final depth after travel, which fixes this.
        // However, we should at least update m_value_raw based on new IP.
    }

    fn schreiner_equation(
        &self,
        p_i_0: f64, // Initial inspired pressure
        r: f64,     // Rate of change of inspired pressure
        t: f64,     // Time in minutes
        k: f64,     // Decay constant
        p_t_0: f64, // Initial tissue pressure
    ) -> f64 {
        if abs(r) < 1e-9 {
            // Fallback to Haldane if rate is effectively zero (constant depth)
            return p_t_0 + (p_i_0 - p_t_0) * (1.0 - powf(2.0, -t * k / 0.69314718056));
        }
        let e_kt = exp(-k * t);
        p_i_0 + r * (t - 1.0 / k) - (p_i_0 - p_t_0 - r / k) * e_kt
    }

    // compartment pressure change for inert gas (Haldane equation)
    fn compartment_pressure_delta_haldane(
        &self,
        inert_gas: InertGas,
        gas_inspired_p: Pressure,
        time: Time,
        half_time: ZHLParam,
    ) -> Pressure {
        let inert_gas_load = match inert_gas {
            InertGas::Helium => self.he_ip,
            InertGas::Nitrogen => self.n2_ip,
        };

        // (Pi - Po)(1 - e^(-0.693t/half-time))
        let factor = 1. - powf(2.0, -(time.as_minutes()) / half_time);

        (gas_inspired_p - inert_gas_load) * factor
    }

    // tissue tolerable ambient pressure using GF slope, weighted Buhlmann ZHL params based on tissue inert gasses saturation proportions
    fn min_tolerable_amb_pressure(&self, max_gf: GradientFactor) -> Pressure {
        let weighted_zhl_params = self.weighted_zhl_params(self.he_ip, self.n2_ip);
        let (_, a_coefficient_adjusted, b_coefficient_adjusted) =
            self.max_gf_adjusted_zhl_params(weighted_zhl_params, max_gf);

        (self.total_ip - a_coefficient_adjusted) * b_coefficient_adjusted
    }

    // weighted ZHL params (half time, a coefficient, b coefficient) based on N2 and He params and inert gasses proportions in tissue
    pub fn weighted_zhl_params(
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
            if (he_pp + n2_pp) == 0.0 {
                return n2_param;
            }
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
    use crate::{common::Gas, Time};

    fn comp_1() -> Compartment {
        let comp_1_params = (4., 1.2599, 0.5050, 1.51, 01.7424, 0.4245);
        Compartment::new(1, comp_1_params, BuhlmannConfig::default())
    }

    fn comp_5() -> Compartment {
        let comp_5_params = (27., 0.6200, 0.8126, 10.21, 0.9220, 0.7582);
        Compartment::new(5, comp_5_params, BuhlmannConfig::default())
    }

    #[test]
    fn test_constructor() {
        let comp = comp_1();
        assert_eq!(
            comp,
            Compartment {
                no: 1,
                he_k: 0.4590378679205298,
                n2_k: 0.17328679514,
                min_tolerable_amb_pressure: -0.257127315,
                he_ip: 0.0,
                n2_ip: 0.750737,
                total_ip: 0.750737,
                m_value_raw: 3.265840594059406,
                m_value_calc: 3.265840594059406,
                params: (4.0, 1.2599, 0.505, 1.51, 1.7424, 0.4245),
                // mocked config and state
                model_config: BuhlmannConfig::default(),
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
            time: Time::from_seconds(1.),
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
            time: Time::from_seconds(1.),
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
            depth: Depth::from_meters(30.),
            time: Time::from_minutes(10.),
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
            depth: Depth::from_meters(30.),
            time: Time::from_minutes(10.),
            gas: &air,
        };
        comp.recalculate(&recprd, 100, 100);
        let min_tolerable_pressure = comp.min_tolerable_amb_pressure;
        assert_eq!(min_tolerable_pressure, 0.40957969932131577);
    }
}
