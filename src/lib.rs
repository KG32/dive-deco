mod zhl_16_values;
mod gas;
mod global_types;
mod compartment;
mod step;

use zhl_16_values::zhl_16_values;

pub fn run() {
    let values = zhl_16_values();
    dbg!(values);
}
