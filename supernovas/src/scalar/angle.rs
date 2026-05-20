//! Angular quantity in radians, normalized to (-π, π].

use core::{
    f64::consts::TAU,
    ffi::{CStr, c_char, c_int},
    fmt,
    ops::{Add, Neg, Sub},
    str::FromStr,
};

use supernovas_ffi::{
    novas_print_dms, novas_separator_type::NOVAS_SEP_UNITS_AND_SPACES, novas_str_degrees,
};

use crate::{
    error::{Error, Result},
    unit,
};

/// An angle, stored as a finite number of radians normalized to (-π, π].
///
/// All constructors reject non-finite inputs, so the inner value is always
/// a finite real once you hold an `Angle`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angle(f64);

impl Angle {
    /// Construct from radians. Returns [`Error::NotFinite`] for NaN or infinity.
    pub fn from_radians(rad: f64) -> Result<Self> {
        if !rad.is_finite() {
            return Err(Error::NotFinite);
        }
        Ok(Angle(normalize(rad)))
    }

    /// Construct from degrees.
    pub fn from_degrees(deg: f64) -> Result<Self> {
        Self::from_radians(deg * unit::DEG)
    }

    /// Construct from arc-minutes.
    pub fn from_arcmin(arcmin: f64) -> Result<Self> {
        Self::from_radians(arcmin * unit::ARCMIN)
    }

    /// Construct from arc-seconds.
    pub fn from_arcsec(arcsec: f64) -> Result<Self> {
        Self::from_radians(arcsec * unit::ARCSEC)
    }

    /// Construct from milli-arc-seconds.
    pub fn from_mas(mas: f64) -> Result<Self> {
        Self::from_radians(mas * unit::MAS)
    }

    /// Construct from micro-arc-seconds.
    pub fn from_uas(uas: f64) -> Result<Self> {
        Self::from_radians(uas * unit::UAS)
    }

    /// The angle in radians, in (-π, π].
    pub fn rad(self) -> f64 {
        self.0
    }

    /// The angle in degrees, in (-180, 180].
    pub fn deg(self) -> f64 {
        self.0 / unit::DEG
    }

    /// The angle in arc-minutes.
    pub fn arcmin(self) -> f64 {
        self.0 / unit::ARCMIN
    }

    /// The angle in arc-seconds.
    pub fn arcsec(self) -> f64 {
        self.0 / unit::ARCSEC
    }

    /// The angle in milli-arc-seconds.
    pub fn mas(self) -> f64 {
        self.0 / unit::MAS
    }

    /// The angle in micro-arc-seconds.
    pub fn uas(self) -> f64 {
        self.0 / unit::UAS
    }

    /// The angle as a fraction of a full turn, in [0, 1).
    pub fn fraction(self) -> f64 {
        let f = self.0 / TAU;
        if f >= 0.0 { f } else { 1.0 + f }
    }
}

impl approx::AbsDiffEq for Angle {
    type Epsilon = f64;

    /// Default tolerance: 1 micro-arcsecond, matching the C++ wrapper's
    /// `Angle::operator==` precision.
    fn default_epsilon() -> Self::Epsilon {
        unit::UAS
    }

    /// Wrap-aware comparison: folds `self - other` into (-π, π] before
    /// taking the absolute difference, so e.g. `+179.9999°` and `-179.9999°`
    /// compare equal at arc-second precision.
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        wrapped_diff(self.0, other.0).abs() <= epsilon
    }
}

impl Add for Angle {
    type Output = Angle;
    fn add(self, rhs: Angle) -> Angle {
        Angle(normalize(self.0 + rhs.0))
    }
}

impl Sub for Angle {
    type Output = Angle;
    fn sub(self, rhs: Angle) -> Angle {
        Angle(normalize(self.0 - rhs.0))
    }
}

impl Neg for Angle {
    type Output = Angle;
    fn neg(self) -> Angle {
        Angle(normalize(-self.0))
    }
}

impl FromStr for Angle {
    type Err = Error;

    /// Parse from a DMS string (e.g. `"12:34:56.789"`, `"12d34m56.789s"`) or
    /// decimal degrees. See the C-side `novas_str_degrees()` for the full
    /// accepted grammar.
    fn from_str(s: &str) -> Result<Self> {
        let bytes = s.as_bytes();
        if bytes.contains(&0) || bytes.len() >= 64 {
            return Err(Error::Parse);
        }
        let mut buf = [0u8; 64];
        buf[..bytes.len()].copy_from_slice(bytes);
        let cs = CStr::from_bytes_with_nul(&buf[..=bytes.len()]).map_err(|_| Error::Parse)?;
        let deg = unsafe { novas_str_degrees(cs.as_ptr()) };
        if !deg.is_finite() {
            return Err(Error::Parse);
        }
        Self::from_degrees(deg)
    }
}

impl fmt::Display for Angle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals: c_int = f.precision().map_or(3, |p| p.try_into().unwrap_or(3));
        let mut buf = [0 as c_char; 100];
        let n = unsafe {
            novas_print_dms(
                self.deg(),
                NOVAS_SEP_UNITS_AND_SPACES,
                decimals,
                buf.as_mut_ptr(),
                buf.len() as c_int,
            )
        };
        if n < 0 {
            return Err(fmt::Error);
        }
        let cs = unsafe { CStr::from_ptr(buf.as_ptr()) };
        f.write_str(cs.to_str().map_err(|_| fmt::Error)?)
    }
}

/// Fold `r` into (-π, π].
///
/// Uses floor-based reduction to avoid the half-away-from-zero rounding of
/// `f64::round()`, which would otherwise flip the exact boundaries
/// ±π onto each other inconsistently.
pub(super) fn normalize(r: f64) -> f64 {
    // First into [0, 2π).
    let r = r - TAU * (r / TAU).floor();
    // Then shift the upper half down into (-π, 0), leaving the boundary at +π.
    if r > core::f64::consts::PI {
        r - TAU
    } else {
        r
    }
}

/// Wrap-aware difference: returns `a - b` folded into (-π, π].
pub(super) fn wrapped_diff(a: f64, b: f64) -> f64 {
    normalize(a - b)
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "std"))]
    use std::format;

    use super::*;

    const TIGHT: f64 = 1e-12;

    #[test]
    fn rejects_non_finite() {
        assert!(matches!(
            Angle::from_radians(f64::NAN),
            Err(Error::NotFinite)
        ));
        assert!(matches!(
            Angle::from_degrees(f64::INFINITY),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn round_trip_units() {
        let a = Angle::from_degrees(45.0).unwrap();
        assert!((a.deg() - 45.0).abs() < TIGHT);
        assert!((a.rad() - core::f64::consts::FRAC_PI_4).abs() < TIGHT);
        assert!((a.arcmin() - 45.0 * 60.0).abs() < 1e-9);
        assert!((a.arcsec() - 45.0 * 3600.0).abs() < 1e-7);
    }

    #[test]
    fn normalizes_into_range() {
        // 370° should fold to 10°
        let a = Angle::from_degrees(370.0).unwrap();
        assert!((a.deg() - 10.0).abs() < TIGHT);
        // -190° should fold to 170°
        let b = Angle::from_degrees(-190.0).unwrap();
        assert!((b.deg() - 170.0).abs() < TIGHT);
    }

    #[test]
    fn approx_eq_across_wrap() {
        use approx::assert_abs_diff_eq;
        let a = Angle::from_degrees(179.9999).unwrap();
        let b = Angle::from_degrees(-179.9999).unwrap();
        // 0.0002° apart across the wrap; well within 1 arc-second
        assert_abs_diff_eq!(a, b, epsilon = unit::ARCSEC);
    }

    #[test]
    fn arithmetic() {
        let a = Angle::from_degrees(170.0).unwrap();
        let b = Angle::from_degrees(20.0).unwrap();
        // 190° folds to -170°
        assert!(((a + b).deg() - -170.0).abs() < 1e-10);
        assert!(((a - b).deg() - 150.0).abs() < 1e-10);
        assert!(((-a).deg() - -170.0).abs() < 1e-10);
    }

    #[test]
    fn parses_dms_string() {
        let a: Angle = "12:34:56.789".parse().unwrap();
        let expected = 12.0 + 34.0 / 60.0 + 56.789 / 3600.0;
        assert!((a.deg() - expected).abs() < 1e-9);
    }

    #[test]
    fn displays_dms() {
        let a = Angle::from_degrees(12.5826).unwrap();
        let s = format!("{a}");
        assert!(s.contains("12"), "expected DMS string, got {s:?}");
    }
}
