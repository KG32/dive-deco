//! Math utilities for std/no-std compatibility

#[cfg(feature = "no-std")]
use libm;

/// Absolute value for f64
#[inline]
pub fn abs(val: f64) -> f64 {
    #[cfg(not(feature = "no-std"))]
    {
        val.abs()
    }
    #[cfg(feature = "no-std")]
    {
        libm::fabs(val)
    }
}

/// Ceiling function for f64
#[inline]
pub fn ceil(val: f64) -> f64 {
    #[cfg(not(feature = "no-std"))]
    {
        val.ceil()
    }
    #[cfg(feature = "no-std")]
    {
        libm::ceil(val)
    }
}

/// Power function for f64
#[inline]
pub fn powf(base: f64, exp: f64) -> f64 {
    #[cfg(not(feature = "no-std"))]
    {
        base.powf(exp)
    }
    #[cfg(feature = "no-std")]
    {
        libm::pow(base, exp)
    }
}

/// Round function for f64
#[inline]
pub fn round(val: f64) -> f64 {
    #[cfg(not(feature = "no-std"))]
    {
        val.round()
    }
    #[cfg(feature = "no-std")]
    {
        libm::round(val)
    }
}
