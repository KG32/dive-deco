mod deco_model;
mod gas;
mod global_types;
mod record;
mod deco;
mod cns_table;
mod ox_tox;
mod sim;

pub use deco_model::{DecoModel, DiveState, DecoModelConfig, ConfigValidationErr};
pub use gas::{Gas, PartialPressures, InertGas};
pub use global_types::{
    Depth,
    Pressure,
    Seconds,
    Minutes,
    MinutesSigned,
    GradientFactors,
    GradientFactor,
    MbarPressure,
    AscentRatePerMinute,
    CNSPercent,
    NDLType,
};
pub use record::RecordData;
pub use deco::{Deco, DecoStage, DecoStageType, DecoRuntime};
pub use cns_table::{CNSCoeffRow, CNS_COEFFICIENTS};
pub use ox_tox::OxTox;
pub use sim::Sim;
