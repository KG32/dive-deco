use dive_deco::{ BuehlmannConfig, BuehlmannModel, CeilingType, DecoModel, NDLType };

fn main() {
    // model with default config (GF 100/100)
    let default_config = BuehlmannConfig::default();
    let model_1 = BuehlmannModel::new(default_config);
    println!("{:?}", model_1.config()); // BuehlmannConfig { gf: (100, 100) }

    // model with full config instance
    let config_instance = BuehlmannConfig {
        gf: (85, 85),
        surface_pressure: 1013,
        deco_ascent_rate: 9.,
        ceiling_type: CeilingType::Actual,
        round_ceiling: false,
    };
    let model_2 = BuehlmannModel::new(config_instance);
    println!("{:?}", model_2.config());


    // model with fluent-interface-like config
    let config_with_gf = BuehlmannConfig::default().gradient_factors(30, 70);
    let model_3 = BuehlmannModel::new(config_with_gf);
    println!("{:?}", model_3.config()); // BuehlmannConfig { gf: (30, 70) }
}
