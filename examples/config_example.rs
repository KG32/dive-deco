use dive_deco::{ BuehlmannConfig, BuehlmannModel, DecoModel };

fn main() {
    // model with default config (GF 100/100)
    let default_config = BuehlmannConfig::default();
    let model_1 = BuehlmannModel::new(default_config);
    println!("{:?}", model_1.config()); // BuehlmannConfig { gf: (100, 100) }

    // model with config instance
    let config_instance = BuehlmannConfig {
        gf: (85, 85),
        surface_pressure: 1013,
    };
    let model_2 = BuehlmannModel::new(config_instance);
    println!("{:?}", model_2.config());


    // model with fluent-interface-like config
    let config_with_gf = BuehlmannConfig::new().gradient_factors(70, 70);
    let model_3 = BuehlmannModel::new(config_with_gf);
    println!("{:?}", model_3.config()); // BuehlmannConfig { gf: (70, 70) }
}
