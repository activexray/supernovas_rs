//! Equinox: a (name, reference-system, date) triple identifying a
//! particular equatorial coordinate system.
//!
//! Equinoxes pin down the orientation of the equator and the position of
//! the equinox or CIO that anchors longitude/RA. Common choices like
//! [`Equinox::ICRS`], [`Equinox::J2000`], [`Equinox::B1950`] are available
//! as constants; date-parameterized systems like Mean-of-Date (MOD),
//! True-of-Date (TOD), and CIRS are built via [`Equinox::mod_at`] /
//! [`Equinox::tod_at`] / [`Equinox::cirs_at`].

use core::fmt;

use supernovas_ffi::{
    NOVAS_JD_B1900, NOVAS_JD_B1950, NOVAS_JD_HIP, NOVAS_JD_J2000, NOVAS_JD_MJD0,
    NOVAS_JULIAN_YEAR_DAYS, novas_equator_type,
    novas_equator_type::{NOVAS_GCRS_EQUATOR, NOVAS_MEAN_EQUATOR, NOVAS_TRUE_EQUATOR},
};

use crate::{
    ReferenceSystem,
    error::{Error, Result},
};

/// A specific equatorial coordinate system.
///
/// Carries the system's name, its [`ReferenceSystem`] tag, and the (TT-based)
/// Julian date that pins down a date-dependent system (mean equator and
/// equinox of date, etc.). For date-independent systems like ICRS the JD is
/// just a placeholder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Equinox {
    name: &'static str,
    system: ReferenceSystem,
    jd_tt: f64,
}

impl Equinox {
    /// ICRS - the fixed extragalactic frame. JD is unused but kept at
    /// J2000 by convention.
    pub const ICRS: Equinox = Equinox {
        name: "ICRS",
        system: ReferenceSystem::Icrs,
        jd_tt: NOVAS_JD_J2000,
    };

    /// Mean equator and equinox at J2000.0 (12 TT, 1 January 2000). Also
    /// known as FK5.
    pub const J2000: Equinox = Equinox {
        name: "J2000",
        system: ReferenceSystem::J2000,
        jd_tt: NOVAS_JD_J2000,
    };

    /// Hipparcos catalog reference - mean equator and equinox of date at
    /// J1991.25.
    pub const HIPPARCOS: Equinox = Equinox {
        name: "HIP",
        system: ReferenceSystem::Mod,
        jd_tt: NOVAS_JD_HIP,
    };

    /// B1950 - mean equator and equinox at the 1950.0 Besselian epoch.
    /// Also known as FK4.
    pub const B1950: Equinox = Equinox {
        name: "B1950",
        system: ReferenceSystem::Mod,
        jd_tt: NOVAS_JD_B1950,
    };

    /// B1900 - mean equator and equinox at the 1900.0 Besselian epoch.
    pub const B1900: Equinox = Equinox {
        name: "B1900",
        system: ReferenceSystem::Mod,
        jd_tt: NOVAS_JD_B1900,
    };

    /// Mean equator and equinox at the given (TT-based) Julian date.
    pub fn mod_at(jd_tt: f64) -> Result<Self> {
        Self::at("MOD", ReferenceSystem::Mod, jd_tt)
    }

    /// True equator and equinox at the given (TT-based) Julian date.
    pub fn tod_at(jd_tt: f64) -> Result<Self> {
        Self::at("TOD", ReferenceSystem::Tod, jd_tt)
    }

    /// CIRS at the given (TT-based) Julian date.
    pub fn cirs_at(jd_tt: f64) -> Result<Self> {
        Self::at("CIRS", ReferenceSystem::Cirs, jd_tt)
    }

    /// General constructor with explicit name, system, and (TT-based) JD.
    /// `name` must be a `'static` string - typically a literal.
    pub fn at(name: &'static str, system: ReferenceSystem, jd_tt: f64) -> Result<Self> {
        if !jd_tt.is_finite() {
            return Err(Error::NotFinite);
        }
        Ok(Equinox {
            name,
            system,
            jd_tt,
        })
    }

    /// The system's name (e.g. `"ICRS"`, `"J2000"`, `"B1950"`).
    #[must_use]
    pub fn name(self) -> &'static str {
        self.name
    }

    /// The reference-system tag.
    #[must_use]
    pub fn system(self) -> ReferenceSystem {
        self.system
    }

    /// The TT-based Julian date that pins down the system's date-dependent
    /// orientation. For date-independent systems (ICRS) this is a
    /// placeholder.
    #[must_use]
    pub fn jd(self) -> f64 {
        self.jd_tt
    }

    /// The TT-based Modified Julian Date (`jd - 2400000.5`).
    #[must_use]
    pub fn mjd(self) -> f64 {
        self.jd_tt - NOVAS_JD_MJD0
    }

    /// The Julian-year epoch (e.g. `2000.0` for J2000, `2026.5` for an
    /// equinox at mid-2026).
    #[must_use]
    pub fn epoch(self) -> f64 {
        2000.0 + (self.jd_tt - NOVAS_JD_J2000) / NOVAS_JULIAN_YEAR_DAYS
    }

    /// `true` for ICRS-like systems (ICRS, GCRS).
    #[must_use]
    pub fn is_icrs(self) -> bool {
        matches!(self.system, ReferenceSystem::Icrs | ReferenceSystem::Gcrs)
    }

    /// `true` for mean-of-date systems (MOD, J2000 - both use the
    /// dynamical *mean* equator).
    #[must_use]
    pub fn is_mod(self) -> bool {
        matches!(self.system, ReferenceSystem::Mod | ReferenceSystem::J2000)
    }

    /// `true` for true-of-date systems (TOD; TIRS uses true equator too).
    #[must_use]
    pub fn is_true(self) -> bool {
        matches!(self.system, ReferenceSystem::Tod | ReferenceSystem::Tirs)
    }

    /// The equator type the C-side `equ2ecl` / `ecl2equ` need for this
    /// equinox, or `None` if the system isn't a direct candidate. CIRS is
    /// routed through TOD by callers; TIRS and ITRS have no equinox-based
    /// mapping at all (their longitudes are offset from TOD by the Earth
    /// rotation angle, so an equator type alone can't describe them).
    pub(crate) fn equator_type_for_ecliptic(self) -> Option<novas_equator_type> {
        match self.system {
            ReferenceSystem::Icrs | ReferenceSystem::Gcrs => Some(NOVAS_GCRS_EQUATOR),
            ReferenceSystem::J2000 | ReferenceSystem::Mod => Some(NOVAS_MEAN_EQUATOR),
            ReferenceSystem::Tod => Some(NOVAS_TRUE_EQUATOR),
            ReferenceSystem::Cirs | ReferenceSystem::Tirs | ReferenceSystem::Itrs => None,
        }
    }
}

impl approx::AbsDiffEq for Equinox {
    type Epsilon = f64;

    /// Default tolerance: 1 second of TT (≈ 1.16 × 10⁻⁵ days), matching
    /// the C++ `Equinox::operator==` precision.
    fn default_epsilon() -> Self::Epsilon {
        1.0 / 86_400.0
    }

    /// Equinoxes compare equal when their reference systems match **and**
    /// their TT Julian dates are within `epsilon` days of each other.
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.system == other.system && (self.jd_tt - other.jd_tt).abs() <= epsilon
    }
}

impl fmt::Display for Equinox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

#[cfg(test)]
mod tests {
    use approx::{AbsDiffEq, assert_abs_diff_eq};

    use super::*;

    #[test]
    fn predefined_constants_have_right_systems() {
        assert_eq!(Equinox::ICRS.system(), ReferenceSystem::Icrs);
        assert_eq!(Equinox::J2000.system(), ReferenceSystem::J2000);
        assert_eq!(Equinox::HIPPARCOS.system(), ReferenceSystem::Mod);
        assert_eq!(Equinox::B1950.system(), ReferenceSystem::Mod);
    }

    #[test]
    fn j2000_epoch_is_2000() {
        assert!((Equinox::J2000.epoch() - 2000.0).abs() < 1e-9);
    }

    #[test]
    fn b1950_epoch_is_about_1950() {
        // Besselian 1950 ≈ Julian 1949.999...; tolerate a year.
        assert!((Equinox::B1950.epoch() - 1950.0).abs() < 1.0);
    }

    #[test]
    fn mjd_offset_is_jd_minus_2400000_5() {
        assert!((Equinox::J2000.mjd() - (NOVAS_JD_J2000 - NOVAS_JD_MJD0)).abs() < 1e-9);
    }

    #[test]
    fn rejects_non_finite_jd() {
        assert!(matches!(Equinox::mod_at(f64::NAN), Err(Error::NotFinite)));
    }

    #[test]
    fn date_factories_carry_the_jd_through() {
        let e = Equinox::mod_at(2_460_676.5).unwrap();
        assert_eq!(e.system(), ReferenceSystem::Mod);
        assert_eq!(e.jd(), 2_460_676.5);
    }

    #[test]
    fn predicates() {
        assert!(Equinox::ICRS.is_icrs());
        assert!(Equinox::J2000.is_mod()); // J2000 uses the mean equator
        assert!(!Equinox::J2000.is_true());

        let tod = Equinox::tod_at(NOVAS_JD_J2000).unwrap();
        assert!(tod.is_true());
        assert!(!tod.is_mod());
    }

    #[test]
    fn abs_diff_eq_matches_system_and_date() {
        let a = Equinox::mod_at(NOVAS_JD_J2000).unwrap();
        let b = Equinox::mod_at(NOVAS_JD_J2000 + 1e-7).unwrap();
        // Same system, within 1-second tolerance.
        assert_abs_diff_eq!(a, b);

        let c = Equinox::tod_at(NOVAS_JD_J2000).unwrap();
        // Same date but different system: not equal.
        assert!(!a.abs_diff_eq(&c, 1.0));
    }

    #[test]
    fn name_getter() {
        assert_eq!(Equinox::ICRS.name(), "ICRS");
        assert_eq!(Equinox::J2000.name(), "J2000");
        assert_eq!(Equinox::B1950.name(), "B1950");
        assert_eq!(Equinox::HIPPARCOS.name(), "HIP");
        assert_eq!(Equinox::B1900.name(), "B1900");
    }

    #[test]
    fn b1900_is_near_1900() {
        assert!((Equinox::B1900.epoch() - 1900.0).abs() < 1.0);
    }

    #[test]
    fn hipparcos_epoch_is_near_1991() {
        assert!((Equinox::HIPPARCOS.epoch() - 1991.25).abs() < 0.1);
    }

    #[test]
    fn display_renders_name() {
        assert_eq!(format!("{}", Equinox::J2000), "J2000");
        assert_eq!(format!("{}", Equinox::ICRS), "ICRS");
    }

    #[test]
    fn cirs_is_not_icrs_or_mod_or_true() {
        let cirs = Equinox::cirs_at(NOVAS_JD_J2000).unwrap();
        assert!(!cirs.is_icrs());
        assert!(!cirs.is_mod());
        assert!(!cirs.is_true());
    }

    #[test]
    fn equator_type_for_ecliptic_returns_none_for_unmappable_systems() {
        let cirs = Equinox::cirs_at(NOVAS_JD_J2000).unwrap();
        assert!(cirs.equator_type_for_ecliptic().is_none());
        // Earth-rotating systems: no equator type can describe them - their
        // RA origin is offset from TOD by the Earth rotation angle.
        let tirs = Equinox::at("TIRS", ReferenceSystem::Tirs, NOVAS_JD_J2000).unwrap();
        assert!(tirs.equator_type_for_ecliptic().is_none());
        let itrs = Equinox::at("ITRS", ReferenceSystem::Itrs, NOVAS_JD_J2000).unwrap();
        assert!(itrs.equator_type_for_ecliptic().is_none());
    }

    #[test]
    fn equator_type_for_ecliptic_returns_some_for_common_systems() {
        assert!(Equinox::ICRS.equator_type_for_ecliptic().is_some());
        assert!(Equinox::J2000.equator_type_for_ecliptic().is_some());
        assert!(
            Equinox::tod_at(NOVAS_JD_J2000)
                .unwrap()
                .equator_type_for_ecliptic()
                .is_some()
        );
    }
}
