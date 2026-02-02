use super::zhl_values::{
    ZHLParam, ZHLParams, HE_DECAY_1S, HE_DECAY_60S, N2_DECAY_1S, N2_DECAY_60S,
};
use crate::{
    common::{
        abs, exp, BreathingSource, Depth, GasMix, GradientFactor, InertGas, PartialPressures,
        Pressure,
    },
    BuhlmannConfig, Time,
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
        let init_gas = BreathingSource::OpenCircuit(GasMix::air());
        use crate::common::physics::depth_to_pressure;
        let p_amb = depth_to_pressure(
            Depth::zero(),
            model_config.surface_pressure,
            model_config.water_density,
        );
        let init_gas_compound_pressures = init_gas.inspired_partial_pressures(p_amb);
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
        compartment.m_value_raw = compartment.m_value(p_amb, 100);
        compartment.m_value_calc = compartment.m_value_raw;
        compartment.min_tolerable_amb_pressure = compartment.min_tolerable_amb_pressure(gf_high);

        compartment
    }

    pub fn recalculate(
        &mut self,
        p_amb: Pressure,
        inspired_pp: PartialPressures,
        time: Time,
        max_gf: GradientFactor,
    ) {
        // 1. Update Tissue Loading
        let (he_inert_pressure, n2_inert_pressure) =
            self.compartment_inert_pressure(inspired_pp, time);

        self.he_ip = he_inert_pressure;
        self.n2_ip = n2_inert_pressure;
        self.total_ip = he_inert_pressure + n2_inert_pressure;

        if self.total_ip.is_nan() {
            println!(
                "recalculate (Haldane) NaN detected: n2_ip={}, he_ip={}",
                self.n2_ip, self.he_ip
            );
        }

        // 2. Calculate Weighted Params ONCE
        let weighted_params = self.weighted_zhl_params(self.he_ip, self.n2_ip);
        let (_, a_weighted, b_weighted) = weighted_params;

        // 3. Calculate Raw M-Value (GF 100)
        self.m_value_raw = a_weighted + (p_amb / b_weighted);

        // 4. Calculate Adjusted M-Value (GF Low/High)
        if max_gf == 100 {
            // Optimization: If GF is 100, skip the adjustment math
            self.m_value_calc = self.m_value_raw;
            self.min_tolerable_amb_pressure = (self.total_ip - a_weighted) * b_weighted;
        } else {
            // Apply GF Scaling
            let (_, a_calc, b_calc) = self.max_gf_adjusted_zhl_params(weighted_params, max_gf);
            self.m_value_calc = a_calc + (p_amb / b_calc);
            self.min_tolerable_amb_pressure = (self.total_ip - a_calc) * b_calc;
        }
    }

    // tissue ceiling as depth
    pub fn ceiling(&self) -> Depth {
        use crate::common::physics::pressure_to_depth;

        let floor_pressure = self.min_tolerable_amb_pressure;
        let ceiling_depth = pressure_to_depth(
            floor_pressure,
            self.model_config.surface_pressure,
            self.model_config.water_density,
        );

        ceiling_depth
    }

    pub fn supersaturation(&self, p_amb: Pressure, p_surf: Pressure) -> Supersaturation {
        let m_value = self.m_value(p_amb, 100);
        let m_value_surf = self.m_value(p_surf, 100);
        let gf_99 = ((self.total_ip - p_amb) / (m_value - p_amb)) * 100.;
        let gf_surf = ((self.total_ip - p_surf) / (m_value_surf - p_surf)) * 100.;

        Supersaturation { gf_99, gf_surf }
    }

    fn m_value(&self, p_amb: Pressure, max_gf: GradientFactor) -> Pressure {
        let weighted_zhl_params = self.weighted_zhl_params(self.he_ip, self.n2_ip);
        let (_, a_coeff_adjusted, b_coeff_adjusted) =
            self.max_gf_adjusted_zhl_params(weighted_zhl_params, max_gf);

        a_coeff_adjusted + (p_amb / b_coeff_adjusted)
    }

    fn compartment_inert_pressure(
        &self,
        inspired_pp: PartialPressures,
        time: Time,
    ) -> (Pressure, Pressure) {
        // (he, n2)
        let PartialPressures {
            n2: n2_inspired,
            he: he_inspired_pp,
            ..
        } = inspired_pp;

        // tissue saturation pressure change for inert gasses
        let he_p_comp_delta = self.compartment_pressure_delta_haldane(
            InertGas::Helium,
            he_inspired_pp,
            time,
            self.he_k,
        );
        let n2_p_comp_delta = self.compartment_pressure_delta_haldane(
            InertGas::Nitrogen,
            n2_inspired,
            time,
            self.n2_k,
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

        // Calling code is expected to call recalculate() with the final p_amb after travel
        // to correctly update min_tolerable_pressure and m_value_calc.
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
            // P = P_i + (P_old - P_i) * e^(-k * t)
            return p_t_0 + (p_i_0 - p_t_0) * (1.0 - exp(-k * t));
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
        k: f64,
    ) -> Pressure {
        let inert_gas_load = match inert_gas {
            InertGas::Helium => self.he_ip,
            InertGas::Nitrogen => self.n2_ip,
        };

        let t_sec = time.as_seconds();
        let idx = (self.no - 1) as usize;
        let p_delta = if (t_sec - 1.0).abs() < f64::EPSILON {
            // Optimization: LUT for 1s
            match inert_gas {
                InertGas::Nitrogen => N2_DECAY_1S[idx],
                InertGas::Helium => HE_DECAY_1S[idx],
            }
        } else if (t_sec - 60.0).abs() < f64::EPSILON {
            // Optimization: LUT for 60s
            match inert_gas {
                InertGas::Nitrogen => N2_DECAY_60S[idx],
                InertGas::Helium => HE_DECAY_60S[idx],
            }
        } else {
            // Fallback: Standard calculation
            // P = P_i + (P_old - P_i) * (1 - e^(-k * t))
            1. - exp(-time.as_minutes() * k)
        };

        (gas_inspired_p - inert_gas_load) * p_delta
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
        let (n2_half_time, n2_a_coeff, n2_b_coeff, he_half_time, he_a_coeff, he_b_coeff) =
            self.params;

        // OPTIMIZATION: Fast path for Air/Nitrox (No Helium)
        if he_pp <= f64::EPSILON {
            return (n2_half_time, n2_a_coeff, n2_b_coeff);
        }

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
