use crate::common::{Depth, MbarPressure, Pressure};

/// Standard gravity in m/s^2
pub const GRAVITY_MSS: f64 = 9.80665;

/// Density of water in kg/m^3
pub type WaterDensity = f64;

/// Common water density constants
pub mod density {
    /// Fresh water density (approx 1000 kg/m^3)
    pub const FRESH: super::WaterDensity = 1000.0;
    /// Salt water density (approx 1020 kg/m^3)
    /// This is the default used to match previous `depth/10` approximation
    pub const SALT: super::WaterDensity = 1020.0;
    /// EN13319 standard density (1030 kg/m^3)
    pub const EN13319: super::WaterDensity = 1030.0;
}

/// Calculate ambient pressure (bar) at a given depth (m)
/// P = P_surface + (rho * g * h)
/// Result in bar (1 bar = 100,000 Pa)
pub fn depth_to_pressure(
    depth: Depth,
    surface_pressure: MbarPressure,
    density: WaterDensity,
) -> Pressure {
    let p_surf_bar = (surface_pressure as f64) / 1000.0;
    let hydrostatic_pressure_pa = density * GRAVITY_MSS * depth.as_meters();
    let hydrostatic_pressure_bar = hydrostatic_pressure_pa / 100_000.0;
    p_surf_bar + hydrostatic_pressure_bar
}

/// Calculate depth (m) from a given ambient pressure (bar)
/// h = (P - P_surface) / (rho * g)
pub fn pressure_to_depth(
    pressure: Pressure,
    surface_pressure: MbarPressure,
    density: WaterDensity,
) -> Depth {
    let p_surf_bar = (surface_pressure as f64) / 1000.0;
    let p_delta_bar = pressure - p_surf_bar;
    // prevent negative depth if pressure < surface
    if p_delta_bar <= 0.0 {
        return Depth::zero();
    }
    let p_delta_pa = p_delta_bar * 100_000.0;
    let depth_m = p_delta_pa / (density * GRAVITY_MSS);
    Depth::from_meters(depth_m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_to_pressure_default_1020() {
        // Test that 1020 density approximates the old "depth / 10" rule
        // 10m depth, 1013 mbar surface
        // Old: 1.013 + 1.0 = 2.013
        // New: 1.013 + (1020 * 9.80665 * 10) / 100000 = 1.013 + 1.000278...
        let depth = Depth::from_meters(10.0);
        let p = depth_to_pressure(depth, 1013, density::SALT);
        assert!((p - 2.013).abs() < 0.001, "Expected ~2.013, got {}", p);
    }

    #[test]
    fn test_pressure_to_depth_fresh() {
        // 10m fresh water
        // P = 1.013 + (1000 * 9.80665 * 10) / 100000 = 1.013 + 0.980665 = 1.993665
        let depth = Depth::from_meters(10.0);
        let p = depth_to_pressure(depth, 1013, density::FRESH);
        assert!((p - 1.993665).abs() < 0.00001);

        // Round trip
        let d = pressure_to_depth(p, 1013, density::FRESH);
        assert!((d.as_meters() - 10.0).abs() < 0.0001);
    }
}
