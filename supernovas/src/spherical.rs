//! Spherical-coordinate types: directions on a sphere, with reference-frame-
//! specific newtype wrappers.
//!
//! [`Spherical`] is the geometric base shape (longitude, latitude). The
//! typed variants — [`Galactic`], [`Horizontal`] — add domain accessors and
//! prevent accidental mixing of reference systems.
//!
//! `Equatorial` and `Ecliptic` coordinate types are not yet implemented.
//! Refraction-corrected and frame-converted variants of [`Horizontal`] are
//! also pending.

mod base;
mod galactic;
mod horizontal;

pub use base::Spherical;
pub use galactic::Galactic;
pub use horizontal::Horizontal;
