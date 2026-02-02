use dive_deco::{BreathingSource, DecoModel, Depth, Gas, Time};

pub mod fixtures;

#[test]
fn test_cns() {
    let mut model = fixtures::model_default();

    let nitrox = BreathingSource::OpenCircuit(Gas::new(0.32, 0.));

    model.record(Depth::from_meters(20.), Time::from_minutes(40.), &nitrox);
    model.record_travel_with_rate(Depth::zero(), 9., &nitrox);

    let cns = model.cns();

    assert_close_to_abs!(cns as f64, 12., 1.);
}

#[test]
fn test_cns_multi_stage() {
    let mut model = fixtures::model_default();
    let nitrox = BreathingSource::OpenCircuit(Gas::new(0.32, 0.));
    let air = BreathingSource::OpenCircuit(Gas::air());

    model.record_travel_with_rate(Depth::from_meters(36.58), 12.19, &nitrox);
    model.record(Depth::from_meters(36.58), Time::from_minutes(22.), &nitrox);
    model.record_travel_with_rate(Depth::zero(), 1.22, &nitrox);
    model.record(Depth::zero(), Time::from_minutes(10.), &air);
    let cns = model.cns();
    assert_close_to_abs!(cns, 25.9, 0.1);
}
