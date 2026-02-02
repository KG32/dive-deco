use crate::common::gas::{BreathingSource, GasMix};
use alloc::vec::Vec;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Configuration for automatic setpoint behavior.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

impl Default for SetpointConfig {
    fn default() -> Self {
        Self {
            low_setpoint: 0.7,
            high_setpoint: 1.3,
            switch_depth_descent: Some(20.0), // Standard descent switch
            switch_depth_ascent: Some(6.0),   // Shallow safety switch
        }
    }
}

/// The internal state of the controller.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ControllerState {
    Low,
    High,
    /// User has manually overridden the auto logic.
    ManualOverride(f64),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
        self.handle_auto_switch(current_depth);

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
        match self.state {
            ControllerState::Low => {
                // Only switch if descent switching is enabled AND we are deep enough
                if let Some(trigger_depth) = self.config.switch_depth_descent {
                    if depth >= trigger_depth {
                        self.state = ControllerState::High;
                    }
                }
            }
            ControllerState::High => {
                // Only switch if ascent switching is explicitly enabled
                if let Some(trigger_depth) = self.config.switch_depth_ascent {
                    if depth <= trigger_depth {
                        self.state = ControllerState::Low;
                    }
                }
            }
            ControllerState::ManualOverride(_) => {
                // Manual overrides typically disable auto-switching until reset
            }
        }
    }

    /// Manual intervention: "Persisting diluent usage during setpoint switches"
    /// The user changes the setpoint, but the diluent remains the same.
    pub fn set_manual_setpoint(&mut self, sp: f64) {
        self.state = ControllerState::ManualOverride(sp);
    }

    /// Reset to auto mode
    pub fn reset_to_auto(&mut self) {
        // Logic to determine if we should be High or Low based on current state?
        // Simplest is to default to Low or check logic. For now, we'll reset to Low.
        // A smarter implementation might check current depth vs thresholds.
        self.state = ControllerState::Low;
    }

    /// Manual intervention: Changing the diluent gas.
    pub fn switch_diluent(&mut self, new_diluent: GasMix) {
        self.active_diluent = new_diluent;
    }
}

/// High-level structure to manage resources (Diluent, Bailout).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DiveMode {
    /// Normal operation: Breathing from the loop.
    ClosedCircuit,
    /// Emergency: Breathing Open Circuit gas.
    BailoutOC {
        /// Index into `bailout_gases` vector.
        gas_index: usize,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiveComputer {
    /// The finite resource of diluent gas (e.g., 3L tank). (Not actively tracked quantity here yet)
    pub diluent_supply: GasMix,

    /// The finite resource of bailout gases (e.g., AL80s).
    pub bailout_gases: Vec<GasMix>,

    /// The Logic Controller for the loop.
    pub ccr_controller: SetpointController,

    /// Current operational mode.
    pub mode: DiveMode,
}

impl DiveComputer {
    pub fn new(
        diluent: GasMix,
        bailout_gases: Vec<GasMix>,
        config: SetpointConfig,
        mode: DiveMode,
    ) -> Self {
        Self {
            diluent_supply: diluent,
            bailout_gases,
            ccr_controller: SetpointController::new(config, diluent),
            mode,
        }
    }

    pub fn step(&mut self, depth: f64) -> BreathingSource {
        // 1. Tick the CCR controller regardless of mode (it tracks depth)
        // This ensures that if we return to the loop, it's in the correct state (High/Low)
        // for the current depth.
        let ccr_source = self.ccr_controller.tick(depth);

        match self.mode {
            DiveMode::ClosedCircuit => ccr_source,
            DiveMode::BailoutOC { gas_index } => {
                // Fetch the specific bailout gas
                // If index is invalid, fallback to diluent OC? or panic?
                // Safe handling: clamp or standard gas
                if let Some(gas) = self.bailout_gases.get(gas_index) {
                    BreathingSource::OpenCircuit(*gas)
                } else {
                    // Fallback to diluent if bailout index invalid
                    BreathingSource::OpenCircuit(self.diluent_supply)
                }
            }
        }
    }

    pub fn switch_to_bailout(&mut self, gas_index: usize) {
        self.mode = DiveMode::BailoutOC { gas_index };
    }

    pub fn switch_to_ccr(&mut self) {
        self.mode = DiveMode::ClosedCircuit;
    }
}
