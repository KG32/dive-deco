use crate::common::global_types::{Pressure, Depth};

#[derive(Debug, Clone, Copy)]
pub struct Gas {
    o2_pp: Pressure,
    n2_pp: Pressure,
    he_pp: Pressure,
}

#[derive(Debug, PartialEq)]
pub struct GasPP {
    pub o2: Pressure,
    pub n2: Pressure,
    pub he: Pressure,
}

impl Gas {
    /// init new gas with partial pressures (eg. 0.21, 0. for air)
    /// helium currently not supported
    pub fn new(o2_pp: Pressure, he_pp: Pressure) -> Gas {
        if !(0. ..=1.).contains(&o2_pp) {
            panic!("Invalid O2 partial pressure");
        }
        if !(0. ..=1.).contains(&he_pp) {
            panic!("Invalid He partial pressure");
        }
        if (o2_pp + he_pp) > 1. {
            panic!("Invalid partial pressures, can't exceed 1ATA in total");
        }
        // @todo helium
        if he_pp != 0. {
            panic!("Helium not supported");
        }

        Gas {
            o2_pp,
            he_pp,
            n2_pp: 1. - (o2_pp + he_pp),
        }
    }

    pub fn partial_pressures(&self, depth: &Depth) -> GasPP {
        let gas_pressure = 1. + (depth / 10.);
        GasPP {
            o2: self.o2_pp * gas_pressure,
            n2: self.n2_pp * gas_pressure,
            he: self.he_pp * gas_pressure,
        }
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

    #[ignore = "trimix unsupported"]
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
    #[should_panic]
    fn test_unsupported_helium() {
        Gas::new(0.18, 0.35);
    }

    #[test]
    fn test_partial_pressures() {
        let air = Gas::new(0.21, 0.);
        let partial_pressures = air.partial_pressures(&10.);
        assert_eq!(partial_pressures, GasPP { o2: 0.42, n2: 1.58, he: 0. });
    }
}
