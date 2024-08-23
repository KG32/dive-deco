use crate::common::{Depth,Seconds,Gas};

#[derive(Debug)]
pub struct RecordData<'a> {
    pub depth: Depth,
    pub time: Seconds,
    pub gas: &'a Gas,
}
