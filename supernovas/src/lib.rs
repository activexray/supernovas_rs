//! Safe Rust bindings to [SuperNOVAS](https://github.com/sigmyne/supernovas), a
//! high-precision astrometry library based on NOVAS (Naval Observatory Vector
//! Astrometry Software).

#![cfg_attr(not(feature = "std"), no_std)]
// `Error` embeds a fixed-capacity inline FFI message buffer (see
// [`error::FfiMessage`]) so it carries a human-readable description without
// allocating, identically under `std` and `no_std`. That makes the enum
// ~140 bytes and trips `result_large_err`; boxing it to satisfy the lint would
// reintroduce the heap allocation we deliberately avoid, so we accept the
// larger `Result` instead.
#![allow(clippy::result_large_err)]
// This crate does heavy f64 astronomy and integer↔float Julian-date
// arithmetic, where lossy / sign-changing / truncating casts and exact float
// comparisons (e.g. bit-identity checks against the C library) are intentional
// and correct. The pedantic lints that flag those produce only noise here, so
// they are allowed crate-wide; every other pedantic lint stays denied (see the
// workspace `Cargo.toml`).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::float_cmp,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc
)]

// Test binaries always link std; make it available in test code.
#[cfg(all(test, not(feature = "std")))]
extern crate std;

pub use supernovas_ffi as sys;

pub mod apparent;
pub mod debug;
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
pub mod timescale;
pub mod unit;
pub mod vector;

pub use apparent::{Apparent, ReferenceSystem};
pub use debug::{DebugMode, enable_debug_mode, get_debug_mode};
#[cfg(feature = "anise")]
pub use ephemeris::AniseEphemeris;
#[cfg(feature = "calceph")]
pub use ephemeris::CalcephEphemeris;
#[cfg(any(feature = "calceph", feature = "anise"))]
pub use ephemeris::{Ephemeris, EphemerisProvider, PlanetProvider};
pub use equinox::Equinox;
pub use error::{Error, FfiMessage, Result, set_provider_error, take_provider_error};
pub use frame::{Accuracy, Frame};
pub use observer::{Observer, Site, Weather};
pub use refraction::Refraction;
pub use scalar::{Angle, Coordinate, Interval, Pressure, ScalarVelocity, Temperature, TimeAngle};
pub use source::{
    CatalogEntry, EphemObject, OrbitalElements, OrbitalObject, Planet, SolarBody, Source,
};
pub use spherical::{Ecliptic, Equatorial, Galactic, Horizontal, Spherical};
pub use time::Time;
pub use timescale::Timescale;
pub use vector::{Position, Velocity};
