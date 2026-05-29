//! Astronomical timescale selector.

use core::fmt;

use supernovas_ffi::novas_timescale;

/// An astronomical timescale.
///
/// Identifies which timescale a Julian date or time interval is expressed in.
/// Pass a `Timescale` to [`crate::Time::from_jd`], [`crate::Time::jd`],
/// [`crate::Interval::from_seconds`], and related methods to avoid leaking
/// raw FFI constants into user code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Timescale {
    /// Barycentric Coordinate Time (TCB).
    Tcb,
    /// Barycentric Dynamical Time (TDB).
    Tdb,
    /// Geocentric Coordinate Time (TCG).
    Tcg,
    /// Terrestrial Time (TT).
    Tt,
    /// International Atomic Time (TAI).
    Tai,
    /// GPS Time.
    Gps,
    /// Universal Coordinated Time (UTC).
    Utc,
    /// UT1 earth rotation time, based on IERS Bulletin A.
    Ut1,
}

impl Timescale {
    pub(crate) fn to_sys(self) -> novas_timescale {
        match self {
            Timescale::Tcb => novas_timescale::NOVAS_TCB,
            Timescale::Tdb => novas_timescale::NOVAS_TDB,
            Timescale::Tcg => novas_timescale::NOVAS_TCG,
            Timescale::Tt => novas_timescale::NOVAS_TT,
            Timescale::Tai => novas_timescale::NOVAS_TAI,
            Timescale::Gps => novas_timescale::NOVAS_GPS,
            Timescale::Utc => novas_timescale::NOVAS_UTC,
            Timescale::Ut1 => novas_timescale::NOVAS_UT1,
        }
    }
}

impl fmt::Display for Timescale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Timescale::Tcb => "TCB",
            Timescale::Tdb => "TDB",
            Timescale::Tcg => "TCG",
            Timescale::Tt => "TT",
            Timescale::Tai => "TAI",
            Timescale::Gps => "GPS",
            Timescale::Utc => "UTC",
            Timescale::Ut1 => "UT1",
        })
    }
}
