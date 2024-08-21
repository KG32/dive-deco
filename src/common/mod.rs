mod deco_model;
mod gas;
mod global_types;
mod step;
mod deco;
mod cns_table;
mod ox_tox;

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
    CNSPercent
};
pub use step::StepData;
pub use deco::{Deco, DecoStage, DecoStageType, DecoRuntime};
pub use cns_table::{CNSCoeffRow, CNS_COEFFICIENTS};
pub use ox_tox::OxTox;
