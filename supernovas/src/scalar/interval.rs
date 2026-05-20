//! Signed time interval.

use core::{
    fmt,
    ops::{Add, Neg, Sub},
};

use supernovas_sys::{novas_timescale, novas_timescale::NOVAS_TT};

use crate::{
    error::{Error, Result},
    unit,
};

/// A signed time interval, stored as seconds in a chosen astronomical
/// timescale.
///
/// Constructors reject non-finite inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    seconds: f64,
    timescale: novas_timescale,
}

impl Interval {
    /// Construct from a number of seconds in the given timescale.
    pub fn from_seconds(seconds: f64, timescale: novas_timescale) -> Result<Self> {
        if !seconds.is_finite() {
            return Err(Error::NotFinite);
        }
        Ok(Interval { seconds, timescale })
    }

    /// Construct from milliseconds (in TT).
    pub fn from_millis(ms: f64) -> Result<Self> {
        Self::from_seconds(ms * unit::MS, NOVAS_TT)
    }

    /// Construct from minutes (in TT).
    pub fn from_minutes(min: f64) -> Result<Self> {
        Self::from_seconds(min * unit::MIN, NOVAS_TT)
    }

    /// Construct from hours (in TT).
    pub fn from_hours(hours: f64) -> Result<Self> {
        Self::from_seconds(hours * unit::HOUR, NOVAS_TT)
    }

    /// Construct from days (in TT).
    pub fn from_days(days: f64) -> Result<Self> {
        Self::from_seconds(days * unit::DAY, NOVAS_TT)
    }

    /// Construct from Julian years (in TT).
    pub fn from_julian_years(years: f64) -> Result<Self> {
        Self::from_seconds(years * unit::JULIAN_YEAR, NOVAS_TT)
    }

    /// Construct from Julian centuries (in TT).
    pub fn from_julian_centuries(cy: f64) -> Result<Self> {
        Self::from_seconds(cy * unit::JULIAN_CENTURY, NOVAS_TT)
    }

    /// The interval's timescale.
    pub fn timescale(self) -> novas_timescale {
        self.timescale
    }

    /// The interval in milliseconds.
    pub fn millis(self) -> f64 {
        self.seconds / unit::MS
    }

    /// The interval in seconds.
    pub fn seconds(self) -> f64 {
        self.seconds
    }

    /// The interval in minutes.
    pub fn minutes(self) -> f64 {
        self.seconds / unit::MIN
    }

    /// The interval in hours.
    pub fn hours(self) -> f64 {
        self.seconds / unit::HOUR
    }

    /// The interval in days.
    pub fn days(self) -> f64 {
        self.seconds / unit::DAY
    }

    /// The interval in weeks.
    pub fn weeks(self) -> f64 {
        self.seconds / unit::WEEK
    }

    /// The interval in tropical years (at J2000).
    pub fn years(self) -> f64 {
        self.seconds / unit::YEAR
    }

    /// The interval in Julian years.
    pub fn julian_years(self) -> f64 {
        self.seconds / unit::JULIAN_YEAR
    }

    /// The interval in Julian centuries.
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
}
