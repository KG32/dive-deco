use dive_deco::{DecoModel, BuehlmannModel, BuehlmannConfig, Gas};

fn main() {
    // initialize a Buehlmann ZHL-16C deco model with default config (GF 100/100)
    let config = BuehlmannConfig::default();
    let mut model = BuehlmannModel::new(config);

    let air = Gas::new(0.21, 0.);
    let depth = 30.;
    let bottom_time_minutes = 10;

    // a simulated instantaneous drop to 20m with 20 minutes bottom time using air
    model.step(depth, bottom_time_minutes * 60, &air);

    // current NDL (no-decompression limit)
    let current_ndl = model.ndl();
    println!("NDL: {} min", current_ndl); // output: NDL: 5 min
}
