//! Galactic coordinates: galactic longitude `l` and galactic latitude `b`.

use core::fmt;

use supernovas_ffi::gal2equ;

use super::{Equatorial, Spherical};
use crate::{
    Angle, Coordinate, Equinox, Position,
    error::{Error, Result},
    unit,
};

/// A direction on the sky in the Galactic coordinate system (`l`, `b`).
///
/// `l` is measured along the galactic plane from the galactic center,
/// increasing toward the direction of galactic rotation. `b` is measured
/// perpendicular to the galactic plane, positive toward the north galactic
/// pole.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Galactic(Spherical);

impl Galactic {
    /// Construct from typed `l` and `b`.
    #[must_use]
    pub fn new(l: Angle, b: Angle) -> Self {
        Galactic(Spherical::new(l, b))
    }

    /// Construct from `l` and `b` in radians.
    pub fn from_radians(l: f64, b: f64) -> Result<Self> {
        Ok(Galactic(Spherical::from_radians(l, b)?))
    }

    /// Construct from `l` and `b` in degrees.
    pub fn from_degrees(l: f64, b: f64) -> Result<Self> {
        Ok(Galactic(Spherical::from_degrees(l, b)?))
    }

    /// Galactic longitude.
    #[must_use]
    pub fn l(self) -> Angle {
        self.0.longitude()
    }

    /// Galactic latitude.
    #[must_use]
    pub fn b(self) -> Angle {
        self.0.latitude()
    }

    /// The bare [`Spherical`] view for cases that don't care about the
    /// reference system.
    #[must_use]
    pub fn as_spherical(self) -> Spherical {
        self.0
    }

    /// Great-circle angular separation between this direction and `other`.
    #[must_use]
    pub fn distance_to(self, other: Galactic) -> Angle {
        self.0.distance_to(other.0)
    }

    /// Cartesian position at the given distance along this direction, in
    /// galactic-aligned axes.
    #[must_use]
    pub fn xyz(self, distance: Coordinate) -> Position {
        self.0.xyz(distance)
    }

    /// Convert to equatorial coordinates in ICRS.
    ///
    /// The galactic ↔ ICRS rotation is fixed (defined by the galactic pole
    /// and centre directions in ICRS), so no date / accuracy parameter is
    /// needed.
    pub fn to_equatorial_icrs(self) -> Result<Equatorial> {
        let mut ra_h = 0.0_f64;
        let mut dec_d = 0.0_f64;
        // SAFETY: gal2equ writes the two output doubles on a 0 return.
        let rc = unsafe {
            gal2equ(
                self.l().deg(),
                self.b().deg(),
                &raw mut ra_h,
                &raw mut dec_d,
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Equatorial::from_hours_and_degrees(ra_h, dec_d, Equinox::ICRS)
    }
}

impl fmt::Display for Galactic {
    /// Renders as `l=<l> b=<b>` with both as DMS.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (l, b) = (self.l(), self.b());
        if let Some(p) = f.precision() {
            write!(f, "l={l:.p$} b={b:.p$}")
        } else {
            write!(f, "l={l} b={b}")
        }
    }
}

impl approx::AbsDiffEq for Galactic {
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
        let g = Galactic::from_degrees(45.0, -30.0).unwrap();
        assert!((g.l().deg() - 45.0).abs() < 1e-12);
        assert!((g.b().deg() - -30.0).abs() < 1e-12);
    }

    #[test]
    fn distance_between_pole_and_anti_pole() {
        let np = Galactic::from_degrees(0.0, 90.0).unwrap();
        let sp = Galactic::from_degrees(0.0, -90.0).unwrap();
        assert!((np.distance_to(sp).rad() - core::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn approx_eq() {
        let a = Galactic::from_degrees(180.0, 0.0).unwrap();
        let b = Galactic::from_degrees(-180.0, 0.0).unwrap();
        assert_abs_diff_eq!(a, b, epsilon = unit::ARCSEC);
    }

    #[test]
    fn from_radians_round_trip() {
        use core::f64::consts::PI;
        let g = Galactic::from_radians(PI / 2.0, PI / 6.0).unwrap();
        assert!((g.l().rad() - PI / 2.0).abs() < 1e-12);
        assert!((g.b().rad() - PI / 6.0).abs() < 1e-12);
    }

    #[test]
    fn as_spherical_gives_same_coords() {
        let g = Galactic::from_degrees(30.0, 15.0).unwrap();
        let s = g.as_spherical();
        assert!((s.longitude().deg() - 30.0).abs() < 1e-12);
        assert!((s.latitude().deg() - 15.0).abs() < 1e-12);
    }

    #[test]
    fn xyz_has_finite_components() {
        let g = Galactic::from_degrees(0.0, 30.0).unwrap();
        let d = crate::Coordinate::from_kpc(1.0).unwrap();
        let p = g.xyz(d);
        assert!(p.x().m().is_finite());
        assert!(p.y().m().is_finite());
        assert!(p.z().m().is_finite());
    }

    #[test]
    fn display_contains_l_and_b() {
        let g = Galactic::from_degrees(120.0, -10.0).unwrap();
        let s = format!("{g}");
        assert!(s.contains("l="), "got: {s}");
        assert!(s.contains("b="), "got: {s}");
    }
}
