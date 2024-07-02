use dive_deco::{ BuehlmannModel, BuehlmannConfig, DecoModel, Gas, GradientFactors };

pub fn model_default() -> BuehlmannModel {
    BuehlmannModel::default()
}

pub fn model_gf(gf: GradientFactors) -> BuehlmannModel {
    let (gf_low, gf_high) = gf;
    let config_with_gf = BuehlmannConfig::new().gradient_factors(gf_low, gf_high);
    BuehlmannModel::new(config_with_gf)
}

pub fn gas_air() -> Gas {
    Gas::new(0.21, 0.)
}

#[macro_export]
macro_rules! assert_close_to_abs {
    ($a:expr, $b:expr, $tolerance:expr) => {
        if ($a - $b).abs() > $tolerance {
            panic!("{} is not close to {} with tolerance of {}", $a, $b, $tolerance);
        }
    };
}

#[macro_export]
macro_rules! assert_close_to_percent {
    ($a:expr, $b:expr, $tolerance_percent:expr) => {
        let tolerance = $b * ($tolerance_percent / 100.0);
        if ($a - $b).abs() > tolerance {
            panic!("{} is not close to {} within {} percent tolerance ({})", $a, $b, $tolerance_percent, tolerance);
        }
    };
}
