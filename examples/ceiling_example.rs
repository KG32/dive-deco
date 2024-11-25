use dive_deco::{BuehlmannConfig, BuehlmannModel, DecoModel, Depth, Gas, Unit};

fn main() {
    let mut model = BuehlmannModel::new(BuehlmannConfig::default());

    let nitrox_32 = Gas::new(0.32, 0.);

    // ceiling after 20 min at 20 meters using EAN32 - ceiling at 0m
    model.record(Depth::m(20.), 20 * 60, &nitrox_32);
    println!("Ceiling: {}m", model.ceiling()); // Ceiling: 0m

    // ceiling after another 42 min at 30 meters using EAN32 - ceiling at 3m
    model.record(Depth::m(30.), 42 * 60, &nitrox_32);
    println!("Ceiling: {},", model.ceiling()); // Ceiling: 3.004(..)m
}
