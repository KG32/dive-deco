use crate::common::deco::{DecoCalculationError, DecoRuntime};
use crate::common::global_types::{CeilingType, MbarPressure};
use crate::common::ox_tox::OxTox;
use crate::common::{AscentRatePerMinute, Cns, Gas, Otu};
use crate::common::{Depth, Time};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConfigValidationErr {
    pub field: String,
    pub reason: String,
}

impl ConfigValidationErr {
    pub fn new(field: &str, reason: &str) -> Self {
        Self {
            field: String::from(field),
            reason: String::from(reason),
        }
    }
}

pub trait DecoModelConfig {
    fn validate(&self) -> Result<(), ConfigValidationErr>;
    fn surface_pressure(&self) -> MbarPressure;
    fn deco_ascent_rate(&self) -> AscentRatePerMinute;
    fn ceiling_type(&self) -> CeilingType;
    fn round_ceiling(&self) -> bool;
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiveState {
    pub depth: Depth,
    pub time: Time,
    pub gas: Gas,
    pub ox_tox: OxTox,
}

pub trait DecoModel {
    type ConfigType: DecoModelConfig;

    // default
    fn default() -> Self;

    /// model init
    fn new(config: Self::ConfigType) -> Self;

    /// get model config
    fn config(&self) -> Self::ConfigType;

    /// get model dive state
    fn dive_state(&self) -> DiveState;

    /// record (depth: meters, time: seconds)
    fn record(&mut self, depth: Depth, time: Time, gas: &Gas);

    /// record linear ascent / descent record given travel time
    fn record_travel(&mut self, target_depth: Depth, time: Time, gas: &Gas);

    /// register linear ascent / descent record given rate
    fn record_travel_with_rate(
        &mut self,
        target_depth: Depth,
        rate: AscentRatePerMinute,
        gas: &Gas,
    );

    /// current non decompression limit (NDL)
    fn ndl(&self) -> Time;

    /// current decompression ceiling in meters
    fn ceiling(&self) -> Depth;

    /// deco stages, TTL
    fn deco(&self, gas_mixes: Vec<Gas>) -> Result<DecoRuntime, DecoCalculationError>;

    /// central nervous system oxygen toxicity
    fn cns(&self) -> Cns;

    /// pulmonary oxygen toxicity
    fn otu(&self) -> Otu;

    /// is in deco check
    fn in_deco(&self) -> bool {
        let ceiling_type = self.config().ceiling_type();
        match ceiling_type {
            CeilingType::Actual => self.ceiling() > Depth::zero(),
            CeilingType::Adaptive => {
                let current_gas = self.dive_state().gas;
                let runtime = self.deco(vec![current_gas]).unwrap();
                let deco_stages = runtime.deco_stages;
                deco_stages.len() > 1
            }
        }
    }
}
