//! Apparent place of a source as observed from a [`Frame`].
//!
//! An [`Apparent`] bundles the frame the place was computed in, the chosen
//! [`ReferenceSystem`], and the resulting `(α, δ, distance, rv)` from
//! SuperNOVAS's `novas_sky_pos`. From there you can read out RA/Dec in the
//! source system or convert to horizontal coordinates.

use core::mem::MaybeUninit;

use supernovas_ffi::{
    novas_app_to_hor, novas_reference_system,
    novas_reference_system::{
        NOVAS_CIRS, NOVAS_GCRS, NOVAS_ICRS, NOVAS_ITRS, NOVAS_J2000, NOVAS_MOD, NOVAS_TIRS,
        NOVAS_TOD,
    },
    novas_sky_pos, sky_pos,
};

use crate::{
    Accuracy, Angle, Coordinate, Ecliptic, Equatorial, Equinox, Frame, Galactic, Horizontal,
    Refraction, ScalarVelocity, TimeAngle,
    error::{Error, Result},
    source::Source,
};

/// An equatorial reference system for sky-position computations.
///
/// Mirrors the C-side `novas_reference_system`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReferenceSystem {
    /// Geocentric Celestial Reference System (essentially ICRS with
    /// observer-relative aberration and gravitational deflection).
    Gcrs,
    /// True equator and equinox of date.
    Tod,
    /// Celestial Intermediate Reference System — the modern IAU 2006
    /// equivalent of an equator-of-date system, with origin at the CIO.
    Cirs,
    /// International Celestial Reference System — the fixed extragalactic
    /// frame.
    Icrs,
    /// Mean equator and equinox of J2000.0.
    J2000,
    /// Mean equator and equinox of date.
    Mod,
    /// Terrestrial Intermediate Reference System (rotates with Earth).
    Tirs,
    /// International Terrestrial Reference System (Earth-fixed).
    Itrs,
}

impl ReferenceSystem {
    pub(crate) fn to_sys(self) -> novas_reference_system {
        match self {
            ReferenceSystem::Gcrs => NOVAS_GCRS,
            ReferenceSystem::Tod => NOVAS_TOD,
            ReferenceSystem::Cirs => NOVAS_CIRS,
            ReferenceSystem::Icrs => NOVAS_ICRS,
            ReferenceSystem::J2000 => NOVAS_J2000,
            ReferenceSystem::Mod => NOVAS_MOD,
            ReferenceSystem::Tirs => NOVAS_TIRS,
            ReferenceSystem::Itrs => NOVAS_ITRS,
        }
    }
}

/// The apparent place of a source as seen from a particular [`Frame`].
///
/// Stores the underlying `sky_pos` so you can read RA/Dec in the originating
/// [`ReferenceSystem`] or convert to a different output frame: [`Horizontal`],
/// [`Equatorial`], [`Ecliptic`], or [`Galactic`].
#[derive(Debug, Clone, Copy)]
pub struct Apparent {
    frame: Frame,
    system: ReferenceSystem,
    sky: sky_pos,
}

impl Apparent {
    /// The reference system the underlying RA/Dec are expressed in.
    pub fn reference_system(self) -> ReferenceSystem {
        self.system
    }

    /// The frame this apparent place was computed in.
    pub fn frame(self) -> Frame {
        self.frame
    }

    /// Right ascension in the apparent's reference system.
    pub fn ra(self) -> TimeAngle {
        TimeAngle::from_hours(self.sky.ra).expect("sky_pos.ra is finite by construction")
    }

    /// Declination in the apparent's reference system.
    pub fn dec(self) -> Angle {
        Angle::from_degrees(self.sky.dec).expect("sky_pos.dec is finite by construction")
    }

    /// Geometric distance to the source. Returns `0` (an unrepresentable
    /// distance) for sidereal sources, matching the SuperNOVAS convention.
    ///
    /// For catalog stars, treat this as "not available" — the underlying C
    /// API doesn't carry parallax distance through `sky_pos`. Use
    /// `CatalogEntry`'s parallax accessor if you need the distance.
    pub fn distance(self) -> Coordinate {
        // sky_pos.dis is in AU. NaN-safe via Coordinate's constructor.
        Coordinate::from_au(self.sky.dis)
            .unwrap_or_else(|_| Coordinate::from_meters(0.0).expect("zero is finite"))
    }

    /// Apparent radial velocity (positive = receding).
    pub fn radial_velocity(self) -> ScalarVelocity {
        ScalarVelocity::from_km_per_s(self.sky.rv).expect("sky_pos.rv is finite by construction")
    }

    /// The corresponding [`Equinox`] for this apparent's reference system
    /// at this apparent's frame time.
    ///
    /// For date-dependent systems (MOD, TOD, CIRS) the equinox carries the
    /// frame's TT Julian date. For date-independent systems (ICRS, J2000,
    /// GCRS) the equinox is the pre-built constant.
    pub fn equinox(self) -> Equinox {
        let jd = self.frame.tt_jd();
        match self.system {
            ReferenceSystem::Icrs | ReferenceSystem::Gcrs => Equinox::ICRS,
            ReferenceSystem::J2000 => Equinox::J2000,
            ReferenceSystem::Mod => Equinox::mod_at(jd).expect("frame TT JD is finite"),
            ReferenceSystem::Tod => Equinox::tod_at(jd).expect("frame TT JD is finite"),
            ReferenceSystem::Cirs => Equinox::cirs_at(jd).expect("frame TT JD is finite"),
            // Earth-rotating systems aren't really "equinoxes"; pin them
            // to TOD-at-time as the closest equatorial proxy.
            ReferenceSystem::Tirs | ReferenceSystem::Itrs => {
                Equinox::tod_at(jd).expect("frame TT JD is finite")
            }
        }
    }

    /// View this apparent place as an [`Equatorial`] (RA, Dec, equinox).
    ///
    /// No transformation happens — this is just re-tagging the underlying
    /// RA/Dec with a typed equinox derived from
    /// [`Self::reference_system`] and the frame's TT date.
    pub fn equatorial(self) -> Equatorial {
        Equatorial::new(self.ra(), self.dec(), self.equinox())
    }

    /// View this apparent place as an [`Ecliptic`] (λ, β, equinox).
    ///
    /// Routes through [`Self::equatorial`] then `Equatorial::to_ecliptic`.
    /// For sources in CIRS this transparently re-routes via TOD; ITRS
    /// sources return [`Error::UnsupportedSystem`].
    pub fn ecliptic(self, accuracy: Accuracy) -> Result<Ecliptic> {
        self.equatorial().to_ecliptic(accuracy)
    }

    /// View this apparent place as a [`Galactic`] (l, b).
    ///
    /// Routes through ICRS via [`Self::equatorial`] then
    /// [`Equatorial::to_galactic`].
    pub fn galactic(self, accuracy: Accuracy) -> Result<Galactic> {
        self.equatorial().to_galactic(accuracy)
    }

    /// Convert to horizontal (azimuth/elevation) coordinates for this
    /// frame's observer, with no atmospheric refraction.
    ///
    /// Equivalent to [`Self::to_horizontal_with_refraction`]
    /// with [`Refraction::None`].
    pub fn to_horizontal(self) -> Result<Horizontal> {
        self.to_horizontal_with_refraction(Refraction::None)
    }

    /// Convert to horizontal coordinates, applying the requested
    /// atmospheric refraction model.
    ///
    /// Pass [`Refraction::Optical`] for visible-band telescopes (uses the
    /// per-site weather data stored in the [`Frame`]'s observer);
    /// [`Refraction::Radio`] for radio observatories;
    /// [`Refraction::Standard`] for a weather-agnostic standard atmosphere
    /// approximation; or [`Refraction::None`] to skip refraction entirely.
    pub fn to_horizontal_with_refraction(self, refraction: Refraction) -> Result<Horizontal> {
        let mut az_deg: f64 = 0.0;
        let mut el_deg: f64 = 0.0;
        // SAFETY: novas_app_to_hor writes the two output doubles on a zero
        // return. The refraction-model callback is either NULL (no
        // refraction) or one of the SuperNOVAS-provided built-ins, all of
        // which match the `RefractionModel` ABI exactly.
        let rc = unsafe {
            novas_app_to_hor(
                self.frame.as_novas_frame(),
                self.system.to_sys(),
                self.sky.ra,
                self.sky.dec,
                refraction.to_sys(),
                &mut az_deg as *mut f64,
                &mut el_deg as *mut f64,
            )
        };
        if rc != 0 {
            return Err(Error::Ffi);
        }
        Horizontal::from_degrees(az_deg, el_deg)
    }
}

/// Compute the apparent place of any [`Source`] for the given frame and
/// reference system.
///
/// Used internally by [`Source::apparent_in`] and [`Frame::observe`].
pub(crate) fn apparent_of_source_in(
    source: &(impl Source + ?Sized),
    frame: &Frame,
    system: ReferenceSystem,
) -> Result<Apparent> {
    let mut sky = MaybeUninit::<sky_pos>::zeroed();
    // SAFETY: novas_sky_pos initializes *sky on a zero return.
    let rc = unsafe {
        novas_sky_pos(
            source.as_object(),
            frame.as_novas_frame(),
            system.to_sys(),
            sky.as_mut_ptr(),
        )
    };
    if rc != 0 {
        return Err(Error::Ffi);
    }
    Ok(Apparent {
        frame: *frame,
        system,
        sky: unsafe { sky.assume_init() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Accuracy, CatalogEntry, Observer, Time, Weather};

    fn vega() -> CatalogEntry {
        CatalogEntry::icrs(
            "Vega",
            "18:36:56.336".parse().unwrap(),
            "+38:47:01.28".parse().unwrap(),
        )
        .unwrap()
    }

    fn ovro_frame() -> Frame {
        let obs = Observer::geodetic(37.234, -118.282, 1222.0).unwrap();
        let t = Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap();
        Frame::new(Accuracy::Reduced, &obs, &t).unwrap()
    }

    #[test]
    fn cirs_apparent_then_horizontal_matches_frame_observe() {
        let frame = ovro_frame();
        let vega = vega();
        let apparent = vega.apparent_in(&frame, ReferenceSystem::Cirs).unwrap();
        let via_split = apparent.to_horizontal().unwrap();
        let via_observe = frame.observe(&vega).unwrap();
        // The two paths should give bit-identical results.
        assert_eq!(via_split.azimuth().rad(), via_observe.azimuth().rad());
        assert_eq!(via_split.elevation().rad(), via_observe.elevation().rad());
    }

    #[test]
    fn apparent_ra_dec_round_trip() {
        let frame = ovro_frame();
        let apparent = vega().apparent_in(&frame, ReferenceSystem::Icrs).unwrap();
        // ICRS RA/Dec should be very close to the input catalog values
        // (small offset from aberration / gravitational deflection).
        let ra_h = apparent.ra().hours();
        let dec_d = apparent.dec().deg();
        let expected_ra_h = 18.0 + 36.0 / 60.0 + 56.336 / 3600.0;
        let expected_dec_d = 38.0 + 47.0 / 60.0 + 1.28 / 3600.0;
        // Aberration of ~20 arcsec ≈ 0.0056 deg ≈ 1.3e-3 hr
        assert!(
            (ra_h - expected_ra_h).abs() < 1e-3,
            "RA {ra_h} vs expected {expected_ra_h}"
        );
        assert!(
            (dec_d - expected_dec_d).abs() < 1e-2,
            "Dec {dec_d} vs expected {expected_dec_d}"
        );
    }

    #[test]
    fn different_reference_systems_give_different_ra() {
        let frame = ovro_frame();
        let vega = vega();
        let icrs = vega.apparent_in(&frame, ReferenceSystem::Icrs).unwrap();
        let cirs = vega.apparent_in(&frame, ReferenceSystem::Cirs).unwrap();
        // Precession between ICRS and CIRS at 2026 differs by several
        // arcminutes — definitely more than 1 arcsec.
        assert!((icrs.ra().hours() - cirs.ra().hours()).abs() > 1e-4);
    }

    #[test]
    fn refraction_none_matches_bare_to_horizontal() {
        let frame = ovro_frame();
        let apparent = vega().apparent_in(&frame, ReferenceSystem::Cirs).unwrap();
        let plain = apparent.to_horizontal().unwrap();
        let explicit_none = apparent
            .to_horizontal_with_refraction(Refraction::None)
            .unwrap();
        // Bit-identical: both should resolve through novas_app_to_hor with
        // a NULL refraction callback.
        assert_eq!(plain.azimuth().rad(), explicit_none.azimuth().rad());
        assert_eq!(plain.elevation().rad(), explicit_none.elevation().rad());
    }

    /// Atmospheric refraction lifts the apparent elevation of sources.
    /// Refracted el should be **above** the geometric el, with the
    /// difference larger at lower elevations and ~tens of arcseconds for
    /// Polaris-like elevations (~37°).
    #[test]
    fn standard_refraction_lifts_elevation() {
        let frame = ovro_frame();
        let polaris = CatalogEntry::icrs(
            "Polaris",
            "02:31:49.10".parse().unwrap(),
            "+89:15:50.79".parse().unwrap(),
        )
        .unwrap();
        let apparent = polaris.apparent_in(&frame, ReferenceSystem::Cirs).unwrap();
        let geometric = apparent.to_horizontal().unwrap();
        let refracted = apparent
            .to_horizontal_with_refraction(Refraction::Standard)
            .unwrap();
        let delta_arcsec = (refracted.elevation().deg() - geometric.elevation().deg()) * 3600.0;
        assert!(
            delta_arcsec > 0.0,
            "refraction should lift elevation, got Δel = {delta_arcsec} arcsec"
        );
        // Polaris is at ~37° elevation from OVRO; the lift should be
        // somewhere in the 30 arcsec – 5 arcmin range.
        assert!(
            (30.0..300.0).contains(&delta_arcsec),
            "Δel = {delta_arcsec} arcsec looks suspicious for ~37° elevation"
        );
    }

    #[test]
    fn equatorial_view_matches_apparent_ra_dec() {
        let frame = ovro_frame();
        let apparent = vega().apparent_in(&frame, ReferenceSystem::Cirs).unwrap();
        let eq = apparent.equatorial();
        // RA/Dec round-trip through Equatorial without modification.
        assert_eq!(eq.ra().rad(), apparent.ra().rad());
        assert_eq!(eq.dec().rad(), apparent.dec().rad());
        // CIRS at the frame's TT date — system tag should be a CIRS
        // equinox at that JD.
        assert_eq!(eq.system().system(), ReferenceSystem::Cirs);
        assert!((eq.system().jd() - frame.tt_jd()).abs() < 1e-9);
    }

    #[test]
    fn apparent_galactic_matches_known_vega_values() {
        let frame = ovro_frame();
        let icrs_apparent = vega().apparent_in(&frame, ReferenceSystem::Icrs).unwrap();
        let g = icrs_apparent.galactic(Accuracy::Reduced).unwrap();
        // Vega's galactic coordinates: l ≈ 67.45°, b ≈ +19.24°.
        // Aberration nudges this by ≲ 20 arcsec; tolerate 0.1°.
        assert!(
            (g.l().deg() - 67.45).abs() < 0.1,
            "apparent galactic l = {} should be near 67.45°",
            g.l().deg()
        );
        assert!(
            (g.b().deg() - 19.24).abs() < 0.1,
            "apparent galactic b = {} should be near 19.24°",
            g.b().deg()
        );
    }

    #[test]
    fn apparent_ecliptic_via_round_trip_matches_apparent_equatorial() {
        let frame = ovro_frame();
        let apparent = vega().apparent_in(&frame, ReferenceSystem::J2000).unwrap();
        let ecl = apparent.ecliptic(Accuracy::Reduced).unwrap();
        let back = ecl.to_equatorial(Accuracy::Reduced).unwrap();
        let eq = apparent.equatorial();
        // Round-trip through ecliptic should close to within numerical precision.
        let sep = eq.distance_to(back).uas();
        assert!(sep < 100.0, "round-trip drift {sep} µas exceeded tolerance");
    }

    #[test]
    fn equatorial_uses_constant_equinoxes_for_fixed_systems() {
        let frame = ovro_frame();
        let icrs = vega().apparent_in(&frame, ReferenceSystem::Icrs).unwrap();
        // ICRS apparent gives back the ICRS constant equinox, regardless
        // of the frame date.
        assert_eq!(icrs.equatorial().system(), Equinox::ICRS);

        let j2000 = vega().apparent_in(&frame, ReferenceSystem::J2000).unwrap();
        assert_eq!(j2000.equatorial().system(), Equinox::J2000);
    }

    /// Optical-band refraction (which uses the site's local weather)
    /// should give a slightly different answer from the
    /// weather-agnostic standard model.
    #[test]
    fn optical_refraction_uses_site_weather() {
        let obs = Observer::Geodetic(
            crate::Site::from_degrees(37.234, -118.282, 1222.0)
                .unwrap()
                .with_weather(Weather::standard()),
        );
        let t = crate::Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap();
        let frame = Frame::new(Accuracy::Reduced, &obs, &t).unwrap();
        let polaris = CatalogEntry::icrs(
            "Polaris",
            "02:31:49.10".parse().unwrap(),
            "+89:15:50.79".parse().unwrap(),
        )
        .unwrap();
        let apparent = polaris.apparent_in(&frame, ReferenceSystem::Cirs).unwrap();
        let standard = apparent
            .to_horizontal_with_refraction(Refraction::Standard)
            .unwrap();
        let optical = apparent
            .to_horizontal_with_refraction(Refraction::Optical)
            .unwrap();
        // Both apply refraction (so both elevations are above the
        // geometric), and both should be within a few arcsec of each other
        // — but they need not be bit-identical because Standard ignores
        // weather entirely.
        let diff_arcsec = (optical.elevation().deg() - standard.elevation().deg()).abs() * 3600.0;
        assert!(
            diff_arcsec < 60.0,
            "optical vs standard refraction shouldn't differ by more than ~1 arcmin at 37°, got {diff_arcsec}"
        );
    }
}
