use crate::{DecoModel, Depth, Gas, Minutes};

use super::{AscentRatePerMinute, DecoModelConfig, DiveState, MbarPressure};

const DEFAULT_ASCENT_RATE: AscentRatePerMinute = 9.;
// todo move to model config
const DEFAULT_CEILING_WINDOW: Depth = 3.;

#[derive(Copy, Clone, Debug, PartialEq)]
enum DecoAction {
    AscentToCeil,
    AscentToGasSwitchDepth,
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
    pub duration: Minutes,
    pub gas: Gas,
}

#[derive(Clone, Debug)]
pub struct DecoRuntime {
    pub deco_stages: Vec<DecoStage>,
    pub tts: Minutes,
}

impl DecoRuntime {
    pub fn new() -> Self {
        Self {
            deco_stages: vec![],
            tts: 0,
        }
    }

    pub fn calc(&mut self, mut sim_model: impl DecoModel, gas_mixes: Vec<Gas>) -> Self {
        // loop until no deco action
        loop {
            let DiveState {
                depth: pre_stage_depth,
                time: pre_stage_time,
                gas: pre_stage_gas
            } = sim_model.dive_state();
            let ceiling = sim_model.ceiling();
            let next_deco_stage = self.next_deco_action(&sim_model, gas_mixes.clone());
            let (deco_action, next_switch_gas) = next_deco_stage;
            let mut deco_stages: Vec<DecoStage> = vec![];
            match deco_action {
                None => { break; },
                Some(deco_action) => {
                    match deco_action {
                        // ascent to min depth (deco stop or surface)
                        DecoAction::AscentToCeil => {
                            sim_model.step_travel_with_rate(&self.round_ceiling(&ceiling), &DEFAULT_ASCENT_RATE, &pre_stage_gas);
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
                                // travel to gas MOD, switch to gas and continue ascent to ceiling

                                // travel to MOD
                                let switch_gas_mod = next_switch_gas.max_operating_depth(1.6);
                                sim_model.step_travel_with_rate(&switch_gas_mod, &DEFAULT_ASCENT_RATE, &pre_stage_gas);
                                let state_after_travel_to_mod = sim_model.dive_state();
                                let gas_switch_depth = state_after_travel_to_mod.depth;
                                deco_stages.push(DecoStage {
                                    stage_type: DecoStageType::Ascent,
                                    start_depth: pre_stage_depth,
                                    end_depth: gas_switch_depth,
                                    duration: state_after_travel_to_mod.time - pre_stage_time,
                                    gas: state_after_travel_to_mod.gas,
                                });

                                // switch gas @todo configurable gas change duration?
                                sim_model.step(&sim_model.dive_state().depth, &(1 * 60), &next_switch_gas);
                                // @todo optional oxygen window stop?
                                deco_stages.push(DecoStage {
                                    stage_type: DecoStageType::GasSwitch,
                                    start_depth: gas_switch_depth,
                                    end_depth: gas_switch_depth,
                                    duration: 0,
                                    gas: sim_model.dive_state().gas,
                                });
                            }
                        },
                        // deco stop, stop
                        DecoAction::Stop => {
                            sim_model.step(&pre_stage_depth, &1, &pre_stage_gas);
                            let sim_state = sim_model.dive_state();
                            // @todo dedupe here on deco instead of of add deco stage?
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
            // add new deco stages from iteration to deco runtime stages
            deco_stages.into_iter().for_each(|deco_stage| self.add_deco_stage(deco_stage));
        }

        Self {
            deco_stages: self.deco_stages.clone(),
            tts: self.tts
        }
    }

    fn next_deco_action(&self, sim_model: &impl DecoModel, gas_mixes: Vec<Gas>) -> (Option<DecoAction>, Option<Gas>) {
        let DiveState { depth: current_depth, gas: current_gas, .. } = sim_model.dive_state();
        let surface_pressure = sim_model.config().surface_pressure();

        // end deco simulation - surface
        if current_depth == 0. {
            return (None, None);
        }

        if !sim_model.in_deco() {
            // no deco obligations - linear ascent
            return (Some(DecoAction::AscentToCeil), None);
        } else {
            // check check for next switch gas
            let next_switch_gas = self.next_switch_gas(&current_depth, &current_gas, gas_mixes, surface_pressure);
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
    fn next_switch_gas(&self, current_depth: &Depth, current_gas: &Gas, gas_mixes: Vec<Gas>, surface_pressure: MbarPressure) -> Option<Gas> {
        let current_gas_partial_pressures = current_gas.partial_pressures(&current_depth, surface_pressure.clone());
        let mut next_switch_gasses = gas_mixes
            .into_iter()
            .filter(|gas| {
                // switch gas is more oxygen-rich than current and not TMX
                let partial_pressures = gas.partial_pressures(&current_depth, surface_pressure);
                partial_pressures.o2 > current_gas_partial_pressures.o2
                && partial_pressures.he == 0.
            })
            .collect::<Vec<Gas>>();

            next_switch_gasses.sort_by(|a, b| {
                let x = a.gas_pressures_compound(1.);
                let y = b.gas_pressures_compound(1.);
                x.o2.partial_cmp(&y.o2).unwrap()
            });

            next_switch_gasses.first().copied()
    }

    fn add_deco_stage(&mut self, stage: DecoStage) {
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

        self.tts += stage.duration;
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
            (20., air, vec![air, trimix], None),
        ];

        let runtime = DecoRuntime::new();
        for case in test_cases.into_iter() {
            dbg!(&case);
            let (current_depth, current_gas, available_gas_mixes, expected_switch_gas) = case;
            let res = runtime.next_switch_gas(&current_depth, &current_gas, available_gas_mixes, 1000);
            assert_eq!(res, expected_switch_gas);
        }
    }
}
