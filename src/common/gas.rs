use crate::common::global_types::Pressure;
use alloc::string::String;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{round, Depth};

// alveolar water vapor pressure assuming 47 mm Hg at 37C (Buhlmann's value)
const ALVEOLI_WATER_VAPOR_PRESSURE: f64 = 0.0627;

/// Represents the composition of a physical gas mixture in a cylinder.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GasMix {
    pub fraction_o2: f64,
    pub fraction_he: f64,
}

pub type Gas = GasMix; // Compatibility alias, though we will deprecate usage

#[derive(Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PartialPressures {
    pub o2: Pressure,
    pub n2: Pressure,
    pub he: Pressure,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InertGas {
    Helium,
    Nitrogen,
}

/// Defines the source mechanism for the breathing gas.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BreathingSource {
    /// Standard Open Circuit: The diver breathes a fixed mix directly.
    /// Partial pressures vary linearly with ambient pressure.
    OpenCircuit(GasMix),

    /// Closed Circuit Rebreather: The diver breathes from a loop.
    /// ppO2 is maintained at `setpoint` using `diluent`.
    ClosedCircuit { setpoint: f64, diluent: GasMix },
}

impl core::fmt::Display for GasMix {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:.0}/{:.0}",
            self.fraction_o2 * 100.,
            self.fraction_he * 100.
        )
    }
}

impl GasMix {
    /// init new gas with fractions (eg. 0.21, 0. for air)
    pub fn new(o2_fraction: f64, he_fraction: f64) -> Self {
        if !(0. ..=1.).contains(&o2_fraction) {
            panic!("Invalid O2 fraction");
        }
        if !(0. ..=1.).contains(&he_fraction) {
            panic!("Invalid He fraction [{he_fraction}]");
        }
        if (o2_fraction + he_fraction) > 1. {
            panic!("Invalid fractions, can't exceed 1.0 in total");
        }

        Self {
            fraction_o2: o2_fraction,
            fraction_he: he_fraction,
        }
    }

    pub fn fraction_n2(&self) -> f64 {
        round((1.0 - self.fraction_o2 - self.fraction_he) * 100.0) / 100.0
    }

    pub fn id(&self) -> String {
        let mut s = String::new();
        let _ = core::fmt::write(
            &mut s,
            format_args!(
                "{:.0}/{:.0}",
                self.fraction_o2 * 100.,
                self.fraction_he * 100.
            ),
        );
        s
    }

    /// gas partial pressures (Open Circuit physics default)
    pub fn partial_pressures(&self, ambient_pressure: Pressure) -> PartialPressures {
        PartialPressures {
            o2: self.fraction_o2 * ambient_pressure,
            n2: self.fraction_n2() * ambient_pressure,
            he: self.fraction_he * ambient_pressure,
        }
    }

    /// gas partial pressures in alveoli taking into account alveolar water vapor pressure
    /// (Open Circuit physics default)
    pub fn inspired_partial_pressures(&self, ambient_pressure: Pressure) -> PartialPressures {
        let gas_pressure = ambient_pressure - ALVEOLI_WATER_VAPOR_PRESSURE;
        self.partial_pressures(gas_pressure)
    }

    /// MOD (Open Circuit)
    pub fn max_operating_depth(&self, pp_o2_limit: Pressure) -> Depth {
        Depth::from_meters(10. * ((pp_o2_limit / self.fraction_o2) - 1.))
    }

    /// END (Open Circuit)
    pub fn equivalent_narcotic_depth(&self, depth: Depth) -> Depth {
        // @todo refactor
        let mut end = (depth + Depth::from_meters(10.)) * Depth::from_meters(1. - self.fraction_he)
            - Depth::from_meters(10.);
        if end < Depth::zero() {
            end = Depth::zero();
        }
        end
    }

    // TODO standard nitrox (bottom and deco) and trimix gasses
    pub fn air() -> Self {
        Self::new(0.21, 0.)
    }
}

impl BreathingSource {
    /// Calculates the partial pressures breathed by the diver at a given ambient pressure.
    ///
    /// # Arguments
    /// * `ambient_pressure` - Absolute pressure in bar (Depth + Surface Pressure).
    pub fn calculate_pressures(&self, ambient_pressure: f64) -> PartialPressures {
        match self {
            BreathingSource::OpenCircuit(mix) => mix.partial_pressures(ambient_pressure),
            BreathingSource::ClosedCircuit { setpoint, diluent } => {
                // CCR Physics: Fixed Setpoint with Physical Constraints

                // 1. Determine effective ppO2 (The "Impossible Setpoint" Constraint)
                // A diver cannot breathe a ppO2 higher than the ambient pressure
                // (assuming pure O2 injection).
                let effective_pp_o2 = if *setpoint >= ambient_pressure {
                    ambient_pressure
                } else {
                    *setpoint
                };

                // 2. Calculate the "Inert Pressure Space"
                // The remaining pressure in the loop must be filled by the diluent's inert components.
                let total_inert_pressure = ambient_pressure - effective_pp_o2;

                if total_inert_pressure <= f64::EPSILON {
                    return PartialPressures {
                        o2: effective_pp_o2,
                        he: 0.0,
                        n2: 0.0,
                    };
                }

                // 3. Determine Inert Gas Ratios from Diluent
                // The ratio of He:N2 in the loop is constant and equal to the ratio in the Diluent.
                let diluent_inert_fraction = diluent.fraction_he + diluent.fraction_n2();

                // Edge Case: 100% O2 Diluent (Oxygen Rebreather)
                if diluent_inert_fraction <= f64::EPSILON {
                    return PartialPressures {
                        o2: effective_pp_o2,
                        he: 0.0,
                        n2: 0.0,
                    };
                }

                // Distribute the inert pressure according to the diluent's ratio
                let he_ratio = diluent.fraction_he / diluent_inert_fraction;
                let n2_ratio = diluent.fraction_n2() / diluent_inert_fraction;

                PartialPressures {
                    o2: effective_pp_o2,
                    he: total_inert_pressure * he_ratio,
                    n2: total_inert_pressure * n2_ratio,
                }
            }
        }
    }

    /// Calculate inspired partial pressures (alveolar)
    pub fn inspired_partial_pressures(&self, ambient_pressure: Pressure) -> PartialPressures {
        // For physics calculations involving tissue loading, we use alveolar pressure
        // P_alv = P_amb - P_water_vapor
        let gas_pressure = ambient_pressure - ALVEOLI_WATER_VAPOR_PRESSURE;

        self.calculate_pressures(gas_pressure)
    }

    /// Max Operating Depth (MOD) calculations.
    /// For OC: Derived from gas fraction and ppO2 limit.
    /// For CCR: Theoretically depth-independent for the loop (setpoint constant),
    /// but practically limited by Diluent or equipment.
    /// Returning a "safe" deep limit for CCR to defer to Diluent checks or assume valid.
    pub fn max_operating_depth(&self, pp_o2_limit: Pressure) -> Depth {
        match self {
            BreathingSource::OpenCircuit(mix) => mix.max_operating_depth(pp_o2_limit),
            // CCR maintains setpoint (mostly), so it doesn't really have a MOD based on High-ppO2
            // in the same way (unless setpoint > limit).
            // Effectively valid everywhere if setpoint is valid.
            // Returning 1000m to represent "no limit from mix" for deco switching logic,
            // assuming SetpointController handles PPO2 management.
            BreathingSource::ClosedCircuit { .. } => Depth::from_meters(1000.0),
        }
    }

    /// Equivalent Narcotic Depth (END).
    /// Used for gas density/narcosis checks.
    pub fn equivalent_narcotic_depth(&self, depth: Depth) -> Depth {
        match self {
            BreathingSource::OpenCircuit(mix) => mix.equivalent_narcotic_depth(depth),
            BreathingSource::ClosedCircuit { diluent, .. } => {
                // CCR END is based on the Diluent's Inert Gas composition (N2/He ratio)
                // mapped to the loop total pressure.
                // Loop has P_amb pressure.
                // Inert PP = P_amb - PPO2.
                // But simplistically for standard END calc, we often use Diluent's properties.
                // Refined approach:
                // END = (Depth + 10m) * (1 - Fraction_He) - 10m.
                // For CCR, Fraction_He is roughly same as Diluent Fraction_He (ignoring metabolic O2 consumption effects on inert ratio).
                diluent.equivalent_narcotic_depth(depth)
            }
        }
    }

    /// Returns the O2 fraction of the source.
    /// For OC, returns the gas mix O2 fraction.
    /// For CCR, returns the Diluent O2 fraction (mostly for identification/logging).
    pub fn fraction_o2(&self) -> f64 {
        match self {
            BreathingSource::OpenCircuit(mix) => mix.fraction_o2,
            BreathingSource::ClosedCircuit { diluent, .. } => diluent.fraction_o2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_gas_air() {
        let air = GasMix::new(0.21, 0.);
        assert_eq!(air.fraction_o2, 0.21);
        assert_eq!(air.fraction_n2(), 0.79);
        assert_eq!(air.fraction_he, 0.);
    }

    #[test]
    fn test_valid_gas_tmx() {
        let tmx = GasMix::new(0.18, 0.35);
        assert_eq!(tmx.fraction_o2, 0.18);
        assert_eq!(tmx.fraction_he, 0.35);
        assert_eq!(tmx.fraction_n2(), 0.47);
    }

    #[test]
    #[should_panic]
    fn test_invalid_o2_high() {
        GasMix::new(1.1, 0.);
    }

    #[test]
    #[should_panic]
    fn test_invalid_o2_low() {
        GasMix::new(-3., 0.);
    }

    #[test]
    #[should_panic]
    fn test_invalid_partial_pressures() {
        GasMix::new(0.5, 0.51);
    }

    #[test]
    fn test_partial_pressures_air() {
        let air = GasMix::air();
        // 10m depth + 1000mbar surface = 2 bar absolute
        let partial_pressures = air.partial_pressures(2.0);
        assert_eq!(
            partial_pressures,
            PartialPressures {
                o2: 0.42,
                n2: 1.58,
                he: 0.
            }
        );
    }

    #[test]
    fn partial_pressures_tmx() {
        let tmx = GasMix::new(0.21, 0.35);
        // 10m depth + 1000mbar surface = 2 bar absolute
        let partial_pressures = tmx.partial_pressures(2.0);
        assert_eq!(
            partial_pressures,
            PartialPressures {
                o2: 0.42,
                he: 0.70,
                n2: 0.88
            }
        )
    }

    #[test]
    fn test_inspired_partial_pressures() {
        let air = GasMix::air();
        // 10m depth + 1000mbar surface = 2 bar absolute
        let inspired_partial_pressures = air.inspired_partial_pressures(2.0);
        assert_eq!(
            inspired_partial_pressures,
            PartialPressures {
                o2: 0.406833,
                n2: 1.530467,
                he: 0.0
            }
        );
    }

    #[test]
    fn test_mod() {
        // o2, he, max_ppo2, MOD
        let test_cases = [
            (0.21, 0., 1.4, 56.66666666666666),
            (0.50, 0., 1.6, 22.),
            (0.21, 0.35, 1.4, 56.66666666666666),
            (0., 0., 1.4, f64::INFINITY),
        ];
        for (pp_o2, pe_he, max_pp_o2, expected_mod) in test_cases {
            let gas = GasMix::new(pp_o2, pe_he);
            let calculated_mod = gas.max_operating_depth(max_pp_o2);
            assert_eq!(calculated_mod, Depth::from_meters(expected_mod));
        }
    }

    #[test]
    fn test_end() {
        // depth, o2, he, END
        let test_cases = [
            (60., 0.21, 0.40, 32.),
            (0., 0.21, 0.40, 0.),
            (40., 0.21, 0., 40.),
        ];
        for (depth, o2_pp, he_pp, expected_end) in test_cases {
            let tmx = GasMix::new(o2_pp, he_pp);
            let calculated_end = tmx.equivalent_narcotic_depth(Depth::from_meters(depth));
            assert_eq!(calculated_end, Depth::from_meters(expected_end));
        }
    }

    #[test]
    fn test_id() {
        let ean32 = GasMix::new(0.32, 0.);
        assert_eq!(ean32.id(), "32/0");
        let tmx2135 = GasMix::new(0.21, 0.35);
        assert_eq!(tmx2135.id(), "21/35");
    }
}
