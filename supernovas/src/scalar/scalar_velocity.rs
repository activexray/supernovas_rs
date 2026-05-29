//! Scalar velocity (signed speed along a single axis).
//!
//! Despite the name, this represents a *speed*: a signed scalar that may
//! describe e.g. a radial velocity (positive = receding). For full 3D
//! velocity vectors, see `Velocity`.

use core::{
    fmt,
    ops::{Add, Neg, Sub},
};

use supernovas_ffi::{NOVAS_C, novas_v2z, novas_z2v};

use crate::{
    error::{Error, Result},
    unit,
};

/// A signed speed, stored internally as meters per second.
///
/// Constructors reject non-finite inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScalarVelocity(f64);

impl ScalarVelocity {
    /// Construct from meters per second.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if `v` is not finite.
    pub fn from_m_per_s(v: f64) -> Result<Self> {
        if !v.is_finite() {
            return Err(Error::NotFinite);
        }
        Ok(ScalarVelocity(v))
    }

    /// Construct from kilometers per second.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if the resulting speed is not finite.
    pub fn from_km_per_s(v: f64) -> Result<Self> {
        Self::from_m_per_s(v * unit::KM_PER_S)
    }

    /// Construct from astronomical units per day.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if the resulting speed is not finite.
    pub fn from_au_per_day(v: f64) -> Result<Self> {
        Self::from_m_per_s(v * unit::AU_PER_DAY)
    }

    /// Construct from a dimensionless redshift `z = Δλ/λ_rest`, using the
    /// special-relativistic relation `1 + z = √((1+β)/(1−β))`. Delegates to
    /// the `SuperNOVAS` C-side `novas_z2v()`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if `z` is not finite or the resulting
    /// speed is not finite.
    pub fn from_redshift(z: f64) -> Result<Self> {
        if !z.is_finite() {
            return Err(Error::NotFinite);
        }
        let km_per_s = unsafe { novas_z2v(z) };
        Self::from_km_per_s(km_per_s)
    }

    /// The speed in meters per second.
    #[must_use]
    pub fn m_per_s(self) -> f64 {
        self.0
    }

    /// The speed in kilometers per second.
    #[must_use]
    pub fn km_per_s(self) -> f64 {
        self.0 / unit::KM_PER_S
    }

    /// The speed in astronomical units per day.
    #[must_use]
    pub fn au_per_day(self) -> f64 {
        self.0 / unit::AU_PER_DAY
    }

    /// Dimensionless `β = v / c`.
    #[must_use]
    pub fn beta(self) -> f64 {
        self.0 / NOVAS_C
    }

    /// Lorentz factor `γ = 1 / √(1 − β²)`.
    #[must_use]
    pub fn gamma(self) -> f64 {
        let b = self.beta();
        1.0 / (1.0 - b * b).sqrt()
    }

    /// Relativistic Doppler redshift `z = √((1+β)/(1−β)) − 1`. Delegates to
    /// the `SuperNOVAS` C-side `novas_v2z()`.
    #[must_use]
    pub fn redshift(self) -> f64 {
        unsafe { novas_v2z(self.km_per_s()) }
    }

    /// Absolute magnitude as an unsigned speed.
    #[must_use]
    pub fn abs(self) -> ScalarVelocity {
        ScalarVelocity(self.0.abs())
    }
}

impl Add for ScalarVelocity {
    type Output = ScalarVelocity;
    fn add(self, rhs: ScalarVelocity) -> ScalarVelocity {
        ScalarVelocity(self.0 + rhs.0)
    }
}

impl Sub for ScalarVelocity {
    type Output = ScalarVelocity;
    fn sub(self, rhs: ScalarVelocity) -> ScalarVelocity {
        ScalarVelocity(self.0 - rhs.0)
    }
}

impl Neg for ScalarVelocity {
    type Output = ScalarVelocity;
    fn neg(self) -> ScalarVelocity {
        ScalarVelocity(-self.0)
    }
}

impl fmt::Display for ScalarVelocity {
    /// Renders as km/s, the astronomical convention. Use `{:.N}` to control
    /// decimal places (default 3).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = f.precision().unwrap_or(3);
        write!(f, "{:.decimals$} km/s", self.km_per_s())
    }
}

impl approx::AbsDiffEq for ScalarVelocity {
    type Epsilon = f64;

    /// Default tolerance: 1 mm/s, matching the C++ wrapper's
    /// `ScalarVelocity::operator==` precision.
    fn default_epsilon() -> Self::Epsilon {
        unit::MM / unit::SEC
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
            ScalarVelocity::from_m_per_s(f64::NAN),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn km_per_s_round_trip() {
        let v = ScalarVelocity::from_km_per_s(29.5).unwrap();
        assert!((v.m_per_s() - 29_500.0).abs() < 1e-9);
        assert!((v.km_per_s() - 29.5).abs() < 1e-12);
    }

    #[test]
    fn redshift_round_trip() {
        let v = ScalarVelocity::from_redshift(0.1).unwrap();
        assert!((v.redshift() - 0.1).abs() < 1e-12);
    }

    #[test]
    fn beta_of_c_over_two() {
        let v = ScalarVelocity::from_m_per_s(0.5 * NOVAS_C).unwrap();
        assert!((v.beta() - 0.5).abs() < 1e-12);
        // γ at β=0.5 is 2/√3 ≈ 1.1547
        assert!((v.gamma() - 2.0 / 3f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn au_per_day_round_trip() {
        let v = ScalarVelocity::from_au_per_day(1.0).unwrap();
        assert!((v.au_per_day() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn abs_of_negative() {
        let v = ScalarVelocity::from_km_per_s(-30.0).unwrap();
        assert!((v.abs().km_per_s() - 30.0).abs() < 1e-12);
    }

    #[test]
    fn arithmetic_ops() {
        let a = ScalarVelocity::from_km_per_s(10.0).unwrap();
        let b = ScalarVelocity::from_km_per_s(3.0).unwrap();
        assert!(((a + b).km_per_s() - 13.0).abs() < 1e-12);
        assert!(((a - b).km_per_s() - 7.0).abs() < 1e-12);
        assert!(((-a).km_per_s() - -10.0).abs() < 1e-12);
    }

    #[test]
    fn display_renders_km_per_s() {
        let v = ScalarVelocity::from_km_per_s(29.5).unwrap();
        let s = format!("{v}");
        assert!(s.contains("km/s"), "got: {s}");
    }
}
