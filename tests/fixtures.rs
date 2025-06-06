use dive_deco::{BuhlmannConfig, BuhlmannModel, DecoModel, Gas, GradientFactors};

pub fn model_default() -> BuhlmannModel {
    BuhlmannModel::default()
}

pub fn model_gf(gf: GradientFactors) -> BuhlmannModel {
    let (gf_low, gf_high) = gf;
    let config_with_gf = BuhlmannConfig::new().with_gradient_factors(gf_low, gf_high);
    BuhlmannModel::new(config_with_gf)
}

pub fn gas_air() -> Gas {
    Gas::new(0.21, 0.)
}

#[macro_export]
macro_rules! assert_close_to_abs {
    ($a:expr, $b:expr, $tolerance:expr) => {
        if ($a - $b).abs() > $tolerance {
            panic!(
                "{} is not close to {} with tolerance of {}",
                $a, $b, $tolerance
            );
        }
    };
}

#[macro_export]
macro_rules! assert_close_to_percent {
    ($a:expr, $b:expr, $tolerance_percent:expr) => {
        let tolerance = $b * ($tolerance_percent / 100.0);
        if ($a - $b).abs() > tolerance {
            panic!(
                "{} is not close to {} within {} percent tolerance ({})",
                $a, $b, $tolerance_percent, tolerance
            );
        }
    };
}
