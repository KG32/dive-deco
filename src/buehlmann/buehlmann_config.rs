use crate::common::{DecoModelConfig, ConfigValidationErr, GradientFactors, MbarPressure};

const GF_RANGE_ERR_MSG: &str = "GF values have to be in 1-100 range";
const GF_ORDER_ERR_MSG: &str = "GFLow can't be higher than GFHigh";
const SURFACE_PRESSURE_ERR_MSG: &str = "Surface pressure must be in milibars in 500-1500 range";

#[derive(Copy, Clone, Debug)]
pub struct BuehlmannConfig {
    pub gf: GradientFactors,
    pub surf_pressure: MbarPressure,
}

impl BuehlmannConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn gradient_factors(mut self, gf_high: u8, gf_low: u8) -> Self {
        self.gf = (gf_high, gf_low);
        self
    }

    pub fn surface_pressure(mut self, surf_pressure: MbarPressure) -> Self {
        self.surf_pressure = surf_pressure;
        self
    }
}

impl Default for BuehlmannConfig {
    fn default() -> Self {
        Self {
            gf: (100, 100),
            surf_pressure: 1013
        }
    }
}

impl DecoModelConfig for BuehlmannConfig {
    fn validate(&self) -> Result<(), ConfigValidationErr> {
        let Self { gf, surf_pressure: surface_pressure } = self;

        self.validate_gradient_factors(gf)?;
        self.validate_surface_pressure(surface_pressure)?;

        Ok(())
    }
}

impl BuehlmannConfig {
    fn validate_gradient_factors(&self, gf: &GradientFactors) -> Result<(), ConfigValidationErr> {
        let (gf_low, gf_high) = gf;
        let gf_range = 1..=100;

        if !gf_range.contains(gf_low) || !gf_range.contains(gf_high) {
            return Err(ConfigValidationErr {
                field: "gf",
                reason: GF_RANGE_ERR_MSG
            });
        }

        if gf_low > gf_high {
            return Err(ConfigValidationErr {
                field: "gf",
                reason: GF_ORDER_ERR_MSG
            });
        }

        // TMP - only uniform gradient factors implemented for now
        if gf_low != gf_high {
            return Err(ConfigValidationErr {
                field: "gf",
                reason: "Currently only uniform gradient factors supported"
            });
        }

        Ok(())
    }

    fn validate_surface_pressure(&self, surface_pressure: &MbarPressure) -> Result<(), ConfigValidationErr> {
        let mbar_pressure_range = 500..=1500;
        if !mbar_pressure_range.contains(surface_pressure) {
            return Err(ConfigValidationErr {
                field: "surface_pressure",
                reason: SURFACE_PRESSURE_ERR_MSG,
            });
        }

        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BuehlmannConfig::default();
        assert_eq!(config.validate(), Ok(()));
        assert_eq!(config.gf, (100, 100));
    }

    #[ignore = "variable GFs not implemented yet"]
    #[test]
    fn test_variable_gradient_factors() {
        let config = BuehlmannConfig::new().gradient_factors(30, 70);
        assert_eq!(config.validate(), Ok(()));
        assert_eq!(config.gf, (30, 70));
    }

    #[test]
    fn test_gf_range() {
        let invalid_gf_range_cases = vec![(1, 101), (0, 99), (120, 240)];
        for case in invalid_gf_range_cases {
            let (gf_low, gf_high) = case;
            let config = BuehlmannConfig::new().gradient_factors(gf_low, gf_high);
            assert_eq!(config.validate(), Err(ConfigValidationErr { field: "gf", reason: GF_RANGE_ERR_MSG }));
        }
    }

    #[test]
    fn test_gf_order() {
        let config = BuehlmannConfig::new().gradient_factors(90, 80);
        assert_eq!(config.validate(), Err(ConfigValidationErr { field: "gf", reason: GF_ORDER_ERR_MSG }));
    }

    #[test]
    fn test_temporal_uniform_gf_check() {
        let config = BuehlmannConfig::new().gradient_factors(30, 70);
        assert_eq!(config.validate(), Err(ConfigValidationErr { field: "gf", reason: "Currently only uniform gradient factors supported" }));
    }

    #[test]
    fn test_surface_pressure_config() {
        let config = BuehlmannConfig::new().surface_pressure(1032);
        assert_eq!(config.validate(), Ok(()));
        assert_eq!(config.surf_pressure, 1032);
    }

    #[test]
    fn test_invalid_surface_pressure_values() {
        let invalid_surface_pressure_cases = vec![0, 100, 2000];
        for invalid_case in invalid_surface_pressure_cases {
            let config = BuehlmannConfig::new().surface_pressure(invalid_case);
            assert_eq!(config.validate(), Err(ConfigValidationErr { field: "surface_pressure", reason: SURFACE_PRESSURE_ERR_MSG }));
        }
    }
}
