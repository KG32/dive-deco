use dive_deco::{DecoModel, DecoRuntime, Gas};

pub mod fixtures;

#[test]
fn runtime_ascent_no_deco() {
    let air = fixtures::gas_air();
    let mut model = fixtures::model_default();
    model.step(&20., &(5 * 60), &air);

    let DecoRuntime { deco_events: runtime_events, tts } = model.runtime(vec![air]);
    assert_eq!(runtime_events.len(), 1); // single continuous ascent
    assert_eq!(tts / 60, 2); // tts in minutes
}

#[test]
fn deco_runtime_single_gas() {
    let air = fixtures::gas_air();
    let mut model = fixtures::model_default();
    model.step(&40., &(20 * 60), &air);

    let DecoRuntime {
        deco_events,
        tts
    } = model.runtime(vec![air]);

    assert_close_to_percent!(tts as f64, 800., 1.); // 13.(3) min todo round to 14
    assert_eq!(deco_events.len(), 5);
}

#[test]
fn deco_runtime_multi_gas() {
    let air = Gas::new(0.21, 0.);
    let ean_50 = Gas::new(0.50, 0.);

    let mut model = fixtures::model_default();

    model.step(&40., &(20 * 60), &air);

    let DecoRuntime {
        deco_events,
        tts
    } = model.runtime(vec![air, ean_50]);
    dbg!(deco_events);

    assert_close_to_abs!(tts as f64, 590., 30.);
}
