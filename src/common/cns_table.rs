use std::ops::RangeInclusive;

// (PO2 Range, slope, intercept)
pub type CNSCoeffRow = (RangeInclusive<f64>, i32, i32);

pub const CNS_COEFFICIENTS: [CNSCoeffRow; 7] = [
    (0.5..=0.6, -1800, 1800),
    (0.6..=0.7, -1500, 1620),
    (0.7..=0.8, -1200, 1410),
    (0.8..=0.9, -900, 1170),
    (0.9..=1.1, -600, 900),
    (1.1..=1.5, -300, 570),
    (1.5..=1.6, -750, 1245),
];
