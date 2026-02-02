use dive_deco::{
    BreathingSource, BuhlmannModel, DecoModel, Depth, DiveComputer, DiveMode, GasMix,
    SetpointConfig, SetpointController, Time,
};

#[test]
fn test_ccr_breathing_source_pressures() {
    let diluent = GasMix::new(0.21, 0.35); // 21/35 Tmx
    let source = BreathingSource::ClosedCircuit {
        setpoint: 1.3,
        diluent,
    };

    // At 30m (4 bar)
    let p_amb = 4.0;
    let pressures = source.calculate_pressures(p_amb);
    assert_eq!(pressures.o2, 1.3);

    // Inert pressure = 4.0 - 1.3 = 2.7
    // Diluent inert fraction = 0.35 (He) + 0.44 (N2) = 0.79
    // He ratio = 0.35 / 0.79 = 0.443...
    // N2 ratio = 0.44 / 0.79 = 0.556...
    let inert_total = diluent.fraction_he + diluent.fraction_n2();
    let expected_he = (0.35 / inert_total) * 2.7;
    let expected_n2 = (0.44 / inert_total) * 2.7;

    assert!((pressures.he - expected_he).abs() < 1e-6);
    assert!((pressures.n2 - expected_n2).abs() < 1e-6);

    // Test impossible setpoint (Surface, 1 bar)
    let pressures_surf = source.calculate_pressures(1.0);
    assert_eq!(pressures_surf.o2, 1.0); // limited by ambient
    assert_eq!(pressures_surf.he, 0.0);
    assert_eq!(pressures_surf.n2, 0.0);
}

#[test]
fn test_setpoint_controller_logic() {
    let config = SetpointConfig {
        low_setpoint: 0.7,
        high_setpoint: 1.3,
        switch_depth_descent: Some(10.0),
        switch_depth_ascent: Some(6.0),
    };
    let diluent = GasMix::air();
    let mut controller = SetpointController::new(config, diluent);

    // Start at surface
    let source = controller.tick(0.0);
    if let BreathingSource::ClosedCircuit { setpoint, .. } = source {
        assert_eq!(setpoint, 0.7);
    } else {
        panic!("Expected CCR source");
    }

    // Descend to 9m (below switch depth)
    if let BreathingSource::ClosedCircuit { setpoint, .. } = controller.tick(9.0) {
        assert_eq!(setpoint, 0.7);
    }

    // Descend to 11m (above switch depth) -> should switch to high
    if let BreathingSource::ClosedCircuit { setpoint, .. } = controller.tick(11.0) {
        assert_eq!(setpoint, 1.3);
    }

    // Stay at 11m -> still high
    if let BreathingSource::ClosedCircuit { setpoint, .. } = controller.tick(11.0) {
        assert_eq!(setpoint, 1.3);
    }

    // Ascend to 7m -> should stay at High (ascent switch is 6m)
    if let BreathingSource::ClosedCircuit { setpoint, .. } = controller.tick(7.0) {
        assert_eq!(setpoint, 1.3);
    }

    // Ascend to 5m -> should switch back to low
    if let BreathingSource::ClosedCircuit { setpoint, .. } = controller.tick(5.0) {
        assert_eq!(setpoint, 0.7);
    }
}

#[test]
fn test_dive_computer_integration() {
    let config = SetpointConfig {
        low_setpoint: 0.7,
        high_setpoint: 1.3,
        switch_depth_descent: Some(10.0),
        switch_depth_ascent: Some(6.0),
    };
    let diluent = GasMix::air();
    let bailout = GasMix::new(0.5, 0.0); // EAN50

    let mut computer = DiveComputer::new(diluent, vec![bailout], config, DiveMode::ClosedCircuit);

    // Initial state: CCR Low
    let source = computer.step(0.0);
    assert_eq!(source.fraction_o2(), 0.21); // diluent O2 is used for identity

    // Switch to bailout
    computer.switch_to_bailout(0);
    let source_bail = computer.step(10.0);
    assert!(matches!(source_bail, BreathingSource::OpenCircuit(_)));
    assert_eq!(source_bail.fraction_o2(), 0.5);

    // Switch back to CCR
    computer.switch_to_ccr();
    let source_ccr = computer.step(20.0);
    if let BreathingSource::ClosedCircuit { setpoint, .. } = source_ccr {
        assert_eq!(setpoint, 1.3); // Deep depth should trigger high SP
    } else {
        panic!("Expected CCR source");
    }
}

#[test]
fn test_ccr_deco_calculation() {
    let config = SetpointConfig {
        low_setpoint: 0.7,
        high_setpoint: 1.3,
        switch_depth_descent: Some(10.0),
        switch_depth_ascent: Some(6.0),
    };
    let diluent = GasMix::air();
    let mut computer = DiveComputer::new(diluent, vec![], config, DiveMode::ClosedCircuit);
    let mut model = BuhlmannModel::default();

    // Dive to 40m for 20 mins
    let depth = Depth::from_meters(40.0);
    let time = Time::from_minutes(20.0);

    // In a real loop, we'd tick every second
    let source = computer.step(40.0);
    model.record(depth, time, &source);

    // Calculate deco
    let deco_runtime = model.deco(vec![source]).unwrap();

    // Ensure deco stages use the CCR source
    assert!(deco_runtime.deco_stages.len() > 0);
    for stage in deco_runtime.deco_stages {
        assert!(matches!(stage.gas, BreathingSource::ClosedCircuit { .. }));
    }

    println!("CCR TTS: {:?}", deco_runtime.tts);
}
