mod common;
mod buehlmann;

pub use common::{DecoModel, Gas, Depth, Minutes, Seconds, Pressure};

pub use buehlmann::{BuehlmannModel, BuehlmannConfig};
