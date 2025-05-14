use dive_deco::{BuhlmannConfig, BuhlmannModel, DecoModel, Depth, Gas, Time};

fn main() {
    // initialize a Buhlmann ZHL-16C deco model with default config (GF 100/100)
    let config = BuhlmannConfig::default();
    let mut model = BuhlmannModel::new(config);

    let air = Gas::new(0.21, 0.);
    let depth = Depth::from_meters(30.);
    let bottom_time = Time::from_minutes(10.);

    // a simulated instantaneous drop to 20m with 20 minutes bottom time using air
    model.record(depth, bottom_time, &air);

    // current NDL (no-decompression limit)
    let current_ndl = model.ndl();
    println!("NDL: {} min", current_ndl.as_minutes()); // output: NDL: 5 min
}
