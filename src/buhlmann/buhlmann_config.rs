use crate::{
    common::{
        AscentRatePerMinute, ConfigValidationErr, DecoModelConfig, DecoStopFormatting, Depth,
        GradientFactor, GradientFactors, MbarPressure,
    },
    CeilingType,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(test)]
use alloc::vec;

const GF_RANGE_ERR_MSG: &str = "GF values have to be in 1-100 range";
const GF_ORDER_ERR_MSG: &str = "GFLow can't be higher than GFHigh";
const SURFACE_PRESSURE_ERR_MSG: &str = "Surface pressure must be in milibars in 500-1500 range";
const DECO_ASCENT_RATE_ERR_MSG: &str = "Ascent rate must in 1-30 m/s range";

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BuhlmannConfig {
    pub gf: GradientFactors,
    pub surface_pressure: MbarPressure,
    pub deco_ascent_rate: AscentRatePerMinute,
    pub ceiling_type: CeilingType,
    pub round_ceiling: bool,
    pub recalc_all_tissues_m_values: bool,
    pub water_density: f64,
    pub stop_formatting: DecoStopFormatting,
    pub last_stop_depth: Depth,
}

impl BuhlmannConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_gradient_factors<T: Into<GradientFactor>>(mut self, gf_low: T, gf_high: T) -> Self {
        self.gf = (gf_low.into(), gf_high.into());
        self
    }

    pub fn with_surface_pressure<T: Into<MbarPressure>>(mut self, surface_pressure: T) -> Self {
        self.surface_pressure = surface_pressure.into();
        self
    }

    pub fn with_deco_ascent_rate<T: Into<AscentRatePerMinute>>(
        mut self,
        deco_ascent_rate: T,
    ) -> Self {
        self.deco_ascent_rate = deco_ascent_rate.into();
        self
    }

    pub fn with_ceiling_type(mut self, ceiling_type: CeilingType) -> Self {
        self.ceiling_type = ceiling_type;
        self
    }

    pub fn with_round_ceiling(mut self, round_ceiling: bool) -> Self {
        self.round_ceiling = round_ceiling;
        self
    }

    pub fn with_all_m_values_recalculated(mut self, recalc_all_tissues_m_values: bool) -> Self {
        self.recalc_all_tissues_m_values = recalc_all_tissues_m_values;
        self
    }

    pub fn with_water_density(mut self, water_density: f64) -> Self {
        self.water_density = water_density;
        self
    }

    /// Set water density using D6 firmware code logic (0-4 approx)
    /// Density = 1000.0 + (code * 10.0)
    pub fn with_d6_salinity_code(mut self, code: u8) -> Self {
        self.water_density = 1000.0 + (code as f64 * 10.0);
        self
    }

    pub fn with_stop_formatting(mut self, formatting: DecoStopFormatting) -> Self {
        self.stop_formatting = formatting;
        self
    }

    pub fn with_last_stop_depth(mut self, depth: Depth) -> Self {
        self.last_stop_depth = depth;
        self
    }
}

impl Default for BuhlmannConfig {
    fn default() -> Self {
        Self {
            gf: (100, 100),
            surface_pressure: 1013,
            deco_ascent_rate: 10.,
            ceiling_type: CeilingType::Actual,
            round_ceiling: false,
            recalc_all_tissues_m_values: true,
            // Default to EN13319 standard (1030 kg/m3)
            water_density: crate::common::WaterDensities::EN13319,
            stop_formatting: DecoStopFormatting::Metric,
            last_stop_depth: Depth::from_meters(3.0),
        }
    }
}

impl DecoModelConfig for BuhlmannConfig {
    fn validate(&self) -> Result<(), ConfigValidationErr> {
        let Self {
            gf,
            surface_pressure,
            deco_ascent_rate,
            ..
        } = self;

        self.validate_gradient_factors(gf)?;
        self.validate_surface_pressure(surface_pressure)?;
        self.validate_deco_ascent_rate(deco_ascent_rate)?;
        self.validate_water_density(self.water_density)?;

        Ok(())
    }

    fn surface_pressure(&self) -> MbarPressure {
        self.surface_pressure
    }

    fn deco_ascent_rate(&self) -> AscentRatePerMinute {
        self.deco_ascent_rate
    }

    fn ceiling_type(&self) -> CeilingType {
        self.ceiling_type
    }

    fn round_ceiling(&self) -> bool {
        self.round_ceiling
    }

    fn water_density(&self) -> f64 {
        self.water_density
    }

    fn stop_formatting(&self) -> DecoStopFormatting {
        self.stop_formatting
    }

    fn last_stop_depth(&self) -> Depth {
        self.last_stop_depth
    }
}

impl BuhlmannConfig {
    fn validate_water_density(&self, water_density: f64) -> Result<(), ConfigValidationErr> {
        if !(900.0..=1100.0).contains(&water_density) {
            return Err(ConfigValidationErr::new(
                "water_density",
                "Water density must be between 900 and 1100 kg/m3",
            ));
        }
        Ok(())
    }
    fn validate_gradient_factors(&self, gf: &GradientFactors) -> Result<(), ConfigValidationErr> {
        let (gf_low, gf_high) = gf;
        let gf_range = 1..=100;

        if !gf_range.contains(gf_low) || !gf_range.contains(gf_high) {
            return Err(ConfigValidationErr::new("gf", GF_RANGE_ERR_MSG));
        }

        if gf_low > gf_high {
            return Err(ConfigValidationErr::new("gf", GF_ORDER_ERR_MSG));
        }

        Ok(())
    }

    fn validate_surface_pressure(
        &self,
        surface_pressure: &MbarPressure,
    ) -> Result<(), ConfigValidationErr> {
        let mbar_pressure_range = 500..=1500;
        if !mbar_pressure_range.contains(surface_pressure) {
            return Err(ConfigValidationErr::new(
                "surface_pressure",
                SURFACE_PRESSURE_ERR_MSG,
            ));
        }

        Ok(())
    }

    fn validate_deco_ascent_rate(
        &self,
        deco_ascent_rate: &AscentRatePerMinute,
    ) -> Result<(), ConfigValidationErr> {
        let ascent_rate_range = 1.0..=30.0;
        if !ascent_rate_range.contains(deco_ascent_rate) {
            return Err(ConfigValidationErr::new(
                "deco_ascent_rate",
                DECO_ASCENT_RATE_ERR_MSG,
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BuhlmannConfig::default();
        assert_eq!(config.validate(), Ok(()));
        assert_eq!(config.gf, (100, 100));
        assert_eq!(config.deco_ascent_rate, 10.);
        assert_eq!(config.ceiling_type, CeilingType::Actual);
        assert!(!config.round_ceiling);
    }

    #[test]
    fn test_variable_gradient_factors() {
        let config = BuhlmannConfig::new().with_gradient_factors(30, 70);
        assert_eq!(config.validate(), Ok(()));
        assert_eq!(config.gf, (30, 70));
    }

    #[test]
    fn test_gf_range() {
        let invalid_gf_range_cases = vec![(1, 101), (0, 99), (120, 240)];
        for case in invalid_gf_range_cases {
            let (gf_low, gf_high) = case;
            let config = BuhlmannConfig::new().with_gradient_factors(gf_low, gf_high);
            assert_eq!(
                config.validate(),
                Err(ConfigValidationErr::new("gf", GF_RANGE_ERR_MSG))
            );
        }
    }

    #[test]
    fn test_gf_order() {
        let config = BuhlmannConfig::new().with_gradient_factors(90, 80);
        assert_eq!(
            config.validate(),
            Err(ConfigValidationErr::new("gf", GF_ORDER_ERR_MSG))
        );
    }

    #[test]
    fn test_surface_pressure_config() {
        let config = BuhlmannConfig::new().with_surface_pressure(1032);
        assert_eq!(config.validate(), Ok(()));
        assert_eq!(config.surface_pressure, 1032);
    }

    #[test]
    fn test_invalid_surface_pressure_values() {
        let invalid_surface_pressure_cases = vec![0, 100, 2000];
        for invalid_case in invalid_surface_pressure_cases {
            let config = BuhlmannConfig::new().with_surface_pressure(invalid_case);
            assert_eq!(
                config.validate(),
                Err(ConfigValidationErr::new(
                    "surface_pressure",
                    SURFACE_PRESSURE_ERR_MSG
                ))
            );
        }
    }

    #[test]
    fn test_deco_ascent_rate_config() {
        let config = BuhlmannConfig::new().with_deco_ascent_rate(15.5);
        assert_eq!(config.validate(), Ok(()));
        assert_eq!(config.deco_ascent_rate, 15.5);
    }

    #[test]
    fn test_invalid_deco_ascent_rate_values() {
        let invalid_deco_ascent_rate_cases = vec![-3., 0.5, 31.0, 50.5];
        for invalid_case in invalid_deco_ascent_rate_cases {
            let config = BuhlmannConfig::new().with_deco_ascent_rate(invalid_case);
            assert_eq!(
                config.validate(),
                Err(ConfigValidationErr::new(
                    "deco_ascent_rate",
                    DECO_ASCENT_RATE_ERR_MSG
                ))
            );
        }
    }
}
