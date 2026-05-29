//! Signed time interval.

use core::{
    fmt,
    ops::{Add, Neg, Sub},
};

use crate::{
    error::{Error, Result},
    timescale::Timescale,
    unit,
};

/// A signed time interval, stored as seconds in a chosen astronomical
/// timescale.
///
/// Constructors reject non-finite inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    seconds: f64,
    timescale: Timescale,
}

impl Interval {
    /// Construct from a number of seconds in the given timescale.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if `seconds` is not finite.
    pub fn from_seconds(seconds: f64, timescale: Timescale) -> Result<Self> {
        if !seconds.is_finite() {
            return Err(Error::NotFinite);
        }
        Ok(Interval { seconds, timescale })
    }

    /// Construct from milliseconds (in TT).
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if the resulting interval is not finite.
    pub fn from_millis(ms: f64) -> Result<Self> {
        Self::from_seconds(ms * unit::MS, Timescale::Tt)
    }

    /// Construct from minutes (in TT).
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if the resulting interval is not finite.
    pub fn from_minutes(min: f64) -> Result<Self> {
        Self::from_seconds(min * unit::MIN, Timescale::Tt)
    }

    /// Construct from hours (in TT).
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if the resulting interval is not finite.
    pub fn from_hours(hours: f64) -> Result<Self> {
        Self::from_seconds(hours * unit::HOUR, Timescale::Tt)
    }

    /// Construct from days (in TT).
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if the resulting interval is not finite.
    pub fn from_days(days: f64) -> Result<Self> {
        Self::from_seconds(days * unit::DAY, Timescale::Tt)
    }

    /// Construct from Julian years (in TT).
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if the resulting interval is not finite.
    pub fn from_julian_years(years: f64) -> Result<Self> {
        Self::from_seconds(years * unit::JULIAN_YEAR, Timescale::Tt)
    }

    /// Construct from Julian centuries (in TT).
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if the resulting interval is not finite.
    pub fn from_julian_centuries(cy: f64) -> Result<Self> {
        Self::from_seconds(cy * unit::JULIAN_CENTURY, Timescale::Tt)
    }

    /// The timescale the interval is expressed in.
    #[must_use]
    pub fn timescale(self) -> Timescale {
        self.timescale
    }

    /// The interval in milliseconds.
    #[must_use]
    pub fn millis(self) -> f64 {
        self.seconds / unit::MS
    }

    /// The interval in seconds.
    #[must_use]
    pub fn seconds(self) -> f64 {
        self.seconds
    }

    /// The interval in minutes.
    #[must_use]
    pub fn minutes(self) -> f64 {
        self.seconds / unit::MIN
    }

    /// The interval in hours.
    #[must_use]
    pub fn hours(self) -> f64 {
        self.seconds / unit::HOUR
    }

    /// The interval in days.
    #[must_use]
    pub fn days(self) -> f64 {
        self.seconds / unit::DAY
    }

    /// The interval in weeks.
    #[must_use]
    pub fn weeks(self) -> f64 {
        self.seconds / unit::WEEK
    }

    /// The interval in tropical years (at J2000).
    #[must_use]
    pub fn years(self) -> f64 {
        self.seconds / unit::YEAR
    }

    /// The interval in Julian years.
    #[must_use]
    pub fn julian_years(self) -> f64 {
        self.seconds / unit::JULIAN_YEAR
    }

    /// The interval in Julian centuries.
    #[must_use]
    pub fn julian_centuries(self) -> f64 {
        self.seconds / unit::JULIAN_CENTURY
    }
}

impl Add for Interval {
    type Output = Interval;
    fn add(self, rhs: Interval) -> Interval {
        Interval {
            seconds: self.seconds + rhs.seconds,
            timescale: self.timescale,
        }
    }
}

impl Sub for Interval {
    type Output = Interval;
    fn sub(self, rhs: Interval) -> Interval {
        Interval {
            seconds: self.seconds - rhs.seconds,
            timescale: self.timescale,
        }
    }
}

impl Neg for Interval {
    type Output = Interval;
    fn neg(self) -> Interval {
        Interval {
            seconds: -self.seconds,
            timescale: self.timescale,
        }
    }
}

impl fmt::Display for Interval {
    /// Auto-scales the unit by magnitude: ps → ns → μs → ms → s → min →
    /// hours → days → years. Use `{:.N}` to control decimal places
    /// (default 3).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = f.precision().unwrap_or(3);
        let d = self.seconds.abs();
        let (value, unit_label) = if d < unit::NS {
            (self.seconds / unit::PS, "ps")
        } else if d < unit::US {
            (self.seconds / unit::NS, "ns")
        } else if d < unit::MS {
            (self.seconds / unit::US, "μs")
        } else if d < 1.0 {
            (self.seconds / unit::MS, "ms")
        } else if d < unit::MIN {
            (self.seconds, "s")
        } else if d < unit::HOUR {
            (self.seconds / unit::MIN, "min")
        } else if d < unit::DAY {
            (self.seconds / unit::HOUR, "h")
        } else if d < unit::JULIAN_YEAR {
            (self.seconds / unit::DAY, "d")
        } else {
            (self.seconds / unit::JULIAN_YEAR, "yr")
        };
        write!(f, "{value:.decimals$} {unit_label}")
    }
}

impl approx::AbsDiffEq for Interval {
    type Epsilon = f64;

    /// Default tolerance: 1 microsecond, matching the C++ wrapper's
    /// `Interval::operator==` precision.
    fn default_epsilon() -> Self::Epsilon {
        unit::US
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        (self.seconds - other.seconds).abs() <= epsilon
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_finite() {
        assert!(matches!(
            Interval::from_hours(f64::NAN),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn round_trip_units() {
        let one_hour = Interval::from_hours(1.0).unwrap();
        assert!((one_hour.seconds() - 3600.0).abs() < 1e-12);
        assert!((one_hour.minutes() - 60.0).abs() < 1e-12);
        assert!((one_hour.millis() - 3_600_000.0).abs() < 1e-9);
        assert!((one_hour.days() - 1.0 / 24.0).abs() < 1e-15);
    }

    #[test]
    fn arithmetic() {
        let a = Interval::from_hours(2.0).unwrap();
        let b = Interval::from_minutes(30.0).unwrap();
        assert!(((a + b).hours() - 2.5).abs() < 1e-12);
        assert!(((a - b).hours() - 1.5).abs() < 1e-12);
        assert!(((-a).seconds() - -7200.0).abs() < 1e-12);
    }

    #[test]
    fn remaining_constructors_and_getters() {
        let dt = Interval::from_millis(500.0).unwrap();
        assert!((dt.millis() - 500.0).abs() < 1e-9);

        let dt = Interval::from_minutes(2.0).unwrap();
        assert!((dt.minutes() - 2.0).abs() < 1e-12);

        let dt = Interval::from_days(1.0).unwrap();
        assert!((dt.days() - 1.0).abs() < 1e-12);
        assert!((dt.weeks() - 1.0 / 7.0).abs() < 1e-15);
        assert!((dt.years()).is_finite());

        let dt = Interval::from_julian_years(1.0).unwrap();
        assert!((dt.julian_years() - 1.0).abs() < 1e-12);

        let dt = Interval::from_julian_centuries(1.0).unwrap();
        assert!((dt.julian_centuries() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn from_seconds_preserves_timescale() {
        let dt = Interval::from_seconds(10.0, Timescale::Tcb).unwrap();
        assert_eq!(dt.timescale(), Timescale::Tcb);
        assert!((dt.seconds() - 10.0).abs() < 1e-12);
    }

    #[test]
    fn display_auto_scales() {
        let ps = Interval::from_seconds(1e-13, Timescale::Tt).unwrap();
        assert!(format!("{ps}").contains("ps"));

        let ns = Interval::from_seconds(1e-9, Timescale::Tt).unwrap();
        assert!(format!("{ns}").contains("ns"));

        let us = Interval::from_seconds(1e-6, Timescale::Tt).unwrap();
        assert!(format!("{us}").contains("μs"));

        let ms = Interval::from_millis(1.0).unwrap();
        assert!(format!("{ms}").contains("ms"));

        let s = Interval::from_seconds(5.0, Timescale::Tt).unwrap();
        assert!(format!("{s}").contains(" s"));

        let min = Interval::from_minutes(3.0).unwrap();
        assert!(format!("{min}").contains("min"));

        let h = Interval::from_hours(2.0).unwrap();
        assert!(format!("{h}").contains(" h"));

        let d = Interval::from_days(3.0).unwrap();
        assert!(format!("{d}").contains(" d"));

        let yr = Interval::from_julian_years(2.0).unwrap();
        assert!(format!("{yr}").contains("yr"));
    }
}
