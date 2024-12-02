use crate::common::{Depth, Gas, Time};

#[derive(Debug)]
pub struct RecordData<'a> {
    pub depth: Depth,
    pub time: Time,
    pub gas: &'a Gas,
}
