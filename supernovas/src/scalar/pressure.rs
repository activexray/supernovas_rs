//! Physical pressure, expressible in pascals and related units.

use core::fmt;

use crate::{
    error::{Error, Result},
    unit,
};

/// A pressure, stored internally as pascals.
///
/// Constructors reject non-finite inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pressure(f64);

impl Pressure {
    /// Construct from pascals.
    pub fn from_pa(pa: f64) -> Result<Self> {
        if !pa.is_finite() {
            return Err(Error::NotFinite);
        }
        Ok(Pressure(pa))
    }

    /// Construct from hectopascals.
    pub fn from_hpa(hpa: f64) -> Result<Self> {
        Self::from_pa(hpa * unit::HPA)
    }

    /// Construct from millibars (= hectopascals).
    pub fn from_mbar(mbar: f64) -> Result<Self> {
        Self::from_pa(mbar * unit::MBAR)
    }

    /// Construct from bars.
    pub fn from_bar(bar: f64) -> Result<Self> {
        Self::from_pa(bar * unit::BAR)
    }

    /// Construct from kilopascals.
    pub fn from_kpa(kpa: f64) -> Result<Self> {
        Self::from_pa(kpa * unit::KPA)
    }

    /// Construct from megapascals.
    pub fn from_mpa(mpa: f64) -> Result<Self> {
        Self::from_pa(mpa * unit::MPA)
    }

    /// Construct from torr (millimeters of mercury).
    pub fn from_torr(torr: f64) -> Result<Self> {
        Self::from_pa(torr * unit::TORR)
    }

    /// Construct from standard atmospheres.
    pub fn from_atm(atm: f64) -> Result<Self> {
        Self::from_pa(atm * unit::ATM)
    }

    /// The pressure in pascals.
    pub fn pa(self) -> f64 {
        self.0
    }

    /// The pressure in hectopascals.
    pub fn hpa(self) -> f64 {
        self.0 / unit::HPA
    }

    /// The pressure in millibars (= hectopascals).
    pub fn mbar(self) -> f64 {
        self.0 / unit::MBAR
    }

    /// The pressure in bars.
    pub fn bar(self) -> f64 {
        self.0 / unit::BAR
    }

    /// The pressure in kilopascals.
    pub fn kpa(self) -> f64 {
        self.0 / unit::KPA
    }

    /// The pressure in megapascals.
    pub fn mpa(self) -> f64 {
        self.0 / unit::MPA
    }

    /// The pressure in torr.
    pub fn torr(self) -> f64 {
        self.0 / unit::TORR
    }

    /// The pressure in standard atmospheres.
    pub fn atm(self) -> f64 {
        self.0 / unit::ATM
    }
}

impl fmt::Display for Pressure {
    /// Renders as millibars (≡ hectopascals), the convention SuperNOVAS
    /// uses internally for atmospheric pressure. Use `{:.N}` to control
    /// decimal places (default 1).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = f.precision().unwrap_or(1);
        write!(f, "{:.decimals$} mbar", self.mbar())
    }
}

impl approx::AbsDiffEq for Pressure {
    type Epsilon = f64;

    /// Default tolerance: 1 Pa.
    fn default_epsilon() -> Self::Epsilon {
        1.0
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
        assert!(matches!(Pressure::from_pa(f64::NAN), Err(Error::NotFinite)));
    }

    #[test]
    fn one_atm_round_trip() {
        let p = Pressure::from_atm(1.0).unwrap();
        assert!((p.pa() - 101_325.0).abs() < 1e-9);
        assert!((p.hpa() - 1013.25).abs() < 1e-9);
        assert!((p.mbar() - 1013.25).abs() < 1e-9);
        assert!((p.torr() - 760.0).abs() < 1e-6);
    }

    #[test]
    fn bar_and_kpa() {
        let one_bar = Pressure::from_bar(1.0).unwrap();
        assert!((one_bar.pa() - 1e5).abs() < 1e-9);
        assert!((one_bar.kpa() - 100.0).abs() < 1e-9);
    }
}
