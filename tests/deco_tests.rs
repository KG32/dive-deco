use dive_deco::{DecoModel, Deco, Gas, DecoStage, DecoStageType};

pub mod fixtures;

#[test]
fn test_deco_ascent_no_deco() {
    let air = fixtures::gas_air();
    let mut model = fixtures::model_default();
    model.step(&20., &(5 * 60), &air);

    let Deco { deco_stages, tts } = model.deco(vec![air]);
    assert_eq!(deco_stages.len(), 1); // single continuous ascent
    assert_eq!(tts / 60, 2); // tts in minutes
}

#[test]
fn test_deco_single_gas() {
    let air = fixtures::gas_air();
    let mut model = fixtures::model_default();
    model.step(&40., &(20 * 60), &air);

    let Deco {
        deco_stages,
        tts
    } = model.deco(vec![air]);

    assert_close_to_percent!(tts as f64, 800., 1.); // 13.(3) min todo round to 14
    assert_eq!(deco_stages.len(), 5);

    let expected_deco_stages =  vec![
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: 40.0,
            end_depth: 6.0,
            duration: 226,
            gas: air
        },
        DecoStage {
            stage_type: DecoStageType::DecoStop,
            start_depth: 6.0,
            end_depth: 6.0,
            duration: 95,
            gas: air
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: 6.0,
            end_depth: 3.0,
            duration: 20,
            gas: air
        },
        DecoStage {
            stage_type: DecoStageType::DecoStop,
            start_depth: 3.0,
            end_depth: 3.0,
            duration: 436,
            gas: air
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: 3.0,
            end_depth: 0.0,
            duration: 20,
            gas: air
        },
    ];

    assert_deco_stages_eq(deco_stages, expected_deco_stages);
}

#[test]
fn test_deco_multi_gas() {
    let mut model = fixtures::model_default();

    let air = Gas::new(0.21, 0.);
    let ean_50 = Gas::new(0.50, 0.);

    model.step(&40.0001, &(20 * 60), &air);

    let Deco {
        deco_stages,
        tts
    } = model.deco(vec![air, ean_50]);

    let expected_deco_stages =  vec![
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: 40.0001,
            end_depth: 22.,
            duration: 120,
            gas: air
        },
        DecoStage {
            stage_type: DecoStageType::GasSwitch,
            start_depth: 22.0,
            end_depth: 22.0,
            duration: 0,
            gas: ean_50
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: 22.,
            end_depth: 6.,
            duration: 106,
            gas: ean_50
        },
        DecoStage {
            stage_type: DecoStageType::DecoStop,
            start_depth: 6.0,
            end_depth: 6.0,
            duration: 39,
            gas: ean_50
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: 6.0,
            end_depth: 3.0,
            duration: 20,
            gas: ean_50
        },
        DecoStage {
            stage_type: DecoStageType::DecoStop,
            start_depth: 3.0,
            end_depth: 3.0,
            duration: 301,
            gas: ean_50
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: 3.0,
            end_depth: 0.0,
            duration: 20,
            gas: ean_50
        },
    ];
    dbg!(&deco_stages);

    assert_deco_stages_eq(deco_stages, expected_deco_stages);
    assert_close_to_abs!(tts as f64, 590., 30.);
}

#[test]
fn test_deco_with_deco_mod_at_bottom() {
    let mut model = fixtures::model_default();
    let air = Gas::air();
    let ean_36 = Gas::new(0.36, 0.);

    model.step(&30., &(30 * 60), &air);

    let Deco { deco_stages, tts } = model.deco(vec![air, ean_36]);

    let expected_deco_stages = vec![
        DecoStage {
            stage_type: DecoStageType::GasSwitch,
            start_depth: 30.0,
            end_depth: 30.0,
            duration: 0,
            gas: ean_36,
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: 30.0,
            end_depth: 3.0,
            duration: 180,
            gas: ean_36,
        },
        DecoStage {
            stage_type: DecoStageType::DecoStop,
            start_depth: 3.0,
            end_depth: 3.0,
            duration: 300,
            gas: ean_36,
        },
        DecoStage {
            stage_type: DecoStageType::Ascent,
            start_depth: 3.0,
            end_depth: 0.0,
            duration: 20,
            gas: ean_36
        },
    ];

    assert_deco_stages_eq(deco_stages, expected_deco_stages);
    assert_eq!(tts, 500);
}

fn assert_deco_stages_eq(deco_stages: Vec<DecoStage>, expected_deco_stages: Vec<DecoStage>) {
    assert_eq!(deco_stages.len(), expected_deco_stages.len());
    for (i, expected_stage) in expected_deco_stages.iter().enumerate() {
        assert_eq!(deco_stages[i].stage_type, expected_stage.stage_type);
        assert_eq!(deco_stages[i].start_depth, expected_stage.start_depth);
        assert_eq!(deco_stages[i].end_depth,expected_stage.end_depth);
        assert_eq!(deco_stages[i].duration, expected_stage.duration);
        assert_eq!(deco_stages[i].gas, expected_stage.gas);
    }
}
