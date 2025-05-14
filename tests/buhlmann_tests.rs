use dive_deco::{
    BuhlmannConfig, BuhlmannModel, CeilingType, DecoModel, Depth, Gas, Supersaturation, Time,
};
pub mod fixtures;

// general high-level model tests
#[test]
#[should_panic]
fn test_should_panic_on_invalid_depth() {
    let mut model = fixtures::model_default();
    model.record(
        Depth::from_meters(-10.),
        Time::from_seconds(1.),
        &fixtures::gas_air(),
    );
}

#[test]
fn test_ceiling() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);
    model.record(Depth::from_meters(40.), Time::from_minutes(30.), &air);
    model.record(Depth::from_meters(30.), Time::from_minutes(30.), &air);
    let calculated_ceiling = model.ceiling();
    assert_close_to_percent!(
        calculated_ceiling.as_meters(),
        Depth::from_meters(7.802523739933558).as_meters(),
        0.5
    );
}

#[test]
fn test_gfs() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);

    model.record(Depth::from_meters(50.), Time::from_minutes(20.), &air);
    assert_eq!(
        model.supersaturation(),
        Supersaturation {
            gf_99: 0.,
            gf_surf: 193.8554997961134
        }
    );

    model.record(Depth::from_meters(40.), Time::from_minutes(10.), &air);
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
    model.record(Depth::from_meters(0.), Time::zero(), &air);
    let Supersaturation { gf_99, gf_surf } = model.supersaturation();
    assert_eq!(gf_99, 0.);
    assert_eq!(gf_surf, 0.);
}

#[test]
fn test_model_records_equality() {
    let mut model1 = fixtures::model_default();
    let mut model2 = fixtures::model_default();

    let air = Gas::new(0.21, 0.);
    let test_depth = Depth::from_meters(50.);
    let test_time = Time::from_minutes(100.);

    model1.record(test_depth, test_time, &air);

    // record every second
    for _i in 1..=test_time.as_seconds() as i32 {
        model2.record(test_depth, Time::from_seconds(1.), &air);
    }

    assert_eq!(
        model1.ceiling().as_meters().floor(),
        model2.ceiling().as_meters().floor()
    );

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
    let config = BuhlmannConfig::default().with_ceiling_type(CeilingType::Actual);
    let mut model = BuhlmannModel::new(config);

    let air = Gas::new(0.21, 0.);
    let depth = Depth::from_meters(30.);

    // with 21/00 at 30m expect NDL 16
    model.record(depth, Time::zero(), &air);
    assert_eq!(model.ndl(), Time::from_minutes(16.));

    // expect NDL 15 after 1 min
    model.record(depth, Time::from_minutes(1.), &air);
    assert_eq!(model.ndl(), Time::from_minutes(15.));
}

#[test]
fn test_adaptive_ndl_calculation() {
    let config = BuhlmannConfig::default().with_ceiling_type(CeilingType::Adaptive);
    let mut model = BuhlmannModel::new(config);

    let air = Gas::new(0.21, 0.);
    let depth = Depth::from_meters(30.);

    // with 21/00 at 30m expect NDL 19
    model.record(depth, Time::zero(), &air);
    assert_eq!(model.ndl(), Time::from_minutes(19.));

    // expect NDL 18 after 1 min
    model.record(depth, Time::from_minutes(1.), &air);
    assert_eq!(model.ndl(), Time::from_minutes(18.));
}

#[test]
fn test_ndl_cut_off() {
    let mut model = fixtures::model_default();
    let air = Gas::new(0.21, 0.);

    model.record(Depth::from_meters(0.), Time::zero(), &air);
    assert_eq!(model.ndl(), Time::from_minutes(99.));

    model.record(Depth::from_meters(10.), Time::from_minutes(10.), &air);
    assert_eq!(model.ndl(), Time::from_minutes(99.));
}

#[test]
fn test_multi_gas_ndl() {
    let mut model =
        BuhlmannModel::new(BuhlmannConfig::default().with_ceiling_type(CeilingType::Actual));
    let air = Gas::new(0.21, 0.);
    let ean_28 = Gas::new(0.28, 0.);

    model.record(Depth::from_meters(30.), Time::zero(), &air);
    assert_eq!(model.ndl(), Time::from_minutes(16.));

    model.record(Depth::from_meters(30.), Time::from_minutes(10.), &air);
    assert_eq!(model.ndl(), Time::from_minutes(6.));

    model.record(Depth::from_meters(30.), Time::zero(), &ean_28);
    assert_eq!(model.ndl(), Time::from_minutes(10.));
}

#[test]
fn test_ndl_with_gf() {
    let mut model = fixtures::model_gf((70, 70));
    let air = Gas::new(0.21, 0.);
    model.record(Depth::from_meters(20.), Time::zero(), &air);
    assert_eq!(model.ndl(), Time::from_minutes(21.));
}

#[test]
fn test_altitude() {
    let mut model = BuhlmannModel::new(BuhlmannConfig::new().with_surface_pressure(700));
    let air = Gas::new(0.21, 0.);
    model.record(Depth::from_meters(40.), Time::from_minutes(60.), &air);
    let Supersaturation { gf_surf, .. } = model.supersaturation();
    assert_eq!(gf_surf, 299.023204474694);
}

#[test]
fn test_example_ceiling_start() {
    let mut model = BuhlmannModel::new(
        BuhlmannConfig::new()
            .with_gradient_factors(30, 70)
            .with_surface_pressure(1013),
    );

    let air = Gas::air();

    // instant drop to 40m on air for 10min
    model.record(Depth::from_meters(40.), Time::from_minutes(10.), &air);
    assert_eq!(model.ceiling().as_meters(), 12.85312294790554);
}

#[test]
fn test_example_ceiling() {
    let mut model = BuhlmannModel::new(
        BuhlmannConfig::new()
            .with_gradient_factors(30, 70)
            .with_surface_pressure(1013),
    );

    let air = Gas::air();
    let ean_50 = Gas::new(0.50, 0.);

    model.record(Depth::from_meters(40.), Time::from_minutes(40.), &air);
    model.record(Depth::from_meters(30.), Time::from_minutes(3.), &air);
    model.record(Depth::from_meters(21.), Time::from_minutes(10.), &ean_50);
    assert_eq!(model.ceiling().as_meters(), 12.455491216740299);
}

#[test]
fn test_example_ceiling_feet() {
    let mut model = BuhlmannModel::new(
        BuhlmannConfig::new()
            .with_gradient_factors(30, 70)
            .with_surface_pressure(1013),
    );

    let air = Gas::air();
    let ean_50 = Gas::new(0.50, 0.);

    model.record(Depth::from_feet(131.234), Time::from_minutes(40.), &air);
    model.record(Depth::from_feet(98.4252), Time::from_minutes(3.), &air);
    model.record(Depth::from_feet(68.8976), Time::from_minutes(10.), &ean_50);
    assert_eq!(model.ceiling().as_feet(), 40.864609154666);
    assert_eq!(model.ceiling().as_meters(), 12.455532471765158);
}

#[test]
fn test_adaptive_ceiling() {
    let mut model = BuhlmannModel::new(
        BuhlmannConfig::new().with_ceiling_type(dive_deco::CeilingType::Adaptive),
    );
    let air = Gas::air();
    model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
    let ceiling = model.ceiling();
    assert_close_to_abs!(ceiling.as_meters(), 4., 0.5);
}

#[test]
fn test_gradual_ascent_with_deco() {
    let mut model = BuhlmannModel::new(
        BuhlmannConfig::new()
            .with_gradient_factors(30, 70)
            .with_surface_pressure(1013),
    );
    let air = Gas::air();
    let ean_50 = Gas::new(0.50, 0.);
    model.record(Depth::from_meters(45.), Time::from_minutes(30.), &air);
    loop {
        let depth = model.dive_state().depth;
        if depth <= Depth::zero() {
            break;
        }
        model.record_travel_with_rate(depth - Depth::from_meters(3.), 10., &air);
        model.deco(vec![air, ean_50]).unwrap();
    }
}

#[test]
fn test_cns_otu() {
    let mut model = BuhlmannModel::default();
    model.record(
        Depth::from_meters(40.),
        Time::from_minutes(10.),
        &Gas::air(),
    );
    model.record_travel_with_rate(Depth::from_meters(0.), 10., &Gas::air());
    assert_close_to_abs!(model.otu(), 13., 1.);
}
