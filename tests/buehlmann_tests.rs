use dive_deco::{ BuehlmannConfig, BuehlmannModel, DecoModel, Gas, Minutes };
pub mod fixtures;

// general high-level model tests

#[test]
fn test_ceiling() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);
    model.step(&40., &(30 * 60), &air);
    model.step(&30., &(30 * 60), &air);
    let calculated_ceiling = model.ceiling();
    assert_eq!(calculated_ceiling, 7.802523739933558);
}

#[test]
fn test_gfs() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);

    model.step(&50., &(20 * 60), &air);
    assert_eq!(model.gfs_current(), (0., 194.3055827400852));

    model.step(&40., &(10 * 60), &air);
    assert_eq!(model.gfs_current(), (0., 209.1025059770294));
}

#[test]
fn test_initial_gfs() {
    let model = fixtures::model_default();
    let (gf_now, gf_surf) = model.gfs_current();
    assert_eq!(gf_now, 0.);
    assert_eq!(gf_surf, 0.);
}

#[test]
fn test_model_steps_equality() {
    let mut model1 = fixtures::model_default();
    let mut model2 = fixtures::model_default();

    let air = Gas::new(0.21, 0.);
    let test_depth = 50.;
    let test_time_minutes: usize = 100;

    model1.step(&test_depth, &(test_time_minutes * 60), &air);

    // step every second
    for _i in 1..=(test_time_minutes * 60) {
        model2.step(&test_depth, &1, &air);
    }

    assert_eq!(model1.ceiling().floor(), model2.ceiling().floor());

    let (model1_gf_now, model1_gf_surf) = model1.gfs_current();
    let (model2_gf_now, model2_gf_surf) = model1.gfs_current();
    assert_eq!(model1_gf_now.floor(), model2_gf_now.floor());
    assert_eq!(model1_gf_surf.floor(), model2_gf_surf.floor());
}

#[test]
fn test_ndl_calculation() {
    let mut model = fixtures::model_default();

    let air = Gas::new(0.21, 0.);
    let depth = 30.;

    // with 21/00 at 30m expect NDL 16
    model.step(&depth, &0, &air);
    assert_eq!(model.ndl(), 16);

    // expect NDL 15 after 1 min
    model.step(&depth, &(1*60), &air);
    assert_eq!(model.ndl(), 15);
}

#[test]
fn test_ndl_cut_off() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);

    model.step(&0., &0, &air);
    assert_eq!(model.ndl(), Minutes::MAX);

    model.step(&10., &(10*60), &air);
    assert_eq!(model.ndl(), Minutes::MAX);
}

#[test]
fn test_multi_gas_ndl() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);
    let ean_28 = Gas::new(0.28, 0.);

    model.step(&30., &(0 * 60), &air);
    assert_eq!(model.ndl(), 16);

    model.step(&30., &(10 * 60), &air);
    assert_eq!(model.ndl(), 6);

    model.step(&30., &(0 * 60), &ean_28);
    assert_eq!(model.ndl(), 10);
}

#[test]
fn test_ndl_with_gf() {
    let mut model = fixtures::model_gf((70, 70));
    let air = Gas::new(0.21, 0.);
    model.step(&20., &(0 * 60), &air);
    assert_eq!(model.ndl(), 21);
}

#[test]
fn test_altitude() {
    let mut model = BuehlmannModel::new(BuehlmannConfig::new().surface_pressure(700));
    model.step(&40., &(60 * 60), &fixtures::gas_air());
    assert_eq!(model.gfs_current().1, 314.27462637570085);
}

#[test]
fn test_example_deco() {
    let mut model = BuehlmannModel::new(
        BuehlmannConfig::new()
            .gradient_factors(30, 70)
            .surface_pressure(1013)
    );

    let air = Gas::air();
    // let ean_50 = Gas::new(0.50, 0.);

    // instant drop to 40m on air for 10min
    model.step(&40., &(10 * 60), &air);
    assert_eq!(model.ceiling(), 15.);
}
