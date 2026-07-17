//! Earth Orientation Parameters (EOP): live IERS data fetch and correction utilities.
//!
//! Requires the `eop` crate feature, which enables CURL in the vendored build
//! so that the `SuperNOVAS` fetch functions are compiled in.
//!
//! # Quick start
//!
//! ```no_run
//! use supernovas::eop;
//!
//! // Fetch EOP for a specific Julian day (blocks up to 5 s).
//! let data = eop::fetch_eop(2_460_676.5, 5_000)?;
//! println!("dut1 = {:.4} s  xp = {}  yp = {}",
//!     data.dut1(), data.xp(), data.yp());
//! # Ok::<(), supernovas::Error>(())
//! ```

use std::{
    ffi::{CStr, CString},
    mem::MaybeUninit,
    path::Path,
};

use supernovas_ffi::{
    novas_diurnal_eop_at_time, novas_eop, novas_eop_series, novas_fetch_eop, novas_fetch_eop_unix,
    novas_get_eop_itrf_year, novas_get_eop_url, novas_is_auto_fetch_eop, novas_itrf_transform_eop,
    novas_reset_eop, novas_set_auto_fetch_eop, novas_set_eop_url, novas_set_leap_list,
};

use crate::{
    Angle, Time,
    error::{Error, Result},
};

/// IERS EOP data series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EopSeries {
    /// Leap-seconds list (since 1972).
    LeapList,
    /// IERS Rapid Service data for IAU 2000 (since 1973, `finals.all.iau2000.txt`).
    RapidIau2000,
    /// IERS C04 long-term data, ITRF 2020, daily at 0 UTC (since 1962).
    C04Iau2000,
    /// IERS C01 long-term data (since 1846).
    C01Iau2000,
}

impl EopSeries {
    fn to_raw(self) -> novas_eop_series {
        match self {
            Self::LeapList => novas_eop_series::EOP_LEAP_LIST,
            Self::RapidIau2000 => novas_eop_series::EOP_RAPID_IAU2000,
            Self::C04Iau2000 => novas_eop_series::EOP_C04_IAU2000_0UTC,
            Self::C01Iau2000 => novas_eop_series::EOP_C01_IAU2000,
        }
    }

    fn from_raw(r: novas_eop_series) -> Self {
        match r {
            novas_eop_series::EOP_LEAP_LIST => Self::LeapList,
            novas_eop_series::EOP_RAPID_IAU2000 => Self::RapidIau2000,
            novas_eop_series::EOP_C04_IAU2000_0UTC => Self::C04Iau2000,
            novas_eop_series::EOP_C01_IAU2000 => Self::C01Iau2000,
        }
    }
}

/// A single Earth Orientation Parameter record from IERS.
#[derive(Debug, Clone, Copy)]
pub struct Eop(novas_eop);

impl Eop {
    fn from_raw(r: novas_eop) -> Self {
        Eop(r)
    }

    /// Which IERS data series this record came from.
    #[must_use]
    pub fn series(&self) -> EopSeries {
        EopSeries::from_raw(self.0.series)
    }

    /// Julian day of the measurement or prognosis (any timescale).
    #[must_use]
    pub fn jd(&self) -> f64 {
        self.0.jd
    }

    /// Leap seconds - TAI − UTC in seconds.
    #[must_use]
    pub fn leap(&self) -> i32 {
        self.0.leap
    }

    /// UT1 − UTC in seconds.
    #[must_use]
    pub fn dut1(&self) -> f32 {
        self.0.dut1
    }

    /// 1-σ error on `dut1`, in seconds.
    #[must_use]
    pub fn dut1_err(&self) -> f32 {
        self.0.dut1_err
    }

    /// Polar offset `x_p` (ITRF x direction).
    #[must_use]
    pub fn xp(&self) -> Angle {
        Angle::from_arcsec(f64::from(self.0.xp)).expect("EOP xp from IERS is always finite")
    }

    /// 1-σ error on `xp`.
    #[must_use]
    pub fn xp_err(&self) -> Angle {
        Angle::from_arcsec(f64::from(self.0.xp_err)).expect("EOP xp_err from IERS is always finite")
    }

    /// Polar offset `y_p` (ITRF y direction).
    #[must_use]
    pub fn yp(&self) -> Angle {
        Angle::from_arcsec(f64::from(self.0.yp)).expect("EOP yp from IERS is always finite")
    }

    /// 1-σ error on `yp`.
    #[must_use]
    pub fn yp_err(&self) -> Angle {
        Angle::from_arcsec(f64::from(self.0.yp_err)).expect("EOP yp_err from IERS is always finite")
    }

    /// Length-of-day differential in seconds.
    #[must_use]
    pub fn lod(&self) -> f32 {
        self.0.lod
    }

    /// 1-σ error on `lod`, in seconds.
    #[must_use]
    pub fn lod_err(&self) -> f32 {
        self.0.lod_err
    }
}

/// Fetch the IERS EOP record closest to `jd` (any timescale).
///
/// `timeout_ms` is the maximum wait time in milliseconds. Pass `0` to use the
/// default timeout.
///
/// Requires network access to the IERS servers.
pub fn fetch_eop(jd: f64, timeout_ms: i64) -> Result<Eop> {
    let mut out = MaybeUninit::<novas_eop>::zeroed();
    // SAFETY: novas_fetch_eop writes *eop on a 0 return.
    let rc = unsafe { novas_fetch_eop(jd, timeout_ms as _, out.as_mut_ptr()) };
    if rc != 0 {
        return Err(Error::ffi(rc));
    }
    Ok(Eop::from_raw(unsafe { out.assume_init() }))
}

/// Fetch the IERS EOP record closest to the given Unix timestamp.
///
/// `timeout_ms` is the maximum wait time in milliseconds.
pub fn fetch_eop_unix(unix_t: i64, timeout_ms: i64) -> Result<Eop> {
    let mut out = MaybeUninit::<novas_eop>::zeroed();
    // SAFETY: novas_fetch_eop_unix writes *eop on a 0 return.
    let rc = unsafe { novas_fetch_eop_unix(unix_t as _, timeout_ms as _, out.as_mut_ptr()) };
    if rc != 0 {
        return Err(Error::ffi(rc));
    }
    Ok(Eop::from_raw(unsafe { out.assume_init() }))
}

/// Clear any cached EOP state inside `SuperNOVAS`.
pub fn reset_eop() {
    // SAFETY: no-arg void function with no preconditions.
    unsafe { novas_reset_eop() };
}

/// Enable (`true`) or disable (`false`) automatic background EOP fetching.
pub fn set_auto_fetch_eop(enabled: bool) -> Result<()> {
    // SAFETY: no preconditions beyond a valid bool → int conversion.
    let rc = unsafe { novas_set_auto_fetch_eop(enabled.into()) };
    if rc != 0 { Err(Error::ffi(rc)) } else { Ok(()) }
}

/// Whether automatic EOP fetching is currently enabled.
#[must_use]
pub fn is_auto_fetch_eop() -> bool {
    // SAFETY: no preconditions.
    unsafe { novas_is_auto_fetch_eop() != 0 }
}

/// Override the IERS download URL for the given `series`.
///
/// `itrf_year` is the ITRF reference year for the new URL (e.g. 2020).
/// `url` must be a valid C string (no interior NUL bytes).
///
/// Because CURL handles the URL, `file://` URLs work too - pass
/// `"file:///absolute/path/to/finals.all.iau2000.txt"` to use a
/// pre-downloaded file instead of the network. See also
/// [`set_eop_file`] for a `Path`-based convenience wrapper.
pub fn set_eop_url(series: EopSeries, itrf_year: i32, url: &str) -> Result<()> {
    let c_url = CString::new(url).map_err(|_| Error::OutOfRange("url contains interior NUL"))?;
    // SAFETY: c_url lives until the end of the statement; novas_set_eop_url
    // copies the string internally and does not retain the pointer.
    let rc = unsafe { novas_set_eop_url(series.to_raw(), itrf_year, c_url.as_ptr()) };
    if rc != 0 { Err(Error::ffi(rc)) } else { Ok(()) }
}

/// Configure `series` to be read from a local pre-downloaded file.
///
/// Equivalent to calling [`set_eop_url`] with a `file://` URL, so CURL is
/// still used internally - the `eop` feature is required.
///
/// The file must be in the official IERS format for the given series:
/// - [`EopSeries::RapidIau2000`][]: `finals.all.iau2000.txt`
/// - [`EopSeries::C04Iau2000`][]: `EOP_20u24_C04_one_file_1962-now.txt`
/// - [`EopSeries::C01Iau2000`][]: `EOP_C01_IAU2000_1846-now.txt`
/// - [`EopSeries::LeapList`][]: `leap-seconds.list` (prefer [`set_leap_list`] - no CURL needed)
///
/// `SuperNOVAS` validates the file format on the first call and caches data
/// across calls, so subsequent EOP lookups for nearby dates are fast.
pub fn set_eop_file(series: EopSeries, itrf_year: i32, path: &Path) -> Result<()> {
    let abs = path
        .canonicalize()
        .map_err(|_| Error::OutOfRange("EOP file path could not be resolved"))?;
    let url = std::format!("file://{}", abs.display());
    set_eop_url(series, itrf_year, &url)
}

/// Return the currently configured IERS URL for `series`, or `None` if unset.
#[must_use]
pub fn get_eop_url(series: EopSeries) -> Option<&'static CStr> {
    // SAFETY: novas_get_eop_url returns a pointer to a static string or NULL.
    let ptr = unsafe { novas_get_eop_url(series.to_raw()) };
    if ptr.is_null() {
        None
    } else {
        // SAFETY: non-null pointer is a static NUL-terminated string owned by
        // the C library.
        Some(unsafe { CStr::from_ptr(ptr) })
    }
}

/// Return the ITRF reference year for the configured URL of `series`.
#[must_use]
pub fn get_eop_itrf_year(series: EopSeries) -> i32 {
    // SAFETY: no preconditions.
    unsafe { novas_get_eop_itrf_year(series.to_raw()) }
}

/// Compute diurnal EOP corrections at the given instant.
///
/// Returns `(dxp, dyp, dut1_s)` where `dxp` and `dyp` are the polar-motion
/// corrections in arcseconds and `dut1_s` is the UT1−UTC correction in seconds.
pub fn diurnal_eop_at_time(time: &Time) -> Result<(Angle, Angle, f64)> {
    let mut dxp = 0.0_f64;
    let mut dyp = 0.0_f64;
    let mut dut1 = 0.0_f64;
    // SAFETY: novas_diurnal_eop_at_time writes the three output doubles on a 0
    // return; the timespec pointer is valid for the duration of the call.
    let rc = unsafe {
        novas_diurnal_eop_at_time(
            time.as_timespec(),
            &raw mut dxp,
            &raw mut dyp,
            &raw mut dut1,
        )
    };
    if rc != 0 {
        return Err(Error::ffi(rc));
    }
    Ok((Angle::from_arcsec(dxp)?, Angle::from_arcsec(dyp)?, dut1))
}

/// Transform EOP parameters (polar motion + UT1) from one ITRF year to another.
///
/// Returns `(to_xp, to_yp, to_dut1_s)`.
#[allow(clippy::similar_names)]
pub fn itrf_transform_eop(
    from_year: i32,
    from_xp: Angle,
    from_yp: Angle,
    from_dut1: f64,
    to_year: i32,
) -> Result<(Angle, Angle, f64)> {
    let mut to_xp = 0.0_f64;
    let mut to_yp = 0.0_f64;
    let mut to_dut1 = 0.0_f64;
    // SAFETY: novas_itrf_transform_eop writes the three output doubles on a 0 return.
    let rc = unsafe {
        novas_itrf_transform_eop(
            from_year,
            from_xp.arcsec(),
            from_yp.arcsec(),
            from_dut1,
            to_year,
            &raw mut to_xp,
            &raw mut to_yp,
            &raw mut to_dut1,
        )
    };
    if rc != 0 {
        return Err(Error::ffi(rc));
    }
    Ok((
        Angle::from_arcsec(to_xp)?,
        Angle::from_arcsec(to_yp)?,
        to_dut1,
    ))
}

/// Load a leap-seconds list from a local file (IERS `leap-seconds.list` format).
///
/// This function does not require CURL; it reads a pre-downloaded file.
pub fn set_leap_list(path: &Path) -> Result<()> {
    let bytes = path.as_os_str().as_encoded_bytes();
    let c_path = CString::new(bytes).map_err(|_| Error::OutOfRange("path contains NUL"))?;
    // SAFETY: c_path lives until the end of the statement.
    let rc = unsafe { novas_set_leap_list(c_path.as_ptr()) };
    if rc != 0 { Err(Error::ffi(rc)) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eop_series_round_trips() {
        for s in [
            EopSeries::LeapList,
            EopSeries::RapidIau2000,
            EopSeries::C04Iau2000,
            EopSeries::C01Iau2000,
        ] {
            assert_eq!(EopSeries::from_raw(s.to_raw()), s);
        }
    }

    #[test]
    fn get_eop_itrf_year_returns_integer() {
        let year = get_eop_itrf_year(EopSeries::RapidIau2000);
        assert!(year >= 0, "expected non-negative ITRF year, got {year}");
    }

    #[test]
    fn is_auto_fetch_eop_does_not_panic() {
        let _ = is_auto_fetch_eop();
    }

    #[test]
    fn diurnal_eop_at_time_returns_finite() {
        let t = crate::Time::from_tt_jd(2_460_676.5, 37, 0.0).unwrap();
        let (dxp, dyp, dut1) = diurnal_eop_at_time(&t).unwrap();
        assert!(dxp.arcsec().is_finite() && dyp.arcsec().is_finite() && dut1.is_finite());
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn itrf_transform_eop_identity_same_year() {
        let xp = Angle::from_arcsec(0.1).unwrap();
        let yp = Angle::from_arcsec(0.2).unwrap();
        let (to_xp, to_yp, to_dut1) = itrf_transform_eop(2020, xp, yp, 0.05, 2020).unwrap();
        assert!((to_xp.arcsec() - 0.1).abs() < 1e-9);
        assert!((to_yp.arcsec() - 0.2).abs() < 1e-9);
        assert!((to_dut1 - 0.05).abs() < 1e-9);
    }
}
