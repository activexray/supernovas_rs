//! Physical temperature, expressible in °C, K, and °F.

use core::fmt;

use crate::error::{Error, Result};

/// A temperature, stored internally as degrees Celsius.
///
/// Constructors reject non-finite inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Temperature(f64);

impl Temperature {
    /// Construct from degrees Celsius.
    pub fn from_celsius(c: f64) -> Result<Self> {
        if !c.is_finite() {
            return Err(Error::NotFinite);
        }
        Ok(Temperature(c))
    }

    /// Construct from kelvin.
    pub fn from_kelvin(k: f64) -> Result<Self> {
        Self::from_celsius(k - 273.15)
    }

    /// Construct from degrees Fahrenheit.
    pub fn from_fahrenheit(f: f64) -> Result<Self> {
        Self::from_celsius((f - 32.0) * 5.0 / 9.0)
    }

    /// The temperature in degrees Celsius.
    pub fn celsius(self) -> f64 {
        self.0
    }

    /// The temperature in kelvin.
    pub fn kelvin(self) -> f64 {
        self.0 + 273.15
    }

    /// The temperature in degrees Fahrenheit.
    pub fn fahrenheit(self) -> f64 {
        self.0 * 9.0 / 5.0 + 32.0
    }
}

impl fmt::Display for Temperature {
    /// Renders as Celsius. Use `{:.N}` to control decimal places
    /// (default 1).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = f.precision().unwrap_or(1);
        write!(f, "{:.decimals$} °C", self.celsius())
    }
}

impl approx::AbsDiffEq for Temperature {
    type Epsilon = f64;

    /// Default tolerance: 0.1 °C.
    fn default_epsilon() -> Self::Epsilon {
        0.1
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        (self.0 - other.0).abs() <= epsilon
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_finite() {
        assert!(matches!(
            Temperature::from_celsius(f64::INFINITY),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn conversions() {
        let t = Temperature::from_celsius(0.0).unwrap();
        assert!((t.kelvin() - 273.15).abs() < 1e-12);
        assert!((t.fahrenheit() - 32.0).abs() < 1e-12);

        let boiling = Temperature::from_celsius(100.0).unwrap();
        assert!((boiling.fahrenheit() - 212.0).abs() < 1e-12);
        assert!((boiling.kelvin() - 373.15).abs() < 1e-12);

        let body = Temperature::from_fahrenheit(98.6).unwrap();
        assert!((body.celsius() - 37.0).abs() < 1e-9);
    }
}
