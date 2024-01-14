use crate::global_types::{Depth,Minutes};
use crate::gas::Gas;

pub struct Step {
    pub depth: Depth,
    pub time: Minutes,
    pub gas: Gas,
}
