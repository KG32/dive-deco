use crate::common::{Gas, Seconds};

use super::Depth;

#[derive(Debug)]
pub struct RecordData<'a> {
    pub depth: Depth,
    pub time: Seconds,
    pub gas: &'a Gas,
}
