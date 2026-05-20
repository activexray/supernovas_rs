//! Horizontal coordinates: azimuth and elevation as seen from a site.

use core::{f64::consts::FRAC_PI_2, fmt};

use super::Spherical;
use crate::{Angle, Coordinate, Position, error::Result, unit};

/// A direction on the local sky, expressed as azimuth and elevation.
///
/// Azimuth is measured eastward from north along the horizon. Elevation is
/// measured upward from the horizon (positive above, negative below).
///
/// Refraction-aware conversions (`to_refracted`, `to_unrefracted`) and the
/// transformation to/from astrometric positions live in the `Frame` layer
/// once it lands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Horizontal(Spherical);

impl Horizontal {
    /// Construct from typed azimuth and elevation.
    pub fn new(azimuth: Angle, elevation: Angle) -> Self {
        Horizontal(Spherical::new(azimuth, elevation))
    }

    /// Construct from azimuth and elevation in radians.
    pub fn from_radians(azimuth: f64, elevation: f64) -> Result<Self> {
        Ok(Horizontal(Spherical::from_radians(azimuth, elevation)?))
    }

    /// Construct from azimuth and elevation in degrees.
    pub fn from_degrees(azimuth: f64, elevation: f64) -> Result<Self> {
        Ok(Horizontal(Spherical::from_degrees(azimuth, elevation)?))
    }

    /// Azimuth (longitude analog).
    pub fn azimuth(self) -> Angle {
        self.0.longitude()
    }

    /// Elevation (latitude analog).
    pub fn elevation(self) -> Angle {
        self.0.latitude()
    }

    /// Zenith angle: complement of the elevation, in `[0, π]`. Zero looks
    /// straight up, π/2 is on the horizon.
    pub fn zenith_angle(self) -> Angle {
        Angle::from_radians(FRAC_PI_2 - self.elevation().rad())
            .expect("FRAC_PI_2 minus a finite angle is finite")
    }

    /// The bare [`Spherical`] view for cases that don't care about the
    /// reference system.
    pub fn as_spherical(self) -> Spherical {
        self.0
    }

    /// Great-circle angular separation between this direction and `other`.
    pub fn distance_to(self, other: Horizontal) -> Angle {
        self.0.distance_to(other.0)
    }

    /// Cartesian position at the given distance along this direction, in
    /// horizon-aligned axes (x toward north, y toward east, z toward zenith
    /// — but be careful: the underlying transform uses the spherical
    /// convention with x toward `lon=0, lat=0`).
    pub fn xyz(self, distance: Coordinate) -> Position {
        self.0.xyz(distance)
    }
}

impl fmt::Display for Horizontal {
    /// Renders azimuth and elevation as decimal degrees — the convention for
    /// observing logs and telescope control. Use `{:.N}` to control decimal
    /// places (default 3).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = f.precision().unwrap_or(3);
        write!(
            f,
            "az={:.decimals$}° el={:.decimals$}°",
            self.azimuth().deg(),
            self.elevation().deg()
        )
    }
}

impl approx::AbsDiffEq for Horizontal {
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        unit::UAS
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.0.abs_diff_eq(&other.0, epsilon)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn round_trip_degrees() {
        let h = Horizontal::from_degrees(180.0, 45.0).unwrap();
        assert!((h.azimuth().deg() - 180.0).abs() < 1e-12);
        assert!((h.elevation().deg() - 45.0).abs() < 1e-12);
    }

    #[test]
    fn zenith_angle_relates_to_elevation() {
        let zenith = Horizontal::from_degrees(0.0, 90.0).unwrap();
        assert!(zenith.zenith_angle().deg().abs() < 1e-12);

        let horizon = Horizontal::from_degrees(0.0, 0.0).unwrap();
        assert!((horizon.zenith_angle().deg() - 90.0).abs() < 1e-12);
    }

    #[test]
    fn approx_eq_across_azimuth_wrap() {
        let a = Horizontal::from_degrees(359.9999, 30.0).unwrap();
        let b = Horizontal::from_degrees(-0.0001, 30.0).unwrap();
        assert_abs_diff_eq!(a, b, epsilon = unit::ARCSEC);
    }
}
