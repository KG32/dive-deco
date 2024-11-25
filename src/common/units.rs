use core::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Sub};

use super::DepthType;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Units {
    Metric,
    Imperial,
}

pub trait Unit<T = f64>: Sized {
    fn from_units(val: T, units: Units) -> Self;
    fn to_units(&self, units: Units) -> T;
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
        write!(f, r"{}m \ {}ft", self.meters(), self.feet())
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
    fn from_units(val: DepthType, units: Units) -> Self {
        match units {
            Units::Metric => Self::m(val),
            Units::Imperial => Self::ft(val),
        }
    }
    fn to_units(&self, units: Units) -> DepthType {
        match units {
            Units::Metric => self.meters(),
            Units::Imperial => self.feet(),
        }
    }
}

impl Depth {
    pub fn zero() -> Self {
        Self { m: 0. }
    }
    pub fn m(val: DepthType) -> Self {
        Self { m: val }
    }
    pub fn ft(val: DepthType) -> Self {
        Self {
            m: Self::ft_to_m(val),
        }
    }
    pub fn meters(&self) -> DepthType {
        self.m
    }
    pub fn feet(&self) -> DepthType {
        Self::m_to_ft(self.m)
    }
    fn m_to_ft(m: DepthType) -> DepthType {
        m * 3.28084
    }
    fn ft_to_m(ft: DepthType) -> DepthType {
        ft / 3.28084
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m_to_ft() {
        let depth = Depth::m(1.);
        let ft = depth.feet();
        assert_eq!(ft, 3.28084);
    }

    #[test]
    fn ft_to_m() {
        let depth = Depth::ft(100.);
        let m = depth.meters();
        assert_eq!(m, 30.47999902464003);
    }

    #[test]
    fn depth_conversion_factors() {
        let depth = Depth::m(1.);
        let ft = depth.feet();
        let new_depth = Depth::ft(ft);
        let m = new_depth.meters();
        assert_eq!(with_precision(m, 5), 1.);
    }

    #[test]
    fn from_units_constructor() {
        let depth_m = Depth::from_units(1., Units::Metric);
        assert_eq!(depth_m.meters(), 1.);
        assert_eq!(depth_m.feet(), 3.28084);

        let depth_ft = Depth::from_units(1., Units::Imperial);
        assert_eq!(with_precision(depth_ft.feet(), 5), 1.);
        assert_eq!(depth_ft.meters(), 0.3047999902464003);
    }

    fn with_precision(x: f64, precision: u32) -> f64 {
        let d = 10_u32.pow(precision) as f64;
        (x * d).round() / d
    }
}
