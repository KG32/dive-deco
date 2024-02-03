use crate::global_types::{Depth,Seconds};
use crate::gas::Gas;

#[derive(Debug)]
pub struct Step<'a> {
    pub depth: &'a Depth,
    pub time: &'a Seconds,
    pub gas: &'a Gas,
}
