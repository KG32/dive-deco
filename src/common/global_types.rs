#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub type Pressure = f64;
pub type DepthType = f64;
pub type GradientFactor = u8;
pub type GradientFactors = (u8, u8);
pub type MbarPressure = i32;
pub type AscentRatePerMinute = f64;
pub type Cns = f64;
pub type Otu = f64;

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NDLType {
    Actual,    // take into consideration off-gassing during ascent
    ByCeiling, // treat NDL as a point when ceiling > 0.
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CeilingType {
    Actual,
    Adaptive,
}
