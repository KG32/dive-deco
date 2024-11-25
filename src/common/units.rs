use core::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Sub};

use super::DepthType;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Units {
    Metric,
    Imperial,
}

pub trait Unit<T = f64>: Sized {
    fn from_units(val: T, units: Units) -> Self {
        match units {
            Units::Metric => Self::from_metric(val),
            Units::Imperial => Self::from_imperial(val),
        }
    }

    fn to_units(&self, units: Units) -> T {
        match units {
            Units::Metric => self.metric(),
            Units::Imperial => self.imperial(),
        }
    }

    fn from_metric(val: T) -> Self;
    fn from_imperial(val: T) -> Self;

    fn metric(&self) -> T;
    fn imperial(&self) -> T;
}

#[derive(Clone, Copy, Debug)]
pub struct Depth {
    m: DepthType,
}
impl Default for Depth {
    fn default() -> Self {
        Self { m: 0. }
    }
}

impl fmt::Display for Depth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, r"{}m \ {}ft", self.metric(), self.imperial())
    }
}

impl PartialEq<Self> for Depth {
    fn eq(&self, other: &Self) -> bool {
        self.m == other.m
    }
}

impl PartialOrd<Self> for Depth {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.m.partial_cmp(&other.m)
    }
}

impl Add<Self> for Depth {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self { m: self.m + rhs.m }
    }
}

impl Sub<Self> for Depth {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self { m: self.m - rhs.m }
    }
}

impl Mul for Depth {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self { m: self.m * rhs.m }
    }
}

impl Div for Depth {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self { m: self.m - rhs.m }
    }
}

impl AddAssign for Depth {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self { m: self.m + rhs.m }
    }
}

impl Unit for Depth {
    fn from_metric(val: DepthType) -> Self {
        Self { m: val }
    }
    fn from_imperial(val: DepthType) -> Self {
        Self { m: val * 0.3048 }
    }
    fn metric(&self) -> DepthType {
        self.m
    }
    fn imperial(&self) -> DepthType {
        self.m * 3.28084
    }
}

impl Depth {
    pub fn zero() -> Self {
        Self { m: 0. }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m_to_ft() {
        let depth = Depth::from_metric(1.);
        let ft = depth.imperial();
        assert_eq!(ft, 3.28084);
    }

    #[test]
    fn ft_to_m() {
        let depth = Depth::from_imperial(100.);
        let m = depth.metric();
        assert_eq!(m, 30.48);
    }

    #[test]
    fn depth_conversion_factors() {
        let depth = Depth::from_metric(1.);
        let ft = depth.imperial();
        let new_depth = Depth::from_imperial(ft);
        let m = new_depth.metric();
        assert_eq!(with_precision(m, 5), 1.);
    }

    #[test]
    fn from_units_constructor() {
        let depth_from_metric = Depth::from_units(1., Units::Metric);
        assert_eq!(depth_from_metric.metric(), 1.);
        assert_eq!(depth_from_metric.imperial(), 3.28084);

        let depth_from_imperial = Depth::from_units(1., Units::Imperial);
        assert_eq!(with_precision(depth_from_imperial.imperial(), 5), 1.);
        assert_eq!(depth_from_imperial.metric(), 0.3048);
    }

    fn with_precision(x: f64, precision: u32) -> f64 {
        let d = 10_u32.pow(precision) as f64;
        (x * d).round() / d
    }
}
