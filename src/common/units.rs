use super::Depth;

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

    fn from_metric(val: T) -> Self;
    fn from_imperial(val: T) -> Self;

    fn metric(&self) -> T;
    fn imperial(&self) -> T;
}

pub struct DepthUnit {
    m: Depth,
}

impl Unit for DepthUnit {
    fn from_metric(val: Depth) -> Self {
        Self { m: val }
    }
    fn from_imperial(val: Depth) -> Self {
        Self { m: val * 0.3048 }
    }
    fn metric(&self) -> Depth {
        self.m
    }
    fn imperial(&self) -> Depth {
        self.m * 3.28084
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m_to_ft() {
        let depth = DepthUnit::from_metric(1.);
        let ft = depth.imperial();
        assert_eq!(ft, 3.28084);
    }

    #[test]
    fn ft_to_m() {
        let depth = DepthUnit::from_imperial(100.);
        let m = depth.metric();
        assert_eq!(m, 30.48);
    }

    #[test]
    fn depth_conversion_factors() {
        let depth = DepthUnit::from_metric(1.);
        let ft = depth.imperial();
        let new_depth = DepthUnit::from_imperial(ft);
        let m = new_depth.metric();
        assert_eq!(with_precision(m, 5), 1.);
    }

    fn with_precision(x: f64, precision: u32) -> f64 {
        let d = 10_u32.pow(precision) as f64;
        (x * d).round() / d
    }
}
