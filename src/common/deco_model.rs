use crate::common::{Depth, Gas, Seconds, Minutes, AscentRatePerMinute, CNSPercent};
use super::{ox_tox::OxTox, Deco, DecoRuntime, MbarPressure};

#[derive(Debug, PartialEq)]
pub struct ConfigValidationErr<'a> {
    pub field: &'a str,
    pub reason: &'a str,
}

pub trait DecoModelConfig {
    fn validate(&self) -> Result<(), ConfigValidationErr>;
    fn surface_pressure(&self) -> MbarPressure;
}

#[derive(Debug)]
pub struct DiveState {
    pub depth: Depth,
    pub time: Seconds,
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

    /// register step (depth: meters, time: seconds)
    fn step(&mut self, depth: Depth, time: Seconds, gas: &Gas);

    /// register linear ascent / descent step given travel time
    fn step_travel(&mut self, target_depth: Depth, time: Seconds, gas: &Gas);

    /// register linear ascent / descent step given rate
    fn step_travel_with_rate(&mut self, target_depth: Depth, rate: AscentRatePerMinute, gas: &Gas);

    /// current non decompression limit (NDL)
    fn ndl(&self) -> Minutes;

    /// current decompression ceiling in meters
    fn ceiling(&self) -> Depth;

    /// deco stages, TTL
    fn deco(&self, gas_mixes: Vec<Gas>) -> DecoRuntime;

    /// central nervous system oxygen toxicity
    fn cns(&self) -> CNSPercent;

    /// is in deco check
    fn in_deco(&self) -> bool {
        self.ceiling() > 0.
    }

}
