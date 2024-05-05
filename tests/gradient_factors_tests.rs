use dive_deco::{ DecoModel, Depth, GradientFactors, Minutes };
pub mod fixtures;

#[test]
fn test_ndl() {
    // (gradient_factors, depth, expected_ndl)
    let test_cases: Vec<(GradientFactors, Depth, Minutes)> = vec![
        // 100/100
        ((100, 100), 21., 40),
        ((100, 100), 15., 88),

        // 70/70
        ((70, 70), 21., 18),
        ((70, 70), 15., 47),
    ];

    let air = fixtures::gas_air();
    for test_case in test_cases {
        let (gradient_factors, test_depth, expected_ndl) = test_case;
        let mut model = fixtures::model_gf(gradient_factors);
        model.step(&test_depth, &0, &air);
        assert_eq!(model.ndl(), expected_ndl);
    }
}

// GFLo

#[test]
fn test_gf_low_ceiling() {
    let mut model = fixtures::model_gf((50, 100));

    let air = fixtures::gas_air();

    model.step(&40., &(10 * 60), &air);

    let ceiling = model.ceiling();

    assert_eq!(ceiling, 0.4707753025628003);
}
