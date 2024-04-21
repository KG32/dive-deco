use dive_deco::{ BuehlmannConfig, BuehlmannModel, DecoModel };

fn main() {
    // model with default config (GF 100/100)
    let default_config = BuehlmannConfig::default();
    let model1 = BuehlmannModel::new(default_config);
    println!("{:?}", model1.config()); // BuehlmannConfig { gf: (100, 100) }

    let config_with_gf = BuehlmannConfig { gf: (70, 70) };
    let model2 = BuehlmannModel::new(config_with_gf);
    println!("{:?}", model2.config()); // BuehlmannConfig { gf: (70, 70) }
}
