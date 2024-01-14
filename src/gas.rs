use crate::global_types::{PartialPressure, Depth};

pub struct Gas {
    o2_pp: PartialPressure,
    n2_pp: PartialPressure,
}

#[derive(Debug, PartialEq)]
pub struct GasPP {
    o2: PartialPressure,
    n2: PartialPressure,
}

impl Gas {
    pub fn new(o2_pp: PartialPressure) -> Gas {
        if o2_pp < 0.0 || o2_pp > 1.0 {
            panic!("Invalid O2 partial pressure");
        }
        Gas {
            o2_pp,
            n2_pp: 1.0 - o2_pp,
        }
    }

    pub fn partial_pressures(&self, depth: Depth) -> GasPP {
        let gas_pressure = 1.0 + (depth / 10.0);
        GasPP {
            o2: &self.o2_pp * gas_pressure,
            n2: &self.n2_pp * gas_pressure,
        }
    }
}


#[cfg(test)]
mod tests {
    use std::error::Error;
    use super::*;

    #[test]
    fn test_valid_gas() -> Result<(), Box<dyn Error>> {
        let air = Gas::new(0.21);
        assert_eq!(air.o2_pp, 0.21);
        assert_eq!(air.n2_pp, 0.79);
        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_invalid_o2_high() {
        Gas::new(1.1);
    }

    #[test]
    #[should_panic]
    fn test_invalid_o2_low() {
        Gas::new(-3.0);
    }

    #[test]
    fn test_partial_pressures() {
        let air = Gas::new(0.21);
        let partial_pressures = air.partial_pressures(10.0);
        assert_eq!(partial_pressures, GasPP { o2: 0.42, n2: 1.58 });
    }
}
