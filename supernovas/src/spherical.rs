//! Spherical-coordinate types: directions on a sphere, with reference-frame-
//! specific newtype wrappers.
//!
//! [`Spherical`] is the geometric base shape (longitude, latitude). The
//! typed variants — [`Galactic`], [`Horizontal`] — add domain accessors and
//! prevent accidental mixing of reference systems.
//!
//! `Equatorial` and `Ecliptic` arrive once the `Equinox` machinery lands;
//! refraction- and frame-aware conversions on `Horizontal` (and the
//! `Apparent` round-trip) follow once `Frame` lands.

mod base;
mod galactic;
mod horizontal;

pub use base::Spherical;
pub use galactic::Galactic;
pub use horizontal::Horizontal;
