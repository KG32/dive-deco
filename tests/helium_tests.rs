use dive_deco::{ DecoModel, Gas, StepData, Supersaturation, };
pub mod fixtures;

#[test]
fn test_tmx_gf_surf() {
    let mut model = fixtures::model_gf((100, 100));

    let tmx = Gas::new(0.21, 0.35);

    let step = StepData { depth: &30., time: &(300 * 60), gas: &tmx };
    model.step(step.depth, step.time, step.gas);

    let Supersaturation { gf_surf, .. } = model.supersaturation();

    assert_close_to_percent!(gf_surf, 335.77, 0.5);
}

#[test]
fn test_tmx_ndl() {
    let mut model = fixtures::model_gf((30, 70));

    let tmx = Gas::new(0.21, 0.35);

    let step = StepData { depth: &20., time: &0, gas: &tmx };
    model.step(step.depth, step.time, step.gas);

    assert_eq!(model.ndl(), 16);
}


// heliox
#[test]
fn test_heliox_gf_surf() {
    let mut model = fixtures::model_gf((100, 100));
    let tmx = Gas::new(0.21, 0.79);

    let step = StepData { depth: &30., time: &(40 * 60), gas: &tmx };

    model.step(step.depth, step.time, step.gas);

    dbg!(&model);

    let Supersaturation { gf_surf, .. } = model.supersaturation();

    assert_close_to_percent!(gf_surf, 201.16, 0.5);
}
