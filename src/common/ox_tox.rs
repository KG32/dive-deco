use crate::{common::CNS_COEFFICIENTS};
use crate::{Pressure, StepData};

use super::{CNSCoeffRow, CNSPercent, DiveState, MbarPressure};

#[derive(Copy, Clone, Debug)]
#[derive(PartialEq)]
pub struct OxTox {
    cns: CNSPercent
}

impl Default for OxTox {
    fn default() -> Self {
        Self {
            cns: 0.
        }
    }
}

impl OxTox {
    pub fn cns(&self) -> CNSPercent {
        self.cns
    }

    pub fn recalculate_cns(&mut self, step: &StepData, surface_pressure: MbarPressure) {
        let cns_fraction = self.calc_cns_segment(step, surface_pressure);
        self.cns += cns_fraction;
    }

    // attempt to assign CNS coefficients
    fn assign_cns_coeffs(&self, pp_o2: Pressure) -> Option<CNSCoeffRow> {
        let mut coeffs_for_range: Option<CNSCoeffRow> = None;
        for row in CNS_COEFFICIENTS.into_iter() {
            let row_range = row.0.clone();
            let in_range_start_exclusive = (&pp_o2 != row_range.start()) && row_range.contains(&pp_o2);
            if in_range_start_exclusive {
                coeffs_for_range = Some(row);
                break;
            }
        }

        coeffs_for_range
    }

    fn calc_cns_segment(&self, step: &StepData, surface_pressure: MbarPressure) -> CNSPercent {
        let current_gas = step.gas;
        let pp_o2 = current_gas
            .inspired_partial_pressures(step.depth, surface_pressure)
            .o2;
        let coeffs_for_range = self.assign_cns_coeffs(pp_o2);
        // only calculate CNS change if o2 partial pressure higher than 0.5
        if let Some((.., slope, intercept)) = coeffs_for_range {
            // time limit for given P02
            let t_lim: f64 = ((slope as f64) * pp_o2) + (intercept as f64);
            return ((*step.time as f64) / (t_lim * 60.)) * 100.;
        } else {
            // PO2 out of cns table range
            // @todo handle CNS half time + above 1.6 extrapolation
            return 0.;
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::Gas;
    use super::*;

    #[test]
    fn test_default() {
        let ox_tox = OxTox::default();
        let OxTox { cns } = ox_tox;
        assert_eq!(cns, 0.);
    }

    #[test]
    fn test_cns_coeffs() {
        let ox_tox = OxTox::default();
        let assignable_cases = [
            (-0.55, false),
            (0.5, false),
            (0.55, true),
            (0.8, true),
            (1.6, true),
            (1.61, false),
        ];

        for (pp_o2, is_assignable) in assignable_cases.into_iter() {
            let row = ox_tox.assign_cns_coeffs(pp_o2);
            if is_assignable {
                assert!(
                    row.unwrap_or_else(||
                        panic!("row for ppO2 {} not found", pp_o2)
                    )
                    .0
                    .contains(&pp_o2));
            } else {
                assert_eq!(row, None);
            }
        }
    }

    #[test]
    fn test_calc_segment() {
        let ox_tox = OxTox::default();

        // static depth segment
        let depth = 36.;
        let time = 20 * 60;
        let ean_32 = Gas::new(0.32, 0.);
        let step = StepData {
            depth: &depth,
            time: &time,
            gas: &ean_32,
        };

        let segment_cns_delta = ox_tox.calc_cns_segment(&step, 1013);
        assert_eq!(segment_cns_delta, 15.018262206843517);
    }
}
