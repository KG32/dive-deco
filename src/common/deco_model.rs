use crate::common::{Depth, Gas, Seconds};

pub trait DecoModel {
    fn new() -> Self;

    fn step(&mut self, depth: &Depth, time: &Seconds, gas: &Gas) -> ();

    fn ceiling(&self) -> Depth;
}
