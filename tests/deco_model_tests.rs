use dive_deco::{DecoModel, Depth, Gas, Time};

pub mod fixtures;

#[test]
fn test_cns() {
    let mut model = fixtures::model_default();

    let nitrox = Gas::new(0.32, 0.);

    model.record(Depth::from_meters(20.), Time::from_minutes(40.), &nitrox);
    model.record_travel_with_rate(Depth::zero(), 9., &nitrox);

    let cns = model.cns();

    assert_close_to_abs!(cns as f64, 12., 1.);
}

#[test]
fn test_cns_multi_stage() {
    let mut model = fixtures::model_default();
    let nitrox = Gas::new(0.32, 0.);

    model.record_travel_with_rate(Depth::from_meters(36.58), 12.19, &nitrox);
    model.record(Depth::from_meters(36.58), Time::from_minutes(22.), &nitrox);
    model.record_travel_with_rate(Depth::zero(), 1.22, &nitrox);
    model.record(Depth::zero(), Time::from_minutes(10.), &Gas::air());
    let cns = model.cns();
    assert_close_to_abs!(cns, 26., 1.);
}

#[test]
fn test_record_surface_interval() {
    let air = Gas::air();
    let trimix = Gas::new(0.21, 0.35);
    let nitrox = Gas::new(0.32, 0.);
    let surface_interval = Time::from_minutes(60.);
    let mut model = fixtures::model_default();
    let mut expected_model = fixtures::model_default();

    for dive_point in [
        (Depth::from_meters(30.), Time::from_minutes(20.), &trimix),
        (Depth::from_meters(18.), Time::from_minutes(15.), &nitrox),
    ] {
        model.record(dive_point.0, dive_point.1, dive_point.2);
        expected_model.record(dive_point.0, dive_point.1, dive_point.2);
    }

    model.record_travel_with_rate(Depth::zero(), 9., &nitrox);
    expected_model.record_travel_with_rate(Depth::zero(), 9., &nitrox);
    model
        .record_surface_interval(surface_interval)
        .expect("surface interval should be recorded");
    expected_model.record(Depth::zero(), surface_interval, &air);

    assert_eq!(model.dive_state().depth, Depth::zero());
    assert_eq!(model.dive_state().time, expected_model.dive_state().time);
    assert_eq!(model.ceiling(), expected_model.ceiling());
    assert_eq!(model.ndl(), expected_model.ndl());
}

#[test]
fn test_record_surface_interval_fails_below_surface() {
    let air = Gas::air();
    let mut model = fixtures::model_default();

    model.record(Depth::from_meters(18.), Time::from_minutes(15.), &air);
    let depth = model.dive_state().depth;

    let error = model
        .record_surface_interval(Time::from_minutes(60.))
        .expect_err("surface interval should fail below the surface");

    assert_eq!(
        error,
        format!(
            "Unable to record surface interval at depth ({}m / {}ft)",
            depth.as_meters(),
            depth.as_feet()
        )
    );
}
