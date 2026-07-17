//! Apparent place of a source as observed from a [`Frame`].
//!
//! An [`Apparent`] bundles the frame the place was computed in, the chosen
//! [`ReferenceSystem`], and the resulting `(α, δ, distance, rv)` from
//! `SuperNOVAS`'s `novas_sky_pos`. From there you can read out RA/Dec in the
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
    /// Celestial Intermediate Reference System - the modern IAU 2006
    /// equivalent of an equator-of-date system, with origin at the CIO.
    Cirs,
    /// International Celestial Reference System - the fixed extragalactic
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
    #[must_use]
    pub fn reference_system(self) -> ReferenceSystem {
        self.system
    }

    /// Borrow the underlying C `sky_pos` for FFI calls inside the
    /// safe-wrapper crate (e.g. [`crate::Transform::apply_sky_pos`]).
    pub(crate) fn as_sky_pos(&self) -> &sky_pos {
        &self.sky
    }

    /// The unit direction vector `r_hat` toward the source, in the
    /// apparent's [`ReferenceSystem`].
    ///
    /// This is the dimensionless unit vector `[x, y, z]` with
    /// `x = cos(dec) cos(ra)`, `y = cos(dec) sin(ra)`, `z = sin(dec)`.
    #[must_use]
    pub fn r_hat(self) -> [f64; 3] {
        self.sky.r_hat
    }

    /// Reassemble an [`Apparent`] from its three constituent parts.
    ///
    /// Used by [`crate::Transform::apply_sky_pos`] to re-tag a transformed
    /// sky position with the destination system and the transform's frame.
    #[allow(clippy::large_types_passed_by_value)]
    pub(crate) fn from_parts(frame: Frame, system: ReferenceSystem, sky: sky_pos) -> Self {
        Apparent { frame, system, sky }
    }

    /// The frame this apparent place was computed in.
    #[must_use]
    pub fn frame(self) -> Frame {
        self.frame
    }

    /// Right ascension in the apparent's reference system.
    #[must_use]
    pub fn ra(self) -> TimeAngle {
        TimeAngle::from_hours(self.sky.ra).expect("sky_pos.ra is finite by construction")
    }

    /// Declination in the apparent's reference system.
    #[must_use]
    pub fn dec(self) -> Angle {
        Angle::from_degrees(self.sky.dec).expect("sky_pos.dec is finite by construction")
    }

    /// Geometric distance to the source. Returns `0` (an unrepresentable
    /// distance) for sidereal sources, matching the `SuperNOVAS` convention.
    ///
    /// For catalog stars, treat this as "not available" - the underlying C
    /// API doesn't carry parallax distance through `sky_pos`. Use
    /// `CatalogEntry`'s parallax accessor if you need the distance.
    #[must_use]
    pub fn distance(self) -> Coordinate {
        // sky_pos.dis is in AU. NaN-safe via Coordinate's constructor.
        Coordinate::from_au(self.sky.dis)
            .unwrap_or_else(|_| Coordinate::from_meters(0.0).expect("zero is finite"))
    }

    /// Apparent radial velocity (positive = receding).
    #[must_use]
    pub fn radial_velocity(self) -> ScalarVelocity {
        ScalarVelocity::from_km_per_s(self.sky.rv).expect("sky_pos.rv is finite by construction")
    }

    /// The corresponding [`Equinox`] for this apparent's reference system
    /// at this apparent's frame time.
    ///
    /// For date-dependent systems (MOD, TOD, CIRS) the equinox carries the
    /// frame's TT Julian date. For date-independent systems (ICRS, J2000,
    /// GCRS) the equinox is the pre-built constant. The Earth-rotating
    /// systems (TIRS, ITRS) keep their own tag - their longitudes differ
    /// from any equinox-based system by the Earth rotation angle, so
    /// re-labeling them would silently corrupt downstream conversions;
    /// instead, conversions that need an equinox-based system return
    /// [`crate::Error::UnsupportedSystem`].
    #[must_use]
    pub fn equinox(self) -> Equinox {
        let jd = self.frame.tt_jd();
        match self.system {
            ReferenceSystem::Icrs | ReferenceSystem::Gcrs => Equinox::ICRS,
            ReferenceSystem::J2000 => Equinox::J2000,
            ReferenceSystem::Mod => Equinox::mod_at(jd).expect("frame TT JD is finite"),
            ReferenceSystem::Tod => Equinox::tod_at(jd).expect("frame TT JD is finite"),
            ReferenceSystem::Cirs => Equinox::cirs_at(jd).expect("frame TT JD is finite"),
            ReferenceSystem::Tirs => {
                Equinox::at("TIRS", ReferenceSystem::Tirs, jd).expect("frame TT JD is finite")
            }
            ReferenceSystem::Itrs => {
                Equinox::at("ITRS", ReferenceSystem::Itrs, jd).expect("frame TT JD is finite")
            }
        }
    }

    /// View this apparent place as an [`Equatorial`] (RA, Dec, equinox).
    ///
    /// No transformation happens - this is just re-tagging the underlying
    /// RA/Dec with a typed equinox derived from
    /// [`Self::reference_system`] and the frame's TT date.
    #[must_use]
    pub fn equatorial(self) -> Equatorial {
        Equatorial::new(self.ra(), self.dec(), self.equinox())
    }

    /// View this apparent place as an [`Ecliptic`] (λ, β, equinox).
    ///
    /// Routes through [`Self::equatorial`] then `Equatorial::to_ecliptic`.
    /// For sources in CIRS this transparently re-routes via TOD; the
    /// Earth-rotating systems (TIRS, ITRS) return
    /// [`Error::UnsupportedSystem`].
    pub fn ecliptic(self, accuracy: Accuracy) -> Result<Ecliptic> {
        self.equatorial().to_ecliptic(accuracy)
    }

    /// View this apparent place as a [`Galactic`] (l, b).
    ///
    /// Routes through ICRS via [`Self::equatorial`] then
    /// [`Equatorial::to_galactic`]. The Earth-rotating systems (TIRS, ITRS)
    /// have no ICRS mapping and return an error.
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
                &raw mut az_deg,
                &raw mut el_deg,
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
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
        return Err(Error::ffi(rc));
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
        // arcminutes - definitely more than 1 arcsec.
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
        // CIRS at the frame's TT date - system tag should be a CIRS
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

    #[test]
    fn distance_and_radial_velocity_are_finite() {
        let frame = ovro_frame();
        let apparent = vega().apparent_in(&frame, ReferenceSystem::Icrs).unwrap();
        // Sidereal catalog source: dis = 0 (unset by SuperNOVAS convention).
        assert!(apparent.distance().m().is_finite());
        // Radial velocity is finite (may be ~0 for a zero-rv catalog source).
        assert!(apparent.radial_velocity().km_per_s().is_finite());
    }

    #[test]
    fn frame_getter_round_trips() {
        let frame = ovro_frame();
        let apparent = vega().apparent_in(&frame, ReferenceSystem::Icrs).unwrap();
        // The stored frame has the same TT JD.
        assert!((apparent.frame().tt_jd() - frame.tt_jd()).abs() < 1e-9);
    }

    #[test]
    fn reference_system_getter() {
        let frame = ovro_frame();
        let apparent = vega().apparent_in(&frame, ReferenceSystem::Mod).unwrap();
        assert_eq!(apparent.reference_system(), ReferenceSystem::Mod);
    }

    #[test]
    fn radio_refraction_lifts_elevation() {
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
        let geometric = apparent.to_horizontal().unwrap();
        let radio = apparent
            .to_horizontal_with_refraction(Refraction::Radio)
            .unwrap();
        assert!(
            radio.elevation().deg() > geometric.elevation().deg(),
            "radio refraction should lift elevation"
        );
    }

    /// TIRS/ITRS longitudes are offset from every equinox-based system by
    /// the Earth rotation angle, so re-tagging them (e.g. as TOD) would
    /// silently corrupt conversions. They must keep their own system tag,
    /// and ecliptic conversion must refuse rather than produce garbage.
    #[test]
    fn earth_rotating_systems_keep_their_own_tag_and_refuse_ecliptic() {
        let frame = ovro_frame();
        let vega = vega();
        for system in [ReferenceSystem::Tirs, ReferenceSystem::Itrs] {
            let app = vega.apparent_in(&frame, system).unwrap();
            assert_eq!(app.equinox().system(), system);
            assert_eq!(app.equatorial().system().system(), system);
            assert!(matches!(
                app.ecliptic(Accuracy::Reduced),
                Err(crate::Error::UnsupportedSystem)
            ));
        }
    }

    /// The humidity stored in [`Weather`] must reach the C-side observer:
    /// radio refraction includes a water-vapor term, so dry vs. saturated
    /// air must give measurably different elevations.
    #[test]
    fn radio_refraction_responds_to_humidity() {
        let t = crate::Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap();
        let polaris = CatalogEntry::icrs(
            "Polaris",
            "02:31:49.10".parse().unwrap(),
            "+89:15:50.79".parse().unwrap(),
        )
        .unwrap();
        let el_at_humidity = |rh: f64| {
            let w = Weather::new(
                Some(crate::Temperature::from_celsius(15.0).unwrap()),
                Some(crate::Pressure::from_hpa(1013.25).unwrap()),
                Some(rh),
            )
            .unwrap();
            let site = crate::Site::from_degrees(37.234, -118.282, 1222.0)
                .unwrap()
                .with_weather(w);
            let frame = Frame::new(Accuracy::Reduced, &Observer::Geodetic(site), &t).unwrap();
            polaris
                .apparent_in(&frame, ReferenceSystem::Cirs)
                .unwrap()
                .to_horizontal_with_refraction(Refraction::Radio)
                .unwrap()
                .elevation()
                .arcsec()
        };
        let dry = el_at_humidity(0.0);
        let wet = el_at_humidity(100.0);
        // ~9% of the total refraction (a few arcsec at 37° elevation).
        assert!(
            (wet - dry).abs() > 1.0,
            "humidity had no effect on radio refraction: Δel = {} arcsec",
            (wet - dry).abs()
        );
    }

    /// A site with no explicit weather uses `SuperNOVAS`'s location-based
    /// mean annual estimate, so the weather-dependent refraction models
    /// must still produce a finite, physically sensible result (previously
    /// NaN weather poisoned them into an error).
    #[test]
    fn weather_refraction_works_without_explicit_weather() {
        let t = crate::Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap();
        let site = crate::Site::from_degrees(37.234, -118.282, 1222.0).unwrap();
        let frame = Frame::new(Accuracy::Reduced, &Observer::Geodetic(site), &t).unwrap();
        let polaris = CatalogEntry::icrs(
            "Polaris",
            "02:31:49.10".parse().unwrap(),
            "+89:15:50.79".parse().unwrap(),
        )
        .unwrap();
        let apparent = polaris.apparent_in(&frame, ReferenceSystem::Cirs).unwrap();
        let geometric = apparent.to_horizontal().unwrap();
        for model in [Refraction::Optical, Refraction::Radio] {
            let refracted = apparent.to_horizontal_with_refraction(model).unwrap();
            let lift_arcsec = (refracted.elevation().deg() - geometric.elevation().deg()) * 3600.0;
            assert!(
                (10.0..300.0).contains(&lift_arcsec),
                "{model:?} refraction with default weather looks wrong: Δel = {lift_arcsec} arcsec"
            );
        }
    }

    #[test]
    fn reference_system_to_sys_covers_all_variants() {
        // Each call exercises a distinct arm of to_sys(); we just check the
        // return is non-zero (all variants map to a distinct C constant).
        let _ = ReferenceSystem::Gcrs.to_sys();
        let _ = ReferenceSystem::Tod.to_sys();
        let _ = ReferenceSystem::Cirs.to_sys();
        let _ = ReferenceSystem::Icrs.to_sys();
        let _ = ReferenceSystem::J2000.to_sys();
        let _ = ReferenceSystem::Mod.to_sys();
        let _ = ReferenceSystem::Tirs.to_sys();
        let _ = ReferenceSystem::Itrs.to_sys();
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn equinox_for_date_dependent_systems() {
        let frame = ovro_frame();
        let vega = vega();
        // equinox() for Mod returns a Mod-system equinox at the frame's JD.
        let mod_app = vega.apparent_in(&frame, ReferenceSystem::Mod).unwrap();
        let eq_mod = mod_app.equinox();
        assert_eq!(eq_mod.system(), ReferenceSystem::Mod);
        assert!((eq_mod.jd() - frame.tt_jd()).abs() < 1e-9);

        // equinox() for Tod returns a Tod-system equinox.
        let tod_app = vega.apparent_in(&frame, ReferenceSystem::Tod).unwrap();
        let eq_tod = tod_app.equinox();
        assert_eq!(eq_tod.system(), ReferenceSystem::Tod);
    }

    #[test]
    fn gcrs_apparent_computes_and_has_correct_system() {
        let frame = ovro_frame();
        let app = vega().apparent_in(&frame, ReferenceSystem::Gcrs).unwrap();
        assert_eq!(app.reference_system(), ReferenceSystem::Gcrs);
        // equinox() for GCRS returns the constant ICRS equinox.
        assert_eq!(app.equinox(), crate::Equinox::ICRS);
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
        // - but they need not be bit-identical because Standard ignores
        // weather entirely.
        let diff_arcsec = (optical.elevation().deg() - standard.elevation().deg()).abs() * 3600.0;
        assert!(
            diff_arcsec < 60.0,
            "optical vs standard refraction shouldn't differ by more than ~1 arcmin at 37°, got {diff_arcsec}"
        );
    }
}
