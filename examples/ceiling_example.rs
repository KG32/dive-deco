use dive_deco::{BuhlmannConfig, BuhlmannModel, DecoModel, Depth, Gas, Time};

fn main() {
    let mut model = BuhlmannModel::new(BuhlmannConfig::default());

    let nitrox_32 = Gas::new(0.32, 0.);

    // ceiling after 20 min at 20 meters using EAN32 - ceiling at 0m
    model.record(Depth::from_meters(20.), Time::from_minutes(20.), &nitrox_32);
    println!("Ceiling: {}m", model.ceiling()); // Ceiling: 0m

    // ceiling after another 42 min at 30 meters using EAN32 - ceiling at 3m
    model.record(Depth::from_meters(30.), Time::from_minutes(42.), &nitrox_32);
    println!("Ceiling: {},", model.ceiling()); // Ceiling: 3.004(..)m
}
