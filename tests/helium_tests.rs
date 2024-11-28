use dive_deco::{DecoModel, Depth, Gas, RecordData, Supersaturation, Time, Unit};
pub mod fixtures;

#[test]
fn test_tmx_gf_surf() {
    let mut model = fixtures::model_gf((100, 100));

    let tmx = Gas::new(0.21, 0.35);

    model.record(Depth::from_meters(30.), Time::from_minutes(300.), &tmx);

    let Supersaturation { gf_surf, .. } = model.supersaturation();

    assert_close_to_percent!(gf_surf, 335.77, 1.);
}

#[test]
fn test_tmx_ndl() {
    let mut model = fixtures::model_gf((30, 70));

    let tmx = Gas::new(0.21, 0.35);

    model.record(Depth::from_meters(20.), Time::zero(), &tmx);

    assert_eq!(model.ndl(), Time::from_minutes(17.));
}

// heliox
#[test]
fn test_heliox_gf_surf() {
    let mut model = fixtures::model_gf((100, 100));
    let tmx = Gas::new(0.21, 0.79);
    model.record(Depth::from_meters(30.), Time::from_minutes(40.), &tmx);

    let Supersaturation { gf_surf, .. } = model.supersaturation();

    assert_close_to_percent!(gf_surf, 197.93, 1.);
}
