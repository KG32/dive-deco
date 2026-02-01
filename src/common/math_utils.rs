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

/// Simple e^x wrapper
#[inline]
pub fn exp(exponent: f64) -> f64 {
    #[cfg(not(feature = "no-std"))]
    {
        exponent.exp()
    }
    #[cfg(feature = "no-std")]
    {
        libm::exp(exponent)
    }
}

/// Simple ln(x) wrapper
#[inline]
pub fn ln(val: f64) -> f64 {
    #[cfg(not(feature = "no-std"))]
    {
        val.ln()
    }
    #[cfg(feature = "no-std")]
    {
        libm::log(val)
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

/// Base-2 exponential function for f64
#[inline]
pub fn exp2(val: f64) -> f64 {
    #[cfg(not(feature = "no-std"))]
    {
        val.exp2()
    }
    #[cfg(feature = "no-std")]
    {
        libm::exp2(val)
    }
}
