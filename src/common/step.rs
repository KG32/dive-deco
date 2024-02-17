use crate::common::{Depth,Seconds,Gas};

#[derive(Debug)]
pub struct Step<'a> {
    pub depth: &'a Depth,
    pub time: &'a Seconds,
    pub gas: &'a Gas,
}
