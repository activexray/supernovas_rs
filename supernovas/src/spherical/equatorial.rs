//! Equatorial coordinates: right ascension and declination in a chosen
//! [`Equinox`].

use core::fmt;

use supernovas_ffi::{
    equ2ecl, equ2gal, novas_icrs_to_sys, novas_sys_to_icrs, radec2vector, vector2radec,
};

use super::{Ecliptic, Galactic, Spherical};
use crate::{
    Accuracy, Angle, Equinox, ReferenceSystem, TimeAngle,
    error::{Error, Result},
    unit,
};

/// A direction on the sky in equatorial coordinates (`α`, `δ`), tagged
/// with the [`Equinox`] those coordinates are measured in.
///
/// Right ascension is exposed as a [`TimeAngle`] (the astronomical
/// convention of hours/minutes/seconds); declination is a plain [`Angle`].
/// Internally the pair is stored as a [`Spherical`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Equatorial {
    sph: Spherical,
    system: Equinox,
}

impl Equatorial {
    /// Construct from typed RA and Dec in the given equinox.
    pub fn new(ra: TimeAngle, dec: Angle, system: Equinox) -> Self {
        // TimeAngle lives in [0, 2π); Angle in (-π, π]. Convert TimeAngle
        // to Angle (lossless) for the underlying Spherical.
        Equatorial {
            sph: Spherical::new(Angle::from(ra), dec),
            system,
        }
    }

    /// Construct from RA in hours and Dec in degrees.
    pub fn from_hours_and_degrees(ra_hours: f64, dec_deg: f64, system: Equinox) -> Result<Self> {
        Ok(Equatorial::new(
            TimeAngle::from_hours(ra_hours)?,
            Angle::from_degrees(dec_deg)?,
            system,
        ))
    }

    /// Construct from RA and Dec in degrees.
    pub fn from_degrees(ra_deg: f64, dec_deg: f64, system: Equinox) -> Result<Self> {
        Ok(Equatorial::new(
            TimeAngle::from_radians(ra_deg * unit::DEG)?,
            Angle::from_degrees(dec_deg)?,
            system,
        ))
    }

    /// Construct from RA and Dec in radians.
    pub fn from_radians(ra_rad: f64, dec_rad: f64, system: Equinox) -> Result<Self> {
        Ok(Equatorial::new(
            TimeAngle::from_radians(ra_rad)?,
            Angle::from_radians(dec_rad)?,
            system,
        ))
    }

    /// Right ascension, in `[0, 24h)`.
    pub fn ra(self) -> TimeAngle {
        TimeAngle::from_angle(self.sph.longitude())
    }

    /// Declination, in `[-90°, 90°]`.
    pub fn dec(self) -> Angle {
        self.sph.latitude()
    }

    /// The equinox these coordinates are measured in.
    pub fn system(self) -> Equinox {
        self.system
    }

    /// The bare [`Spherical`] view (longitude = RA folded into `(-π, π]`,
    /// latitude = Dec).
    pub fn as_spherical(self) -> Spherical {
        self.sph
    }

    /// Great-circle angular separation between this direction and `other`.
    ///
    /// **Note:** the separation is computed in each side's own equinox.
    /// If `self` and `other` are in different equinoxes, the result is
    /// only meaningful if you've already established that the two systems
    /// align to within your precision target.
    pub fn distance_to(self, other: Equatorial) -> Angle {
        self.sph.distance_to(other.sph)
    }

    /// Convert to another equinox.
    ///
    /// Routes through ICRS internally via SuperNOVAS's
    /// `novas_sys_to_icrs` + `novas_icrs_to_sys`. Both calls accept a JD,
    /// so date-dependent input systems (MOD/TOD/CIRS) carry their epoch
    /// through correctly.
    ///
    /// Returns `self` unchanged when the source and target equinoxes
    /// already agree (within ~1 s of TT — see [`Equinox`]'s `AbsDiffEq`).
    pub fn to_system(self, target: Equinox, accuracy: Accuracy) -> Result<Equatorial> {
        if approx::AbsDiffEq::abs_diff_eq(
            &self.system,
            &target,
            <Equinox as approx::AbsDiffEq>::default_epsilon(),
        ) {
            return Ok(Equatorial {
                sph: self.sph,
                system: target,
            });
        }
        // Unit vector in the source system.
        let mut v = [0.0_f64; 3];
        // SAFETY: radec2vector writes 3 doubles into v on a 0 return; the
        // input is finite by Equatorial's construction invariants.
        let rc = unsafe { radec2vector(self.ra().hours(), self.dec().deg(), 1.0, v.as_mut_ptr()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }

        // src system → ICRS unit vector.
        let mut to_icrs = [0.0_f64; 3];
        // SAFETY: novas_sys_to_icrs writes 3 doubles into out on a 0
        // return; in/out aliasing isn't relied on here.
        let rc = unsafe {
            novas_sys_to_icrs(
                self.system.system().to_sys(),
                v.as_ptr(),
                self.system.jd(),
                accuracy.to_sys(),
                to_icrs.as_mut_ptr(),
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }

        // ICRS → target system unit vector.
        let mut out = [0.0_f64; 3];
        // SAFETY: same contract as novas_sys_to_icrs above.
        let rc = unsafe {
            novas_icrs_to_sys(
                to_icrs.as_ptr(),
                target.jd(),
                accuracy.to_sys(),
                target.system().to_sys(),
                out.as_mut_ptr(),
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }

        // Unit vector → RA/Dec in the target system.
        let mut ra_h = 0.0_f64;
        let mut dec_d = 0.0_f64;
        // SAFETY: vector2radec reads 3 doubles from `out` and writes the
        // two output scalars on a 0 return.
        let rc = unsafe { vector2radec(out.as_ptr(), &mut ra_h, &mut dec_d) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Self::from_hours_and_degrees(ra_h, dec_d, target)
    }

    /// Convert to ICRS.
    pub fn to_icrs(self, accuracy: Accuracy) -> Result<Equatorial> {
        self.to_system(Equinox::ICRS, accuracy)
    }

    /// Convert to J2000 (mean equator + equinox).
    pub fn to_j2000(self, accuracy: Accuracy) -> Result<Equatorial> {
        self.to_system(Equinox::J2000, accuracy)
    }

    /// Convert to mean equator and equinox of date.
    pub fn to_mod(self, jd_tt: f64, accuracy: Accuracy) -> Result<Equatorial> {
        self.to_system(Equinox::mod_at(jd_tt)?, accuracy)
    }

    /// Convert to true equator and equinox of date.
    pub fn to_tod(self, jd_tt: f64, accuracy: Accuracy) -> Result<Equatorial> {
        self.to_system(Equinox::tod_at(jd_tt)?, accuracy)
    }

    /// Convert to CIRS (Celestial Intermediate Reference System).
    pub fn to_cirs(self, jd_tt: f64, accuracy: Accuracy) -> Result<Equatorial> {
        self.to_system(Equinox::cirs_at(jd_tt)?, accuracy)
    }

    /// Convert to ecliptic coordinates at the same equinox.
    ///
    /// Uses `equ2ecl`, dispatching the equator type from the equinox.
    /// CIRS sources are auto-routed through TOD (CIRS has no direct
    /// equator-type mapping); ITRS sources return [`Error::UnsupportedSystem`]
    /// since they aren't a valid equatorial-to-ecliptic input. Returns
    /// [`Error::Ffi`] if the underlying C call fails.
    pub fn to_ecliptic(self, accuracy: Accuracy) -> Result<Ecliptic> {
        // CIRS → TOD as a preprocessing step.
        let eq = match self.system.system() {
            ReferenceSystem::Cirs => self.to_tod(self.system.jd(), accuracy)?,
            ReferenceSystem::Itrs => return Err(Error::UnsupportedSystem),
            _ => self,
        };
        let coord_sys = eq
            .system
            .equator_type_for_ecliptic()
            .ok_or(Error::UnsupportedSystem)?;
        let mut elon = 0.0_f64;
        let mut elat = 0.0_f64;
        // SAFETY: equ2ecl writes the two output doubles on a 0 return.
        let rc = unsafe {
            equ2ecl(
                eq.system.jd(),
                coord_sys,
                accuracy.to_sys(),
                eq.ra().hours(),
                eq.dec().deg(),
                &mut elon,
                &mut elat,
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ecliptic::from_degrees(elon, elat, eq.system)
    }

    /// Convert to galactic coordinates.
    ///
    /// `equ2gal` assumes ICRS input; sources in other equinoxes are first
    /// transformed to ICRS via [`Self::to_icrs`].
    pub fn to_galactic(self, accuracy: Accuracy) -> Result<Galactic> {
        let icrs = if self.system.is_icrs() {
            self
        } else {
            self.to_icrs(accuracy)?
        };
        let mut glon = 0.0_f64;
        let mut glat = 0.0_f64;
        // SAFETY: equ2gal writes the two output doubles on a 0 return.
        let rc = unsafe { equ2gal(icrs.ra().hours(), icrs.dec().deg(), &mut glon, &mut glat) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Galactic::from_degrees(glon, glat)
    }
}

// Keep the ReferenceSystem -> novas_reference_system mapping accessible
// inside this module without re-implementing it here. The `ReferenceSystem`
// enum lives in `crate::apparent`; expose its `to_sys` via the same path.
//
// This isn't `pub(crate)` accessible from the apparent module today; in
// follow-ups we may want to lift it to a common `Reference` module.

impl fmt::Display for Equatorial {
    /// Renders as `RA <hms> Dec <dms> (system)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (ra, dec, sys) = (self.ra(), self.dec(), self.system);
        if let Some(p) = f.precision() {
            write!(f, "RA {ra:.p$} Dec {dec:.p$} ({sys})")
        } else {
            write!(f, "RA {ra} Dec {dec} ({sys})")
        }
    }
}

impl approx::AbsDiffEq for Equatorial {
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        unit::UAS
    }

    /// Treats two [`Equatorial`] as equal when their great-circle
    /// separation is within `epsilon` radians. Equinox tags are **not**
    /// compared — combine with an explicit equinox check if that matters.
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.sph.abs_diff_eq(&other.sph, epsilon)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn round_trip_hours_and_degrees() {
        let e = Equatorial::from_hours_and_degrees(18.6156, 38.7837, Equinox::J2000).unwrap();
        assert!((e.ra().hours() - 18.6156).abs() < 1e-9);
        assert!((e.dec().deg() - 38.7837).abs() < 1e-9);
        assert_eq!(e.system(), Equinox::J2000);
    }

    #[test]
    fn ra_lifts_negative_angles_into_24h() {
        // Construct via radians with a longitude that Spherical would
        // store as negative — RA should still come out in [0, 24).
        let e = Equatorial::from_degrees(350.0, 0.0, Equinox::ICRS).unwrap();
        let h = e.ra().hours();
        assert!((0.0..24.0).contains(&h), "RA out of range: {h}");
        // 350° → 23h20m
        assert!((h - 350.0 / 15.0).abs() < 1e-9);
    }

    #[test]
    fn distance_to_self_is_zero() {
        let e = Equatorial::from_hours_and_degrees(12.0, 30.0, Equinox::J2000).unwrap();
        assert!(e.distance_to(e).rad().abs() < 1e-12);
    }

    #[test]
    fn approx_eq_ignores_equinox_tag() {
        let a = Equatorial::from_hours_and_degrees(12.0, 30.0, Equinox::ICRS).unwrap();
        let b = Equatorial::from_hours_and_degrees(12.0, 30.0, Equinox::J2000).unwrap();
        // Same numerical RA/Dec, different equinox tag → still
        // approximately equal by our definition (great-circle separation).
        assert_abs_diff_eq!(a, b, epsilon = unit::UAS);
    }

    /// Vega's J2000 catalog position, used as a known reference for the
    /// conversion round-trip tests below.
    fn vega_j2000() -> Equatorial {
        Equatorial::from_hours_and_degrees(
            18.0 + 36.0 / 60.0 + 56.336 / 3600.0,
            38.0 + 47.0 / 60.0 + 1.28 / 3600.0,
            Equinox::J2000,
        )
        .unwrap()
    }

    #[test]
    fn to_same_system_is_essentially_a_no_op() {
        let vega = vega_j2000();
        let same = vega.to_system(Equinox::J2000, Accuracy::Reduced).unwrap();
        // Equatorial::to_system short-circuits when source ≈ target, so
        // the round-trip should be bit-identical here.
        assert_eq!(vega.ra().rad(), same.ra().rad());
        assert_eq!(vega.dec().rad(), same.dec().rad());
    }

    #[test]
    fn icrs_to_j2000_round_trip() {
        let vega = vega_j2000();
        let icrs = vega.to_icrs(Accuracy::Reduced).unwrap();
        let back = icrs.to_j2000(Accuracy::Reduced).unwrap();
        // ICRS and J2000 differ by the frame-tie rotation (a few mas).
        // A round-trip through both directions should close to within
        // numerical precision.
        assert_abs_diff_eq!(vega, back, epsilon = unit::MAS);
    }

    #[test]
    fn j2000_to_mod_at_recent_date_drifts() {
        let vega = vega_j2000();
        // Precession by ~26 years (from J2000 to 2026) should move Vega
        // by several arcminutes.
        let mod_2026 = vega.to_mod(2_461_236.75, Accuracy::Reduced).unwrap();
        let sep_arcsec = vega.distance_to(mod_2026).arcsec();
        assert!(
            (60.0..36_000.0).contains(&sep_arcsec),
            "expected ~arcmin-scale precession drift, got {sep_arcsec} arcsec"
        );
    }

    #[test]
    fn galactic_round_trip_is_identity() {
        let vega = vega_j2000();
        let vega_icrs = vega.to_icrs(Accuracy::Reduced).unwrap();
        let g = vega_icrs.to_galactic(Accuracy::Reduced).unwrap();
        let back = g.to_equatorial_icrs().unwrap();
        assert_abs_diff_eq!(vega_icrs, back, epsilon = unit::UAS);
    }

    /// Vega's ecliptic coordinates at J2000: λ ≈ 285.4°, β ≈ +61.7°.
    #[test]
    fn vega_ecliptic_round_trip() {
        let vega = vega_j2000();
        let ecl = vega.to_ecliptic(Accuracy::Reduced).unwrap();
        let back = ecl.to_equatorial(Accuracy::Reduced).unwrap();
        assert_abs_diff_eq!(vega, back, epsilon = unit::UAS);
        // Sanity check the magnitudes. Ecliptic stores longitude in
        // (-180°, 180°]; 285.4° → -74.6° in that range.
        let lon_in_0_360 = (ecl.longitude().deg() + 360.0).rem_euclid(360.0);
        assert!(
            (lon_in_0_360 - 285.4).abs() < 0.5,
            "Vega ecliptic λ = {lon_in_0_360}° should be near 285.4°"
        );
        assert!(
            (ecl.latitude().deg() - 61.7).abs() < 0.5,
            "Vega ecliptic β = {} should be near 61.7°",
            ecl.latitude().deg()
        );
    }

    #[test]
    fn cirs_to_ecliptic_routes_through_tod() {
        let vega = vega_j2000();
        let cirs = vega.to_cirs(2_461_236.75, Accuracy::Reduced).unwrap();
        // Going from CIRS directly to ecliptic should work (the
        // implementation auto-routes through TOD internally).
        let ecl = cirs.to_ecliptic(Accuracy::Reduced).unwrap();
        // Round-tripping back to equatorial should preserve position.
        let back = ecl.to_equatorial(Accuracy::Reduced).unwrap();
        let sep = back.distance_to(cirs.to_tod(2_461_236.75, Accuracy::Reduced).unwrap());
        assert!(sep.arcsec() < 1.0);
    }

    /// Vega's galactic coordinates: roughly (l, b) ≈ (67.4°, +19.2°).
    /// Cross-check against the known reference values.
    #[test]
    fn vega_galactic_matches_known_values() {
        let vega_icrs = vega_j2000().to_icrs(Accuracy::Reduced).unwrap();
        let g = vega_icrs.to_galactic(Accuracy::Reduced).unwrap();
        assert!(
            (g.l().deg() - 67.45).abs() < 0.1,
            "Vega galactic l = {} should be near 67.45°",
            g.l().deg()
        );
        assert!(
            (g.b().deg() - 19.24).abs() < 0.1,
            "Vega galactic b = {} should be near 19.24°",
            g.b().deg()
        );
    }
}
