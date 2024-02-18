use crate::common::{Depth, Gas, Seconds};

pub trait DecoModel {
    /// model init
    fn new() -> Self;

    /// add register step (depth: meters, time: seconds)
    fn step(&mut self, depth: &Depth, time: &Seconds, gas: &Gas);

    /// current decompression ceiling in meters
    fn ceiling(&self) -> Depth;
}
