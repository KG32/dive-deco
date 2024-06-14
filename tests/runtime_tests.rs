use dive_deco::{DecoModel, Runtime};

pub mod fixtures;

#[test]
fn runtime_ascent_no_deco() {
    let air = fixtures::gas_air();
    let mut model = fixtures::model_default();
    model.step(&20., &(5 * 60), &air);

    let Runtime { runtime_events, tts } = model.runtime(vec![&air]);
    assert_eq!(runtime_events.len(), 1); // single continuous ascent
    assert_eq!(tts / 60, 2); // tts in minutes
}
