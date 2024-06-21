mod deco_model;
mod gas;
mod global_types;
mod step;
mod deco;

pub use deco_model::{DecoModel, DiveState, DecoModelConfig, ConfigValidationErr};
pub use gas::{Gas, PartialPressures, InertGas};
pub use global_types::{Depth, Pressure, Seconds, Minutes, GradientFactors, GradientFactor, MbarPressure, AscentRatePerMinute};
pub use step::StepData;
pub use deco::{Deco, DecoStage, DecoStageType};
