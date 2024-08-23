use dive_deco::{ BuehlmannConfig, BuehlmannModel, DecoModel, Gas, Minutes, Supersaturation };
pub mod fixtures;

// general high-level model tests

#[test]
fn test_ceiling() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);
    model.step(40., 30 * 60, &air);
    model.step(30., 30 * 60, &air);
    let calculated_ceiling = model.ceiling();
    assert_close_to_percent!(calculated_ceiling, 7.802523739933558, 0.5);
}

#[test]
fn test_gfs() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);

    model.step(50., 20 * 60, &air);
    assert_eq!(model.supersaturation(), Supersaturation { gf_99: 0., gf_surf: 193.8554997961134 });

    model.step(40., 10 * 60, &air);
    assert_eq!(model.supersaturation(), Supersaturation { gf_99: 0., gf_surf: 208.00431699178796 });
}

#[test]
fn test_initial_gfs() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);
    model.step(0., 0, &air);
    let Supersaturation { gf_99, gf_surf } = model.supersaturation();
    assert_eq!(gf_99, 0.);
    assert_eq!(gf_surf, 0.);
}

#[test]
fn test_model_steps_equality() {
    let mut model1 = fixtures::model_default();
    let mut model2 = fixtures::model_default();

    let air = Gas::new(0.21, 0.);
    let test_depth = 50.;
    let test_time_minutes: Minutes = 100;

    model1.step(test_depth, test_time_minutes * 60, &air);

    // step every second
    for _i in 1..=(test_time_minutes * 60) {
        model2.step(test_depth, 1, &air);
    }

    assert_eq!(model1.ceiling().floor(), model2.ceiling().floor());

    let Supersaturation { gf_99: model1_gf_99, gf_surf: model1_gf_surf} = model1.supersaturation();
    let Supersaturation { gf_99: model2_gf_99, gf_surf: model2_gf_surf } = model1.supersaturation();
    assert_eq!(model1_gf_99.floor(), model2_gf_99.floor());
    assert_eq!(model1_gf_surf.floor(), model2_gf_surf.floor());
}

#[test]
fn test_ndl_calculation() {
    let mut model = fixtures::model_default();

    let air = Gas::new(0.21, 0.);
    let depth = 30.;

    // with 21/00 at 30m expect NDL 16
    model.step(depth, 0, &air);
    assert_eq!(model.ndl(), 16);

    // expect NDL 15 after 1 min
    model.step(depth, 60, &air);
    assert_eq!(model.ndl(), 15);
}

#[test]
fn test_ndl_cut_off() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);

    model.step(0., 0, &air);
    assert_eq!(model.ndl(), Minutes::MAX);

    model.step(10., 10*60, &air);
    assert_eq!(model.ndl(), Minutes::MAX);
}

#[test]
fn test_multi_gas_ndl() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);
    let ean_28 = Gas::new(0.28, 0.);

    model.step(30., 0 * 60, &air);
    assert_eq!(model.ndl(), 16);

    model.step(30., 10 * 60, &air);
    assert_eq!(model.ndl(), 6);

    model.step(30., 0 * 60, &ean_28);
    assert_eq!(model.ndl(), 10);
}

#[test]
fn test_ndl_with_gf() {
    let mut model = fixtures::model_gf((70, 70));
    let air = Gas::new(0.21, 0.);
    model.step(20., 0 * 60, &air);
    assert_eq!(model.ndl(), 21);
}

#[test]
fn test_altitude() {
    let mut model = BuehlmannModel::new(BuehlmannConfig::new().surface_pressure(700));
    let air = Gas::new(0.21, 0.);
    model.step(40., 60 * 60, &air);
    let Supersaturation { gf_surf, ..} = model.supersaturation();
    assert_eq!(gf_surf, 299.023204474694);
}

#[test]
fn test_example_deco_start() {
    let mut model = BuehlmannModel::new(
        BuehlmannConfig::new()
            .gradient_factors(30, 70)
            .surface_pressure(1013)
    );

    let air = Gas::air();
    // let ean_50 = Gas::new(0.50, 0.);

    // instant drop to 40m on air for 10min
    model.step(40., 10 * 60, &air);
    assert_eq!(model.ceiling(), 12.85312294790554);
}

#[test]
fn test_example_deco() {
    let mut model = BuehlmannModel::new(
        BuehlmannConfig::new()
            .gradient_factors(30, 70)
            .surface_pressure(1013)
    );

    let air = Gas::air();
    let ean_50 = Gas::new(0.50, 0.);

    model.step(40., 40 * 60, &air);
    model.step(30., 3 * 60, &air);
    model.step(21., 10 * 60, &ean_50);

    assert_eq!(model.ceiling(), 12.455491216740299);
}

#[test]
#[should_panic]
fn test_should_panic_on_invalid_depth() {
    let mut model = fixtures::model_default();
    model.step(-10., 1, &fixtures::gas_air());
}
