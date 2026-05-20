//! Spherical coordinates (longitude, latitude) — direction on a sphere.

use core::fmt;

use supernovas_ffi::novas_sep;

use crate::{Angle, Coordinate, Position, error::Result, unit};

/// A direction on the unit sphere, specified by longitude and latitude.
///
/// This is the geometric base shape used by typed coordinate systems
/// ([`super::Galactic`], [`super::Horizontal`]). For most use you'll reach
/// for one of those instead, but `Spherical` is exposed for cases where the
/// reference system is implicit or doesn't matter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spherical {
    lon: Angle,
    lat: Angle,
}

impl Spherical {
    /// Construct from typed longitude and latitude. Infallible: both inputs
    /// are already validated as finite by [`Angle`].
    pub fn new(longitude: Angle, latitude: Angle) -> Self {
        Spherical {
            lon: longitude,
            lat: latitude,
        }
    }

    /// Construct from longitude and latitude in radians.
    pub fn from_radians(longitude: f64, latitude: f64) -> Result<Self> {
        Ok(Spherical::new(
            Angle::from_radians(longitude)?,
            Angle::from_radians(latitude)?,
        ))
    }

    /// Construct from longitude and latitude in degrees.
    pub fn from_degrees(longitude: f64, latitude: f64) -> Result<Self> {
        Ok(Spherical::new(
            Angle::from_degrees(longitude)?,
            Angle::from_degrees(latitude)?,
        ))
    }

    /// Longitude.
    pub fn longitude(self) -> Angle {
        self.lon
    }

    /// Latitude.
    pub fn latitude(self) -> Angle {
        self.lat
    }

    /// Great-circle angular separation between this direction and `other`.
    ///
    /// Delegates to the SuperNOVAS C-side `novas_sep()`, which uses the
    /// numerically-stable Vincenty formulation.
    pub fn distance_to(self, other: Spherical) -> Angle {
        let sep_deg = unsafe {
            novas_sep(
                self.lon.deg(),
                self.lat.deg(),
                other.lon.deg(),
                other.lat.deg(),
            )
        };
        Angle::from_degrees(sep_deg).expect("novas_sep on finite inputs returns finite")
    }

    /// Cartesian position at the given distance along this direction:
    /// `(d·cos(lat)·cos(lon), d·cos(lat)·sin(lon), d·sin(lat))`.
    pub fn xyz(self, distance: Coordinate) -> Position {
        let (lon, lat) = (self.lon.rad(), self.lat.rad());
        let (slat, clat) = lat.sin_cos();
        let (slon, clon) = lon.sin_cos();
        let d = distance.m();
        Position::from_meters_array([d * clat * clon, d * clat * slon, d * slat])
            .expect("finite inputs produce finite components")
    }
}

impl fmt::Display for Spherical {
    /// Renders as `(lon, lat)` with both as DMS via [`Angle`]'s display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (lon, lat) = (self.lon, self.lat);
        if let Some(p) = f.precision() {
            write!(f, "({lon:.p$}, {lat:.p$})")
        } else {
            write!(f, "({lon}, {lat})")
        }
    }
}

impl approx::AbsDiffEq for Spherical {
    type Epsilon = f64;

    /// Default tolerance: 1 micro-arcsecond.
    fn default_epsilon() -> Self::Epsilon {
        unit::UAS
    }

    /// Treats `self` and `other` as equal when their great-circle angular
    /// separation is within `epsilon` radians.
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.distance_to(*other).rad() <= epsilon.abs()
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn distance_to_self_is_zero() {
        let p = Spherical::from_degrees(30.0, 45.0).unwrap();
        assert!(p.distance_to(p).rad().abs() < 1e-12);
    }

    #[test]
    fn distance_north_pole_to_south_pole_is_pi() {
        let np = Spherical::from_degrees(0.0, 90.0).unwrap();
        let sp = Spherical::from_degrees(0.0, -90.0).unwrap();
        assert!((np.distance_to(sp).rad() - core::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn xyz_equator_x_axis() {
        // Lon=0, lat=0 at distance 1 m -> (1, 0, 0)
        let p = Spherical::from_degrees(0.0, 0.0).unwrap();
        let pos = p.xyz(Coordinate::from_meters(1.0).unwrap());
        assert!((pos.x().m() - 1.0).abs() < 1e-12);
        assert!(pos.y().m().abs() < 1e-12);
        assert!(pos.z().m().abs() < 1e-12);
    }

    #[test]
    fn xyz_north_pole_z_axis() {
        // Lat=90° at distance 5 m -> (0, 0, 5)
        let p = Spherical::from_degrees(0.0, 90.0).unwrap();
        let pos = p.xyz(Coordinate::from_meters(5.0).unwrap());
        assert!(pos.x().m().abs() < 1e-12);
        assert!(pos.y().m().abs() < 1e-12);
        assert!((pos.z().m() - 5.0).abs() < 1e-12);
    }

    #[test]
    fn abs_diff_eq_uses_great_circle() {
        let a = Spherical::from_degrees(179.9999, 0.0).unwrap();
        let b = Spherical::from_degrees(-179.9999, 0.0).unwrap();
        // 0.0002° apart across the wrap; well within 1 arc-second
        assert_abs_diff_eq!(a, b, epsilon = unit::ARCSEC);
    }
}
