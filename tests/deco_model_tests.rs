use dive_deco::{DecoModel, Depth, Gas, Time, Unit};

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
    assert_close_to_abs!(cns, 27.5, 1.);
}
