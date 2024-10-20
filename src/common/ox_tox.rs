use std::cmp::Ordering;

use crate::common::CNS_COEFFICIENTS;
use crate::{Minutes, Pressure, RecordData, Seconds};

use super::global_types::Otu;
use super::{CNSCoeffRow, Cns, MbarPressure};

const CNS_ELIMINATION_HALF_TIME_MINUTES: Minutes = 90;
const CNS_LIMIT_OVER_MAX_PP02: Seconds = 400;
const OTU_EQUATION_EXPONENT: f64 = -0.8333;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OxTox {
    cns: Cns,
    otu: Otu,
}

impl Default for OxTox {
    fn default() -> Self {
        Self { cns: 0., otu: 0. }
    }
}

impl OxTox {
    pub fn cns(&self) -> Cns {
        self.cns
    }

    pub fn otu(&self) -> Otu {
        self.otu
    }

    pub fn recalculate(&mut self, record: &RecordData, surface_pressure: MbarPressure) {
        self.recalculate_cns(record, surface_pressure);
        self.recalculate_otu(record, surface_pressure);
    }

    fn recalculate_cns(&mut self, record: &RecordData, surface_pressure: MbarPressure) {
        let RecordData { depth, time, gas } = *record;

        let pp_o2 = gas.inspired_partial_pressures(depth, surface_pressure).o2;

        // attempt to assign CNS coefficients by o2 partial pressure
        let coeffs_for_range = self.assign_cns_coeffs(pp_o2);
        // only calculate CNS change if o2 partial pressure higher than 0.5
        if let Some((.., slope, intercept)) = coeffs_for_range {
            // time limit for given P02
            let t_lim = ((slope as f64) * pp_o2) + (intercept as f64);
            self.cns += ((time as f64) / (t_lim * 60.)) * 100.;
        } else {
            // PO2 out of cns table range
            if (depth == 0.) && (pp_o2 <= 0.5) {
                // eliminate CNS with half time
                self.cns /= 2_f64.powf((time / (CNS_ELIMINATION_HALF_TIME_MINUTES * 60)) as f64);
            } else if pp_o2 > 1.6 {
                // increase CNS by a constant when ppO2 higher than 1.6
                self.cns += ((time as f64) / CNS_LIMIT_OVER_MAX_PP02 as f64) * 100.;
            }
        }
    }

    fn recalculate_otu(&mut self, record: &RecordData, surface_pressure: MbarPressure) {
        let RecordData { depth, time, gas } = *record;
        let pp_o2 = gas.inspired_partial_pressures(depth, surface_pressure).o2;

        let otu_delta = match pp_o2.total_cmp(&0.5) {
            Ordering::Less => 0.,
            Ordering::Equal | Ordering::Greater => {
                (time as f64 / 60.) * (0.5 / (pp_o2 - 0.5)).powf(OTU_EQUATION_EXPONENT)
            }
        };
        self.otu += otu_delta;
    }

    // find CNS coefficients by o2 partial pressure
    fn assign_cns_coeffs(&self, pp_o2: Pressure) -> Option<CNSCoeffRow> {
        let mut coeffs_for_range: Option<CNSCoeffRow> = None;
        for row in CNS_COEFFICIENTS.into_iter() {
            let row_range = row.0.clone();
            let in_range_start_exclusive =
                (&pp_o2 != row_range.start()) && row_range.contains(&pp_o2);
            if in_range_start_exclusive {
                coeffs_for_range = Some(row);
                break;
            }
        }

        coeffs_for_range
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gas;

    #[test]
    fn test_default() {
        let ox_tox = OxTox::default();
        let OxTox { cns, otu } = ox_tox;
        assert_eq!(cns, 0.);
        assert_eq!(otu, 0.);
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
            (1.66, false),
        ];

        for (pp_o2, is_assignable) in assignable_cases.into_iter() {
            let row = ox_tox.assign_cns_coeffs(pp_o2);
            if is_assignable {
                assert!(row
                    .unwrap_or_else(|| panic!("row for ppO2 {} not found", pp_o2))
                    .0
                    .contains(&pp_o2));
            } else {
                assert_eq!(row, None);
            }
        }
    }

    #[test]
    fn test_cns_segment() {
        let mut ox_tox = OxTox::default();

        // static depth segment
        let depth = 36.;
        let time = 20 * 60;
        let ean_32 = Gas::new(0.32, 0.);
        let record = RecordData {
            depth,
            time,
            gas: &ean_32,
        };

        ox_tox.recalculate_cns(&record, 1013);
        assert_eq!(ox_tox.cns(), 15.018262206843517);
    }

    #[test]
    fn test_cns_half_time_elimination() {
        let mut ox_tox = OxTox::default();
        // CNS ~50%
        let record = RecordData {
            depth: 30.,
            time: (75 * 60),
            gas: &Gas::new(0.35, 0.),
        };
        ox_tox.recalculate_cns(&record, 1013);
        assert_eq!(ox_tox.cns, 48.31898259550245);
        // 2x 90 mins half time
        let mut i = 0;
        while i < 2 {
            ox_tox.recalculate_cns(
                &RecordData {
                    depth: 0.,
                    time: (90 * 60),
                    gas: &Gas::air(),
                },
                1013,
            );
            i += 1;
        }
        assert_eq!(ox_tox.cns, 12.079745648875612);
    }

    #[test]
    fn test_cns_above_max_ppo2() {
        let mut ox_tox = OxTox::default();
        let record = RecordData {
            depth: 30.,
            time: 400,
            gas: &Gas::new(0.5, 0.),
        };
        ox_tox.recalculate_cns(&record, 1013);
        assert_eq!(ox_tox.cns(), 100.)
    }

    #[test]
    fn test_otu_surface() {
        let mut ox_tox = OxTox::default();
        let record = RecordData {
            depth: 0.,
            time: 60 * 60,
            gas: &Gas::air(),
        };

        ox_tox.recalculate_otu(&record, 1013);
        assert_eq!(ox_tox.otu(), 0.);
    }

    #[test]
    fn test_otu_segment() {
        let mut ox_tox = OxTox::default();
        let ean32 = Gas::new(0.32, 0.);
        let record = RecordData {
            depth: 36.,
            time: 22 * 60,
            gas: &ean32,
        };
        ox_tox.recalculate_otu(&record, 1013);
        assert_eq!(ox_tox.otu(), 37.75920807052313);
    }
}
