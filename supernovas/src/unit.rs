//! Conversion factors from common units to SI base units.
//!
//! Each constant is the value of "1 unit" expressed in SI base units, so you
//! can convert by multiplying: `0.5 * unit::DEG` is half a degree in radians.
//!
//! Mirrors the relevant subset of C++ `supernovas::Unit`. Categories are
//! added as their corresponding wrapper types come online.

use supernovas_sys::{
    NOVAS_ARCSEC, NOVAS_BESSELIAN_YEAR_DAYS, NOVAS_DAY, NOVAS_DEGREE, NOVAS_HOURANGLE, NOVAS_KM,
    NOVAS_LIGHT_YEAR, NOVAS_TROPICAL_YEAR_DAYS,
};

// -- Angle ------------------------------------------------------------------

/// 1 radian (the SI unit of plane angle).
pub const RAD: f64 = 1.0;

/// 1 degree expressed in radians.
pub const DEG: f64 = NOVAS_DEGREE;

/// 1 hour-angle (one twenty-fourth of a full turn) in radians.
pub const HOUR_ANGLE: f64 = NOVAS_HOURANGLE;

/// 1 arc-minute in radians.
pub const ARCMIN: f64 = DEG / 60.0;

/// 1 arc-second in radians.
pub const ARCSEC: f64 = NOVAS_ARCSEC;

/// 1 milli-arc-second in radians.
pub const MAS: f64 = 1e-3 * ARCSEC;

/// 1 micro-arc-second in radians.
pub const UAS: f64 = 1e-6 * ARCSEC;

// -- Length -----------------------------------------------------------------

/// 1 meter (the SI unit of length).
pub const M: f64 = 1.0;

/// 1 centimeter in meters.
pub const CM: f64 = 1e-2;

/// 1 millimeter in meters.
pub const MM: f64 = 1e-3;

/// 1 micrometer in meters.
pub const UM: f64 = 1e-6;

/// 1 nanometer in meters.
pub const NM: f64 = 1e-9;

/// 1 ångström in meters.
pub const ANGSTROM: f64 = 1e-10;

/// 1 kilometer in meters.
pub const KM: f64 = NOVAS_KM;

/// 1 astronomical unit in meters (IAU 2012 Resolution B2).
///
/// The upstream `NOVAS_AU` macro is not exposed in our bindings, so we mirror
/// the literal from `include/novas.h`.
pub const AU: f64 = 1.495978707e+11;

/// 1 parsec in meters (= 1 AU / 1 arc-second).
pub const PC: f64 = AU / ARCSEC;

/// 1 kiloparsec in meters.
pub const KPC: f64 = 1e3 * PC;

/// 1 megaparsec in meters.
pub const MPC: f64 = 1e6 * PC;

/// 1 gigaparsec in meters.
pub const GPC: f64 = 1e9 * PC;

/// 1 light-year in meters.
pub const LYR: f64 = NOVAS_LIGHT_YEAR;

// -- Time -------------------------------------------------------------------

/// 1 picosecond in seconds.
pub const PS: f64 = 1e-12;

/// 1 nanosecond in seconds.
pub const NS: f64 = 1e-9;

/// 1 microsecond in seconds.
pub const US: f64 = 1e-6;

/// 1 millisecond in seconds.
pub const MS: f64 = 1e-3;

/// 1 second (the SI unit of time).
pub const SEC: f64 = 1.0;

/// 1 minute in seconds.
pub const MIN: f64 = 60.0;

/// 1 hour in seconds.
pub const HOUR: f64 = 3600.0;

/// 1 day in seconds.
pub const DAY: f64 = NOVAS_DAY;

/// 1 week in seconds.
pub const WEEK: f64 = 7.0 * DAY;

/// 1 tropical year in seconds (at J2000).
pub const YEAR: f64 = NOVAS_TROPICAL_YEAR_DAYS * DAY;

/// 1 tropical century in seconds (at J2000).
pub const CENTURY: f64 = 100.0 * YEAR;

/// 1 Besselian year in seconds.
pub const BESSELIAN_YEAR: f64 = NOVAS_BESSELIAN_YEAR_DAYS * DAY;

/// 1 Julian year in seconds.
pub const JULIAN_YEAR: f64 = 365.25 * DAY;

/// 1 Julian century in seconds.
pub const JULIAN_CENTURY: f64 = 100.0 * JULIAN_YEAR;

// -- Speed ------------------------------------------------------------------

/// 1 m/s (the SI unit of speed).
pub const M_PER_S: f64 = 1.0;

/// 1 km/s in m/s.
pub const KM_PER_S: f64 = KM;

/// 1 AU/day in m/s.
pub const AU_PER_DAY: f64 = AU / DAY;

// -- Pressure ---------------------------------------------------------------

/// 1 pascal (the SI unit of pressure).
pub const PA: f64 = 1.0;

/// 1 hectopascal in pascals.
pub const HPA: f64 = 100.0;

/// 1 millibar in pascals (= 1 hPa).
pub const MBAR: f64 = HPA;

/// 1 bar in pascals.
pub const BAR: f64 = 1e5;

/// 1 kilopascal in pascals.
pub const KPA: f64 = 1e3;

/// 1 megapascal in pascals.
pub const MPA: f64 = 1e6;

/// 1 torr (millimeter of mercury) in pascals.
pub const TORR: f64 = 133.322_368_421_1;

/// 1 standard atmosphere in pascals.
pub const ATM: f64 = 101_325.0;
