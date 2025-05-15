use core::{
    cmp::Ordering,
    ops::{Add, AddAssign, Div, Mul, Sub},
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Time {
    s: f64,
}

impl Add for Time {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self { s: self.s + rhs.s }
    }
}
impl Sub for Time {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self { s: self.s - rhs.s }
    }
}
impl AddAssign for Time {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self { s: self.s + rhs.s }
    }
}
impl Mul<Self> for Time {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self { s: self.s * rhs.s }
    }
}
impl Mul<u8> for Time {
    type Output = Self;
    fn mul(self, rhs: u8) -> Self::Output {
        Self {
            s: self.s * rhs as f64,
        }
    }
}
impl Div<Self> for Time {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self { s: self.s / rhs.s }
    }
}
impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.s.partial_cmp(&other.s)
    }
}

impl Time {
    pub fn from_seconds<T: Into<f64>>(val: T) -> Self {
        Self { s: val.into() }
    }
    pub fn from_minutes<T: Into<f64>>(val: T) -> Self {
        Self {
            s: val.into() * 60.,
        }
    }
    pub fn zero() -> Self {
        Self { s: 0. }
    }
    pub fn as_seconds(&self) -> f64 {
        self.s
    }
    pub fn as_minutes(&self) -> f64 {
        self.s / 60.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_seconds() {
        let time = Time::from_seconds(120.0);
        assert_eq!(time.as_seconds(), 120.0);
    }

    #[test]
    fn test_from_minutes() {
        let time = Time::from_minutes(2.0);
        assert_eq!(time.as_seconds(), 120.0);
    }

    #[test]
    fn test_as_seconds() {
        let time = Time::from_minutes(2.);
        assert_eq!(time.as_seconds(), 120.);
    }

    #[test]
    fn test_as_minutes() {
        let time = Time::from_seconds(30.0);
        assert_eq!(time.as_minutes(), 0.5);
    }

    #[test]
    fn test_into_time() {
        Time::from_seconds(1.);
        Time::from_seconds(1);
        Time::from_minutes(1.);
        Time::from_minutes(1);
    }
}
