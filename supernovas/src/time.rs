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
            return Err(Error::ffi(rc));
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
            return Err(Error::ffi(rc));
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

#[cfg(feature = "hifitime")]
impl Time {
    /// Convert to a hifitime [`Epoch`], preserving the full TT Julian date.
    ///
    /// The mapping is lossless to ~1 nanosecond (the precision of hifitime's
    /// internal `Duration`). The resulting `Epoch` carries no `dut1`
    /// information (hifitime doesn't model UT1−UTC), so round-tripping through
    /// [`From<hifitime::Epoch>`] will lose the original `dut1`.
    pub fn to_epoch(self) -> hifitime::Epoch {
        let (ijd, fjd) = self.tt_split_jd();
        // Avoid combining ijd+fjd as a single f64 (≈40 µs precision loss).
        // Work in integer nanoseconds throughout.
        // J1900 reference: JDE 2415020.5 (= integer part 2415020, fraction 0.5)
        // TT − TAI = 32.184 s exactly by SI definition
        const NS_PER_DAY: i128 = 86_400_000_000_000;
        const TT_TAI_NS: i128 = 32_184_000_000; // 32.184 s in ns
        const J1900_JD_INT: i64 = 2_415_020;

        let int_ns = (ijd as i128 - J1900_JD_INT as i128) * NS_PER_DAY;
        // Subtract 0.5 for the fractional part of JDE 2415020.5
        let frac_ns = ((fjd - 0.5) * NS_PER_DAY as f64).round() as i128;
        let tai_ns = int_ns + frac_ns - TT_TAI_NS;

        let duration = hifitime::Duration::from_total_nanoseconds(tai_ns);
        hifitime::Epoch::from_duration(duration, hifitime::TimeScale::TAI)
    }

    /// Convert from a hifitime [`Epoch`] with an explicit UT1−UTC offset.
    ///
    /// The leap-second count (`TAI − UTC`) is derived from hifitime's built-in
    /// IERS table. Use [`From<hifitime::Epoch>`] when `dut1` is unavailable —
    /// it defaults to `0.0`, which introduces at most ~0.9 s of error in
    /// UT1-dependent quantities (negligible for az/el at arcsecond accuracy).
    pub fn from_epoch_with_dut1(epoch: hifitime::Epoch, dut1: f64) -> Result<Self> {
        let leap_seconds = epoch.leap_seconds_iers();
        // Use integer nanoseconds to avoid single-f64 JDE precision loss.
        // to_tai_duration() is exact (Duration → i128, no f64 JDE involved).
        const NS_PER_DAY: i128 = 86_400_000_000_000;
        const TT_TAI_NS: i128 = 32_184_000_000;
        // J1900 noon (JDE 2415020.5) expressed as ns: 2415020 full days + 12 h
        const J1900_NOON_NS: i128 = 2_415_020 * NS_PER_DAY + 43_200_000_000_000;

        let tai_ns = epoch.to_tai_duration().total_nanoseconds();
        let tt_jde_ns = tai_ns + TT_TAI_NS + J1900_NOON_NS;

        let ijd = tt_jde_ns.div_euclid(NS_PER_DAY) as i64;
        let fjd = tt_jde_ns.rem_euclid(NS_PER_DAY) as f64 / NS_PER_DAY as f64;

        Self::from_split_jd(NOVAS_TT, ijd, fjd, leap_seconds, dut1)
    }
}

#[cfg(feature = "hifitime")]
impl From<hifitime::Epoch> for Time {
    /// Convert from a hifitime [`Epoch`], assuming `dut1 = 0`.
    ///
    /// The leap-second count is derived from hifitime's built-in IERS table.
    /// Use [`Time::from_epoch_with_dut1`] when you have a precise UT1−UTC value.
    fn from(epoch: hifitime::Epoch) -> Self {
        Time::from_epoch_with_dut1(epoch, 0.0)
            .expect("hifitime Epoch always yields a finite TT JDE")
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

#[cfg(all(test, feature = "hifitime"))]
mod hifitime_tests {
    use super::*;

    #[test]
    fn round_trips_through_hifitime() {
        // Use fjd = 0.75 (exactly representable in f64) to exercise the
        // ns-precision path; compare split JD to avoid tt_jd() catastrophic
        // cancellation when testing sub-nanosecond differences.
        let t = Time::from_tt_jd(2_451_545.75, 37, 0.3).unwrap();
        let t2 = Time::from(t.to_epoch());
        let (ijd, fjd) = t.tt_split_jd();
        let (ijd2, fjd2) = t2.tt_split_jd();
        assert_eq!(ijd, ijd2, "integer JD should round-trip exactly");
        // At most 1 ns = 1.16e-14 days from frac_ns rounding; dut1 is not preserved.
        assert!(
            (fjd - fjd2).abs() < 2e-14,
            "fjd diff {} days > 2 ns",
            (fjd - fjd2).abs()
        );
    }

    #[test]
    fn tt_tai_offset_correct() {
        // J2000.0 TT (JDE 2451545.0) should sit 32.184 s before
        // 2000-01-01 12:00:00 TAI in hifitime's timeline.
        let t = Time::from_tt_jd(2_451_545.0, 32, 0.0).unwrap();
        let j2000_tai = hifitime::Epoch::from_gregorian_tai_at_noon(2000, 1, 1);
        let diff_s = (j2000_tai - t.to_epoch()).to_seconds();
        // Integer-ns arithmetic makes this exact; allow 1 µs for any rounding
        // inside hifitime's gregorian→duration path.
        assert!((diff_s - 32.184).abs() < 1e-6, "TT−TAI offset: {diff_s}");
    }

    #[test]
    fn from_epoch_with_dut1_round_trips() {
        let t = Time::from_tt_jd(2_451_545.75, 37, 0.5).unwrap();
        let t2 = Time::from_epoch_with_dut1(t.to_epoch(), 0.5).unwrap();
        let (ijd, fjd) = t.tt_split_jd();
        let (ijd2, fjd2) = t2.tt_split_jd();
        assert_eq!(ijd, ijd2);
        assert!(
            (fjd - fjd2).abs() < 2e-14,
            "fjd diff {} days > 2 ns",
            (fjd - fjd2).abs()
        );
    }
}
