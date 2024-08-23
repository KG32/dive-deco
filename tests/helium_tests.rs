use dive_deco::{ DecoModel, Gas, RecordData, Supersaturation, };
pub mod fixtures;

#[test]
fn test_tmx_gf_surf() {
    let mut model = fixtures::model_gf((100, 100));

    let tmx = Gas::new(0.21, 0.35);

    let record = RecordData { depth: 30., time: (300 * 60), gas: &tmx };
    model.record(record.depth, record.time, record.gas);

    let Supersaturation { gf_surf, .. } = model.supersaturation();

    assert_close_to_percent!(gf_surf, 335.77, 1.);
}

#[test]
fn test_tmx_ndl() {
    let mut model = fixtures::model_gf((30, 70));

    let tmx = Gas::new(0.21, 0.35);

    let record = RecordData { depth: 20., time: 0, gas: &tmx };
    model.record(record.depth, record.time, record.gas);

    assert_eq!(model.ndl(), 17);
}


// heliox
#[test]
fn test_heliox_gf_surf() {
    let mut model = fixtures::model_gf((100, 100));
    let tmx = Gas::new(0.21, 0.79);

    let record = RecordData { depth: 30., time: (40 * 60), gas: &tmx };

    model.record(record.depth, record.time, record.gas);

    let Supersaturation { gf_surf, .. } = model.supersaturation();

    assert_close_to_percent!(gf_surf, 197.93, 1.);
}
