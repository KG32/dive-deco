use crate::{BuehlmannModel, DecoModel, Depth, Gas, Minutes};

use super::{AscentRatePerMinute, DiveState};

const DEFAULT_ASCENT_RATE: AscentRatePerMinute = 9.;


#[derive(Copy, Clone, Debug)]
pub enum DecoEventType {
    Ascent,
    Stop,
    GasSwitch
}

#[derive(Copy, Clone, Debug)]
pub struct DecoEvent {
    pub event_type: DecoEventType,
    pub end_depth: Depth,
    pub duration: Minutes,
    pub gas: Gas,
}

#[derive(Clone, Debug)]
pub struct Runtime {
    pub runtime_events: Vec<DecoEvent>,
    pub tts: Minutes,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            runtime_events: vec![],
            tts: 0,
        }
    }

    pub fn calc(&mut self, mut sim_model: impl DecoModel, gas_mixes: Vec<&Gas>) -> Self {
        loop {
            let DiveState { time: initial_time, .. } = sim_model.dive_state();
            let next_event_type = self.next_event_type(&sim_model, gas_mixes.clone());
            let deco_event: DecoEvent;
            match next_event_type {
                None => { break; },
                Some(event_type) => {
                    match event_type {
                        DecoEventType::Ascent => {
                            sim_model.step_travel_with_rate(&0., &DEFAULT_ASCENT_RATE, &sim_model.dive_state().gas);
                            let current_sim_state = sim_model.dive_state();
                            let current_sim_time = current_sim_state.time;
                            deco_event = DecoEvent {
                                event_type,
                                end_depth: 0.,
                                duration: current_sim_time - initial_time,
                                gas: current_sim_state.gas,
                            }
                        },
                        DecoEventType::Stop => todo!(),
                        DecoEventType::GasSwitch => todo!(),
                    }
                }
            }
            self.add_deco_event(deco_event);
        }

        Self {
            runtime_events: self.runtime_events.clone(),
            tts: self.tts
        }
    }

    fn next_event_type(&self, model: &impl DecoModel, _gas_mixes: Vec<&Gas>) -> Option<DecoEventType> {
        let DiveState { depth, .. } = model.dive_state();

        if depth == 0. {
            return None;
        }

        if !model.in_deco() {
            return Some(DecoEventType::Ascent);
        }

        None
    }

    fn add_deco_event(&mut self, event: DecoEvent) {
        self.runtime_events.push(event);
        self.tts += event.duration;
    }
}
