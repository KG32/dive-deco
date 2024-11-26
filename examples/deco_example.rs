use dive_deco::{BuehlmannConfig, BuehlmannModel, DecoModel, Depth, Gas, Unit};

fn main() {
    let config = BuehlmannConfig::new().with_gradient_factors(30, 70);
    let mut model = BuehlmannModel::new(config);

    // bottom gas
    let air = Gas::air();
    // deco gases
    let ean_50 = Gas::new(0.5, 0.);
    let oxygen = Gas::new(1., 0.);
    let available_gas_mixes = vec![air, ean_50, oxygen];

    let bottom_depth = Depth::from_meters(40.);
    let bottom_time = 20 * 60; // 20 min

    // descent to 40m at a rate of 9min/min using air
    model.record_travel_with_rate(bottom_depth, 9., &available_gas_mixes[0]);

    // 20 min bottom time
    model.record(bottom_depth, bottom_time, &air);

    // calculate deco runtime providing available gasses
    let deco_runtime = model.deco(available_gas_mixes);

    println!("{:#?}", deco_runtime);
}
