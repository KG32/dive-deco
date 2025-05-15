use alloc::vec;
use alloc::vec::Vec;
use dive_deco::{DecoModel, Depth, DepthType, GradientFactors, Time};
pub mod fixtures;

#[test]
fn test_ndl() {
    // (gradient_factors, depth, expected_ndl)
    let test_cases: Vec<(GradientFactors, DepthType, Time)> = vec![
        // 100/100
        ((100, 100), 21., Time::from_minutes(40.)),
        ((100, 100), 15., Time::from_minutes(90.)),
        // 70/70
        ((70, 70), 21., Time::from_minutes(19.)),
        ((70, 70), 15., Time::from_minutes(47.)),
    ];

    let air = fixtures::gas_air();
    for test_case in test_cases {
        let (gradient_factors, test_depth, expected_ndl) = test_case;
        let mut model = fixtures::model_gf(gradient_factors);
        model.record(Depth::from_meters(test_depth), Time::zero(), &air);
        assert_eq!(model.ndl(), expected_ndl);
    }
}

// GFLo

#[test]
fn test_gf_low_ceiling() {
    let mut model = fixtures::model_gf((50, 100));

    let air = fixtures::gas_air();

    model.record(Depth::from_meters(40.), Time::from_minutes(10.), &air);

    let ceiling = model.ceiling();

    assert_close_to_abs!(ceiling.as_meters(), 8., 0.5);
}
