use crate::global_types::{Depth,Minutes};
use crate::gas::Gas;

pub struct Step<'a> {
    pub depth: Depth,
    pub time: Minutes,
    pub gas: &'a Gas,
}
