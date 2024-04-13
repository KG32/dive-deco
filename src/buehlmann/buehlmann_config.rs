use crate::common::{DecoModelConfig, ConfigValidationErr, GradientFactors};

const GF_RANGE_ERR_MSG: &str = "GF values have to be in 1-100 range";
const GF_ORDER_ERR_MSG: &str = "GFLow can't be higher than GFHigh";

impl DecoModelConfig for BuehlmannConfig {
    fn validate(&self) -> Result<(), ConfigValidationErr> {
        let Self { gf } = self;
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

        // TMP - GF low not implemented yet
        if gf_low != gf_high {
            return Err(ConfigValidationErr {
                field: "gf",
                reason: "Currently only uniform gradient factors supported"
            });
        }

        Ok(())
    }
}

impl Default for BuehlmannConfig {
    fn default() -> Self {
        Self {
            gf: (100, 100)
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BuehlmannConfig {
    pub gf: GradientFactors
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
        let config = BuehlmannConfig { gf: (30, 70) };
        assert_eq!(config.validate(), Ok(()));
        assert_eq!(config.gf, (30, 70));
    }

    #[test]
    fn test_gf_range() {
        let invalid_gf_range_cases = vec![(1, 101), (0, 99), (120, 240)];
        for case in invalid_gf_range_cases {
            let config = BuehlmannConfig { gf: case };
            assert_eq!(config.validate(), Err(ConfigValidationErr { field: "gf", reason: GF_RANGE_ERR_MSG }));
        }
    }

    #[test]
    fn test_gf_order() {
        let config = BuehlmannConfig { gf: (90, 80) };
        assert_eq!(config.validate(), Err(ConfigValidationErr { field: "gf", reason: GF_ORDER_ERR_MSG }));
    }

    #[test]
    fn test_temporal_uniform_gf_check() {
        let config = BuehlmannConfig { gf: (30, 70) };
        assert_eq!(config.validate(), Err(ConfigValidationErr { field: "gf", reason: "Currently only uniform gradient factors supported"}));
    }
}
