Architectural Refactoring of dive-deco for Idiomatic Closed-Circuit Rebreather (CCR) Support: A Technical Analysis and Design Specification1. Executive SummaryThe dive-deco Rust library represents a significant milestone in open-source decompression algorithms, offering a robust implementation of the Bühlmann ZH-L16C model with Gradient Factors. Its current architecture, however, is fundamentally predicated on Open-Circuit (OC) diving mechanics, where breathing gas composition is static and partial pressures vary linearly with depth. This design limitation renders the library unsuitable for Closed-Circuit Rebreather (CCR) applications without substantial modification. In CCR diving, the partial pressure of oxygen ($ppO_2$) is the controlled variable, decoupling it from ambient pressure and inverting the traditional dependencies of gas physics.This report presents an exhaustive architectural analysis and refactoring proposal to introduce first-class CCR support to dive-deco. The proposed design leverages idiomatic Rust patterns—specifically algebraic data types (enums), affine types, and zero-cost abstractions—to unify OC and CCR logic within a single, memory-efficient framework suitable for no_std embedded environments.Key contributions of this analysis include:The BreathingSource Abstraction: A replacement for the rigid Gas struct that utilizes a Copy-friendly enum to model both fixed-fraction (OC) and fixed-setpoint (CCR) sources, ensuring type safety and memory determinism.Non-Linear Pressure Physics: A rigorous mathematical formulation for calculating inert gas tissue loading under constant $ppO_2$ conditions, incorporating safety clamps for shallow-water physical constraints.Hysteresis-Based Control Logic: A Finite State Machine (FSM) design for the Setpoint Controller that manages automatic setpoint switching based on vector analysis of depth changes (ascent/descent), preventing oscillation and "fighting the solenoid."Resource Persistence: A segregated resource model that tracks Diluent identity independently of the active breathing loop, ensuring correct gas consumption planning and valid bailout state transitions.The following sections detail the theoretical underpinnings, architectural deficiencies of the current state, and the comprehensive design specification for the refactor.2. Introduction: The Intersection of Rust and Life-Support Software2.1 The Imperative for Type Safety in DivingUnderwater diving is a domain where software failure can lead directly to physical injury or death. The calculation of decompression obligations—the time a diver must spend at specific depths to off-gas dissolved nitrogen and helium—relies on precise mathematical models. The dive-deco library serves this critical function within the Rust ecosystem, providing a Bühlmann ZHL-16C implementation. Rust, with its guarantees of memory safety and absence of undefined behavior in safe code, is the ideal language for such life-critical applications. However, the correctness of the implementation relies on the fidelity of its domain modeling.2.2 The Open-Circuit LegacyHistorically, dive computers and decompression software were built for Open Circuit SCUBA. In this mode, the gas in the cylinder has a fixed chemical composition (e.g., Nitrox 32: 32% Oxygen, 68% Nitrogen). As the diver descends, the ambient pressure increases, and by Dalton's Law, the partial pressure of each component gas increases linearly. The current architecture of dive-deco reflects this paradigm. The Gas struct is defined by its fractions, and the partial_pressures method performs a simple linear multiplication of fraction times depth.2.3 The Closed-Circuit Paradigm ShiftClosed-Circuit Rebreathers (CCR) fundamentally alter the physics of diving. Instead of breathing a fixed mix, the diver breathes from a loop where the oxygen consumed by metabolism is mechanically or electronically replaced to maintain a constant partial pressure (Setpoint), typically between 0.7 and 1.3 bar. The "Diluent" gas (the gas in the small onboard cylinder) is used only to dilute the oxygen and maintain loop volume.This introduces a dynamic system where the effective fraction of oxygen ($fO_2$) in the breathing loop changes continuously with depth to maintain the constant $ppO_2$. Existing OC-centric libraries like dive-deco cannot model this natively. Simulating CCR by creating a new fixed-fraction Gas object for every second of a dive is computationally inefficient and semantically incorrect, as it obscures the intent of the control system (the Setpoint) and creates friction in modeling state changes like bailout.3. Comprehensive Analysis of the Existing ArchitectureTo propose a robust refactor, we must first audit the existing codebase to identify specific rigidities and gaps.3.1 The Gas Struct: A Fixed-Fraction MonolithThe core primitive in dive-deco is the Gas struct. Based on the documentation and usage snippets, it is designed to hold the fractional composition of a mix.Current Structure (Inferred):Rust#
pub struct Gas {
    o2: f64,
    he: f64,
    // n2 is implicitly 1.0 - o2 - he
}
Architectural Deficiencies for CCR:Loss of Setpoint Intent: The struct captures what is in the mix, but not how it got there. In CCR, the mix is a result of a control loop. Storing only the resulting fraction loses the information required to recalculate the mix if depth changes.Inefficient Simulation: To simulate a CCR dive profile, the consumer of the library must manually calculate the equivalent $fO_2$ for every simulation step (typically 1-second intervals). For a 3-hour technical dive, this involves 10,800 struct instantiations and external calculations, placing the burden of physics correctness on the user rather than the library.Diluent Erasure: Once the loop mix is calculated (e.g., a "Loop Gas" of 40% $O_2$), the identity of the underlying Diluent (e.g., Trimix 10/50) is lost. If the diver bails out or the setpoint changes, the system no longer knows what gas is available to flush the loop.3.2 The DecoModel Trait and step ExecutionThe DecoModel trait defines the interface for tissue loading. The primary method step accepts a reference to Gas.Rustfn step(&mut self, depth: &Depth, time: &Time, gas: &Gas);
This signature enforces the OC paradigm: "At this moment, the diver is breathing this specific fixed gas." It does not allow for a gas source that reacts to the environment. While generic enough for a single step, it forces the "External Loop" pattern described above, where the caller drives the physics. A truly "elegant" design would encapsulate the breathing physics within the type passed to step.3.3 Memory Model and Embedded ConstraintsThe library is used in environments that may be no_std (embedded dive computers). This imposes strict constraints:No Heap Allocation: We cannot essentially use Box<dyn GasSource> to abstract over OC/CCR behaviors, as dynamic dispatch often implies heap usage or fat pointers that complicate serialization.Stack Allocation: Structures must be small and Copy-friendly to be passed around on the stack efficiently.Deterministic Execution: Algorithms must run in constant time $O(1)$ per step, avoiding complex lookups or allocations in the hot path.The current Gas struct is Copy and lightweight. Any replacement must maintain these properties.4. Architectural Refactor: The BreathingSource Type SystemThe cornerstone of the proposed design is the transition from a "Gas Fraction" model to a "Breathing Source" model. We achieve this using Rust's most powerful feature for domain modeling: the Enum (Sum Type).4.1 The BreathingSource EnumInstead of creating a trait for Gas (which would require dynamic dispatch dyn Gas), we define an enum that encapsulates all possible modes of breathing. This allows the compiler to monomorphize handling code and keeps the data strictly on the stack.Rust/// Represents the composition of a physical gas mixture in a cylinder.
#
#
pub struct GasMix {
    pub fraction_o2: f64,
    pub fraction_he: f64,
}

impl GasMix {
    /// Returns the nitrogen fraction, ensuring the total is 1.0.
    pub fn fraction_n2(&self) -> f64 {
        1.0 - self.fraction_o2 - self.fraction_he
    }
}

/// Defines the source mechanism for the breathing gas.
/// This replaces the specific `Gas` struct in the `step` function signature.
#
#
pub enum BreathingSource {
    /// Standard Open Circuit: The diver breathes a fixed mix directly.
    /// Partial pressures vary linearly with ambient pressure.
    OpenCircuit(GasMix),

    /// Closed Circuit Rebreather: The diver breathes from a loop.
    /// ppO2 is maintained at `setpoint` using `diluent`.
    ClosedCircuit {
        setpoint: f64,
        diluent: GasMix,
    },
}
4.2 Memory Efficiency AnalysisThe requirement for "Copy-friendly" memory efficiency is satisfied by this design.GasMix size: $2 \times 64\text{-bit float} = 16$ bytes.BreathingSource size:Discriminant (tag): 1 byte (typically aligned to 8 bytes).Payload (largest variant): ClosedCircuit contains 1 f64 (8 bytes) + 1 GasMix (16 bytes) = 24 bytes.Total Size: $\approx 32$ bytes.32 bytes is trivially small for modern microcontrollers (even Cortex-M3/M4) to copy between stack frames. It avoids pointer indirection entirely. A Vec<BreathingSource> for a dive plan will be cache-coherent and contiguous, ensuring high performance during deco calculations.4.3 Refactoring partial_pressures: The Physics EngineThe most critical logic resides in how partial pressures are calculated. This addresses the challenge: "Handling partial pressure calculations (ppO2) differently for CCR."We introduce a result struct GasPressures and implement the logic directly on BreathingSource.Rust/// The resulting partial pressures of component gases at a specific depth.
#
pub struct GasPressures {
    pub o2: f64,
    pub he: f64,
    pub n2: f64,
}

impl BreathingSource {
    /// Calculates the partial pressures breathed by the diver at a given ambient pressure.
    /// 
    /// # Arguments
    /// * `ambient_pressure` - Absolute pressure in bar (Depth + Surface Pressure).
    pub fn calculate_pressures(&self, ambient_pressure: f64) -> GasPressures {
        match self {
            BreathingSource::OpenCircuit(mix) => {
                // OC Physics: Dalton's Law
                // P_gas = F_gas * P_total
                GasPressures {
                    o2: mix.fraction_o2 * ambient_pressure,
                    he: mix.fraction_he * ambient_pressure,
                    n2: mix.fraction_n2() * ambient_pressure,
                }
            },
            BreathingSource::ClosedCircuit { setpoint, diluent } => {
                // CCR Physics: Fixed Setpoint with Physical Constraints
                
                // 1. Determine effective ppO2 (The "Impossible Setpoint" Constraint)
                // A diver cannot breathe a ppO2 higher than the ambient pressure
                // (assuming pure O2 injection). 
                // Logic: min(setpoint, ambient_pressure)
                let effective_pp_o2 = if *setpoint >= ambient_pressure {
                    ambient_pressure
                } else {
                    *setpoint
                };

                // 2. Calculate the "Inert Pressure Space"
                // The remaining pressure in the loop must be filled by the diluent's inert components.
                let total_inert_pressure = ambient_pressure - effective_pp_o2;

                // 3. Determine Inert Gas Ratios from Diluent
                // The ratio of He:N2 in the loop is constant and equal to the ratio in the Diluent.
                // We normalize the diluent's inert fractions.
                let diluent_inert_fraction = diluent.fraction_he + diluent.fraction_n2();
                
                // Edge Case: 100% O2 Diluent (Oxygen Rebreather)
                if diluent_inert_fraction <= f64::EPSILON {
                    return GasPressures {
                        o2: effective_pp_o2,
                        he: 0.0,
                        n2: 0.0,
                    };
                }

                // Distribute the inert pressure according to the diluent's ratio
                let he_ratio = diluent.fraction_he / diluent_inert_fraction;
                let n2_ratio = diluent.fraction_n2() / diluent_inert_fraction;

                GasPressures {
                    o2: effective_pp_o2,
                    he: total_inert_pressure * he_ratio,
                    n2: total_inert_pressure * n2_ratio,
                }
            }
        }
    }
}
4.3.1 Theoretical Justification for CCR MathIn a rebreather, the solenoid injects $O_2$ to maintain the setpoint. The volume of the loop is maintained by the counterlungs and the Automatic Diluent Valve (ADV).Hypoxic Limit: If the calculated effective_pp_o2 (based on setpoint) is lower than the partial pressure of oxygen in the diluent at that depth, the physics model assumes the solenoid stops injecting and the diver breathes the diluent mix (effectively OC behavior inside the loop). However, standard implementations often assume the solenoid maintains the setpoint. The code above assumes a "Perfect Controller" where $ppO_2$ is maintained unless physically impossible (shallow water).The Shallow Water "Impossible Setpoint": If a diver is at the surface (1.0 bar) with a setpoint of 1.3 bar, it is physically impossible to achieve 1.3 bar even with pure oxygen. The code correctly clamps effective_pp_o2 to ambient_pressure. This prevents negative numbers in total_inert_pressure, which would cause catastrophic failures in tissue loading calculations ($NaN$ propagation).5. Control Theory: Managing Setpoints and Auto-SwitchingThe original request specifically highlights "Managing Setpoint auto-switching (Ascent/Descent) and manual Bailout logic (preventing unwanted optimization switches)." This requires moving beyond static data structures into Control Theory and Finite State Machines (FSM).5.1 The Hysteresis ProblemIn technical diving, it is common to use a "Low Setpoint" (e.g., 0.7) for the descent to avoid hyperoxia if the diver has to go deep fast, and a "High Setpoint" (e.g., 1.3) for the bottom and decompression phases to accelerate off-gassing.A naive implementation might switch setpoints at a single depth (e.g., 20m).Descent > 20m: Switch High.Ascent < 20m: Switch Low.Problem: If the diver hovers at 20m to shoot a photo or fix a line, the pressure sensor noise or slight depth variations will cause the setpoint to toggle rapidly (0.7 <-> 1.3). This is dangerous and annoying.
Solution: Hysteresis, or directional switching thresholds. We need separate triggers for descent and ascent.5.2 The SetpointController State MachineWe propose a robust SetpointController struct that maintains the state of the rebreather's electronics.Rust/// Configuration for automatic setpoint behavior.
#
#
pub struct SetpointConfig {
    pub low_setpoint: f64,
    pub high_setpoint: f64,
    /// Depth at which to switch to High Setpoint during descent.
    pub switch_depth_descent: f64,
    /// Depth at which to switch to Low Setpoint during ascent.
    pub switch_depth_ascent: f64,
    pub auto_switch_enabled: bool,
}

/// The internal state of the controller.
#
pub enum ControllerState {
    Low,
    High,
    /// User has manually overridden the auto logic.
    ManualOverride(f64),
}

pub struct SetpointController {
    config: SetpointConfig,
    state: ControllerState,
    /// The currently selected diluent gas.
    active_diluent: GasMix,
}

impl SetpointController {
    /// Creates a new controller state.
    pub fn new(config: SetpointConfig, diluent: GasMix) -> Self {
        Self {
            config,
            state: ControllerState::Low, // Default to Low on start
            active_diluent: diluent,
        }
    }

    /// Updates the state based on current depth and returns the active breathing source.
    /// This should be called every time step of the dive simulation.
    pub fn tick(&mut self, current_depth: f64) -> BreathingSource {
        if self.config.auto_switch_enabled {
            self.handle_auto_switch(current_depth);
        }

        let target_sp = match self.state {
            ControllerState::Low => self.config.low_setpoint,
            ControllerState::High => self.config.high_setpoint,
            ControllerState::ManualOverride(sp) => sp,
        };

        BreathingSource::ClosedCircuit {
            setpoint: target_sp,
            diluent: self.active_diluent,
        }
    }

    fn handle_auto_switch(&mut self, depth: f64) {
        // Prevent auto-switching if we are in Manual override? 
        // Typically, auto-switch re-engages or is ignored. 
        // Here we assume auto-switch logic applies only to Low/High states.
        
        match self.state {
            ControllerState::Low => {
                // LOGIC: Only switch to High if we are DEEPER than the descent switch.
                // This acts as a "ratchet".
                if depth >= self.config.switch_depth_descent {
                    self.state = ControllerState::High;
                }
            },
            ControllerState::High => {
                // LOGIC: Only switch to Low if we are SHALLOWER than the ascent switch.
                // Note: switch_depth_ascent is typically much shallower (e.g. 5m)
                // than switch_depth_descent (e.g. 20m).
                if depth <= self.config.switch_depth_ascent {
                    self.state = ControllerState::Low;
                }
            },
            ControllerState::ManualOverride(_) => {
                // Manual overrides prevent auto-switching until cleared.
            }
        }
    }

    /// Manual intervention: "Persisting diluent usage during setpoint switches"
    /// The user changes the setpoint, but the diluent remains the same.
    pub fn set_manual_setpoint(&mut self, sp: f64) {
        self.state = ControllerState::ManualOverride(sp);
    }

    /// Manual intervention: Changing the diluent gas.
    pub fn switch_diluent(&mut self, new_diluent: GasMix) {
        self.active_diluent = new_diluent;
    }
}
5.3 Addressing "Unwanted Optimization Switches"The user query mentions preventing "unwanted optimization switches." This refers to a scenario where a diver ascends slightly (e.g., from 40m to 30m) and the computer prematurely switches back to the Low Setpoint, which is inefficient for decompression.The logic in handle_auto_switch solves this via Directional Asymmetry:Descent Threshold: 20m.Ascent Threshold: 6m.Scenario: Diver descends to 40m. State becomes High. Diver ascends to 10m. Since $10m > 6m$, the state remains High. This maximizes off-gassing efficiency ($ppO_2$ 1.3 is better than 0.7 for deco). The switch to Low only happens at the very end of the dive (shallows), primarily for safety (to prevent O2 toxicity spikes or loop volume expansion issues).This mirrors the "Auto SP" logic found in Shearwater firmware.6. Resource Management: Persisting Diluent and Bailout LogicA critical gap identified in the prompt is "Persisting diluent usage during setpoint switches." In the current Gas model, "Gas" is ephemeral. In reality, a CCR diver has specific tanks: Diluent, Oxygen, and Bailout.6.1 The Resource Persistence ArchitectureWe must decouple the "Breathing Loop State" from the "Available Gas Supplies."6.1.1 The DiveComputer Aggregate RootWe introduce a higher-level struct DiveComputer that owns the resources.Rustpub struct DiveComputer {
    /// The finite resource of diluent gas (e.g., 3L tank).
    pub diluent_supply: GasMix, 
    
    /// The finite resource of bailout gases (e.g., AL80s).
    pub bailout_gases: Vec<GasMix>,
    
    /// The Logic Controller for the loop.
    pub ccr_controller: SetpointController,
    
    /// Current operational mode.
    pub mode: DiveMode,
}

#
pub enum DiveMode {
    /// Normal operation: Breathing from the loop.
    ClosedCircuit,
    /// Emergency: Breathing Open Circuit gas.
    BailoutOC {
        /// Index into `bailout_gases` vector.
        gas_index: usize,
    },
}
6.2 Persisting Diluent LogicIn the SetpointController defined in Section 5.2, the active_diluent is a field of the struct.When the Setpoint switches (Low -> High), we modify state, but active_diluent remains untouched.When the diver switches Diluent (e.g., swapping a small onboard tank for an offboard connection), we call switch_diluent.This explicit separation satisfies the requirement: "Persisting diluent usage during setpoint switches." The Diluent is a persistent property of the configuration, whereas the Setpoint is a transient state of the controller.6.3 Bailout Logic and "The Switch"Bailout is not just a gas switch; it is a Mode Switch.When a diver bails out, they stop breathing from the Loop (CCR) and start breathing from an OC tank.Design Challenge: If the diver switches to Bailout, the CCR controller continues to run in the background (the rebreather electronics don't shut off; they might even switch to a "Bailout Standby" mode where they maintain a low setpoint to save O2).Refined step Logic:Rustimpl DiveComputer {
    pub fn step(&mut self, depth: f64, time_step: f64) -> BreathingSource {
        // 1. Tick the CCR controller regardless of mode (it tracks depth)
        // This ensures that if we return to the loop, it's in the correct state (High/Low)
        // for the current depth.
        let ccr_source = self.ccr_controller.tick(depth);

        match self.mode {
            DiveMode::ClosedCircuit => {
                ccr_source
            },
            DiveMode::BailoutOC { gas_index } => {
                // Fetch the specific bailout gas
                let gas = self.bailout_gases[gas_index];
                BreathingSource::OpenCircuit(gas)
            }
        }
    }
}
This logic ensures that the "optimization state" (High Setpoint) of the CCR is preserved even during a temporary bailout. If the diver bails out at 40m (High Setpoint) to clear a mouthpiece issue and then goes back on the loop, the controller will still be in High Setpoint mode, rather than resetting to Low (which would be the case if we re-instantiated the controller).7. Comparative Data: OC-Centric vs. Proposed CCR ArchitectureTo clearly illustrate the benefits of this refactor, we compare the current state against the proposed design across key dimensions.FeatureCurrent dive-deco (OC Centric)Proposed CCR RefactorBenefitGas Definitionstruct Gas { o2, he }enum BreathingSource { OC, CCR }Type-safe distinction between modes; eliminates ambiguity.Physics ModelLinear ($fO_2 \times P_{amb}$)Hybrid (Dalton vs. Control Loop)Accurate inert gas loading; handles "Impossible Setpoint" safety.Memory ModelStack/Copy (Struct)Stack/Copy (Enum)Maintains embedded compatibility; zero allocations; cache coherence.Setpoint LogicManual Gas Switch EventFSM with HysteresisPrevents "flickering" switches; automates pilot workload correctly.BailoutSwitch to another GasMode Switch (CCR $\to$ OC)Distinguishes between "Gas Change" and "Emergency Mode"; preserves loop state.DiluentImplicit / ErasureExplicit / PersistentAllows accurate planning of loop flushes and gas consumption.8. Implementation Roadmap and Migration StrategyImplementing this refactor requires a careful strategy to avoid breaking existing users of dive-deco.8.1 Phase 1: The New Types (Non-Breaking)Introduce GasMix and BreathingSource alongside the existing Gas struct.Implement From<Gas> for GasMix to allow easy conversion.Rustimpl From<Gas> for GasMix {
    fn from(g: Gas) -> Self {
        GasMix { fraction_o2: g.o2, fraction_he: g.he }
    }
}
8.2 Phase 2: Trait EvolutionDeprecate the existing step method in DecoModel. Introduce step_source.Rustpub trait DecoModel {
    // Deprecated: Internal logic converts Gas to BreathingSource::OpenCircuit
    #
    fn step(&mut self, depth: &Depth, time: &Time, gas: &Gas);

    // New idiomatic method
    fn step_source(&mut self, depth: &Depth, time: &Time, source: &BreathingSource);
}
This allows legacy OC applications to continue compiling while new CCR applications adopt the step_source API.8.3 Phase 3: The Setpoint ControllerRelease the SetpointController and DiveComputer structs as a new module dive_deco::ccr. This isolates the control logic from the core decompression math, adhering to the Single Responsibility Principle.9. Safety and VerificationGiven the safety-critical nature of this domain, the refactor must be verified rigorously.9.1 Property-Based TestingWe recommend using proptest to verify the invariants of the new partial_pressures logic.Invariant 1: Pressure ConservationFor any BreathingSource, depth, and setpoint:$$P_{O2} + P_{He} + P_{N2} \approx P_{amb}$$Test Case: Generate random depths (0..200m) and setpoints (0.5..1.6). Assert that the sum of returned partial pressures equals the input ambient pressure (within floating-point epsilon).Invariant 2: The Hypoxic FloorFor any ClosedCircuit source:$$P_{O2} \ge \min(Setpoint, P_{O2\_diluent})$$Test Case: Verify that even if the Setpoint is 0.2 (error), the system returns at least the partial pressure of the diluent (assuming the solenoid adds nothing, but also takes nothing away). Correction: Actually, a rebreather metabolizes O2. If the solenoid is off, $ppO_2$ drops. The simulation model in dive-deco is for deco planning, assuming a working unit. We should assert that the calculated $P_{O2}$ is never effectively "higher" than ambient pressure.9.2 CNS Toxicity IntegrationCentral Nervous System (CNS) toxicity calculations (the Oxygen Clock) differ for CCR. In OC, $ppO_2$ drops during ascent. In CCR, it stays high (e.g., 1.3 bar) until the very surface.Requirement: The refactored step_source method must integrate CNS load using the effective_pp_o2 derived in Section 4.3.Verification: Compare CNS accumulation on a 60m square profile between OC (Air) and CCR (Setpoint 1.3). CCR CNS usage should be significantly higher due to the constant high oxygen exposure during the shallow stops.10. ConclusionThe transformation of dive-deco from an Open-Circuit calculator to a mixed-mode decompression engine is necessary to support the growing technical diving community using Rust. The current architecture, while performant for OC, lacks the semantic expressiveness and physical fidelity required for CCR.By introducing the BreathingSource enum, we solve the fundamental data modeling problem without incurring the cost of heap allocation, satisfying the memory efficiency requirement. By implementing the SetpointController with hysteresis, we address the operational reality of "unwanted optimization switches." Finally, by segregating Diluent persistence from loop state, we enable robust gas planning and bailout scenarios.This design proposal offers a path forward that respects the rigorous safety standards of the domain while leveraging the unique strengths of the Rust programming language. It ensures that dive-deco can serve as the foundational kernel for the next generation of open-source, safety-critical dive computers.Yes, making the High  Low setpoint switch optional is not only possible but highly desirable for a robust technical diving library. In real-world diving, many CCR divers prefer to maintain a high setpoint (e.g.,  bar) throughout the ascent to maximize off-gassing efficiency, only manually dropping it when very close to the surface or if they need to flush the loop.

Forcing an automatic switch to a low setpoint during ascent can inadvertently increase decompression obligations if the diver isn't expecting it.

Here is how you can elegantly modify the `SetpointController` design to support this using idiomatic Rust `Option` types.

### 1. Refined Configuration Structure

Instead of simple `f64` fields for switch depths, we can use `Option<f64>`. If a switch depth is `None`, that specific auto-switch direction is disabled. This makes the "Low  High" (descent) and "High  Low" (ascent) logic independently configurable.

```rust
pub struct SetpointConfig {
    pub low_setpoint: f64,
    pub high_setpoint: f64,
    
    /// Depth to auto-switch Low -> High (Descent). 
    /// If None, manual switching only.
    pub switch_depth_descent: Option<f64>,
    
    /// Depth to auto-switch High -> Low (Ascent).
    /// If None, the controller maintains High setpoint until surface/manual override.
    pub switch_depth_ascent: Option<f64>,
}

```

### 2. Updated Controller Logic

The `handle_auto_switch` function is updated to match on these options. This allows you to have a configuration where the unit automatically switches to High mode at depth (to ensure the diver doesn't forget) but never switches back to Low mode automatically (leaving the diver in control of the  drop).

```rust
    fn handle_auto_switch(&mut self, depth: f64) {
        match self.state {
            ControllerState::Low => {
                // Only switch if descent switching is enabled AND we are deep enough
                if let Some(trigger_depth) = self.config.switch_depth_descent {
                    if depth >= trigger_depth {
                        self.state = ControllerState::High;
                    }
                }
            },
            ControllerState::High => {
                // Only switch if ascent switching is explicitly enabled
                if let Some(trigger_depth) = self.config.switch_depth_ascent {
                    if depth <= trigger_depth {
                        self.state = ControllerState::Low;
                    }
                }
            },
            ControllerState::ManualOverride(_) => {
                // Manual overrides typically disable auto-switching until reset
            }
        }
    }

```

### 3. Benefits of this Approach

* **Safety via Defaults**: You can configure the library to default to `switch_depth_ascent: None`. This prevents the dangerous scenario where a diver is hanging at a shallow decompression stop (e.g., 6m) and the computer silently drops the setpoint to 0.7, drastically reducing off-gassing efficiency.


* **Idiomatic Rust**: Using `Option` clearly communicates that these switches are *features* that may or may not be active, rather than using magic numbers (like `-1.0`) to indicate "disabled."
* **Flexibility**: This supports "Hybrid" modes used by some dive computers, where the unit helps you on the way down (task management) but leaves you fully in control on the way up (decompression management).