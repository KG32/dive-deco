use dive_deco::{ DecoModel, Gas };
pub mod fixtures;

#[ignore = "wip"]
#[test]
fn test_tmx_gf_surf() {
    let mut model = fixtures::model_gf((100, 100));

    let gas = Gas::new(0.21, 0.35);

    model.step(&50., &(5 * 60), &gas);

    close_to_percent!(model.gfs_current().1, 104., 5.);
}
