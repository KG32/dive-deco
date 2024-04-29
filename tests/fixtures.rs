use dive_deco::{ BuehlmannModel, BuehlmannConfig, DecoModel, Gas, GradientFactors };

pub fn model_default() -> BuehlmannModel {
    BuehlmannModel::new(BuehlmannConfig::default())
}

pub fn model_gf(gf: GradientFactors) -> BuehlmannModel {
    let (gf_low, gf_high) = gf;
    let config_with_gf = BuehlmannConfig::new().gradient_factors(gf_low, gf_high);
    BuehlmannModel::new(config_with_gf)
}

pub fn gas_air() -> Gas {
    Gas::new(0.21, 0.)
}
