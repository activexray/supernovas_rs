//! Astronomical time.
//!
//! A [`Time`] represents an instant on the astronomical timeline, carrying
//! enough information to convert between any of the standard timescales
//! (UTC, UT1, TAI, TT, TCG, TDB, TCB, GPS). Internally it wraps the C-side
//! `novas_timespec`.
//!
//! Construction takes a Julian date in a specified [`Timescale`] plus the
//! corresponding leap-second count and UT1−UTC offset; `SuperNOVAS` uses
//! those to derive all other timescales internally.

use core::{
    cmp::Ordering,
    fmt,
    mem::MaybeUninit,
    ops::{Add, Sub},
};

use supernovas_ffi::{
    novas_get_split_time, novas_get_time, novas_offset_time, novas_set_split_time,
    novas_set_unix_time, novas_time_leap, novas_timescale_offset, novas_timespec,
};

use crate::{
    error::{Error, Result},
    scalar::Interval,
    timescale::Timescale,
};

/// An instant on the astronomical timeline.
///
/// Construction is via Julian date (in a chosen [`Timescale`]) plus
/// leap-second count and UT1−UTC offset. Once you have a `Time`,
/// `SuperNOVAS` can render it in any supported timescale via [`Self::jd`]
/// and [`Self::split_jd`].
///
/// `PartialEq` and `Ord` compare by Terrestrial Time Julian date — the
/// internal canonical representation — ignoring any difference in `dut1`.
#[derive(Debug, Clone, Copy)]
pub struct Time(novas_timespec);

impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        self.0.ijd_tt == other.0.ijd_tt && self.0.fjd_tt == other.0.fjd_tt
    }
}

impl Eq for Time {}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        #[allow(clippy::useless_conversion)]
        let si = i64::from(self.0.ijd_tt);
        #[allow(clippy::useless_conversion)]
        let oi = i64::from(other.0.ijd_tt);
        match si.cmp(&oi) {
            Ordering::Equal => self
                .0
                .fjd_tt
                .partial_cmp(&other.0.fjd_tt)
                .expect("fjd_tt is always finite"),
            ord => ord,
        }
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
    pub fn from_jd(scale: Timescale, jd: f64, leap_seconds: i32, dut1: f64) -> Result<Self> {
        if !jd.is_finite() || !dut1.is_finite() {
            return Err(Error::NotFinite);
        }
        let ijd = jd.floor() as i64;
        let fjd = jd - ijd as f64;
        Self::from_split_jd(scale, ijd, fjd, leap_seconds, dut1)
    }

    /// Convenience: UTC Julian date.
    pub fn from_utc_jd(jd_utc: f64, leap_seconds: i32, dut1: f64) -> Result<Self> {
        Self::from_jd(Timescale::Utc, jd_utc, leap_seconds, dut1)
    }

    /// Convenience: Terrestrial Time Julian date.
    ///
    /// TT doesn't depend on leap seconds or UT1, but the timespec stores
    /// them for later cross-timescale conversions, so they're still required.
    pub fn from_tt_jd(jd_tt: f64, leap_seconds: i32, dut1: f64) -> Result<Self> {
        Self::from_jd(Timescale::Tt, jd_tt, leap_seconds, dut1)
    }

    /// Construct from a split Julian date (integer + fraction). Useful for
    /// preserving sub-nanosecond precision when the integer day is large.
    pub fn from_split_jd(
        scale: Timescale,
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
            novas_set_split_time(
                scale.to_sys(),
                ijd as _,
                fjd,
                leap_seconds,
                dut1,
                ts.as_mut_ptr(),
            )
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

    /// Construct from the system clock (wall time).
    ///
    /// Equivalent to reading the OS clock and converting to a [`Time`] using
    /// the given leap-second count and UT1−UTC offset. Requires `std`.
    #[cfg(feature = "std")]
    pub fn now(leap_seconds: i32, dut1: f64) -> Result<Self> {
        if !dut1.is_finite() {
            return Err(Error::NotFinite);
        }
        let mut ts = MaybeUninit::<novas_timespec>::zeroed();
        let rc =
            unsafe { supernovas_ffi::novas_set_current_time(leap_seconds, dut1, ts.as_mut_ptr()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(Time(unsafe { ts.assume_init() }))
    }

    /// The Julian date in the requested timescale.
    ///
    /// Precision: the underlying timespec stores integer and fractional parts
    /// separately, so sub-nanosecond information may be lost in the returned
    /// `f64` sum. Use [`Self::split_jd`] when that matters.
    #[must_use]
    pub fn jd(self, scale: Timescale) -> f64 {
        unsafe { novas_get_time(core::ptr::addr_of!(self.0), scale.to_sys()) }
    }

    /// The Julian date in the requested timescale as an `(integer_jd, fractional_jd)`
    /// pair, preserving the full timespec precision.
    #[must_use]
    pub fn split_jd(self, scale: Timescale) -> (i64, f64) {
        let mut ijd: core::ffi::c_long = 0;
        let fjd = unsafe {
            novas_get_split_time(
                core::ptr::addr_of!(self.0),
                scale.to_sys(),
                core::ptr::addr_of_mut!(ijd),
            )
        };
        #[allow(clippy::useless_conversion)]
        (i64::from(ijd), fjd)
    }

    /// The Terrestrial Time Julian date, as a single `f64` sum.
    ///
    /// Shorthand for `self.jd(Timescale::Tt)` that reads the stored fields
    /// directly without an FFI call. Use [`Self::tt_split_jd`] when
    /// sub-nanosecond precision matters.
    #[must_use]
    pub fn tt_jd(self) -> f64 {
        self.0.ijd_tt as f64 + self.0.fjd_tt
    }

    /// The Terrestrial Time Julian date as a `(integer_jd, fractional_jd)`
    /// pair, preserving the full timespec precision.
    ///
    /// Shorthand for `self.split_jd(Timescale::Tt)` that avoids an FFI call.
    #[must_use]
    pub fn tt_split_jd(self) -> (i64, f64) {
        // `c_long` is `i64` on 64-bit Unix and `i32` on Windows / 32-bit;
        // `i64::from` covers both (identity on the former, widening on the
        // latter). Clippy can't tell from a single target, so we silence
        // its overzealous `useless_conversion`.
        #[allow(clippy::useless_conversion)]
        (i64::from(self.0.ijd_tt), self.0.fjd_tt)
    }

    /// Returns the leap second count (TAI − UTC) associated with this time.
    ///
    /// This is the value passed to the constructor; it does not change with
    /// timescale conversion. As of mid-2026 the count is **37**.
    #[must_use]
    pub fn leap_seconds(self) -> i32 {
        unsafe { novas_time_leap(core::ptr::addr_of!(self.0)) }
    }

    /// The clock offset of `scale` relative to `reference` at this instant,
    /// in seconds (`scale − reference`).
    ///
    /// For example, `t.timescale_offset(Timescale::Tdb, Timescale::Tt)` gives
    /// the TDB−TT difference (typically a few milliseconds).
    #[must_use]
    pub fn timescale_offset(self, scale: Timescale, reference: Timescale) -> f64 {
        unsafe {
            novas_timescale_offset(
                core::ptr::addr_of!(self.0),
                scale.to_sys(),
                reference.to_sys(),
            )
        }
    }

    /// Borrow the underlying C representation, for passing to FFI functions
    /// that take a `*const novas_timespec`.
    #[must_use]
    pub fn as_timespec(&self) -> &novas_timespec {
        &self.0
    }
}

impl Add<Interval> for Time {
    type Output = Time;

    fn add(self, rhs: Interval) -> Time {
        let mut out = MaybeUninit::<novas_timespec>::zeroed();
        // novas_offset_time only fails on NULL pointer args, which we don't pass.
        let rc = unsafe {
            novas_offset_time(core::ptr::addr_of!(self.0), rhs.seconds(), out.as_mut_ptr())
        };
        assert_eq!(rc, 0, "novas_offset_time failed");
        Time(unsafe { out.assume_init() })
    }
}

impl Sub<Interval> for Time {
    type Output = Time;

    fn sub(self, rhs: Interval) -> Time {
        self + (-rhs)
    }
}

impl Sub<Time> for Time {
    type Output = Interval;

    /// Returns the difference `self − rhs` as a TT interval.
    fn sub(self, rhs: Time) -> Interval {
        #[allow(clippy::useless_conversion)]
        let dij = i64::from(self.0.ijd_tt) - i64::from(rhs.0.ijd_tt);
        let dfj = self.0.fjd_tt - rhs.0.fjd_tt;
        let seconds = (dij as f64 + dfj) * 86_400.0;
        Interval::from_seconds(seconds, Timescale::Tt)
            .expect("TT JD difference is always finite for valid Times")
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
    #[must_use]
    pub fn to_epoch(self) -> hifitime::Epoch {
        // Avoid combining ijd+fjd as a single f64 (≈40 µs precision loss).
        // Work in integer nanoseconds throughout.
        // J1900 reference: JDE 2415020.5 (= integer part 2415020, fraction 0.5)
        // TT − TAI = 32.184 s exactly by SI definition
        const NS_PER_DAY: i128 = 86_400_000_000_000;
        const TT_TAI_NS: i128 = 32_184_000_000; // 32.184 s in ns
        const J1900_JD_INT: i128 = 2_415_020;
        let (ijd, fjd) = self.tt_split_jd();

        let int_ns = (i128::from(ijd) - J1900_JD_INT) * NS_PER_DAY;
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
        // Use integer nanoseconds to avoid single-f64 JDE precision loss.
        // to_tai_duration() is exact (Duration → i128, no f64 JDE involved).
        const NS_PER_DAY: i128 = 86_400_000_000_000;
        const TT_TAI_NS: i128 = 32_184_000_000;
        // J1900 noon (JDE 2415020.5) expressed as ns: 2415020 full days + 12 h
        const J1900_NOON_NS: i128 = 2_415_020 * NS_PER_DAY + 43_200_000_000_000;
        let leap_seconds = epoch.leap_seconds_iers();

        let tai_ns = epoch.to_tai_duration().total_nanoseconds();
        let tt_jde_ns = tai_ns + TT_TAI_NS + J1900_NOON_NS;

        let ijd = tt_jde_ns.div_euclid(NS_PER_DAY) as i64;
        let fjd = tt_jde_ns.rem_euclid(NS_PER_DAY) as f64 / NS_PER_DAY as f64;

        Self::from_split_jd(Timescale::Tt, ijd, fjd, leap_seconds, dut1)
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

    #[test]
    fn from_split_jd_directly() {
        let t = Time::from_split_jd(Timescale::Tt, 2_451_545, 0.0, 37, 0.0).unwrap();
        let (ijd, fjd) = t.tt_split_jd();
        assert_eq!(ijd, 2_451_545);
        assert!(fjd.abs() < 1e-9);
    }

    #[test]
    fn from_split_jd_rejects_non_finite() {
        assert!(matches!(
            Time::from_split_jd(Timescale::Tt, 0, f64::NAN, 0, 0.0),
            Err(Error::NotFinite)
        ));
        assert!(matches!(
            Time::from_split_jd(Timescale::Tt, 0, 0.0, 0, f64::INFINITY),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn from_unix_non_zero() {
        // 1000 seconds past Unix epoch — larger gap to avoid leap-second
        // edge effects near the epoch (leap count was 0 before 1972).
        let t0 = Time::from_unix(0, 0, 0, 0.0).unwrap();
        let t1 = Time::from_unix(1_000, 0, 0, 0.0).unwrap();
        let diff = (t1.tt_jd() - t0.tt_jd()) * 86_400.0;
        assert!((diff - 1_000.0).abs() < 1e-3, "expected 1000 s, got {diff}");
    }

    #[test]
    fn from_unix_rejects_non_finite_dut1() {
        assert!(matches!(
            Time::from_unix(0, 0, 0, f64::NAN),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn as_timespec_gives_same_jd() {
        let t = Time::from_tt_jd(JD_J2000_TT, 37, 0.0).unwrap();
        let ts = t.as_timespec();
        let reconstructed = ts.ijd_tt as f64 + ts.fjd_tt;
        assert!((reconstructed - JD_J2000_TT).abs() < 1e-9);
    }

    #[test]
    fn display_contains_jd() {
        let t = Time::from_tt_jd(JD_J2000_TT, 37, 0.0).unwrap();
        let s = format!("{t}");
        assert!(s.contains("JD"), "got: {s}");
        assert!(s.contains("TT"), "got: {s}");
    }

    #[test]
    fn partial_eq() {
        let a = Time::from_tt_jd(JD_J2000_TT, 37, 0.0).unwrap();
        let b = Time::from_tt_jd(JD_J2000_TT, 37, 0.0).unwrap();
        let c = Time::from_tt_jd(JD_J2000_TT + 1.0, 37, 0.0).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn eq_ignores_dut1() {
        // Two times at the same TT instant but different dut1 should compare equal.
        let a = Time::from_tt_jd(JD_J2000_TT, 37, 0.0).unwrap();
        let b = Time::from_tt_jd(JD_J2000_TT, 37, 0.5).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn ord() {
        let t0 = Time::from_tt_jd(JD_J2000_TT, 37, 0.0).unwrap();
        let t1 = Time::from_tt_jd(JD_J2000_TT + 1.0, 37, 0.0).unwrap();
        assert!(t0 < t1);
        assert!(t1 > t0);
        assert_eq!(t0.cmp(&t0), core::cmp::Ordering::Equal);
    }

    #[test]
    fn add_interval() {
        let t = Time::from_tt_jd(JD_J2000_TT, 37, 0.0).unwrap();
        let dt = Interval::from_seconds(86_400.0, Timescale::Tt).unwrap();
        let t2 = t + dt;
        let diff = (t2.tt_jd() - (JD_J2000_TT + 1.0)).abs();
        assert!(diff < 1e-9, "expected +1 day, got {diff} days");
    }

    #[test]
    fn sub_interval() {
        let t = Time::from_tt_jd(JD_J2000_TT, 37, 0.0).unwrap();
        let dt = Interval::from_seconds(86_400.0, Timescale::Tt).unwrap();
        let t2 = t - dt;
        let diff = (t2.tt_jd() - (JD_J2000_TT - 1.0)).abs();
        assert!(diff < 1e-9, "expected -1 day, got {diff} days");
    }

    #[test]
    fn sub_time() {
        let t0 = Time::from_tt_jd(JD_J2000_TT, 37, 0.0).unwrap();
        let t1 = Time::from_tt_jd(JD_J2000_TT + 1.0, 37, 0.0).unwrap();
        let dt = t1 - t0;
        assert!(
            (dt.seconds() - 86_400.0).abs() < 1e-6,
            "expected 86400 s, got {}",
            dt.seconds()
        );
    }

    #[test]
    fn jd_timescale() {
        // UTC and TT Julian dates differ at J2000.0 (32 leap s + 32.184 s offset).
        let t = Time::from_tt_jd(JD_J2000_TT, 32, 0.0).unwrap();
        let utc_jd = t.jd(Timescale::Utc);
        let tt_jd = t.jd(Timescale::Tt);
        // TT = UTC + 64.184 s with leap=32, so TT is 64.184 s ahead.
        // f64 subtraction of two ~2.45M-day JDs loses ~50 µs; allow 1 ms.
        let diff_s = (tt_jd - utc_jd) * 86_400.0;
        assert!(
            (diff_s - 64.184).abs() < 1e-3,
            "expected 64.184 s, got {diff_s}"
        );
    }

    #[test]
    fn split_jd_timescale() {
        let t = Time::from_tt_jd(JD_J2000_TT, 37, 0.0).unwrap();
        let (ijd, fjd) = t.split_jd(Timescale::Tt);
        assert!((ijd as f64 + fjd - JD_J2000_TT).abs() < 1e-9);
    }

    #[test]
    fn timescale_offset_tdb_tt() {
        // TDB−TT at J2000.0 is < 2 ms in magnitude.
        let t = Time::from_tt_jd(JD_J2000_TT, 37, 0.0).unwrap();
        let offset = t.timescale_offset(Timescale::Tdb, Timescale::Tt);
        assert!(offset.abs() < 2e-3, "TDB−TT too large: {offset} s");
    }

    #[test]
    fn leap_seconds_roundtrips() {
        let t = Time::from_tt_jd(JD_J2000_TT, 37, 0.0).unwrap();
        assert_eq!(t.leap_seconds(), 37);
    }

    #[test]
    fn abs_diff_eq() {
        use approx::AbsDiffEq;
        // Use split JD to avoid fp cancellation when adding tiny fractions to
        // a large JD integer (e.g. 2451545 + 1e-11 rounds back to 2451545.0).
        let a = Time::from_split_jd(Timescale::Tt, 2_451_545, 0.0, 37, 0.0).unwrap();
        // Same instant: within tolerance.
        let b = Time::from_split_jd(Timescale::Tt, 2_451_545, 0.0, 37, 0.0).unwrap();
        assert!(a.abs_diff_eq(&b, Time::default_epsilon()));
        // 2 µs offset — outside 1 µs tolerance.
        let c = Time::from_split_jd(Timescale::Tt, 2_451_545, 2e-6 / 86_400.0, 37, 0.0).unwrap();
        assert!(!a.abs_diff_eq(&c, Time::default_epsilon()));
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

    const NS_PER_DAY: f64 = 86_400e9;

    /// Total round-trip error of `TT JD -> Epoch -> TT JD` in nanoseconds,
    /// combining the integer-day and fractional-day differences (the C side
    /// may re-normalize which integer day carries the fraction).
    fn round_trip_err_ns(ijd: i64, frac: f64) -> f64 {
        let t = Time::from_split_jd(Timescale::Tt, ijd, frac, 37, 0.0).unwrap();
        let back = Time::from(t.to_epoch());
        let (i0, f0) = t.tt_split_jd();
        let (i1, f1) = back.tt_split_jd();
        (((i0 - i1) as f64) + (f0 - f1)).abs() * NS_PER_DAY
    }

    #[test]
    fn round_trip_preserves_ns_across_dates_and_fractions() {
        // The only lossy step in to_epoch() is a single round-to-nearest-ns,
        // so the full round-trip must close to <= 1 ns for every instant —
        // across a wide span of dates and every part of the day.
        for &ijd in &[2_400_001_i64, 2_451_545, 2_460_000, 2_500_000] {
            for k in 0..=20 {
                let frac = f64::from(k) / 20.0; // 0.00, 0.05, ..., 1.00
                let err = round_trip_err_ns(ijd, frac);
                assert!(
                    err <= 1.0,
                    "ijd={ijd} frac={frac}: round-trip off by {err} ns"
                );
            }
        }
    }

    #[test]
    fn large_jd_tiny_fraction_no_cancellation() {
        // A naive `ijd as f64 + fjd` JDE loses ~40 µs of resolution at these
        // magnitudes. The integer-nanosecond path must keep a 1 ns fractional
        // offset intact rather than rounding it away.
        let one_ns = 1.0 / NS_PER_DAY;
        for &ijd in &[2_451_545_i64, 2_460_000, 2_500_000] {
            let err = round_trip_err_ns(ijd, one_ns);
            assert!(err <= 1.0, "ijd={ijd}: 1 ns fraction lost ({err} ns error)");
        }
    }
}
