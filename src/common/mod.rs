mod cns_table;
mod deco;
mod deco_model;
mod depth;
mod gas;
mod global_types;
mod math_utils;
mod ox_tox;
mod record;
mod sim;
mod time;
// CCR module
pub mod ccr;
pub use ccr::{DiveComputer, DiveMode, SetpointConfig, SetpointController};

// pub use cns_table::{CNSCoeffRow, CNS_COEFFICIENTS};
pub use deco::{Deco, DecoCalculationError, DecoRuntime, DecoStage, DecoStageType};
pub use deco_model::{ConfigValidationErr, DecoModel, DecoModelConfig, DiveState};
pub use depth::{Depth, Unit, Units};
pub use time::Time;

pub use gas::{BreathingSource, Gas, GasMix, InertGas, PartialPressures};
pub use global_types::{
    AscentRatePerMinute, CeilingType, Cns, DecoStopFormatting, DepthType, GradientFactor,
    GradientFactors, MbarPressure, NDLType, Otu, Pressure,
};
pub(crate) use math_utils::{abs, ceil, exp, ln, powf, round};
pub use ox_tox::OxTox;
pub use record::RecordData;
pub use sim::Sim;

pub mod physics;
pub use physics::density as WaterDensities;
