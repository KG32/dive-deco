use crate::common::{BreathingSource, Depth, Time};

#[derive(Debug)]
pub struct RecordData<'a> {
    pub depth: Depth,
    pub time: Time,
    pub gas: &'a BreathingSource,
}
