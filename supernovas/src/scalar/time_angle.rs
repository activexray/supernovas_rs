//! Angle expressed as time on the 24-hour circle.

use alloc::ffi::CString;
use core::{
    f64::consts::TAU,
    ffi::{CStr, c_char, c_int},
    fmt,
    ops::{Add, Neg, Sub},
    str::FromStr,
};

use supernovas_sys::{
    novas_print_hms, novas_separator_type::NOVAS_SEP_UNITS_AND_SPACES, novas_str_hours,
};

use super::{Angle, Interval};
use crate::{
    error::{Error, Result},
    unit,
};

/// An angle interpretable as a time value on the 0–24 hour circle, stored
/// internally as radians in `[0, 2π)`.
///
/// Unlike [`Angle`] (which normalizes to `(-π, π]`), `TimeAngle` is always
/// non-negative. Used for things like sidereal time, hour angles, and right
/// ascension when you want time-of-day units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeAngle(f64);

impl TimeAngle {
    /// Construct from radians. Returns [`Error::NotFinite`] for NaN or infinity.
    pub fn from_radians(rad: f64) -> Result<Self> {
        if !rad.is_finite() {
            return Err(Error::NotFinite);
        }
        Ok(TimeAngle(normalize(rad)))
    }

    /// Construct from hours of time (`[0, 24)` after normalization).
    pub fn from_hours(hours: f64) -> Result<Self> {
        Self::from_radians(hours * unit::HOUR_ANGLE)
    }

    /// Construct from minutes of time (`[0, 1440)` after normalization).
    pub fn from_minutes(minutes: f64) -> Result<Self> {
        Self::from_hours(minutes / 60.0)
    }

    /// Construct from seconds of time (`[0, 86400)` after normalization).
    pub fn from_seconds(seconds: f64) -> Result<Self> {
        Self::from_hours(seconds / 3600.0)
    }

    /// Convert an [`Angle`] (in `(-π, π]`) to a `TimeAngle` (in `[0, 2π)`).
    pub fn from_angle(angle: Angle) -> Self {
        TimeAngle(normalize(angle.rad()))
    }

    /// The value in radians, in `[0, 2π)`.
    pub fn rad(self) -> f64 {
        self.0
    }

    /// The value in degrees, in `[0, 360)`.
    pub fn deg(self) -> f64 {
        self.0 / unit::DEG
    }

    /// The value as hours of time, in `[0, 24)`.
    pub fn hours(self) -> f64 {
        self.0 / unit::HOUR_ANGLE
    }

    /// The value as minutes of time, in `[0, 1440)`.
    pub fn minutes(self) -> f64 {
        self.hours() * 60.0
    }

    /// The value as seconds of time, in `[0, 86400)`.
    pub fn seconds(self) -> f64 {
        self.hours() * 3600.0
    }
}

impl From<Angle> for TimeAngle {
    fn from(a: Angle) -> Self {
        Self::from_angle(a)
    }
}

impl From<TimeAngle> for Angle {
    fn from(t: TimeAngle) -> Self {
        // TimeAngle is in [0, 2π); Angle normalizes to (-π, π] via its own
        // construction. Safe to unwrap: t.0 is a finite real.
        Angle::from_radians(t.0).expect("TimeAngle inner value is finite by construction")
    }
}

impl Add for TimeAngle {
    type Output = TimeAngle;
    fn add(self, rhs: TimeAngle) -> TimeAngle {
        TimeAngle(normalize(self.0 + rhs.0))
    }
}

impl Sub for TimeAngle {
    type Output = TimeAngle;
    fn sub(self, rhs: TimeAngle) -> TimeAngle {
        TimeAngle(normalize(self.0 - rhs.0))
    }
}

impl Add<Interval> for TimeAngle {
    type Output = TimeAngle;

    /// Advance a time-of-day by a wall-clock interval (1 second of time ≡ 1 second of arc on the
    /// hour-angle circle).
    fn add(self, rhs: Interval) -> TimeAngle {
        TimeAngle(normalize(
            self.0 + rhs.seconds() * unit::HOUR_ANGLE / 3600.0,
        ))
    }
}

impl Sub<Interval> for TimeAngle {
    type Output = TimeAngle;
    fn sub(self, rhs: Interval) -> TimeAngle {
        TimeAngle(normalize(
            self.0 - rhs.seconds() * unit::HOUR_ANGLE / 3600.0,
        ))
    }
}

impl Neg for TimeAngle {
    type Output = TimeAngle;
    fn neg(self) -> TimeAngle {
        TimeAngle(normalize(-self.0))
    }
}

impl approx::AbsDiffEq for TimeAngle {
    type Epsilon = f64;

    /// Default tolerance: 1 micro-arcsecond, matching [`Angle`]'s default.
    fn default_epsilon() -> Self::Epsilon {
        unit::UAS
    }

    /// Wrap-aware comparison: folds `self - other` into `(-π, π]` before
    /// taking the absolute difference, so `23h59m` and `0h00m` compare equal
    /// at second-of-time precision.
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        super::angle::wrapped_diff(self.0, other.0).abs() <= epsilon
    }
}

impl FromStr for TimeAngle {
    type Err = Error;

    /// Parse from an HMS string (e.g. `"12:34:56.789"`, `"12h34m56.789s"`)
    /// or decimal hours. See the C-side `novas_str_hours()` for the full
    /// accepted grammar.
    fn from_str(s: &str) -> Result<Self> {
        let cs = CString::new(s).map_err(|_| Error::Parse(s.into()))?;
        let hours = unsafe { novas_str_hours(cs.as_ptr()) };
        if !hours.is_finite() {
            return Err(Error::Parse(s.into()));
        }
        Self::from_hours(hours)
    }
}

impl fmt::Display for TimeAngle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals: c_int = f.precision().map_or(3, |p| p.try_into().unwrap_or(3));
        let mut buf = [0 as c_char; 100];
        let n = unsafe {
            novas_print_hms(
                self.hours(),
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

/// Fold `r` into `[0, 2π)`.
fn normalize(r: f64) -> f64 {
    let r = r - TAU * (r / TAU).floor();
    // Guard against floating-point edge cases where r could land exactly on TAU.
    if r >= TAU { 0.0 } else { r }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn rejects_non_finite() {
        assert!(matches!(
            TimeAngle::from_hours(f64::NAN),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn round_trip_units() {
        let t = TimeAngle::from_hours(6.0).unwrap();
        assert!((t.hours() - 6.0).abs() < 1e-12);
        assert!((t.minutes() - 360.0).abs() < 1e-9);
        assert!((t.seconds() - 21600.0).abs() < 1e-7);
        // 6h = 90° = π/2 rad
        assert!((t.rad() - core::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!((t.deg() - 90.0).abs() < 1e-12);
    }

    #[test]
    fn normalizes_into_24h_range() {
        // 25h should fold to 1h
        let t = TimeAngle::from_hours(25.0).unwrap();
        assert!((t.hours() - 1.0).abs() < 1e-12);
        // -1h should fold to 23h
        let t = TimeAngle::from_hours(-1.0).unwrap();
        assert!((t.hours() - 23.0).abs() < 1e-12);
    }

    #[test]
    fn approx_eq_across_midnight() {
        let a = TimeAngle::from_hours(23.99999).unwrap();
        let b = TimeAngle::from_hours(0.00001).unwrap();
        // ~72 ms apart across midnight; well under one second of time
        let one_sec = unit::HOUR_ANGLE / 3600.0;
        assert_abs_diff_eq!(a, b, epsilon = one_sec);
    }

    #[test]
    fn arithmetic() {
        let a = TimeAngle::from_hours(10.0).unwrap();
        let b = TimeAngle::from_hours(16.0).unwrap();
        // 26h → 2h
        assert!(((a + b).hours() - 2.0).abs() < 1e-12);
        // 10h - 16h → -6h → 18h
        assert!(((a - b).hours() - 18.0).abs() < 1e-12);
    }

    #[test]
    fn add_interval_advances_time_of_day() {
        let t = TimeAngle::from_hours(12.0).unwrap();
        let dt = Interval::from_minutes(30.0).unwrap();
        let later = t + dt;
        assert!((later.hours() - 12.5).abs() < 1e-12);
    }

    #[test]
    fn from_angle_lifts_negatives() {
        let a = Angle::from_degrees(-90.0).unwrap();
        let t = TimeAngle::from_angle(a);
        // -90° lifts to 270° = 18h
        assert!((t.hours() - 18.0).abs() < 1e-12);
    }

    #[test]
    fn parses_hms_string() {
        let t: TimeAngle = "12:34:56.789".parse().unwrap();
        let expected = 12.0 + 34.0 / 60.0 + 56.789 / 3600.0;
        assert!((t.hours() - expected).abs() < 1e-9);
    }

    #[test]
    fn displays_hms() {
        let t = TimeAngle::from_hours(12.5).unwrap();
        let s = format!("{t}");
        assert!(s.contains("12"), "expected HMS string, got {s:?}");
    }
}
