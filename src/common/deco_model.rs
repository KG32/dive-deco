use crate::common::{AscentRatePerMinute, CNSPercent, Depth, Gas, Minutes, Seconds};
use super::{global_types::NDLType, ox_tox::OxTox, DecoRuntime, MbarPressure};

#[derive(Debug, PartialEq)]
pub struct ConfigValidationErr<'a> {
    pub field: &'a str,
    pub reason: &'a str,
}

pub trait DecoModelConfig {
    fn validate(&self) -> Result<(), ConfigValidationErr>;
    fn surface_pressure(&self) -> MbarPressure;
    fn deco_ascent_rate(&self) -> AscentRatePerMinute;
    fn ndl_type(&self) -> NDLType;
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

    /// record (depth: meters, time: seconds)
    fn record(&mut self, depth: Depth, time: Seconds, gas: &Gas);

    /// record linear ascent / descent record given travel time
    fn record_travel(&mut self, target_depth: Depth, time: Seconds, gas: &Gas);

    /// register linear ascent / descent record given rate
    fn record_travel_with_rate(&mut self, target_depth: Depth, rate: AscentRatePerMinute, gas: &Gas);

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
        let ndl_type_config = self.config().ndl_type();
        match ndl_type_config {
            NDLType::Actual => {
                let current_gas = self.dive_state().gas;
                let runtime = self.deco(vec![current_gas]);
                let deco_stages = runtime.deco_stages;
                deco_stages.len() > 1

            },
            NDLType::ByCeiling => self.ceiling() > 0.,
        }
    }



    #[deprecated(since="3.0.0", note="Use record method instead")]
    fn step(&mut self, depth: Depth, time: Seconds, gas: &Gas);

    #[deprecated(since="3.0.0", note="Use record_travel method instead")]
    fn step_travel(&mut self, target_depth: Depth, time: Seconds, gas: &Gas);

    #[deprecated(since="3.0.0", note="Use record_travel_with_rate method instead")]
    fn step_travel_with_rate(&mut self, target_depth: Depth, rate: AscentRatePerMinute, gas: &Gas);
}
