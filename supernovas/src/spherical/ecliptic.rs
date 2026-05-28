//! Ecliptic coordinates: longitude `λ` and latitude `β`, measured against
//! the ecliptic plane and the equinox of a chosen [`Equinox`].

use core::fmt;

use supernovas_ffi::ecl2equ;

use super::{Equatorial, Galactic, Spherical};
use crate::{
    Accuracy, Angle, Equinox,
    error::{Error, Result},
    unit,
};

/// A direction on the sky in ecliptic coordinates (`λ`, `β`), tagged with
/// the [`Equinox`] those coordinates are measured in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ecliptic {
    sph: Spherical,
    system: Equinox,
}

impl Ecliptic {
    /// Construct from typed longitude and latitude.
    pub fn new(longitude: Angle, latitude: Angle, system: Equinox) -> Self {
        Ecliptic {
            sph: Spherical::new(longitude, latitude),
            system,
        }
    }

    /// Construct from longitude and latitude in radians.
    pub fn from_radians(longitude: f64, latitude: f64, system: Equinox) -> Result<Self> {
        Ok(Ecliptic::new(
            Angle::from_radians(longitude)?,
            Angle::from_radians(latitude)?,
            system,
        ))
    }

    /// Construct from longitude and latitude in degrees.
    pub fn from_degrees(longitude_deg: f64, latitude_deg: f64, system: Equinox) -> Result<Self> {
        Ok(Ecliptic::new(
            Angle::from_degrees(longitude_deg)?,
            Angle::from_degrees(latitude_deg)?,
            system,
        ))
    }

    /// Ecliptic longitude `λ` (in `(-π, π]`).
    pub fn longitude(self) -> Angle {
        self.sph.longitude()
    }

    /// Ecliptic latitude `β` (in `[-π/2, π/2]`).
    pub fn latitude(self) -> Angle {
        self.sph.latitude()
    }

    /// The equinox these coordinates are measured in.
    pub fn system(self) -> Equinox {
        self.system
    }

    /// The bare [`Spherical`] view.
    pub fn as_spherical(self) -> Spherical {
        self.sph
    }

    /// Great-circle angular separation between this direction and `other`.
    pub fn distance_to(self, other: Ecliptic) -> Angle {
        self.sph.distance_to(other.sph)
    }

    /// Convert to equatorial coordinates at the same equinox.
    ///
    /// Uses `ecl2equ`, dispatching the equator type from the equinox.
    /// Returns [`Error::Parse`] for equinoxes whose system has no
    /// equator-type mapping (CIRS, ITRS).
    pub fn to_equatorial(self, accuracy: Accuracy) -> Result<Equatorial> {
        let coord_sys = self
            .system
            .equator_type_for_ecliptic()
            .ok_or(Error::Parse)?;
        let mut ra_h = 0.0_f64;
        let mut dec_d = 0.0_f64;
        // SAFETY: ecl2equ writes the two output doubles on a 0 return.
        let rc = unsafe {
            ecl2equ(
                self.system.jd(),
                coord_sys,
                accuracy.to_sys(),
                self.longitude().deg(),
                self.latitude().deg(),
                &mut ra_h,
                &mut dec_d,
            )
        };
        if rc != 0 {
            return Err(Error::Parse);
        }
        Equatorial::from_hours_and_degrees(ra_h, dec_d, self.system)
    }

    /// Convert to ecliptic coordinates in a different equinox.
    ///
    /// Routes via equatorial: `ecl(src) → equ(src) → equ(target) → ecl(target)`.
    pub fn to_system(self, target: Equinox, accuracy: Accuracy) -> Result<Ecliptic> {
        if approx::AbsDiffEq::abs_diff_eq(
            &self.system,
            &target,
            <Equinox as approx::AbsDiffEq>::default_epsilon(),
        ) {
            return Ok(Ecliptic {
                sph: self.sph,
                system: target,
            });
        }
        self.to_equatorial(accuracy)?
            .to_system(target, accuracy)?
            .to_ecliptic(accuracy)
    }

    /// Convert to galactic coordinates.
    ///
    /// Routes via ICRS equatorial.
    pub fn to_galactic(self, accuracy: Accuracy) -> Result<Galactic> {
        self.to_equatorial(accuracy)?.to_galactic(accuracy)
    }
}

impl fmt::Display for Ecliptic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (lon, lat, sys) = (self.longitude(), self.latitude(), self.system);
        if let Some(p) = f.precision() {
            write!(f, "λ={lon:.p$} β={lat:.p$} ({sys})")
        } else {
            write!(f, "λ={lon} β={lat} ({sys})")
        }
    }
}

impl approx::AbsDiffEq for Ecliptic {
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        unit::UAS
    }

    /// Treats two [`Ecliptic`] as equal when their great-circle separation
    /// is within `epsilon` radians. Equinox tags are **not** compared.
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.sph.abs_diff_eq(&other.sph, epsilon)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn round_trip_degrees() {
        let e = Ecliptic::from_degrees(120.0, 5.0, Equinox::J2000).unwrap();
        assert!((e.longitude().deg() - 120.0).abs() < 1e-9);
        assert!((e.latitude().deg() - 5.0).abs() < 1e-9);
        assert_eq!(e.system(), Equinox::J2000);
    }

    #[test]
    fn approx_eq_ignores_equinox_tag() {
        let a = Ecliptic::from_degrees(120.0, 5.0, Equinox::ICRS).unwrap();
        let b = Ecliptic::from_degrees(120.0, 5.0, Equinox::J2000).unwrap();
        assert_abs_diff_eq!(a, b, epsilon = unit::UAS);
    }

    #[test]
    fn ecliptic_to_equatorial_round_trip() {
        use crate::Accuracy;
        let e = Ecliptic::from_degrees(120.0, 5.0, Equinox::J2000).unwrap();
        let eq = e.to_equatorial(Accuracy::Reduced).unwrap();
        let back = eq.to_ecliptic(Accuracy::Reduced).unwrap();
        assert_abs_diff_eq!(e, back, epsilon = unit::UAS);
    }

    #[test]
    fn ecliptic_to_galactic_via_equatorial() {
        use crate::Accuracy;
        // Ecliptic position near the galactic centre direction (l=0).
        // The galactic centre is at roughly RA 17h45m, Dec -29° (ICRS).
        // Picking a known ecliptic position and verifying the resulting
        // galactic l/b are finite and in their valid ranges.
        let e = Ecliptic::from_degrees(0.0, 0.0, Equinox::J2000).unwrap();
        let g = e.to_galactic(Accuracy::Reduced).unwrap();
        let l = g.l().deg();
        let b = g.b().deg();
        assert!(l.is_finite() && b.is_finite());
        assert!((-90.0..=90.0).contains(&b));
    }
}
