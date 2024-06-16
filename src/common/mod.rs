mod deco_model;
mod gas;
mod global_types;
mod step;
mod runtime;

pub use deco_model::{DecoModel, DiveState, DecoModelConfig, ConfigValidationErr};
pub use gas::{Gas, PartialPressures, InertGas};
pub use global_types::{Depth, Pressure, Seconds, Minutes, GradientFactors, GradientFactor, MbarPressure, AscentRatePerMinute};
pub use step::StepData;
pub use runtime::{DecoRuntime, DecoEvent, DecoEventType};
