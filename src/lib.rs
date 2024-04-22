mod common;
mod buehlmann;

pub use common::{DecoModel, Gas, Depth, Minutes, Seconds, Pressure, GradientFactors};

pub use buehlmann::{BuehlmannModel, BuehlmannConfig};
