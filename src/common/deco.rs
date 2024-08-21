use crate::{DecoModel, Depth, Gas, Minutes};
use super::{global_types::MinutesSigned, AscentRatePerMinute, DecoModelConfig, DiveState, MbarPressure, Seconds};

// @todo move to model config
const DEFAULT_ASCENT_RATE: AscentRatePerMinute = 9.;
const DEFAULT_CEILING_WINDOW: Depth = 3.;
const DEFAULT_MAX_END_DEPTH: Depth = 30.;

#[derive(Copy, Clone, Debug, PartialEq)]
enum DecoAction {
    AscentToCeil,
    AscentToGasSwitchDepth,
    SwitchGas,
    Stop,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DecoStageType {
    Ascent,
    DecoStop,
    GasSwitch
}

#[derive(Copy, Clone, Debug)]
pub struct DecoStage {
    pub stage_type: DecoStageType,
    pub start_depth: Depth,
    pub end_depth: Depth,
    pub duration: Seconds,
    pub gas: Gas,
}

#[derive(Clone, Debug)]
#[derive(Default)]
pub struct Deco {
    deco_stages: Vec<DecoStage>,
    tts_seconds: Seconds,
    sim: bool,
}


#[derive(Debug)]
pub struct DecoRuntime {
    // runtime
    pub deco_stages: Vec<DecoStage>,
    // current TTS in minutes
    pub tts: Minutes,
    // TTS @+5 (TTS in 5 min given current depth and gas mix)
    pub tts_at_5: Minutes,
    // TTS Δ+5 (absolute change in TTS after 5 mins given current depth and gas mix)
    pub tts_delta_at_5: MinutesSigned,
}

impl Deco {
    pub fn new_sim() -> Self {
        let deco = Self::default();
        deco.fork()
    }

    pub fn calc<T: DecoModel + Clone>(&mut self, deco_model: T, gas_mixes: Vec<Gas>) -> DecoRuntime {
        // run model simulation until no deco stages
        let mut sim_model = deco_model.clone();
        loop {
            let DiveState {
                depth: pre_stage_depth,
                time: pre_stage_time,
                gas: pre_stage_gas,
                ..
            } = sim_model.dive_state();
            let ceiling = sim_model.ceiling();
            let next_deco_stage = self.next_deco_action(&sim_model, gas_mixes.clone());
            let (deco_action, next_switch_gas) = next_deco_stage;
            let mut deco_stages: Vec<DecoStage> = vec![];

            // handle deco actions
            match deco_action {
                // deco obligation cleared
                None => { break; },

                // handle mandatory deco stage
                Some(deco_action) => {
                    match deco_action {
                        // ascent to min depth (deco stop or surface)
                        DecoAction::AscentToCeil => {
                            sim_model.step_travel_with_rate(self.deco_stop_depth(ceiling), DEFAULT_ASCENT_RATE, &pre_stage_gas);
                            let current_sim_state = sim_model.dive_state();
                            let current_sim_time = current_sim_state.time;
                            deco_stages.push(DecoStage {
                                stage_type: DecoStageType::Ascent,
                                start_depth: pre_stage_depth,
                                end_depth: current_sim_state.depth,
                                duration: current_sim_time - pre_stage_time,
                                gas: current_sim_state.gas,
                            })
                        },

                        // ascent to min depth with gas switch on next deco gas maximum operating depth
                        DecoAction::AscentToGasSwitchDepth => {
                            // @todo unwrap and handler err
                            if let Some(next_switch_gas) = next_switch_gas {
                                // travel to MOD
                                let switch_gas_mod = next_switch_gas.max_operating_depth(1.6);
                                sim_model.step_travel_with_rate(switch_gas_mod, DEFAULT_ASCENT_RATE, &pre_stage_gas);
                                let DiveState {
                                    depth: post_ascent_depth,
                                    time: post_ascent_time,
                                    ..
                                } = sim_model.dive_state();
                                deco_stages.push(DecoStage {
                                    stage_type: DecoStageType::Ascent,
                                    start_depth: pre_stage_depth,
                                    end_depth: post_ascent_depth,
                                    duration: post_ascent_time - pre_stage_time,
                                    gas: pre_stage_gas,
                                });

                                // switch gas @todo configurable gas change duration
                                sim_model.step(sim_model.dive_state().depth, 0, &next_switch_gas);
                                // @todo configurable oxygen window stop
                                let post_switch_state = sim_model.dive_state();
                                deco_stages.push(DecoStage {
                                    stage_type: DecoStageType::GasSwitch,
                                    start_depth: post_ascent_depth,
                                    end_depth: post_switch_state.depth,
                                    duration: 0,
                                    gas: next_switch_gas,
                                });
                            }
                        },

                        // switch gas without ascent
                        DecoAction::SwitchGas => {
                            let switch_gas = next_switch_gas.unwrap();
                            // @todo configurable gas switch duration
                            sim_model.step(pre_stage_depth, 0, &switch_gas);
                            deco_stages.push(DecoStage {
                                stage_type: DecoStageType::GasSwitch,
                                start_depth: pre_stage_depth,
                                end_depth: pre_stage_depth,
                                duration: 0,
                                gas: switch_gas,
                            })
                        },

                        // decompression stop (a series of 1s segments, merged into one on cleared stop)
                        DecoAction::Stop => {
                            sim_model.step(pre_stage_depth, 1, &pre_stage_gas);
                            let sim_state = sim_model.dive_state();
                            // @todo dedupe here on deco instead of of add deco
                            deco_stages.push(DecoStage {
                                stage_type: DecoStageType::DecoStop,
                                start_depth: pre_stage_depth,
                                end_depth: sim_state.depth,
                                duration: sim_state.time - pre_stage_time,
                                gas: sim_state.gas,
                            })
                        }
                    }
                }
            }
            // register deco stages
            deco_stages.into_iter().for_each(|deco_stage| self.register_deco_stage(deco_stage));
        }

        let tts = (self.tts_seconds as f64 / 60.).ceil() as Minutes;
        let mut tts_at_5 = 0;
        let mut tts_delta_at_5: MinutesSigned = 0;
        if !self.sim {
            let mut nested_sim_deco = Deco::new_sim();
            let mut nested_sim_model = deco_model.clone();
            let DiveState { depth: sim_depth, gas: sim_gas, .. } = nested_sim_model.dive_state();
            nested_sim_model.step(sim_depth, 5 * 60, &sim_gas);
            let nested_deco = nested_sim_deco.calc(nested_sim_model, gas_mixes.clone());
            tts_at_5 = nested_deco.tts;
            tts_delta_at_5 = (tts_at_5 - tts) as MinutesSigned;
        }

        DecoRuntime {
            deco_stages: self.deco_stages.clone(),
            tts,
            tts_at_5,
            tts_delta_at_5,
        }
    }

    fn next_deco_action(&self, sim_model: &impl DecoModel, gas_mixes: Vec<Gas>) -> (Option<DecoAction>, Option<Gas>) {
        let DiveState { depth: current_depth, gas: current_gas, .. } = sim_model.dive_state();
        let surface_pressure = sim_model.config().surface_pressure();

        // end deco simulation - surface
        if current_depth <= 0. {
            return (None, None);
        }

        if !sim_model.in_deco() {
            // no deco obligations - linear ascent
            return (Some(DecoAction::AscentToCeil), None);
        } else {
            // check next switch gas
            let next_switch_gas = self.next_switch_gas(current_depth, &current_gas, gas_mixes, surface_pressure);
            // check if within mod @todo min operational depth
            if let Some(switch_gas) = next_switch_gas {
                //switch gas without ascent if within mod of next deco gas
                let gas_mod = switch_gas.max_operating_depth(1.6);
                let gas_end = switch_gas.equivalent_narcotic_depth(current_depth);
                if (switch_gas != current_gas) && (current_depth <= gas_mod) && (gas_end <= DEFAULT_MAX_END_DEPTH) {
                    return (Some(DecoAction::SwitchGas), Some(switch_gas));
                }
            }

            let ceiling = sim_model.ceiling();
            let ceiling_padding = current_depth - ceiling;

            // within deco window
            if ceiling_padding <= DEFAULT_CEILING_WINDOW {
                return (Some(DecoAction::Stop), None);
            }
            // below deco window
            else if ceiling_padding > DEFAULT_CEILING_WINDOW {
                // ascent to next gas switch depth if next gas' MOD below ceiling
                if let Some(next_switch_gas) = next_switch_gas {
                    return (Some(DecoAction::AscentToGasSwitchDepth), Some(next_switch_gas));
                }
                return (Some(DecoAction::AscentToCeil), None);
            }
            // @todo panic if deco stop violated?
        }

        (None, None)
    }

    /// check next deco gas in deco (the one with lowest MOD while more oxygen-rich than current)
    fn next_switch_gas(&self, current_depth: Depth, current_gas: &Gas, gas_mixes: Vec<Gas>, surface_pressure: MbarPressure) -> Option<Gas> {
        let current_gas_partial_pressures = current_gas.partial_pressures(current_depth, surface_pressure);
        // all potential deco gases that are more oxygen-rich than current (inc. trimix / heliox)
        let mut switch_gasses = gas_mixes
            .into_iter()
            .filter(|gas| {
                let partial_pressures = gas.partial_pressures(current_depth, surface_pressure);
                partial_pressures.o2 > current_gas_partial_pressures.o2
            })
            .collect::<Vec<Gas>>();

        // sort deco gasses by o2 content
        switch_gasses.sort_by(|a, b| {
            let x = a.gas_pressures_compound(1.);
            let y = b.gas_pressures_compound(1.);
            x.o2.partial_cmp(&y.o2).unwrap()
        });

        // mix with lowest MOD (by absolute o2 content)
        switch_gasses.first().copied()
    }

    fn register_deco_stage(&mut self, stage: DecoStage) {
        let push_new_stage = match stage.stage_type {
            // dedupe iterative deco stops and merge into one
            DecoStageType::DecoStop => {
                let mut push_new = true;
                let last_stage = self.deco_stages.last_mut();
                if let Some(last_stage) = last_stage {
                    if last_stage.stage_type == stage.stage_type {
                        last_stage.duration += stage.duration;
                        push_new = false;
                    }
                }
                push_new
            },
            _ => true
        };

        if push_new_stage {
            self.deco_stages.push(stage);
        }

        // increment TTS by deco stage duration
        self.tts_seconds += stage.duration;
    }

    // round ceiling up to the bottom of deco window
    fn deco_stop_depth(&self, ceiling: Depth) -> Depth {
        DEFAULT_CEILING_WINDOW * (ceiling / DEFAULT_CEILING_WINDOW).ceil()
    }

    fn fork(&self) -> Self {
        Self {
            sim: true,
            ..self.clone()
        }
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
        let deco = Deco::default();
        for case in test_cases.into_iter() {
            let (input_depth, expected_depth) = case;
            let res = deco.deco_stop_depth(input_depth);
            assert_eq!(res, expected_depth);
        }
    }

    #[test]
    fn test_next_switch_gas() {
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
            (30., air, vec![air, ean_50], Some(ean_50)),
            // air + ean50 + oxygen, ean50 withing MOD, oxygen out
            (20., air, vec![air, ean_50, oxygen], Some(ean_50)),
            // air + ean50 + oxy, deco on ean50, oxygen within MOD
            (5.5, ean_50, vec![air, ean_50, oxygen], Some(oxygen)),
            // air + heliox within o2 MOD, not considered deco gas
            (30., air, vec![air, trimix], Some(trimix)),
        ];

        let deco = Deco::default();
        for case in test_cases.into_iter() {
            let (current_depth, current_gas, available_gas_mixes, expected_switch_gas) = case;
            let res = deco.next_switch_gas(current_depth, &current_gas, available_gas_mixes, 1000);
            assert_eq!(res, expected_switch_gas);
        }
    }
}
