#![cfg_attr(feature = "no-std", no_std)]
extern crate alloc;

mod buhlmann;
mod common;

pub use buhlmann::{
    BuehlmannConfig, BuehlmannModel, BuhlmannConfig, BuhlmannModel, Compartment, Supersaturation,
};

pub use common::{
    CeilingType, Deco, DecoCalculationError, DecoModel, DecoRuntime, DecoStage, DecoStageType,
    Depth, DepthType, DiveState, Gas, GradientFactors, NDLType, Pressure, RecordData, Sim, Time,
    Unit, Units,
};

// Re-export Vec and vec macro from alloc for convenience
pub use alloc::vec;
pub use alloc::vec::Vec;
