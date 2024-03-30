use crate::common::{Depth, Gas, Seconds, Minutes};

#[derive(Debug, PartialEq)]
pub struct ConfigValidationErr<'a> {
    pub field: &'a str,
    pub reason: &'a str,
}

impl<'a> ConfigValidationErr<'a> {
    pub fn new(field: &'a str, reason: &'a str) -> Self {
        Self {
            field,
            reason
        }
    }
}

pub trait DecoModelConfig {
    fn validate(&self) -> Result<(), ConfigValidationErr>;
}

pub trait DecoModel {
    type ConfigType: DecoModelConfig;

    /// model init
    fn new(config: Self::ConfigType) -> Self;

    /// add register step (depth: meters, time: seconds)
    fn step(&mut self, depth: &Depth, time: &Seconds, gas: &Gas);

    /// current non decompression limit (NDL)
    fn ndl(&self) -> Minutes;

    /// current decompression ceiling in meters
    fn ceiling(&self) -> Depth;
}
