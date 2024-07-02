use crate::common::{Depth,Seconds,Gas};

#[derive(Debug)]
pub struct StepData<'a> {
    pub depth: Depth,
    pub time: Seconds,
    pub gas: &'a Gas,
}
