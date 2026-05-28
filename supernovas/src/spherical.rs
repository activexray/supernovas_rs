//! Spherical-coordinate types: directions on a sphere, with reference-frame-
//! specific newtype wrappers.
//!
//! [`Spherical`] is the geometric base shape (longitude, latitude). The
//! typed variants — [`Equatorial`], [`Ecliptic`], [`Galactic`],
//! [`Horizontal`] — add domain accessors and prevent accidental mixing of
//! reference systems.

mod base;
mod ecliptic;
mod equatorial;
mod galactic;
mod horizontal;

pub use base::Spherical;
pub use ecliptic::Ecliptic;
pub use equatorial::Equatorial;
pub use galactic::Galactic;
pub use horizontal::Horizontal;
