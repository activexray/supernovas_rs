//! Safe Rust bindings to [SuperNOVAS](https://github.com/sigmyne/supernovas), a
//! high-precision astrometry library based on NOVAS (Naval Observatory Vector
//! Astrometry Software).
//!
//! # What's implemented
//!
//! - **Scalars**: [`Angle`], [`TimeAngle`], [`Coordinate`], [`Interval`],
//!   [`Pressure`], [`Temperature`], [`ScalarVelocity`] — dimensioned newtypes
//!   with validated constructors and unit-aware accessors.
//! - **Vectors**: [`Position`] and [`Velocity`] — 3-D Cartesian vectors with
//!   arithmetic operators and cross-type ops (`Position / Interval → Velocity`,
//!   `Velocity × Interval → Position`).
//! - **Spherical coordinates**: [`Spherical`] (base shape), [`Horizontal`]
//!   (az/el), [`Equatorial`] (RA/Dec + equinox), [`Ecliptic`] (λ/β + equinox),
//!   [`Galactic`] (`l`/`b`).
//! - **Reference systems**: [`ReferenceSystem`], [`Equinox`] — tag coordinate
//!   sets with the equatorial frame they were computed in.
//! - **Refraction**: [`Refraction`] — choose None / Standard / Optical / Radio
//!   when converting apparent equatorial → horizontal.
//! - **Time**: [`Time`] — UTC, TT, and split Julian dates, plus Unix epoch.
//! - **Observers**: [`Observer`] — geodetic ground site ([`Site`] + [`Weather`])
//!   and geocenter.
//! - **Sources**: [`CatalogEntry`] — ICRS sidereal source with optional proper
//!   motion, parallax, and radial velocity.
//! - **Frame + observation**: [`Frame`] — observer × time snapshot; call
//!   [`Frame::observe`] for a quick az/el, or [`CatalogEntry::apparent_in`]
//!   to get an [`Apparent`] position with access to intermediate RA/Dec and
//!   conversion to any output frame.
//!
//! # Quick start
//!
//! ```no_run
//! use supernovas::{Accuracy, CatalogEntry, Frame, Observer, Site, Time, Weather};
//!
//! // Vega in ICRS J2000
//! let vega = CatalogEntry::icrs(
//!     "Vega",
//!     "18:36:56.336".parse().unwrap(),
//!     "+38:47:01.28".parse().unwrap(),
//! ).unwrap();
//!
//! // OVRO site with standard atmosphere
//! let site = Site::from_degrees(37.234, -118.282, 1222.0).unwrap()
//!     .with_weather(Weather::standard());
//! let observer = Observer::Geodetic(site);
//!
//! // 2026-07-15 06:00 UTC (37 leap seconds, dut1 ≈ 0)
//! let time = Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap();
//!
//! let frame = Frame::new(Accuracy::Reduced, &observer, &time).unwrap();
//! let horizontal = frame.observe(&vega).unwrap();
//! println!("{horizontal}");
//! ```
//!
//! The raw FFI bindings are re-exported as [`sys`] for callers that need to
//! drop down to the C layer directly.

#![cfg_attr(not(feature = "std"), no_std)]

// Test binaries always link std; make it available in test code.
#[cfg(all(test, not(feature = "std")))]
extern crate std;

pub use supernovas_ffi as sys;

pub mod apparent;
#[cfg(any(feature = "calceph", feature = "anise"))]
pub mod ephemeris;
pub mod equinox;
pub mod error;
pub mod frame;
pub mod observer;
pub mod refraction;
pub mod scalar;
pub mod source;
pub mod spherical;
pub mod time;
pub mod unit;
pub mod vector;

pub use apparent::{Apparent, ReferenceSystem};
#[cfg(feature = "anise")]
pub use ephemeris::AniseEphemeris;
#[cfg(feature = "calceph")]
pub use ephemeris::CalcephEphemeris;
#[cfg(any(feature = "calceph", feature = "anise"))]
pub use ephemeris::{Ephemeris, EphemerisProvider, PlanetProvider};
pub use equinox::Equinox;
pub use error::{Error, Result};
pub use frame::{Accuracy, Frame};
pub use observer::{Observer, Site, Weather};
pub use refraction::Refraction;
pub use scalar::{Angle, Coordinate, Interval, Pressure, ScalarVelocity, Temperature, TimeAngle};
pub use source::{
    CatalogEntry, EphemObject, OrbitalElements, OrbitalObject, Planet, SolarBody, Source,
};
pub use spherical::{Ecliptic, Equatorial, Galactic, Horizontal, Spherical};
pub use time::Time;
pub use vector::{Position, Velocity};
