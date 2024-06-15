use std::cmp::Ordering;

use crate::{DecoModel, Depth, Gas, Minutes};

use super::{AscentRatePerMinute, DiveState};

const DEFAULT_ASCENT_RATE: AscentRatePerMinute = 9.;
// todo move to model config
const DEFAULT_CEILING_WINDOW: Depth = 3.;


#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DecoEventType {
    Ascent,
    Stop,
    GasSwitch
}

#[derive(Copy, Clone, Debug)]
pub struct DecoEvent {
    pub event_type: DecoEventType,
    pub start_depth: Depth,
    pub end_depth: Depth,
    pub duration: Minutes,
    pub gas: Gas,
}

#[derive(Clone, Debug)]
pub struct DecoRuntime {
    pub deco_events: Vec<DecoEvent>,
    pub tts: Minutes,
}

impl DecoRuntime {
    pub fn new() -> Self {
        Self {
            deco_events: vec![],
            tts: 0,
        }
    }

    pub fn calc(&mut self, mut sim_model: impl DecoModel, gas_mixes: Vec<&Gas>) -> Self {
        loop {
            let DiveState {
                depth: pre_event_depth,
                time: pre_event_time,
                gas: pre_event_gas
            } = sim_model.dive_state();
            let ceiling = sim_model.ceiling();
            let next_event_type = self.next_event_type(&sim_model, gas_mixes.clone());
            let deco_event: DecoEvent;
            match next_event_type {
                None => { break; },
                Some(event_type) => {
                    match event_type {
                        DecoEventType::Ascent => {
                            sim_model.step_travel_with_rate(&self.round_ceiling(&ceiling), &DEFAULT_ASCENT_RATE, &pre_event_gas);
                            let current_sim_state = sim_model.dive_state();
                            let current_sim_time = current_sim_state.time;
                            deco_event = DecoEvent {
                                event_type,
                                start_depth: pre_event_depth,
                                end_depth: current_sim_state.depth,
                                duration: current_sim_time - pre_event_time,
                                gas: current_sim_state.gas,
                            }
                        },
                        DecoEventType::Stop => {
                            sim_model.step(&pre_event_depth, &1, &pre_event_gas);
                            let sim_state = sim_model.dive_state();
                            deco_event = DecoEvent {
                                event_type,
                                start_depth: pre_event_depth,
                                end_depth: sim_state.depth,
                                duration: sim_state.time - pre_event_time,
                                gas: sim_state.gas,
                            }
                        },
                        DecoEventType::GasSwitch => todo!(),
                    }
                }
            }
            self.add_deco_event(deco_event);
        }

        Self {
            deco_events: self.deco_events.clone(),
            tts: self.tts
        }
    }

    fn next_event_type(&self, sim_model: &impl DecoModel, _gas_mixes: Vec<&Gas>) -> Option<DecoEventType> {
        let DiveState { depth, .. } = sim_model.dive_state();


        // end sim - surface
        if depth == 0. {
            return None;
        }

        if !sim_model.in_deco() {
            // no deco obligations - linear ascent
            return Some(DecoEventType::Ascent);
        } else {
            // check if gas switch needed


            // deco obligations - ascent + stops
            let ceiling = sim_model.ceiling();
            let ceiling_padding = depth - ceiling;
            // within deco window
            if ceiling_padding <= DEFAULT_CEILING_WINDOW {
                return Some(DecoEventType::Stop);
            }
            // below deco window
            else if ceiling_padding > DEFAULT_CEILING_WINDOW {
                return Some(DecoEventType::Ascent);
            }
            // @todo panic if deco stop violated?
        }



        None
    }

    fn add_deco_event(&mut self, event: DecoEvent) {
        // let runtime_events = &self.runtime_events;
        // let DecoEvent { event_type, .. } = event;
        let push_new_event = match event.event_type {
            DecoEventType::Stop => {
                let mut push_new = true;
                let last_event = self.deco_events.last_mut();
                if let Some(last_event) = last_event {
                    if last_event.event_type == event.event_type {
                        last_event.duration += event.duration;
                        push_new = false;
                    }
                }
                push_new
            },
            _ => true
        };

        if push_new_event {
            self.deco_events.push(event);
        }

        self.tts += event.duration;
    }

    fn round_ceiling(&self, ceiling: &Depth) -> Depth {
        let step_size: Depth = 3.;
        step_size * (ceiling / step_size).ceil()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ceiling_rounding() {
        let test_cases: Vec<(Depth, Depth)> = vec![
            (0., 0.),
            (2., 3.),
            (2.999, 3.),
            (3., 3.),
            (3.00001, 6.),
            (12., 12.),
        ];
        let runtime = DecoRuntime::new();
        for case in test_cases.into_iter() {
            let (input_depth, expected_depth) = case;
            let res = runtime.round_ceiling(&input_depth);
            assert_eq!(res, expected_depth);
        }
    }
}
