use dive_deco::{DecoModel, Depth, Gas, Unit};

pub mod fixtures;

#[test]
fn test_cns() {
    let mut model = fixtures::model_default();

    let nitrox = Gas::new(0.32, 0.);

    model.record(Depth::from_metric(20.), 40 * 60, &nitrox);
    model.record_travel_with_rate(Depth::zero(), 9., &nitrox);

    let cns = model.cns();

    assert_close_to_abs!(cns as f64, 12., 1.);
}

#[test]
fn test_cns_multi_stage() {
    let mut model = fixtures::model_default();
    let nitrox = Gas::new(0.32, 0.);

    model.record_travel_with_rate(Depth::from_metric(36.58), 12.19, &nitrox);
    model.record(Depth::from_metric(36.58), 22 * 60, &nitrox);
    model.record_travel_with_rate(Depth::zero(), 1.22, &nitrox);
    model.record(Depth::zero(), 10 * 60, &Gas::air());
    let cns = model.cns();
    assert_close_to_abs!(cns, 27.5, 1.);
}
