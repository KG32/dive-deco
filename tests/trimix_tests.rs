use dive_deco::{ DecoModel, Gas, StepData, };
pub mod fixtures;

#[test]
fn test_tmx_gf_surf() {
    let mut model = fixtures::model_gf((100, 100));

    let tmx = Gas::new(0.21, 0.35);

    let step = StepData { depth: &30., time: &(300 * 60), gas: &tmx };
    model.step(step.depth, step.time, step.gas);

    close_to_percent!(model.gfs_current().1, 337., 5.);
}

    #[test]
    fn test_tmx_ndl() {
        let mut model = fixtures::model_gf((30, 70));

        let tmx = Gas::new(0.21, 0.35);

        let step = StepData { depth: &20., time: &0, gas: &tmx };
        model.step(step.depth, step.time, step.gas);

        assert_eq!(model.ndl(), 17);
    }
