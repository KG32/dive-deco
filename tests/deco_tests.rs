use dive_deco::{
    BuhlmannConfig, BuhlmannModel, CeilingType, DecoModel, DecoRuntime, DecoStage, DecoStageType,
    Depth, Gas, Time,
};

pub mod fixtures;

#[test]
fn test_deco_ascent_no_deco() {
    let air = fixtures::gas_air();
    let mut model = fixtures::model_default();
    model.record(Depth::from_meters(20.), Time::from_minutes(5.), &air);

    let DecoRuntime {
        deco_stages, tts, ..
    } = model.deco(vec![air]).unwrap();
    assert_eq!(deco_stages.len(), 1); // single continuous ascent
    assert_eq!(tts, Time::from_minutes(2.)); // tts in minutes
}

#[test]
fn test_deco_single_gas() {
    let air = fixtures::gas_air();
    let mut model = BuhlmannModel::new(BuhlmannConfig::default().with_deco_ascent_rate(9.));
    model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);

    let DecoRuntime {
        deco_stages, tts, ..
    } = model.deco(vec![air]).unwrap();

    assert_eq!(tts, Time::from_seconds(754.));
    assert_eq!(deco_stages.len(), 5);

    let expected_deco_stages = vec![
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: Depth::from_meters(40.0),
            end_depth: Depth::from_meters(6.0),
            duration: Time::from_seconds(226.),
            gas: air,
        },
        DecoStage {
            stage_type: DecoStageType::DecoStop,
            start_depth: Depth::from_meters(6.0),
            end_depth: Depth::from_meters(6.0),
            duration: Time::from_seconds(88.),
            gas: air,
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: Depth::from_meters(6.0),
            end_depth: Depth::from_meters(3.0),
            duration: Time::from_seconds(20.),
            gas: air,
        },
        DecoStage {
            stage_type: DecoStageType::DecoStop,
            start_depth: Depth::from_meters(3.0),
            end_depth: Depth::from_meters(3.0),
            duration: Time::from_seconds(400.),
            gas: air,
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: Depth::from_meters(3.0),
            end_depth: Depth::from_meters(0.0),
            duration: Time::from_seconds(20.),
            gas: air,
        },
    ];

    assert_deco_stages_eq(deco_stages, expected_deco_stages);
}

#[test]
fn test_deco_multi_gas() {
    let mut model = BuhlmannModel::new(BuhlmannConfig::default().with_deco_ascent_rate(9.));

    let air = Gas::new(0.21, 0.);
    let ean_50 = Gas::new(0.50, 0.);

    model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);

    let DecoRuntime {
        deco_stages, tts, ..
    } = model.deco(vec![air, ean_50]).unwrap();

    let expected_deco_stages = vec![
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: Depth::from_meters(40.),
            end_depth: Depth::from_meters(22.),
            duration: Time::from_seconds(120.),
            gas: air,
        },
        DecoStage {
            stage_type: DecoStageType::GasSwitch,
            start_depth: Depth::from_meters(22.0),
            end_depth: Depth::from_meters(22.0),
            duration: Time::zero(),
            gas: ean_50,
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: Depth::from_meters(22.),
            end_depth: Depth::from_meters(6.),
            duration: Time::from_seconds(106.),
            gas: ean_50,
        },
        DecoStage {
            stage_type: DecoStageType::DecoStop,
            start_depth: Depth::from_meters(6.0),
            end_depth: Depth::from_meters(6.0),
            duration: Time::from_seconds(34.),
            gas: ean_50,
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: Depth::from_meters(6.0),
            end_depth: Depth::from_meters(3.0),
            duration: Time::from_seconds(20.),
            gas: ean_50,
        },
        DecoStage {
            stage_type: DecoStageType::DecoStop,
            start_depth: Depth::from_meters(3.0),
            end_depth: Depth::from_meters(3.0),
            duration: Time::from_seconds(291.),
            gas: ean_50,
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: Depth::from_meters(3.0),
            end_depth: Depth::from_meters(0.0),
            duration: Time::from_seconds(20.),
            gas: ean_50,
        },
    ];

    assert_deco_stages_eq(deco_stages, expected_deco_stages);
    assert_eq!(tts, Time::from_seconds(591.));
}

#[test]
fn test_deco_with_deco_mod_at_bottom() {
    let mut model = BuhlmannModel::new(BuhlmannConfig::default().with_deco_ascent_rate(9.));
    let air = Gas::air();
    let ean_36 = Gas::new(0.36, 0.);

    model.record(Depth::from_meters(30.), Time::from_minutes(30.), &air);

    let DecoRuntime {
        deco_stages, tts, ..
    } = model.deco(vec![air, ean_36]).unwrap();

    let expected_deco_stages = vec![
        DecoStage {
            stage_type: DecoStageType::GasSwitch,
            start_depth: Depth::from_meters(30.0),
            end_depth: Depth::from_meters(30.0),
            duration: Time::zero(),
            gas: ean_36,
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: Depth::from_meters(30.0),
            end_depth: Depth::from_meters(3.0),
            duration: Time::from_seconds(180.),
            gas: ean_36,
        },
        DecoStage {
            stage_type: DecoStageType::DecoStop,
            start_depth: Depth::from_meters(3.0),
            end_depth: Depth::from_meters(3.0),
            duration: Time::from_seconds(268.),
            gas: ean_36,
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: Depth::from_meters(3.0),
            end_depth: Depth::from_meters(0.0),
            duration: Time::from_seconds(20.),
            gas: ean_36,
        },
    ];
    assert_deco_stages_eq(deco_stages, expected_deco_stages);
    assert_eq!(tts, Time::from_seconds(468.));
}

#[test]
fn test_tts_delta() {
    let mut model = fixtures::model_gf((30, 70));
    let air = Gas::air();
    let ean_50 = Gas::new(0.5, 0.);
    let gas_mixes = vec![air, ean_50];
    model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);
    let deco_1 = model.deco(gas_mixes.clone()).unwrap();
    model.record(Depth::from_meters(40.), Time::from_minutes(5.), &air);
    let deco_2 = model.deco(gas_mixes).unwrap();
    assert_eq!(deco_1.tts_at_5, deco_2.tts);
    assert_eq!(deco_1.tts_delta_at_5, deco_2.tts - deco_1.tts);
}

#[test]
fn test_runtime_on_missed_stop() {
    let air = Gas::air();
    let ean_50 = Gas::new(0.50, 0.);
    let available_gas_mixes = vec![air, ean_50];

    let configs = vec![
        BuhlmannConfig::default()
            .with_ceiling_type(dive_deco::CeilingType::Actual)
            .with_gradient_factors(30, 70),
        BuhlmannConfig::default()
            .with_ceiling_type(dive_deco::CeilingType::Adaptive)
            .with_gradient_factors(30, 70),
    ];

    for config in configs.into_iter() {
        let mut model = BuhlmannModel::new(config);
        model.record(Depth::from_meters(40.), Time::from_minutes(30.), &air);
        model.record(Depth::from_meters(22.), Time::zero(), &air);
        let initial_deco = model.deco(available_gas_mixes.clone()).unwrap();
        // 21
        let initial_deco_stop_depth = get_first_deco_stop_depth(initial_deco);

        // between stop and ceiling (18 - 21)
        model.record(Depth::from_meters(20.), Time::zero(), &air);
        let between_deco = model.deco(available_gas_mixes.clone()).unwrap();
        let between_deco_stop_depth = get_first_deco_stop_depth(between_deco);

        // below
        model.record(Depth::from_meters(15.), Time::zero(), &air);
        let below_deco = model.deco(available_gas_mixes.clone()).unwrap();
        let below_deco_stop_depth = get_first_deco_stop_depth(below_deco);

        assert_eq!(
            initial_deco_stop_depth, between_deco_stop_depth,
            "below deco stop, above ceiling"
        );
        assert_eq!(
            initial_deco_stop_depth, below_deco_stop_depth,
            "below ceiling"
        );
    }
}

#[test]
fn test_deco_runtime_integrity() {
    let config = BuhlmannConfig::new()
        .with_gradient_factors(30, 70)
        .with_ceiling_type(CeilingType::Adaptive);
    let mut model = BuhlmannModel::new(config);
    let air = Gas::air();
    let ean_50 = Gas::new(0.50, 0.);
    let oxygen = Gas::new(1., 0.);
    model.record(Depth::from_meters(40.), Time::from_minutes(20.), &air);

    let deco_runtime = model.deco(vec![air, ean_50, oxygen]).unwrap();
    let deco_stages = deco_runtime.deco_stages;

    deco_stages.iter().reduce(|a, b| {
        if b.stage_type == DecoStageType::DecoStop {
            let stop_depth_meters = b.start_depth.as_meters();
            let prev_end_depth_meters = a.end_depth.as_meters();
            let epsilon = 1e-9; // Tolerance for floating point comparisons

            // Check 1: Deco stop depth must be a multiple of 3m.
            let rounded_to_3m_multiple = (stop_depth_meters / 3.0).round() * 3.0;
            assert!(
                (stop_depth_meters - rounded_to_3m_multiple).abs() < epsilon,
                "Deco stop depth ({}m) for stage {:?} should be a multiple of 3m (expected approx {}m). Preceded by {:?} ending at {}m.",
                stop_depth_meters,
                b.stage_type,
                rounded_to_3m_multiple,
                a.stage_type,
                prev_end_depth_meters
            );

            // Check 2: Actual depth where previous stage ended must be at or deeper than the displayed stop depth.
            assert!(
                prev_end_depth_meters >= stop_depth_meters - epsilon,
                "Previous stage end depth ({}m) for {:?} should be >= deco stop depth ({}m) for {:?}.",
                prev_end_depth_meters,
                a.stage_type,
                stop_depth_meters,
                b.stage_type
            );

            // Check 3: The difference between actual depth and displayed stop depth should be less than 3m.
            assert!(
                (prev_end_depth_meters - stop_depth_meters) < (3.0 - epsilon),
                "Previous stage end depth ({}m) for {:?} should be within 3m of (and >=) deco stop depth ({}m) for {:?}. Difference: {}m",
                prev_end_depth_meters,
                a.stage_type,
                stop_depth_meters,
                b.stage_type,
                prev_end_depth_meters - stop_depth_meters
            );

        } else {
            // For non-DecoStop stages, maintain strict continuity.
            // Using an epsilon for float comparison.
            assert!(
                (b.start_depth.as_meters() - a.end_depth.as_meters()).abs() < 1e-9,
                "Next stage start depth ({:?}@{}m) should equal previous stage end depth ({:?}@{}m). Diff: {}m",
                b.stage_type,
                b.start_depth,
                a.stage_type,
                a.end_depth,
                b.start_depth.as_meters() - a.end_depth.as_meters()
            );
        }

        // validate gas switch MOD
        if a.stage_type == DecoStageType::GasSwitch {
            let gas_switch_target_mod = a.gas.max_operating_depth(1.6);
            // Using an epsilon for float comparison.
            assert!(
                a.start_depth.as_meters() <= gas_switch_target_mod.as_meters() + 1e-9,
                "Gas switch for {:?} at depth {}m exceeds MOD of {}m",
                a.gas,
                a.start_depth,
                gas_switch_target_mod
            );
        }
        b
    });
}

fn get_first_deco_stop_depth(deco: DecoRuntime) -> Option<Depth> {
    let first_stop = deco
        .deco_stages
        .into_iter()
        .find(|stage| stage.stage_type == DecoStageType::DecoStop);
    if let Some(stop) = first_stop {
        return Some(stop.start_depth);
    }
    None
}

fn assert_deco_stages_eq(deco_stages: Vec<DecoStage>, expected_deco_stages: Vec<DecoStage>) {
    assert_eq!(deco_stages.len(), expected_deco_stages.len());
    for (i, expected_stage) in expected_deco_stages.iter().enumerate() {
        assert_eq!(deco_stages[i].stage_type, expected_stage.stage_type);
        assert_eq!(deco_stages[i].start_depth, expected_stage.start_depth);
        assert_eq!(deco_stages[i].end_depth, expected_stage.end_depth);
        assert_eq!(deco_stages[i].duration, expected_stage.duration);
        assert_eq!(deco_stages[i].gas, expected_stage.gas);
    }
}
