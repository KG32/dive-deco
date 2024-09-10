pub type Pressure = f64;
pub type Depth = f64;
pub type Seconds = u64;
pub type Minutes = u64;
pub type MinutesSigned = i64;
pub type GradientFactor = u8;
pub type GradientFactors = (u8, u8);
pub type MbarPressure = u16;
pub type AscentRatePerMinute = f64;
pub type CNSPercent = f64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NDLType {
    Actual, // take into consideration off-gassing during ascent
    ByCeiling // treat NDL as a point when ceiling > 0.
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CeilingType {
    Actual,
    Adaptive
}
