#![cfg_attr(feature = "no-std", no_std)]
extern crate alloc;

mod buhlmann;
mod common;

pub use buhlmann::{
    BuehlmannConfig, BuehlmannModel, BuhlmannConfig, BuhlmannModel, Compartment, Supersaturation,
};

pub use common::{
    BreathingSource, CeilingType, Deco, DecoCalculationError, DecoModel, DecoRuntime, DecoStage,
    DecoStageType, DecoStopFormatting, Depth, DepthType, DiveComputer, DiveMode, DiveState, Gas,
    GasMix, GradientFactors, NDLType, Pressure, RecordData, SetpointConfig, SetpointController,
    Sim, Time, Unit, Units,
};

// Re-export Vec and vec macro from alloc for convenience
pub use alloc::vec;
pub use alloc::vec::Vec;
