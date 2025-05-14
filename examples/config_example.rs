use dive_deco::{BuhlmannConfig, BuhlmannModel, CeilingType, DecoModel};

fn main() {
    // model with default config (GF 100/100)
    let default_config = BuhlmannConfig::default();
    let model_1 = BuhlmannModel::new(default_config);
    println!("{:?}", model_1.config()); // BuhlmannConfig { gf: (100, 100) }

    // model with full config instance
    let config_instance = BuhlmannConfig {
        gf: (85, 85),
        surface_pressure: 1013,
        deco_ascent_rate: 9.,
        ceiling_type: CeilingType::Actual,
        round_ceiling: false,
        recalc_all_tissues_m_values: true,
    };
    let model_2 = BuhlmannModel::new(config_instance);
    println!("{:?}", model_2.config());

    // model with fluent-interface-like config
    let config_with_gf = BuhlmannConfig::default().with_gradient_factors(30, 70);
    let model_3 = BuhlmannModel::new(config_with_gf);
    println!("{:?}", model_3.config()); // BuhlmannConfig { gf: (30, 70) }
}
