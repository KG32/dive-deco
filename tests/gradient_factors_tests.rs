use dive_deco::{ DecoModel, Depth, Minutes };
mod fixtures;

#[test]
fn test_gf_ndl() {

    // (depth, expected_ndl)
    let test_cases: Vec<(Depth, Minutes)> = vec![
        (21., 39),
        (15., 87)
    ];

    let air = fixtures::gas_air();
    for test_case in test_cases {
        let mut model = fixtures::model_default();
        let (test_depth, expected_ndl) = test_case;
        model.step(&test_depth, &0, &air);
        assert_eq!(model.ndl(), expected_ndl);
    }
}
