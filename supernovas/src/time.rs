//! Astronomical time.
//!
//! A [`Time`] represents an instant on the astronomical timeline, carrying
//! enough information to convert between any of the standard timescales
//! (UTC, UT1, TAI, TT, TCG, TDB, TCB, GPS). Internally it wraps the C-side
//! `novas_timespec`.
//!
//! Construction takes a Julian date in a specified timescale plus the
//! corresponding leap-second count and UT1−UTC offset; SuperNOVAS uses those
//! to derive the others.

use core::{fmt, mem::MaybeUninit};

use supernovas_ffi::{
    novas_set_split_time, novas_set_unix_time, novas_timescale,
    novas_timescale::{NOVAS_TT, NOVAS_UTC},
    novas_timespec,
};

use crate::error::{Error, Result};

/// An instant on the astronomical timeline.
///
/// Construction is via Julian date plus leap-second count and UT1−UTC
/// offset. Once you have a `Time`, SuperNOVAS can render it in any of the
/// supported timescales.
#[derive(Debug, Clone, Copy)]
pub struct Time(novas_timespec);

impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        self.0.ijd_tt == other.0.ijd_tt
            && self.0.fjd_tt == other.0.fjd_tt
            && self.0.tt2tdb == other.0.tt2tdb
            && self.0.ut1_to_tt == other.0.ut1_to_tt
    }
}

impl Time {
    /// Construct from a Julian date in the given timescale.
    ///
    /// - `leap_seconds`: the current count of leap seconds (`TAI − UTC`). As
    ///   of mid-2026 this is **37**.
    /// - `dut1`: UT1 − UTC in seconds, typically `|dut1| < 0.9` s. Pass
    ///   `0.0` if you don't have a precise value — the error contribution
    ///   to az/el is well under an arc-second.
    pub fn from_jd(scale: novas_timescale, jd: f64, leap_seconds: i32, dut1: f64) -> Result<Self> {
        if !jd.is_finite() || !dut1.is_finite() {
            return Err(Error::NotFinite);
        }
        let ijd = jd.floor() as i64;
        let fjd = jd - ijd as f64;
        Self::from_split_jd(scale, ijd, fjd, leap_seconds, dut1)
    }

    /// Convenience: UTC Julian date.
    pub fn from_utc_jd(jd_utc: f64, leap_seconds: i32, dut1: f64) -> Result<Self> {
        Self::from_jd(NOVAS_UTC, jd_utc, leap_seconds, dut1)
    }

    /// Convenience: Terrestrial Time Julian date.
    ///
    /// TT doesn't depend on leap seconds or UT1, but the timespec stores
    /// them for later cross-timescale conversions, so they're still required.
    pub fn from_tt_jd(jd_tt: f64, leap_seconds: i32, dut1: f64) -> Result<Self> {
        Self::from_jd(NOVAS_TT, jd_tt, leap_seconds, dut1)
    }

    /// Construct from a split Julian date (integer + fraction). Useful for
    /// preserving sub-nanosecond precision when the integer day is large.
    pub fn from_split_jd(
        scale: novas_timescale,
        ijd: i64,
        fjd: f64,
        leap_seconds: i32,
        dut1: f64,
    ) -> Result<Self> {
        if !fjd.is_finite() || !dut1.is_finite() {
            return Err(Error::NotFinite);
        }
        let mut ts = MaybeUninit::<novas_timespec>::zeroed();
        let rc = unsafe {
            novas_set_split_time(scale, ijd as _, fjd, leap_seconds, dut1, ts.as_mut_ptr())
        };
        if rc != 0 {
            return Err(Error::Parse);
        }
        Ok(Time(unsafe { ts.assume_init() }))
    }

    /// Construct from Unix epoch seconds + nanoseconds.
    pub fn from_unix(secs: i64, nanos: i64, leap_seconds: i32, dut1: f64) -> Result<Self> {
        if !dut1.is_finite() {
            return Err(Error::NotFinite);
        }
        let mut ts = MaybeUninit::<novas_timespec>::zeroed();
        let rc = unsafe {
            novas_set_unix_time(secs as _, nanos as _, leap_seconds, dut1, ts.as_mut_ptr())
        };
        if rc != 0 {
            return Err(Error::Parse);
        }
        Ok(Time(unsafe { ts.assume_init() }))
    }

    /// The Terrestrial Time Julian date, as a single-precision sum.
    ///
    /// Precision: the underlying timespec stores integer and fractional
    /// parts separately, so sub-nanosecond information may be lost in this
    /// `f64` sum. Use [`Self::tt_split_jd`] when that matters.
    pub fn tt_jd(self) -> f64 {
        self.0.ijd_tt as f64 + self.0.fjd_tt
    }

    /// The Terrestrial Time Julian date as a `(integer_jd, fractional_jd)`
    /// pair, preserving the full timespec precision.
    pub fn tt_split_jd(self) -> (i64, f64) {
        // `c_long` is `i64` on 64-bit Unix and `i32` on Windows / 32-bit;
        // `i64::from` covers both (identity on the former, widening on the
        // latter). Clippy can't tell that from a single target, so we silence
        // its overzealous `useless_conversion`.
        #[allow(clippy::useless_conversion)]
        (i64::from(self.0.ijd_tt), self.0.fjd_tt)
    }

    /// Borrow the underlying C representation, for passing to FFI functions
    /// that take a `*const novas_timespec`.
    pub fn as_timespec(&self) -> &novas_timespec {
        &self.0
    }
}

impl fmt::Display for Time {
    /// Renders as `JD <tt_jd> TT`. Use `{:.N}` to control decimal places
    /// (default 6, ~85 ms resolution).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = f.precision().unwrap_or(6);
        write!(f, "JD {:.decimals$} TT", self.tt_jd())
    }
}

impl approx::AbsDiffEq for Time {
    type Epsilon = f64;

    /// Default tolerance: 1 microsecond.
    fn default_epsilon() -> Self::Epsilon {
        1e-6 / 86_400.0
    }

    /// Equal when the TT Julian dates differ by less than `epsilon` days.
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        // Compare the split forms component-wise to keep precision.
        let dij = (self.0.ijd_tt - other.0.ijd_tt) as f64;
        let dfj = self.0.fjd_tt - other.0.fjd_tt;
        (dij + dfj).abs() <= epsilon
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    /// J2000.0 expressed as TT Julian date.
    const JD_J2000_TT: f64 = 2_451_545.0;

    #[test]
    fn rejects_non_finite() {
        assert!(matches!(
            Time::from_utc_jd(f64::NAN, 37, 0.0),
            Err(Error::NotFinite)
        ));
        assert!(matches!(
            Time::from_utc_jd(2_451_545.0, 37, f64::INFINITY),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn round_trip_tt_jd() {
        let t = Time::from_tt_jd(JD_J2000_TT, 32, 0.0).unwrap();
        assert!((t.tt_jd() - JD_J2000_TT).abs() < 1e-9);
    }

    #[test]
    fn utc_and_tt_differ_by_tai_offset() {
        // Specifying the *same numerical JD* in UTC vs in TT picks two distinct
        // instants. The UTC=2451545.0 instant occurs (leap + 32.184) seconds
        // later in TT than the TT=2451545.0 instant; with leap=32 that's
        // exactly 64.184 s.
        let utc = Time::from_utc_jd(JD_J2000_TT, 32, 0.0).unwrap();
        let tt = Time::from_tt_jd(JD_J2000_TT, 32, 0.0).unwrap();
        let (_, fjd_utc) = utc.tt_split_jd();
        let (_, fjd_tt) = tt.tt_split_jd();
        let diff_seconds = (fjd_utc - fjd_tt) * 86_400.0;
        assert!(
            (diff_seconds - 64.184).abs() < 1e-12,
            "expected TT - UTC = 64.184 s at J2000.0 with leap=32, got {diff_seconds}"
        );
    }

    #[test]
    fn split_jd_preserves_precision() {
        let t = Time::from_tt_jd(JD_J2000_TT, 32, 0.0).unwrap();
        let (ijd, fjd) = t.tt_split_jd();
        assert!((ijd as f64 + fjd - JD_J2000_TT).abs() < 1e-12);
    }

    #[test]
    fn unix_epoch_is_1970() {
        // Unix epoch (1970-01-01 00:00:00 UTC) is JD 2440587.5 UTC.
        let t = Time::from_unix(0, 0, 0, 0.0).unwrap();
        // TT = UTC + 32.184 (no leap seconds before 1972), so TT_jd ≈ 2440587.5 + 32.184/86400
        let expected_tt = 2_440_587.5 + 32.184 / 86_400.0;
        assert_abs_diff_eq!(t.tt_jd(), expected_tt, epsilon = 1e-9);
    }
}
