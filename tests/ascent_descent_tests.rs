use dive_deco::{ BuehlmannConfig, BuehlmannModel, DecoModel, Gas, Minutes, Supersaturation };
pub mod fixtures;

#[test]
fn saturation_on_descent() {
    let mut model = fixtures::model_default();
    model.step_travel(&40., &(10 * 60), &fixtures::gas_air());

    let Supersaturation { gf_surf, .. } = model.supersaturation();
    assert_close_to_percent!(gf_surf, 62., 10.);
}
