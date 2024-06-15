use crate::common::global_types::{Pressure, Depth, MbarPressure};

// alveolar water vapor pressure assuming 47 mm Hg at 37C (Buehlmann's value)
const ALVEOLI_WATER_VAPOR_PRESSURE: f64 = 0.0627;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gas {
    o2_pp: Pressure,
    n2_pp: Pressure,
    he_pp: Pressure,
}

#[derive(Debug, PartialEq)]
pub struct PartialPressures {
    pub o2: Pressure,
    pub n2: Pressure,
    pub he: Pressure,
}

pub enum InertGas {
    Helium,
    Nitrogen
}

impl Gas {
    /// init new gas with partial pressures (eg. 0.21, 0. for air)
    pub fn new(o2_pp: Pressure, he_pp: Pressure) -> Self {
        if !(0. ..=1.).contains(&o2_pp) {
            panic!("Invalid O2 partial pressure");
        }
        if !(0. ..=1.).contains(&he_pp) {
            panic!("Invalid He partial pressure [{he_pp}]");
        }
        if (o2_pp + he_pp) > 1. {
            panic!("Invalid partial pressures, can't exceed 1ATA in total");
        }

        Self {
            o2_pp,
            he_pp,
            n2_pp: ((1. - (o2_pp + he_pp)) * 100.0).round() / 100.0,
        }
    }

    /// gas partial pressures
    pub fn partial_pressures(&self, depth: &Depth, surface_pressure: MbarPressure) -> PartialPressures {
        let gas_pressure = (surface_pressure as f64 / 1000.) + (depth / 10.);
        self.gas_pressures_compound(gas_pressure)
    }

    /// gas partial pressures in alveoli taking into account alveolar water vapor pressure
    pub fn inspired_partial_pressures(&self, depth: &Depth, surface_pressure: MbarPressure) -> PartialPressures {
        let gas_pressure = ((surface_pressure as f64 / 1000.) + (depth / 10.)) - ALVEOLI_WATER_VAPOR_PRESSURE;
        self.gas_pressures_compound(gas_pressure)
    }

    pub fn gas_pressures_compound(&self, gas_pressure: f64) -> PartialPressures {
        PartialPressures {
            o2: self.o2_pp * gas_pressure,
            n2: self.n2_pp * gas_pressure,
            he: self.he_pp * gas_pressure,
        }
    }

    /// MOD
    pub fn max_operating_depth(&self, pp_o2: Pressure) -> Depth {
        10. * ((pp_o2 / self.o2_pp) - 1.)
    }

    // TODO standard nitrox (bottom and deco) and trimix gasses
    pub fn air() -> Self {
        Self::new(0.21, 0.)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_gas_air() {
        let air = Gas::new(0.21, 0.);
        assert_eq!(air.o2_pp, 0.21);
        assert_eq!(air.n2_pp, 0.79);
        assert_eq!(air.he_pp, 0.);
    }

    #[test]
    fn test_valid_gas_tmx() {
        let tmx = Gas::new(0.18, 0.35);
        assert_eq!(tmx.o2_pp, 0.18);
        assert_eq!(tmx.he_pp, 0.35);
        assert_eq!(tmx.n2_pp, 0.47);
    }

    #[test]
    #[should_panic]
    fn test_invalid_o2_high() {
        Gas::new(1.1, 0.);
    }

    #[test]
    #[should_panic]
    fn test_invalid_o2_low() {
        Gas::new(-3., 0.);
    }

    #[test]
    #[should_panic]
    fn test_invalid_partial_pressures() {
        Gas::new(0.5, 0.51);
    }


    #[test]
    fn test_partial_pressures_air() {
        let air = Gas::new(0.21, 0.);
        let partial_pressures = air.partial_pressures(&10., 1000);
        assert_eq!(partial_pressures, PartialPressures { o2: 0.42, n2: 1.58, he: 0. });
    }

    #[test]
    fn partial_pressures_tmx() {
        let tmx = Gas::new(0.21, 0.35);
        let partial_pressures = tmx.partial_pressures(&10., 1000);
        assert_eq!(partial_pressures, PartialPressures { o2: 0.42, he: 0.70, n2: 0.88})
    }

    #[test]
    fn test_inspired_partial_pressures() {
        let air = Gas::new(0.21, 0.);
        let inspired_partial_pressures = air.inspired_partial_pressures(&10., 1000);
        assert_eq!(inspired_partial_pressures, PartialPressures { o2: 0.406833, n2: 1.530467, he: 0.0 });
    }

    #[test]
    fn test_mod() {
        let air = Gas::new(0.21, 0.);
        let ean_50 = Gas::new(0.50, 0.);
        assert_eq!(air.max_operating_depth(1.4), 56.66666666666666);
        assert_eq!(ean_50.max_operating_depth(1.6), 22.);

    }
}
