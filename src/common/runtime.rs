use std::cmp::Ordering;

use crate::{DecoModel, Depth, Gas, Minutes};

use super::{AscentRatePerMinute, DecoModelConfig, DiveState, MbarPressure};

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

    pub fn calc(&mut self, mut sim_model: impl DecoModel, gas_mixes: Vec<Gas>) -> Self {
        loop {
            let DiveState {
                depth: pre_event_depth,
                time: pre_event_time,
                gas: pre_event_gas
            } = sim_model.dive_state();
            let ceiling = sim_model.ceiling();
            let next_event_res = self.next_event_type(&sim_model, gas_mixes.clone());
            let (next_event_type, switch_gas) = next_event_res;
            let mut new_deco_events: Vec<DecoEvent> = vec![];
            match next_event_type {
                None => { break; },
                Some(event_type) => {
                    match event_type {
                        DecoEventType::Ascent => {
                            sim_model.step_travel_with_rate(&self.round_ceiling(&ceiling), &DEFAULT_ASCENT_RATE, &pre_event_gas);
                            let current_sim_state = sim_model.dive_state();
                            let current_sim_time = current_sim_state.time;
                            new_deco_events.push(DecoEvent {
                                event_type,
                                start_depth: pre_event_depth,
                                end_depth: current_sim_state.depth,
                                duration: current_sim_time - pre_event_time,
                                gas: current_sim_state.gas,
                            })
                        },
                        DecoEventType::Stop => {
                            sim_model.step(&pre_event_depth, &1, &pre_event_gas);
                            let sim_state = sim_model.dive_state();
                            new_deco_events.push(DecoEvent {
                                event_type,
                                start_depth: pre_event_depth,
                                end_depth: sim_state.depth,
                                duration: sim_state.time - pre_event_time,
                                gas: sim_state.gas,
                            })
                        },
                        DecoEventType::GasSwitch => {
                            let switch_gas = switch_gas.unwrap();
                            let switch_depth = switch_gas.max_operating_depth(1.6);


                            new_deco_events.push(DecoEvent {
                                event_type,
                                start_depth: pre_event_depth,
                                end_depth: pre_event_depth,
                                duration: 0,
                                gas: switch_gas
                            })
                        },
                    }
                }
            }
            // add new deco events from iteration to deco runtime events
            new_deco_events.into_iter().for_each(|deco_event| self.add_deco_event(deco_event));
        }

        Self {
            deco_events: self.deco_events.clone(),
            tts: self.tts
        }
    }

    fn next_event_type(&self, sim_model: &impl DecoModel, gas_mixes: Vec<Gas>) -> (Option<DecoEventType>, Option<Gas>) {
        let DiveState { depth: current_depth, gas: current_gas, .. } = sim_model.dive_state();
        let surface_pressure = sim_model.config().surface_pressure();

        // end sim - surface
        if current_depth == 0. {
            return (None, None);
        }

        if !sim_model.in_deco() {
            // no deco obligations - linear ascent
            return (Some(DecoEventType::Ascent), None);
        } else {
            // check if gas switch needed
            let last_event = self.deco_events.last();
            if last_event.is_none() || (last_event.is_some() && last_event.unwrap().event_type == DecoEventType::Ascent) {
                let optimal_switch_gas = self.available_optimal_deco_switch_gas(&current_depth, &current_gas, gas_mixes, surface_pressure);
                if let Some(switch_gas) = optimal_switch_gas {
                    return (Some(DecoEventType::GasSwitch), Some(switch_gas));
                }
            }

            // deco obligations - ascent + stops
            let ceiling = sim_model.ceiling();
            let ceiling_padding = current_depth - ceiling;
            // within deco window
            if ceiling_padding <= DEFAULT_CEILING_WINDOW {
                return (Some(DecoEventType::Stop), None);
            }
            // below deco window
            else if ceiling_padding > DEFAULT_CEILING_WINDOW {
                return (Some(DecoEventType::Ascent), None);
            }
            // @todo panic if deco stop violated?
        }

        (None, None)
    }

    fn available_optimal_deco_switch_gas(&self, current_depth: &Depth, current_gas: &Gas, gas_mixes: Vec<Gas>, surface_pressure: MbarPressure) -> Option<Gas> {
        let current_gas_partial_pressures = current_gas.partial_pressures(&current_depth, surface_pressure.clone());
        let mut switch_candidates = gas_mixes
            .into_iter()
            .filter(|gas| {
                // switch gas MOD suitable for current depth
                // ppO2 1.6 as in deco, @todo configurable max ppO2 in deco
                gas.max_operating_depth(1.6) >= *current_depth
            })
            .filter(|gas| {
                // switch gas is more oxygen-rich than current and not TMX
                let partial_pressures = gas.partial_pressures(&current_depth, surface_pressure);
                partial_pressures.o2 > current_gas_partial_pressures.o2
                && partial_pressures.he == 0.
            })
            .collect::<Vec<Gas>>();

            switch_candidates.sort_by(|a, b| {
                let x = a.partial_pressures(&current_depth, surface_pressure);
                let y = b.partial_pressures(&current_depth, surface_pressure);
                x.o2.partial_cmp(&y.o2).unwrap()
            });

            switch_candidates.first().copied()
    }

    fn add_deco_event(&mut self, event: DecoEvent) {
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

    #[test]
    fn test_potential_switch_gasses() {
        let air = Gas::air();
        let ean_50 = Gas::new(0.5, 0.);
        let oxygen = Gas::new(1., 0.);
        let trimix = Gas::new(0.5, 0.2);

        // potential switch if in deco!
        // [ (current_depth, current_gas, gas_mixes, expected_result) ]
        let test_cases: Vec<(Depth, Gas, Vec<Gas>, Option<Gas>)> = vec![
            // single gas air
            (10., air, vec![air], None),
            // air + ean50 within MOD
            (10., air, vec![air, ean_50], Some(ean_50)),
            // air + ean50 over MOD
            (30., air, vec![air, ean_50], None),
            // air + ean50 + oxygen, ean50 withing MOD, oxygen out
            (20., air, vec![air, ean_50, oxygen], Some(ean_50)),
            // air + ean50 + oxy, deco on ean50, oxygen within MOD
            (5.5, ean_50, vec![air, ean_50, oxygen], Some(oxygen)),
            // air + heliox within o2 MOD, not considered deco gas
            (20., air, vec![air, trimix], None),
        ];

        let runtime = DecoRuntime::new();
        for case in test_cases.into_iter() {
            let (current_depth, current_gas, available_gas_mixes, expected_switch_gas) = case;
            let res = runtime.available_optimal_deco_switch_gas(&current_depth, &current_gas, available_gas_mixes, 1000);
            assert_eq!(res, expected_switch_gas);
        }
    }
}
