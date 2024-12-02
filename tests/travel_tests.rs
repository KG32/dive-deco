use dive_deco::{DecoModel, Depth, Supersaturation, Time};
pub mod fixtures;

#[test]
fn travel_descent() {
    let mut model = fixtures::model_default();
    let target_depth = Depth::from_meters(40.);
    let descent_time = Time::from_minutes(10.);
    model.record_travel(target_depth, descent_time, &fixtures::gas_air());
    let dive_state = model.dive_state();
    let Supersaturation { gf_surf, .. } = model.supersaturation();
    assert_eq!(dive_state.depth, target_depth);
    assert_eq!(dive_state.time, descent_time);
    assert_close_to_percent!(gf_surf, 62., 5.);
}

#[test]
fn travel_ascent() {
    let mut model = fixtures::model_gf((30, 70));
    let air = fixtures::gas_air();
    let initial_depth = Depth::from_meters(40.);
    let bottom_time = Time::from_minutes(20.);
    model.record(initial_depth, bottom_time, &air);

    let target_depth = Depth::from_meters(15.);
    let ascent_time = Time::from_minutes(1.5);
    model.record_travel(target_depth, ascent_time, &air);

    let dive_state = model.dive_state();
    let Supersaturation { gf_99, gf_surf } = model.supersaturation();
    assert_eq!(dive_state.depth, target_depth);
    assert_eq!(dive_state.time, bottom_time + ascent_time);
    assert_close_to_percent!(gf_99, 31., 10.);
    assert_close_to_percent!(gf_surf, 150., 10.);
}

#[test]
#[should_panic]
fn travel_invalid_target_depth() {
    let mut model = fixtures::model_gf((30, 70));
    model.record_travel(
        Depth::from_meters(-10.),
        Time::from_seconds(1.),
        &fixtures::gas_air(),
    );
}

#[test]
fn test_travel_record_with_rate() {
    let mut model = fixtures::model_default();
    let air = fixtures::gas_air();
    let initial_depth = Depth::from_meters(20.);
    let bottom_time = Time::from_minutes(20.);
    let target_depth = Depth::zero();
    let expected_travel_time = Time::from_seconds(133.);
    let travel_rate = 9.;
    model.record(initial_depth, bottom_time, &air);
    model.record_travel_with_rate(target_depth, travel_rate, &air);

    let state = model.dive_state();
    assert_eq!(state.depth, target_depth);
    assert_eq!(state.time, bottom_time + expected_travel_time);
    assert_close_to_percent!(model.supersaturation().gf_99, 61., 5.);
}
