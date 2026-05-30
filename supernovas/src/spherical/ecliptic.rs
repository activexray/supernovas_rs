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
    #[must_use]
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
    #[must_use]
    pub fn longitude(self) -> Angle {
        self.sph.longitude()
    }

    /// Ecliptic latitude `β` (in `[-π/2, π/2]`).
    #[must_use]
    pub fn latitude(self) -> Angle {
        self.sph.latitude()
    }

    /// The equinox these coordinates are measured in.
    #[must_use]
    pub fn system(self) -> Equinox {
        self.system
    }

    /// The bare [`Spherical`] view.
    #[must_use]
    pub fn as_spherical(self) -> Spherical {
        self.sph
    }

    /// Great-circle angular separation between this direction and `other`.
    #[must_use]
    pub fn distance_to(self, other: Ecliptic) -> Angle {
        self.sph.distance_to(other.sph)
    }

    /// Convert to equatorial coordinates at the same equinox.
    ///
    /// Uses `ecl2equ`, dispatching the equator type from the equinox.
    /// Returns [`Error::UnsupportedSystem`] for equinoxes with no ecliptic
    /// mapping (CIRS, ITRS), or [`Error::Ffi`] if the underlying C call fails.
    pub fn to_equatorial(self, accuracy: Accuracy) -> Result<Equatorial> {
        let coord_sys = self
            .system
            .equator_type_for_ecliptic()
            .ok_or(Error::UnsupportedSystem)?;
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
                &raw mut ra_h,
                &raw mut dec_d,
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
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
    /// Renders as `λ=<lon>° β=<lat> (<system>)`.
    ///
    /// Longitude is normalized to `[0°, 360°)` and shown in decimal degrees
    /// (ecliptic longitude is conventionally unsigned). Latitude is shown in
    /// DMS via [`Angle`]'s display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lon_deg = self.longitude().deg().rem_euclid(360.0);
        let lat = self.latitude();
        let sys = self.system;
        if let Some(p) = f.precision() {
            write!(f, "λ={lon_deg:.p$}° β={lat:.p$} ({sys})")
        } else {
            write!(f, "λ={lon_deg:.3}° β={lat} ({sys})")
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

    #[test]
    fn from_radians_round_trip() {
        use core::f64::consts::PI;
        let e = Ecliptic::from_radians(PI / 3.0, PI / 8.0, Equinox::J2000).unwrap();
        assert!((e.longitude().rad() - PI / 3.0).abs() < 1e-12);
        assert!((e.latitude().rad() - PI / 8.0).abs() < 1e-12);
        assert_eq!(e.system(), Equinox::J2000);
    }

    #[test]
    fn as_spherical_gives_same_coords() {
        let e = Ecliptic::from_degrees(45.0, -10.0, Equinox::ICRS).unwrap();
        let s = e.as_spherical();
        assert!((s.longitude().deg() - 45.0).abs() < 1e-9);
        assert!((s.latitude().deg() - -10.0).abs() < 1e-9);
    }

    #[test]
    fn to_system_same_equinox_is_identity() {
        use crate::Accuracy;
        let e = Ecliptic::from_degrees(90.0, 5.0, Equinox::J2000).unwrap();
        let same = e.to_system(Equinox::J2000, Accuracy::Reduced).unwrap();
        assert_abs_diff_eq!(e, same, epsilon = unit::UAS);
    }

    #[test]
    fn distance_to_orthogonal_is_90_deg() {
        use core::f64::consts::FRAC_PI_2;
        let a = Ecliptic::from_degrees(0.0, 0.0, Equinox::J2000).unwrap();
        let b = Ecliptic::from_degrees(90.0, 0.0, Equinox::J2000).unwrap();
        assert!((a.distance_to(b).rad() - FRAC_PI_2).abs() < 1e-12);
    }

    #[test]
    fn to_system_different_equinox_gives_nearby_result() {
        use crate::Accuracy;
        let e = Ecliptic::from_degrees(120.0, 5.0, Equinox::J2000).unwrap();
        // Precession from J2000 to a 2026 MOD equinox: non-identity path.
        let mod_eq = Equinox::mod_at(2_461_236.75).unwrap();
        let converted = e.to_system(mod_eq, Accuracy::Reduced).unwrap();
        // Should be close (within a few arcmin) but not identical.
        let sep_arcsec = e.distance_to(converted).arcsec();
        assert!(
            sep_arcsec < 3600.0,
            "separation {sep_arcsec} arcsec looks too large"
        );
        assert!(sep_arcsec > 0.0, "should have non-zero drift over 26 years");
    }

    #[test]
    fn display_with_precision() {
        let e = Ecliptic::from_degrees(90.0, 5.0, Equinox::J2000).unwrap();
        let s = format!("{e:.2}");
        assert!(s.contains("λ=90.00°"), "got: {s}");
        assert!(s.contains('β'), "got: {s}");
    }

    #[test]
    fn display_contains_lambda_beta_system() {
        let e = Ecliptic::from_degrees(90.0, 5.0, Equinox::J2000).unwrap();
        let s = format!("{e}");
        assert!(s.contains('λ'), "got: {s}");
        assert!(s.contains('β'), "got: {s}");
    }

    #[test]
    fn display_normalizes_longitude_to_0_360() {
        // λ = -74° is 286°; the display must show the positive form.
        let e = Ecliptic::from_degrees(-74.0, 62.0, Equinox::J2000).unwrap();
        let s = format!("{e}");
        assert!(s.contains("286."), "expected 286.xxx°, got: {s}");
    }
}
