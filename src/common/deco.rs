use alloc::vec;
use alloc::vec::Vec;
use core::{cmp::Ordering, fmt};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::common::BreathingSource;
use crate::{DecoModel, Depth, DepthType, Time};

use super::{ceil, DecoModelConfig, DiveState, MbarPressure, Sim};

// @todo move to model config
const DEFAULT_CEILING_WINDOW: DepthType = 3.;
const DEFAULT_MAX_END_DEPTH: DepthType = 30.;

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
enum DecoAction {
    AscentToCeil,
    AscentToGasSwitchDepth,
    SwitchGas,
    Stop,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DecoStageType {
    Ascent,
    DecoStop,
    GasSwitch,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DecoStage {
    pub stage_type: DecoStageType,
    pub start_depth: Depth,
    pub end_depth: Depth,
    pub duration: Time,
    pub gas: BreathingSource,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Deco {
    deco_stages: Vec<DecoStage>,
    tts: Time,
    sim: bool,
}

#[derive(Debug, PartialEq, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DecoRuntime {
    // runtime
    pub deco_stages: Vec<DecoStage>,
    // current TTS in minutes
    pub tts: Time,
    // TTS @+5 (TTS in 5 min given current depth and gas mix)
    pub tts_at_5: Time,
    // TTS Δ+5 (absolute change in TTS after 5 mins given current depth and gas mix)
    pub tts_delta_at_5: Time,
}

#[derive(Debug)]
struct MissedDecoStopViolation;

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DecoCalculationError {
    EmptyGasList,
    CurrentGasNotInList,
}

impl fmt::Display for DecoCalculationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            DecoCalculationError::EmptyGasList => {
                write!(f, "At least one available gas mix required")
            }
            DecoCalculationError::CurrentGasNotInList => write!(
                f,
                "Available gas mixes must include current gas mix used by deco model"
            ),
        }
    }
}

impl Sim for Deco {
    fn fork(&self) -> Self {
        Self {
            sim: true,
            ..self.clone()
        }
    }
    fn is_sim(&self) -> bool {
        self.sim
    }
}

impl Deco {
    pub fn new_sim() -> Self {
        let deco = Self::default();
        deco.fork()
    }

    pub fn calc<T: DecoModel + Clone + Sim>(
        &mut self,
        deco_model: T,
        mut gas_mixes: Vec<BreathingSource>,
    ) -> Result<DecoRuntime, DecoCalculationError> {
        // validate gas mixes
        Self::validate_gas_mixes(&deco_model, &gas_mixes)?;
        // sort deco gasses by o2 content
        // We use a dummy pressure for sorting (1 bar), assuming O2 fraction dominates ppO2 ranking for OC
        // For CCR, ppO2 depends on setpoint, so this sorting is tricky if mixed.
        // Assuming mostly OC switches for now.
        gas_mixes.sort_by(|a: &BreathingSource, b: &BreathingSource| {
            let x = a.calculate_pressures(1.);
            let y = b.calculate_pressures(1.);
            x.o2.partial_cmp(&y.o2).unwrap()
        });

        // run model simulation until no deco stages
        let mut sim_model: T = deco_model.clone();
        let ascent_rate = sim_model.config().deco_ascent_rate();
        loop {
            let DiveState {
                depth: pre_stage_depth,
                time: pre_stage_time,
                gas: pre_stage_gas,
                ..
            } = sim_model.dive_state();
            let ceiling = sim_model.ceiling();

            // handle missed deco stop
            // if missed deco stop, override sim model to depth at the expected stop and rerun the calculation
            let next_deco_action = self.next_deco_action(&sim_model, gas_mixes.clone());
            if let Err(e) = next_deco_action {
                return match e {
                    MissedDecoStopViolation => {
                        sim_model.record(
                            self.deco_stop_depth(ceiling),
                            Time::zero(),
                            &pre_stage_gas,
                        );
                        self.calc(sim_model, gas_mixes)
                    }
                };
            }

            // handle deco actions
            let mut deco_stages: Vec<DecoStage> = vec![];
            let (deco_action, next_switch_gas) = next_deco_action.unwrap();
            match deco_action {
                // deco obligation cleared
                None => {
                    break;
                }

                // handle mandatory deco stage
                Some(deco_action) => {
                    match deco_action {
                        // ascent to min depth (deco stop or surface)
                        DecoAction::AscentToCeil => {
                            sim_model.record_travel_with_rate(
                                self.deco_stop_depth(ceiling),
                                ascent_rate,
                                &pre_stage_gas,
                            );
                            let current_sim_state = sim_model.dive_state();
                            let current_sim_time = current_sim_state.time;
                            deco_stages.push(DecoStage {
                                stage_type: DecoStageType::Ascent,
                                start_depth: pre_stage_depth,
                                end_depth: current_sim_state.depth,
                                duration: current_sim_time - pre_stage_time,
                                gas: current_sim_state.gas,
                            })
                        }

                        // ascent to min depth with gas switch on next deco gas maximum operating depth
                        DecoAction::AscentToGasSwitchDepth => {
                            // @todo unwrap and handler err
                            if let Some(next_switch_gas) = next_switch_gas {
                                // travel to MOD (using 1.6 ppo2 limit default)
                                // We need to check if the source SUPPORTS MOD calculation (OC)
                                // For CCR, MOD is depth limit, usually 1.6 setpoint limit or whatever
                                let switch_gas_mod = match next_switch_gas {
                                    BreathingSource::OpenCircuit(mix) => {
                                        mix.max_operating_depth(1.6)
                                    }
                                    BreathingSource::ClosedCircuit { .. } => {
                                        Depth::from_meters(1000.0)
                                    } // Valid anywhere basically
                                };

                                sim_model.record_travel_with_rate(
                                    switch_gas_mod,
                                    ascent_rate,
                                    &pre_stage_gas,
                                );
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
                                sim_model.record(
                                    sim_model.dive_state().depth,
                                    Time::zero(),
                                    &next_switch_gas,
                                );
                                // @todo configurable oxygen window stop
                                let post_switch_state = sim_model.dive_state();
                                deco_stages.push(DecoStage {
                                    stage_type: DecoStageType::GasSwitch,
                                    start_depth: post_ascent_depth,
                                    end_depth: post_switch_state.depth,
                                    duration: Time::zero(),
                                    gas: next_switch_gas,
                                });
                            }
                        }

                        // switch gas without ascent
                        DecoAction::SwitchGas => {
                            let switch_gas = next_switch_gas.unwrap();
                            // @todo configurable gas switch duration
                            sim_model.record(pre_stage_depth, Time::zero(), &switch_gas);
                            deco_stages.push(DecoStage {
                                stage_type: DecoStageType::GasSwitch,
                                start_depth: pre_stage_depth,
                                end_depth: pre_stage_depth,
                                duration: Time::zero(),
                                gas: switch_gas,
                            })
                        }

                        // decompression stop
                        DecoAction::Stop => {
                            let stop_depth = self.deco_stop_depth(ceiling);
                            let stop_duration =
                                self.find_min_stop_time(&sim_model, gas_mixes.clone(), stop_depth);

                            sim_model.record(pre_stage_depth, stop_duration, &pre_stage_gas);
                            let sim_state = sim_model.dive_state();
                            // @todo dedupe here on deco instead of of add deco
                            deco_stages.push(DecoStage {
                                stage_type: DecoStageType::DecoStop,
                                start_depth: stop_depth,
                                end_depth: stop_depth,
                                duration: sim_state.time - pre_stage_time,
                                gas: sim_state.gas,
                            })
                        }
                    }
                }
            }
            // register deco stages
            deco_stages
                .into_iter()
                .for_each(|deco_stage| self.register_deco_stage(deco_stage));
        }

        let tts = self.tts;
        // let mut tts_at_5 = Time::zero();
        // let mut tts_delta_at_5 = Time::zero();
        // if !self.is_sim() {
        //     let mut nested_sim_deco = Deco::new_sim();
        //     let mut nested_sim_model = deco_model.clone();
        //     let DiveState {
        //         depth: sim_depth,
        //         gas: sim_gas,
        //         ..
        //     } = nested_sim_model.dive_state();
        //     nested_sim_model.record(sim_depth, Time::from_minutes(5.), &sim_gas);
        //     let nested_deco = nested_sim_deco.calc(nested_sim_model, gas_mixes.clone())?;
        //     tts_at_5 = nested_deco.tts;
        //     tts_delta_at_5 = tts_at_5 - tts;
        // }
        // Temporarily disabling TTS@5 to avoid recursion issues or type complexity for now if it was causing issues.
        // Actually the code previously had it enabled. Let's keep it enabled if possible.
        // Re-enabling:
        let mut tts_at_5 = Time::zero();
        let mut tts_delta_at_5 = Time::zero();
        if !self.is_sim() {
            let mut nested_sim_deco = Deco::new_sim();
            let mut nested_sim_model = deco_model.clone();
            let DiveState {
                depth: sim_depth,
                gas: sim_gas,
                ..
            } = nested_sim_model.dive_state();
            nested_sim_model.record(sim_depth, Time::from_minutes(5.), &sim_gas);
            let nested_deco = nested_sim_deco.calc(nested_sim_model, gas_mixes.clone())?;
            tts_at_5 = nested_deco.tts;
            tts_delta_at_5 = tts_at_5 - tts;
        }

        Ok(DecoRuntime {
            deco_stages: self.deco_stages.clone(),
            tts,
            tts_at_5,
            tts_delta_at_5,
        })
    }

    fn next_deco_action(
        &self,
        sim_model: &impl DecoModel,
        gas_mixes: Vec<BreathingSource>,
    ) -> Result<(Option<DecoAction>, Option<BreathingSource>), MissedDecoStopViolation> {
        let DiveState {
            depth: current_depth,
            gas: current_gas,
            ..
        } = sim_model.dive_state();
        let surface_pressure = sim_model.config().surface_pressure();

        // end deco simulation - surface
        if current_depth <= Depth::zero() {
            return Ok((None, None));
        }

        let ceiling = sim_model.ceiling();

        match ceiling.partial_cmp(&Depth::zero()) {
            Some(Ordering::Equal | Ordering::Less) => Ok((Some(DecoAction::AscentToCeil), None)),
            Some(Ordering::Greater) => {
                // check if deco violation
                if current_depth < self.deco_stop_depth(ceiling) {
                    return Err(MissedDecoStopViolation);
                }

                let next_switch_gas = self.next_switch_gas(
                    current_depth,
                    &current_gas,
                    gas_mixes,
                    surface_pressure,
                    sim_model.config().water_density(),
                );
                // check if within mod @todo min operational depth
                if let Some(switch_gas) = next_switch_gas {
                    //switch gas without ascent if within mod of next deco gas
                    let gas_mod = match switch_gas {
                        BreathingSource::OpenCircuit(mix) => mix.max_operating_depth(1.6),
                        BreathingSource::ClosedCircuit { .. } => Depth::from_meters(1000.0),
                    };

                    let gas_end = match switch_gas {
                        BreathingSource::OpenCircuit(mix) => {
                            mix.equivalent_narcotic_depth(current_depth)
                        }
                        BreathingSource::ClosedCircuit { diluent, .. } => {
                            diluent.equivalent_narcotic_depth(current_depth)
                        }
                    };

                    if (switch_gas != current_gas)
                        && (current_depth <= gas_mod)
                        && (gas_end <= Depth::from_meters(DEFAULT_MAX_END_DEPTH))
                    {
                        return Ok((Some(DecoAction::SwitchGas), Some(switch_gas)));
                    }
                }

                // check if already at deco stop depth
                let stop_depth = self.deco_stop_depth(ceiling);
                if current_depth == stop_depth {
                    Ok((Some(DecoAction::Stop), None))
                } else {
                    // ascent to next gas switch depth if next gas' MOD below ceiling
                    if let Some(next_switch_gas) = next_switch_gas {
                        let gas_mod = match next_switch_gas {
                            BreathingSource::OpenCircuit(mix) => mix.max_operating_depth(1.6),
                            BreathingSource::ClosedCircuit { .. } => Depth::from_meters(1000.0),
                        };

                        if gas_mod >= ceiling {
                            return Ok((
                                Some(DecoAction::AscentToGasSwitchDepth),
                                Some(next_switch_gas),
                            ));
                        }
                    }
                    Ok((Some(DecoAction::AscentToCeil), None))
                }
            }
            None => panic!("Ceiling and depth uncomparable"),
        }
    }

    /// check next deco gas in deco (the one with the lowest MOD while more oxygen-rich than current)
    fn next_switch_gas(
        &self,
        current_depth: Depth,
        current_gas: &BreathingSource,
        gas_mixes: Vec<BreathingSource>,
        surface_pressure: MbarPressure,
        water_density: f64,
    ) -> Option<BreathingSource> {
        use crate::common::physics::depth_to_pressure;
        let p_amb = depth_to_pressure(current_depth, surface_pressure, water_density);
        let current_gas_partial_pressures = current_gas.calculate_pressures(p_amb);
        // all potential deco gases that are more oxygen-rich than current (inc. trimix / heliox)
        // mix with the lowest MOD (by absolute o2 content) -- sorting already done in calc
        gas_mixes.into_iter().find(|gas: &BreathingSource| {
            let partial_pressures = gas.calculate_pressures(p_amb);
            partial_pressures.o2 > current_gas_partial_pressures.o2
        })
    }

    fn register_deco_stage(&mut self, stage: DecoStage) {
        // dedupe iterative deco stops and merge into one
        let mut push_new = true;
        let last_stage = self.deco_stages.last_mut();
        if let Some(last_stage) = last_stage {
            if last_stage.stage_type == stage.stage_type {
                last_stage.duration += stage.duration;
                last_stage.end_depth = stage.end_depth;
                push_new = false;
            }
        }
        if push_new {
            self.deco_stages.push(stage);
        }

        // increment TTS by deco stage duration
        self.tts += stage.duration;
    }

    // round ceiling up to the bottom of deco window
    fn deco_stop_depth(&self, ceiling: Depth) -> Depth {
        let depth = DEFAULT_CEILING_WINDOW * ceil(ceiling.as_meters() / DEFAULT_CEILING_WINDOW);
        Depth::from_meters(depth)
    }

    fn validate_gas_mixes<T: DecoModel>(
        deco_model: &T,
        gas_mixes: &[BreathingSource],
    ) -> Result<(), DecoCalculationError> {
        if gas_mixes.is_empty() {
            return Err(DecoCalculationError::EmptyGasList);
        }
        let current_gas = deco_model.dive_state().gas;
        let current_gas_in_available = gas_mixes.iter().find(|gas_mix| **gas_mix == current_gas);
        if current_gas_in_available.is_none() {
            return Err(DecoCalculationError::CurrentGasNotInList);
        }
        Ok(())
    }

    /// Binary search for the minimum time required to clear the current stop
    /// or reach a state where a gas switch is better/possible.
    /// Returns the duration as Time.
    fn find_min_stop_time<T: DecoModel + Sim + Clone>(
        &self,
        current_model: &T,
        gas_mixes: Vec<BreathingSource>,
        current_stop_depth: Depth,
    ) -> Time {
        // We want to find min time t such that next_deco_action is NOT Stop at current_depth
        // OR the stop is cleared (AscentToCeil / AscentToGasSwitchDepth).
        // Max wait time: 24 hours (arbitrary upper bound for binary search).
        // In reality, stops are usually minutes.
        let mut low: u32 = 0;
        let mut high: u32 = 24 * 60; // Minutes

        let check_condition = |time_min: u32| -> bool {
            let mut sim = current_model.fork();
            let state = sim.dive_state();
            sim.record(state.depth, Time::from_minutes(time_min as f64), &state.gas);

            let res = self.next_deco_action(&sim, gas_mixes.clone());
            match res {
                Ok((Some(action), _)) => {
                    // We are "done" with this specific stop logic if:
                    // 1. Action is NOT Stop
                    // 2. OR Action IS Stop but depth changed (shouldn't happen if we are at stop depth)
                    match action {
                        DecoAction::Stop => {
                            // If we are still told to stop, check if the calculated stop depth
                            // is STILL the same as current_stop_depth.
                            let ceiling = sim.ceiling();
                            let new_stop_depth = self.deco_stop_depth(ceiling);
                            // If calculated stop depth is shallower, we can ascend -> condmet
                            // If it's same, we still need to wait -> cond false
                            new_stop_depth < current_stop_depth
                        }
                        _ => true, // Any other action (Ascent, Switch) means we moved on
                    }
                }
                Ok((None, _)) => true, // Deco cleared
                Err(_) => false,       // Error case, assume not done? Or panic?
            }
        };

        // Binary search for first 'true'
        while high - low > 1 {
            let mid = (low + high) / 2;
            if check_condition(mid) {
                high = mid;
            } else {
                low = mid;
            }
        }

        // Return first valid duration found (high bound of the binary search)
        // Granularity is currently in minutes.

        Time::from_minutes(high as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BuhlmannConfig, BuhlmannModel, GasMix};

    #[test]
    fn test_ceiling_rounding() {
        let test_cases: Vec<(Depth, Depth)> = vec![
            (Depth::from_meters(0.), Depth::from_meters(0.)),
            (Depth::from_meters(2.), Depth::from_meters(3.)),
            (Depth::from_meters(2.999), Depth::from_meters(3.)),
            (Depth::from_meters(3.), Depth::from_meters(3.)),
            (Depth::from_meters(3.00001), Depth::from_meters(6.)),
            (Depth::from_meters(12.), Depth::from_meters(12.)),
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
        let air = BreathingSource::OpenCircuit(GasMix::air());
        let ean_50 = BreathingSource::OpenCircuit(GasMix::new(0.5, 0.));
        let oxygen = BreathingSource::OpenCircuit(GasMix::new(1., 0.));
        let trimix = BreathingSource::OpenCircuit(GasMix::new(0.5, 0.2));

        // potential switch if in deco!
        // [ (current_depth, current_gas, gas_mixes, expected_result) ]
        let test_cases: Vec<(
            Depth,
            BreathingSource,
            Vec<BreathingSource>,
            Option<BreathingSource>,
        )> = vec![
            // single gas air
            (Depth::from_meters(10.), air, vec![air], None),
            // air + ean50 within MOD
            (
                Depth::from_meters(10.),
                air,
                vec![air, ean_50],
                Some(ean_50),
            ),
            // air + ean50 over MOD (MOD of EAN50 is 22m, depth 30m -> shouldn't switch?)
            // Wait, next_switch_gas ONLY checks O2 content > current.
            // The MOD check happens in next_deco_action!
            // next_switch_gas returns the CANDIDATE.
            // next_deco_action invalidates it if depth > MOD.
            (
                Depth::from_meters(30.),
                air,
                vec![air, ean_50],
                Some(ean_50),
            ),
            // air + ean50 + oxygen, ean50 within MOD, oxygen out
            // next_switch_gas finds highest O2?
            // "all potential deco gases that are more oxygen-rich... mix with the lowest MOD"
            // Wait, the sorting happens in calc() outside.
            // gas_mixes passed here are sorted by O2?
            // logic: find first gas with O2 > current.
            // If gas_mixes is [Air, EAN50, O2]
            // Current Air. O2(Air) < O2(EAN50). Returns EAN50.
            // Does it consider O2? Yes.
            // Does it consider MOD? No.
            (
                Depth::from_meters(20.),
                air,
                vec![air, ean_50, oxygen],
                Some(ean_50),
            ),
            // air + ean50 + oxy, deco on ean50, oxygen within MOD
            // Current EAN50. Next is O2.
            (
                Depth::from_meters(5.5),
                ean_50,
                vec![air, ean_50, oxygen],
                Some(oxygen),
            ),
            // air + heliox within o2 MOD, not considered deco gas
            // TMX: 50% O2, 20% He.
            // Air: 21% O2.
            // TMX > Air. Returns TMX.
            (
                Depth::from_meters(30.),
                air,
                vec![air, trimix],
                Some(trimix),
            ),
        ];

        let deco = Deco::default();
        for case in test_cases.into_iter() {
            let (current_depth, current_gas, available_gas_mixes, expected_switch_gas) = case;
            let res = deco.next_switch_gas(
                current_depth,
                &current_gas,
                available_gas_mixes,
                1000,
                1020.0,
            );
            assert_eq!(res, expected_switch_gas);
        }
    }

    #[test]
    fn should_err_on_empty_gas_mixes() {
        let mut deco = Deco::default();
        let deco_model = BuhlmannModel::default();
        let deco_res = deco.calc(deco_model, vec![]);
        assert_eq!(deco_res, Err(DecoCalculationError::EmptyGasList));
    }

    #[test]
    fn should_err_on_gas_mixes_without_current_mix() {
        let mut deco = Deco::default();
        let mut deco_model = BuhlmannModel::default();
        let air = BreathingSource::OpenCircuit(GasMix::air());
        let ean50 = BreathingSource::OpenCircuit(GasMix::new(0.50, 0.));
        let tmx2135 = BreathingSource::OpenCircuit(GasMix::new(0.21, 0.35));
        deco_model.record_travel_with_rate(Depth::from_meters(40.), 10., &air);
        let deco_res = deco.calc(deco_model, vec![ean50, tmx2135]);
        assert_eq!(deco_res, Err(DecoCalculationError::CurrentGasNotInList));
    }

    #[test]
    fn no_duplicated_stops_within_deco_padding() {
        let mut deco = Deco::default();
        let mut deco_model =
            BuhlmannModel::new(BuhlmannConfig::default().with_gradient_factors(30, 70));
        let air = BreathingSource::OpenCircuit(GasMix::air());
        let ean50 = BreathingSource::OpenCircuit(GasMix::new(0.50, 0.));
        deco_model.record_travel_with_rate(Depth::from_meters(40.), 10., &air);
        deco_model.record(Depth::from_meters(40.), Time::from_minutes(27.), &air);
        deco_model.record_travel_with_rate(Depth::from_meters(16.), 10., &air);
        deco_model.record(
            Depth::from_meters(16.),
            Time::from_minutes(1.),
            &BreathingSource::OpenCircuit(GasMix::new(0.50, 0.)),
        );

        let mut deco_stop_depths: Vec<Depth> = vec![];
        let deco_res = deco.calc(deco_model, vec![air, ean50]).unwrap();
        for deco_stage in deco_res.deco_stages {
            if deco_stage.stage_type == DecoStageType::DecoStop {
                let deco_stop_depth = deco_stage.start_depth;
                assert_eq!(
                    deco_stop_depths.contains(&deco_stop_depth),
                    false,
                    "{}m deco stop should not repeat",
                    deco_stop_depth
                );
                deco_stop_depths.push(deco_stop_depth);
            }
        }
    }
}
