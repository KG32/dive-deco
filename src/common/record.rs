use crate::common::{Depth, Gas, Seconds};

#[derive(Debug)]
pub struct RecordData<'a> {
    pub depth: Depth,
    pub time: Seconds,
    pub gas: &'a Gas,
}
