use crate::common::{Depth, Gas, Seconds, Minutes};

pub trait SupportedConfigType {}

pub trait DecoModel {
    type ConfigType: SupportedConfigType;

    /// model init
    fn new(config: Self::ConfigType) -> Self;

    /// add register step (depth: meters, time: seconds)
    fn step(&mut self, depth: &Depth, time: &Seconds, gas: &Gas);

    /// current non decompression limit (NDL)
    fn ndl(&self) -> Minutes;

    /// current decompression ceiling in meters
    fn ceiling(&self) -> Depth;
}
