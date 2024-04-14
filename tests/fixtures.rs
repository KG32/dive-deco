use dive_deco::{ BuehlmannModel, BuehlmannConfig, DecoModel, Gas, Minutes, GradientFactors };

pub fn model_default() -> BuehlmannModel {
    BuehlmannModel::new(BuehlmannConfig::default())
}

pub fn model_gf(gf: GradientFactors) -> BuehlmannModel {
    BuehlmannModel::new(BuehlmannConfig { gf })
}

pub fn gas_air() -> Gas {
    Gas::new(0.21, 0.)
}
