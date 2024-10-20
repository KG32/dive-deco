use dive_deco::{
    BuehlmannConfig, BuehlmannModel, CeilingType, DecoModel, Gas, Minutes, Supersaturation,
};
pub mod fixtures;

// general high-level model tests
#[test]
#[should_panic]
fn test_should_panic_on_invalid_depth() {
    let mut model = fixtures::model_default();
    model.record(-10., 1, &fixtures::gas_air());
}

#[test]
fn test_ceiling() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);
    model.record(40., 30 * 60, &air);
    model.record(30., 30 * 60, &air);
    let calculated_ceiling = model.ceiling();
    assert_close_to_percent!(calculated_ceiling, 7.802523739933558, 0.5);
}

#[test]
fn test_gfs() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);

    model.record(50., 20 * 60, &air);
    assert_eq!(
        model.supersaturation(),
        Supersaturation {
            gf_99: 0.,
            gf_surf: 193.8554997961134
        }
    );

    model.record(40., 10 * 60, &air);
    assert_eq!(
        model.supersaturation(),
        Supersaturation {
            gf_99: 0.,
            gf_surf: 208.00431699178796
        }
    );
}

#[test]
fn test_initial_gfs() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);
    model.record(0., 0, &air);
    let Supersaturation { gf_99, gf_surf } = model.supersaturation();
    assert_eq!(gf_99, 0.);
    assert_eq!(gf_surf, 0.);
}

#[test]
fn test_model_records_equality() {
    let mut model1 = fixtures::model_default();
    let mut model2 = fixtures::model_default();

    let air = Gas::new(0.21, 0.);
    let test_depth = 50.;
    let test_time_minutes: Minutes = 100;

    model1.record(test_depth, test_time_minutes * 60, &air);

    // record every second
    for _i in 1..=(test_time_minutes * 60) {
        model2.record(test_depth, 1, &air);
    }

    assert_eq!(model1.ceiling().floor(), model2.ceiling().floor());

    let Supersaturation {
        gf_99: model1_gf_99,
        gf_surf: model1_gf_surf,
    } = model1.supersaturation();
    let Supersaturation {
        gf_99: model2_gf_99,
        gf_surf: model2_gf_surf,
    } = model1.supersaturation();
    assert_eq!(model1_gf_99.floor(), model2_gf_99.floor());
    assert_eq!(model1_gf_surf.floor(), model2_gf_surf.floor());
}

#[test]
fn test_actual_ndl_calculation() {
    let config = BuehlmannConfig::default().with_ceiling_type(CeilingType::Actual);
    let mut model = BuehlmannModel::new(config);

    let air = Gas::new(0.21, 0.);
    let depth = 30.;

    // with 21/00 at 30m expect NDL 16
    model.record(depth, 0, &air);
    assert_eq!(model.ndl(), 16);

    // expect NDL 15 after 1 min
    model.record(depth, 60, &air);
    assert_eq!(model.ndl(), 15);
}

#[test]
fn test_adaptive_ndl_calculation() {
    let config = BuehlmannConfig::default().with_ceiling_type(CeilingType::Adaptive);
    let mut model = BuehlmannModel::new(config);

    let air = Gas::new(0.21, 0.);
    let depth = 30.;

    // with 21/00 at 30m expect NDL 19
    model.record(depth, 0, &air);
    assert_eq!(model.ndl(), 19);

    // expect NDL 18 after 1 min
    model.record(depth, 60, &air);
    assert_eq!(model.ndl(), 18);
}

#[test]
fn test_ndl_cut_off() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);

    model.record(0., 0, &air);
    assert_eq!(model.ndl(), 99);

    model.record(10., 10 * 60, &air);
    assert_eq!(model.ndl(), 99);
}

#[test]
fn test_multi_gas_ndl() {
    let mut model =
        BuehlmannModel::new(BuehlmannConfig::default().with_ceiling_type(CeilingType::Actual));
    let air = Gas::new(0.21, 0.);
    let ean_28 = Gas::new(0.28, 0.);

    model.record(30., 0, &air);
    assert_eq!(model.ndl(), 16);

    model.record(30., 10 * 60, &air);
    assert_eq!(model.ndl(), 6);

    model.record(30., 0, &ean_28);
    assert_eq!(model.ndl(), 10);
}

#[test]
fn test_ndl_with_gf() {
    let mut model = fixtures::model_gf((70, 70));
    let air = Gas::new(0.21, 0.);
    model.record(20., 0, &air);
    assert_eq!(model.ndl(), 21);
}

#[test]
fn test_altitude() {
    let mut model = BuehlmannModel::new(BuehlmannConfig::new().with_surface_pressure(700));
    let air = Gas::new(0.21, 0.);
    model.record(40., 60 * 60, &air);
    let Supersaturation { gf_surf, .. } = model.supersaturation();
    assert_eq!(gf_surf, 299.023204474694);
}

#[test]
fn test_example_ceiling_start() {
    let mut model = BuehlmannModel::new(
        BuehlmannConfig::new()
            .with_gradient_factors(30, 70)
            .with_surface_pressure(1013),
    );

    let air = Gas::air();
    // let ean_50 = Gas::new(0.50, 0.);

    // instant drop to 40m on air for 10min
    model.record(40., 10 * 60, &air);
    assert_eq!(model.ceiling(), 12.85312294790554);
}

#[test]
fn test_example_ceiling() {
    let mut model = BuehlmannModel::new(
        BuehlmannConfig::new()
            .with_gradient_factors(30, 70)
            .with_surface_pressure(1013),
    );

    let air = Gas::air();
    let ean_50 = Gas::new(0.50, 0.);

    model.record(40., 40 * 60, &air);
    model.record(30., 3 * 60, &air);
    model.record(21., 10 * 60, &ean_50);

    assert_eq!(model.ceiling(), 12.455491216740299);
}

#[test]
fn test_adaptive_ceiling() {
    let mut model = BuehlmannModel::new(
        BuehlmannConfig::new().with_ceiling_type(dive_deco::CeilingType::Adaptive),
    );
    let air = Gas::air();
    model.record(40., 20 * 60, &air);
    let ceiling = model.ceiling();
    assert_close_to_abs!(ceiling, 4., 0.5);
}

#[test]
fn test_gradual_ascent_with_deco() {
    let mut model = BuehlmannModel::new(
        BuehlmannConfig::new()
            .with_gradient_factors(30, 70)
            .with_surface_pressure(1013),
    );
    let air = Gas::air();
    let ean50 = Gas::new(0.21, 0.50);
    model.record(45., 30 * 60, &air);
    loop {
        let depth = model.dive_state().depth;
        if depth <= 0. {
            break;
        }
        model.record_travel_with_rate(depth - 3., 10., &air);
        model.deco(vec![air, ean50]).unwrap();
    }
}

#[test]
fn test_cns_otu() {
    let mut model = BuehlmannModel::default();
    model.record(40., 10 * 60, &Gas::air());
    model.record_travel_with_rate(0., 10., &Gas::air());
    assert_close_to_abs!(model.cns(), 4., 1.);
    assert_close_to_abs!(model.otu(), 13., 1.);
}
