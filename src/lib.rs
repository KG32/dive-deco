pub mod model;
pub mod zhl_16_values;
pub mod gas;
pub mod global_types;
pub mod compartment;
pub mod step;

use model::ZHLModel;
use zhl_16_values::zhl_16_values;

pub fn zhl16c() -> ZHLModel {
    let zhl16_values = zhl_16_values();

    ZHLModel::new(zhl16_values.to_vec())
}
