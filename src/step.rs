use crate::global_types::{Depth,Minutes};
use crate::gas::Gas;

pub struct Step<'a> {
    pub depth: &'a Depth,
    pub time: &'a Minutes,
    pub gas: &'a Gas,
}
