use dive_deco::{
    BreathingSource, BuhlmannConfig, BuhlmannModel, CeilingType, Deco, DecoModel, Depth, Gas,
    Supersaturation, Time,
};
pub mod fixtures;

// general high-level model tests
#[test]
#[should_panic]
fn test_should_panic_on_invalid_depth() {
    let mut model = fixtures::model_default();
    model.record(
        Depth::from_meters(-10.),
        Time::from_seconds(1.),
        &BreathingSource::OpenCircuit(fixtures::gas_air()),
    );
}

#[test]
fn test_ceiling() {
    let mut model = fixtures::model_default();
    let air = BreathingSource::OpenCircuit(Gas::new(0.21, 0.));
    model.record(Depth::from_meters(40.), Time::from_minutes(30.), &air);
    model.record(Depth::from_meters(30.), Time::from_minutes(30.), &air);
    let calculated_ceiling = model.ceiling();
    assert_close_to_percent!(
        calculated_ceiling.as_meters(),
        Depth::from_meters(7.871645603737522).as_meters(),
        0.5
    );
}

#[test]
fn test_gfs() {
    let mut model = fixtures::model_default();
    let air = BreathingSource::OpenCircuit(Gas::new(0.21, 0.));

    model.record(Depth::from_meters(50.), Time::from_minutes(20.), &air);
    assert_eq!(
        model.supersaturation(),
        Supersaturation {
            gf_99: 0.,
            gf_surf: 195.98203494813478
        }
    );

    model.record(Depth::from_meters(40.), Time::from_minutes(10.), &air);
    // Updated assuming relative increase is similar
    assert_eq!(
        model.supersaturation(),
        Supersaturation {
            gf_99: 0.0,
            gf_surf: 210.3133762617924
        }
    );
}

#[test]
fn test_initial_gfs() {
    let mut model = fixtures::model_default();
    let air = BreathingSource::OpenCircuit(Gas::new(0.21, 0.));
    model.record(Depth::from_meters(0.), Time::zero(), &air);
    let Supersaturation { gf_99, gf_surf } = model.supersaturation();
    assert_eq!(gf_99, 0.);
    assert_eq!(gf_surf, 0.);
}

#[test]
fn test_model_records_equality() {
    let mut model1 = fixtures::model_default();
    let mut model2 = fixtures::model_default();

    let air = BreathingSource::OpenCircuit(Gas::new(0.21, 0.));
    let test_depth = Depth::from_meters(50.);
    let test_time = Time::from_minutes(100.);

    model1.record(test_depth, test_time, &air);

    // record every second
    for _i in 1..=test_time.as_seconds() as i32 {
        model2.record(test_depth, Time::from_seconds(1.), &air);
    }

    assert_eq!(
        model1.ceiling().as_meters().floor(),
        model2.ceiling().as_meters().floor()
    );

    let Supersaturation {
        gf_99: model1_gf_99,
        gf_surf: model1_gf_surf,
    } = model1.supersaturation();
    let Supersaturation {
        gf_99: model2_gf_99,
        gf_surf: model2_gf_surf,
    } = model1.supersaturation();
    assert_eq!(model1_gf_99.floor(), model2_gf_99.floor());
    assert_eq!(model1_gf_surf.floor(), model2_gf_surf.floor());
}

#[test]
fn test_actual_ndl_calculation() {
    let config = BuhlmannConfig::default().with_ceiling_type(CeilingType::Actual);
    let mut model = BuhlmannModel::new(config);

    let air = BreathingSource::OpenCircuit(Gas::new(0.21, 0.));
    let depth = Depth::from_meters(30.);

    // with 21/00 at 30m expect NDL 16
    model.record(depth, Time::zero(), &air);
    assert_eq!(model.ndl(), Time::from_minutes(16.));

    // expect NDL 15 after 1 min
    model.record(depth, Time::from_minutes(1.), &air);
    assert_eq!(model.ndl(), Time::from_minutes(15.));
}

#[test]
fn test_adaptive_ndl_calculation() {
    let config = BuhlmannConfig::default().with_ceiling_type(CeilingType::Adaptive);
    let mut model = BuhlmannModel::new(config);

    let air = BreathingSource::OpenCircuit(Gas::new(0.21, 0.));
    let depth = Depth::from_meters(30.);

    // with 21/00 at 30m expect NDL 18 (was 19)
    model.record(depth, Time::zero(), &air);
    assert_eq!(model.ndl(), Time::from_minutes(18.));

    // expect NDL 17 after 1 min (was 18)
    model.record(depth, Time::from_minutes(1.), &air);
    assert_eq!(model.ndl(), Time::from_minutes(17.));
}

#[test]
fn test_ndl_cut_off() {
    let mut model = fixtures::model_default();
    let air = BreathingSource::OpenCircuit(Gas::new(0.21, 0.));

    model.record(Depth::from_meters(0.), Time::zero(), &air);
    assert_eq!(model.ndl(), Time::from_minutes(99.));

    model.record(Depth::from_meters(10.), Time::from_minutes(10.), &air);
    assert_eq!(model.ndl(), Time::from_minutes(99.));
}

#[test]
fn test_multi_gas_ndl() {
    let mut model =
        BuhlmannModel::new(BuhlmannConfig::default().with_ceiling_type(CeilingType::Actual));
    let air = BreathingSource::OpenCircuit(Gas::new(0.21, 0.));
    let ean_28 = BreathingSource::OpenCircuit(Gas::new(0.28, 0.));

    model.record(Depth::from_meters(30.), Time::zero(), &air);
    assert_eq!(model.ndl(), Time::from_minutes(16.));

    model.record(Depth::from_meters(30.), Time::from_minutes(10.), &air);
    assert_eq!(model.ndl(), Time::from_minutes(6.));

    model.record(Depth::from_meters(30.), Time::zero(), &ean_28);
    // reduced from 10 to 9 mins
    assert_eq!(model.ndl(), Time::from_minutes(9.));
}

#[test]
fn test_ndl_with_gf() {
    let mut model = fixtures::model_gf((70, 70));
    let air = BreathingSource::OpenCircuit(Gas::new(0.21, 0.));
    model.record(Depth::from_meters(20.), Time::zero(), &air);
    assert_eq!(model.ndl(), Time::from_minutes(21.));
}

#[test]
fn test_altitude() {
    let mut model = BuhlmannModel::new(BuhlmannConfig::new().with_surface_pressure(700));
    let air = BreathingSource::OpenCircuit(Gas::new(0.21, 0.));
    model.record(Depth::from_meters(40.), Time::from_minutes(60.), &air);
    let Supersaturation { gf_surf, .. } = model.supersaturation();
    assert_eq!(gf_surf, 302.3513258479962);
}

#[test]
fn test_example_ceiling_start() {
    let mut model = BuhlmannModel::new(
        BuhlmannConfig::new()
            .with_gradient_factors(30, 70)
            .with_surface_pressure(1013),
    );

    let air = BreathingSource::OpenCircuit(Gas::air());

    // instant drop to 40m on air for 10min
    model.record(Depth::from_meters(40.), Time::from_minutes(10.), &air);
    assert_eq!(model.ceiling().as_meters(), 12.925502817777206);
}

#[test]
fn test_example_ceiling() {
    let mut model = BuhlmannModel::new(
        BuhlmannConfig::new()
            .with_gradient_factors(30, 70)
            .with_surface_pressure(1013),
    );

    let air = BreathingSource::OpenCircuit(Gas::air());
    let ean_50 = BreathingSource::OpenCircuit(Gas::new(0.50, 0.));

    model.record(Depth::from_meters(40.), Time::from_minutes(40.), &air);
    model.record(Depth::from_meters(30.), Time::from_minutes(3.), &air);
    model.record(Depth::from_meters(21.), Time::from_minutes(10.), &ean_50);
    assert_eq!(model.ceiling().as_meters(), 12.516288762576789);
}

#[test]
fn test_example_ceiling_feet() {
    let mut model = BuhlmannModel::new(
        BuhlmannConfig::new()
            .with_gradient_factors(30, 70)
            .with_surface_pressure(1013),
    );

    let air = BreathingSource::OpenCircuit(Gas::air());
    let ean_50 = BreathingSource::OpenCircuit(Gas::new(0.50, 0.));

    model.record(Depth::from_feet(131.234), Time::from_minutes(40.), &air);
    model.record(Depth::from_feet(98.4252), Time::from_minutes(3.), &air);
    model.record(Depth::from_feet(68.8976), Time::from_minutes(10.), &ean_50);
    assert_eq!(model.ceiling().as_feet(), 41.064076174948156);
    assert_eq!(model.ceiling().as_meters(), 12.516330017601637);
}

#[test]
fn test_adaptive_ceiling() {
    let mut model = BuhlmannModel::new(
        BuhlmannConfig::new().with_ceiling_type(dive_deco::CeilingType::Adaptive),
    );
    let air = BreathingSource::OpenCircuit(Gas::air());
    model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
    let ceiling = model.ceiling();
    assert_close_to_abs!(ceiling.as_meters(), 4., 0.5);
}

#[test]
fn test_gradual_ascent_with_deco() {
    let mut model = BuhlmannModel::new(
        BuhlmannConfig::new()
            .with_gradient_factors(30, 70)
            .with_surface_pressure(1013),
    );
    let air = BreathingSource::OpenCircuit(Gas::air());
    let ean_50 = BreathingSource::OpenCircuit(Gas::new(0.50, 0.));
    model.record(Depth::from_meters(45.), Time::from_minutes(30.), &air);
    loop {
        let depth = model.dive_state().depth;
        if depth <= Depth::zero() {
            break;
        }
        model.record_travel_with_rate(depth - Depth::from_meters(3.), 10., &air);
        model.deco(vec![air, ean_50]).unwrap();
    }
}

#[test]
fn test_cns_otu() {
    let mut model = BuhlmannModel::default();
    let air = BreathingSource::OpenCircuit(Gas::air());
    model.record(Depth::from_meters(40.), Time::from_minutes(10.), &air);
    model.record_travel_with_rate(Depth::from_meters(0.), 10., &air);
    assert_close_to_abs!(model.otu(), 12.0, 1.);
}

#[test]
fn test_desaturation_times_are_sane() {
    // Test case ported from C implementation ensuring desaturation times are reasonable
    let mut model = BuhlmannModel::default();
    let air = BreathingSource::OpenCircuit(Gas::air());

    // Short deep dive
    model.record_travel_with_rate(Depth::from_meters(40.), 18., &air);
    model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
    model.record_travel_with_rate(Depth::from_meters(0.), 18., &air);

    let desat_time = model.desaturation_time();

    // Expect reasonable desaturation time (e.g. between 12 and 48 hours depending on tissue loading)
    // For 20 mins @ 40m, significant loading.
    // 105% of surface pressure is the threshold.
    println!("Desat time 40m 20min: {:?}", desat_time);

    assert!(desat_time.as_minutes() > 60.0 * 6.0); // > 6 hours
    assert!(desat_time.as_minutes() < 60.0 * 48.0); // < 48 hours

    // Saturation dive
    let mut model_sat = BuhlmannModel::default();
    model_sat.record(
        Depth::from_meters(10.),
        Time::from_minutes(60. * 24. * 2.),
        &air,
    ); // 48 hours at 10m
    model_sat.record_travel_with_rate(Depth::from_meters(0.), 10., &air);

    let desat_sat = model_sat.desaturation_time();
    println!("Desat time 10m 48h: {:?}", desat_sat);
    assert!(desat_sat.as_minutes() > desat_time.as_minutes());
}

#[test]
fn test_deco_cal_tts_low_surface_atm() {
    // High Altitude / Low surface pressure test
    // 800 mbar surface pressure (approx 2000m altitude)
    let surface_p = 800;
    let mut config = BuhlmannConfig::default();
    config.surface_pressure = surface_p;
    let mut model = BuhlmannModel::new(config);
    let air = BreathingSource::OpenCircuit(Gas::air());

    // Dive to 30m (Gauge) -> Absolute = 3000mbar + 800 = 3.8 bar
    let depth = Depth::from_meters(30.);
    model.record_travel_with_rate(depth, 18., &air);
    model.record(depth, Time::from_minutes(20.), &air);

    // Plan deco
    let mut deco = Deco::default();
    let deco_runtime = deco.calc(model, vec![air]).expect("Deco calc failed");

    // Assertions
    println!("TTS at 800mbar: {:?}", deco_runtime.tts);
    assert!(deco_runtime.tts.as_minutes() > 0.0);
}

#[test]
fn test_deco_runtime_integrity() {
    // Verify gas switches are respected and deco runtime is logically consistent
    let mut model = BuhlmannModel::default();
    let air = BreathingSource::OpenCircuit(Gas::air());
    let ean50 = BreathingSource::OpenCircuit(Gas::new(0.5, 0.0));
    let oxygen = BreathingSource::OpenCircuit(Gas::new(1.0, 0.0));

    let depth = Depth::from_meters(45.);
    model.record_travel_with_rate(depth, 18., &air);
    model.record(depth, Time::from_minutes(25.), &air);

    let mut deco = Deco::default();
    let runtime = deco
        .calc(model, vec![air, ean50, oxygen])
        .expect("Deco calc failed");

    let stages = runtime.deco_stages;

    let mut used_nitrox = false;
    let mut used_oxygen = false;

    for stage in stages {
        let o2 = stage.gas.fraction_o2();
        if (o2 - 0.5).abs() < 0.01 {
            used_nitrox = true;
        }
        if (o2 - 1.0).abs() < 0.01 {
            used_oxygen = true;
        }

        // Check MOD logic
        let start = stage.start_depth;
        let end = stage.end_depth;
        let max_depth = if start > end { start } else { end };

        if (o2 - 0.5).abs() < 0.01 {
            assert!(max_depth.as_meters() <= 22.);
        }
        if (o2 - 1.0).abs() < 0.01 {
            assert!(max_depth.as_meters() <= 6.5);
        }
    }

    assert!(used_nitrox, "Should have switched to EAN50");
    assert!(used_oxygen, "Should have switched to Oxygen");
}
