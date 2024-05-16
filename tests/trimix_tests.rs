use dive_deco::{ DecoModel, Gas, StepData, };
pub mod fixtures;

#[test]
fn test_tmx_gf_surf() {
    let mut model = fixtures::model_gf((100, 100));

    let tmx = Gas::new(0.21, 0.35);

    let step = StepData { depth: &50., time: &(5 * 60), gas: &tmx };
    model.step(step.depth, step.time, step.gas);

    close_to_percent!(model.gfs_current().1, 104., 5.);
}
