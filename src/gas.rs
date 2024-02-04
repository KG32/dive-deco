use crate::global_types::{Pressure, Depth};

#[derive(Debug)]
pub struct Gas {
    o2_pp: Pressure,
    n2_pp: Pressure,
}

#[derive(Debug, PartialEq)]
pub struct GasPP {
    pub o2: Pressure,
    pub n2: Pressure,
}

impl Gas {
    pub fn new(o2_pp: Pressure) -> Gas {
        if !(0. ..=1.).contains(&o2_pp) {
            panic!("Invalid O2 partial pressure");
        }

        Gas {
            o2_pp,
            n2_pp: 1. - o2_pp,
        }
    }

    pub fn partial_pressures(&self, depth: &Depth) -> GasPP {
        let gas_pressure = 1. + (depth / 10.);

        GasPP {
            o2: self.o2_pp * gas_pressure,
            n2: self.n2_pp * gas_pressure,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_gas() {
        let air = Gas::new(0.21);
        assert_eq!(air.o2_pp, 0.21);
        assert_eq!(air.n2_pp, 0.79);
    }

    #[test]
    #[should_panic]
    fn test_invalid_o2_high() {
        Gas::new(1.1);
    }

    #[test]
    #[should_panic]
    fn test_invalid_o2_low() {
        Gas::new(-3.);
    }

    #[test]
    fn test_partial_pressures() {
        let air = Gas::new(0.21);
        let partial_pressures = air.partial_pressures(&10.);
        assert_eq!(partial_pressures, GasPP { o2: 0.42, n2: 1.58 });
    }
}
