//! Catalog (sidereal) source: a fixed direction on the sky in ICRS, with
//! optional proper motion, parallax, and radial velocity.

use core::{
    ffi::{CStr, c_char},
    fmt,
    mem::MaybeUninit,
};

use supernovas_ffi::{
    cat_entry, make_cat_entry, make_cat_object, make_cat_object_sys, make_redshifted_object_sys,
    novas_set_distance, novas_set_lsr_vel, novas_set_redshift, novas_set_ssb_vel, object,
};

use crate::{
    Angle, Coordinate, ScalarVelocity, TimeAngle,
    error::{Error, Result},
};

/// The catalog coordinate system that the source coordinates are expressed in.
///
/// Used with [`CatalogEntry::in_system`] to create sources from non-ICRS
/// catalog data. `SuperNOVAS` converts to ICRS internally via `make_cat_object_sys`.
///
/// The ICRS is the internal representation; all other systems are converted
/// on construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSystem {
    /// International Celestial Reference System (modern default).
    Icrs,
    /// J2000.0 dynamical reference system.
    J2000,
    /// B1950.0 (FK4 precession).
    B1950,
    /// FK4 catalog system (dynamically equivalent to B1950 for most purposes).
    Fk4,
    /// FK5 catalog system.
    Fk5,
}

impl CatalogSystem {
    fn as_ptr(self) -> *const c_char {
        match self {
            Self::Icrs => c"ICRS".as_ptr(),
            Self::J2000 => c"J2000".as_ptr(),
            Self::B1950 => c"B1950".as_ptr(),
            Self::Fk4 => c"FK4".as_ptr(),
            Self::Fk5 => c"FK5".as_ptr(),
        }
    }
}

/// A catalog (sidereal) sky source.
///
/// Stored in ICRS internally. Construct via [`Self::icrs`] for coordinates
/// already in ICRS, or [`Self::in_system`] when starting from B1950/FK4/FK5/J2000
/// catalog data. Refine via the `with_*` builder methods to add proper motion,
/// parallax, radial velocity, or redshift.
#[derive(Clone, Copy)]
pub struct CatalogEntry {
    object: object,
}

impl super::sealed::Sealed for CatalogEntry {}
impl super::Source for CatalogEntry {
    fn as_object(&self) -> &object {
        &self.object
    }
}

impl fmt::Debug for CatalogEntry {
    /// Manual `Debug` impl - the underlying C `object` contains an `orbit`
    /// substructure that `make_cat_object` deliberately leaves
    /// uninitialised (see `SuperNOVAS` upstream `target.c` for the
    /// `memset(source, 0, offsetof(object, orbit))` choice). Reading
    /// arbitrary bytes through the auto-derived `Debug` impl would trigger
    /// UB on the embedded `novas_*` enums; we sidestep that by formatting
    /// only the fields we know are initialised.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CatalogEntry")
            .field(
                "name",
                &cstr_to_str(&self.object.star.starname).unwrap_or(""),
            )
            .field("ra", &self.ra())
            .field("dec", &self.dec())
            .finish_non_exhaustive()
    }
}

impl CatalogEntry {
    /// Construct an ICRS catalog source with no proper motion, no parallax,
    /// and zero radial velocity.
    ///
    /// `name` must be ASCII (no interior NULs) and shorter than 50 bytes
    /// (the `SuperNOVAS` catalog-entry name limit).
    pub fn icrs(name: &str, ra: TimeAngle, dec: Angle) -> Result<Self> {
        Self::make(name, ra, dec, 0.0, 0.0, 0.0, 0.0)
    }

    /// Construct a catalog source whose coordinates are given in a non-ICRS
    /// system and convert them to ICRS via `make_cat_object_sys`.
    ///
    /// Use this for legacy catalog data in B1950/FK4/FK5/J2000 frames.
    /// ICRS coordinates are the internal representation; all other systems
    /// are converted on construction.
    ///
    /// `name` must be ASCII (no interior NULs) and shorter than 50 bytes.
    pub fn in_system(name: &str, ra: TimeAngle, dec: Angle, system: CatalogSystem) -> Result<Self> {
        let entry = make_raw_cat_entry(name, ra, dec, 0.0, 0.0, 0.0, 0.0)?;
        Self::from_cat_entry_sys(&entry, system)
    }

    /// Construct a high-redshift source (quasar, galaxy) from ICRS coordinates
    /// and a spectroscopic redshift `z`.
    ///
    /// This wraps `make_redshifted_object_sys` and is the preferred path for
    /// cosmological sources where `z` is the natural observable.
    ///
    /// `name` must be ASCII (no interior NULs) and shorter than 50 bytes.
    pub fn redshifted_icrs(name: &str, ra: TimeAngle, dec: Angle, z: f64) -> Result<Self> {
        if !z.is_finite() {
            return Err(Error::NotFinite);
        }
        let bytes = name.as_bytes();
        if bytes.contains(&0) || bytes.len() >= 50 {
            return Err(Error::Parse);
        }
        let mut name_buf = [0u8; 50];
        name_buf[..bytes.len()].copy_from_slice(bytes);
        let name_cs =
            CStr::from_bytes_with_nul(&name_buf[..=bytes.len()]).map_err(|_| Error::Parse)?;
        let mut obj = MaybeUninit::<object>::zeroed();
        // SAFETY: make_redshifted_object_sys initializes *obj on a zero return.
        let rc = unsafe {
            make_redshifted_object_sys(
                name_cs.as_ptr(),
                ra.hours(),
                dec.deg(),
                c"ICRS".as_ptr(),
                z,
                obj.as_mut_ptr(),
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(CatalogEntry {
            object: unsafe { obj.assume_init() },
        })
    }

    // ── Refinement builders ───────────────────────────────────────────────

    /// Builder: attach proper motion in RA and Dec (mas/yr).
    pub fn with_proper_motion_mas_per_yr(self, pm_ra: f64, pm_dec: f64) -> Result<Self> {
        if !pm_ra.is_finite() || !pm_dec.is_finite() {
            return Err(Error::NotFinite);
        }
        let star = self.object.star;
        Self::make(
            cstr_to_str(&star.starname).unwrap_or(""),
            TimeAngle::from_hours(star.ra)?,
            Angle::from_degrees(star.dec)?,
            pm_ra,
            pm_dec,
            star.parallax,
            star.radialvelocity,
        )
    }

    /// Builder: attach trigonometric parallax in arcseconds.
    pub fn with_parallax(self, parallax: Angle) -> Result<Self> {
        let star = self.object.star;
        Self::make(
            cstr_to_str(&star.starname).unwrap_or(""),
            TimeAngle::from_hours(star.ra)?,
            Angle::from_degrees(star.dec)?,
            star.promora,
            star.promodec,
            parallax.mas(),
            star.radialvelocity,
        )
    }

    /// Builder: set distance in parsecs.
    ///
    /// Alternative to [`with_parallax`](Self::with_parallax). Useful for
    /// extragalactic sources where a distance measurement is more natural
    /// than a parallax angle.
    pub fn with_distance(mut self, d: Coordinate) -> Result<Self> {
        // SAFETY: novas_set_distance writes to the cat_entry on a 0 return;
        // the pointer is valid for the duration of the call.
        let rc = unsafe { novas_set_distance(&raw mut self.object.star, d.pc()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(self)
    }

    /// Builder: set the Solar System Barycenter (SSB) radial velocity.
    ///
    /// This is the preferred setter for most modern stellar catalog velocities
    /// (e.g., Gaia, RAVE, APOGEE), which are typically reported relative to
    /// the SSB.
    ///
    /// Supersedes [`with_radial_velocity`](Self::with_radial_velocity) for
    /// catalogs that provide heliocentric/barycentric velocities.
    pub fn with_ssb_velocity(mut self, rv: ScalarVelocity) -> Result<Self> {
        // SAFETY: novas_set_ssb_vel writes to the cat_entry on a 0 return.
        let rc = unsafe { novas_set_ssb_vel(&raw mut self.object.star, rv.km_per_s()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(self)
    }

    /// Builder: set the Local Standard of Rest (LSR) radial velocity.
    ///
    /// Used in galactic radio astronomy where velocities are conventionally
    /// referenced to the LSR rather than the SSB. `epoch_jd` is the TT Julian
    /// date of the source position epoch (e.g., `2451545.0` for J2000).
    pub fn with_lsr_velocity(mut self, rv: ScalarVelocity, epoch_jd: f64) -> Result<Self> {
        if !epoch_jd.is_finite() {
            return Err(Error::NotFinite);
        }
        // SAFETY: novas_set_lsr_vel writes to the cat_entry on a 0 return.
        let rc = unsafe { novas_set_lsr_vel(&raw mut self.object.star, epoch_jd, rv.km_per_s()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(self)
    }

    /// Builder: set the spectroscopic redshift `z`.
    ///
    /// The preferred setter for cosmological sources (quasars, galaxies)
    /// where the redshift is the fundamental observable.
    /// For stellar sources with a Doppler velocity, prefer
    /// [`with_ssb_velocity`](Self::with_ssb_velocity).
    pub fn with_redshift(mut self, z: f64) -> Result<Self> {
        if !z.is_finite() {
            return Err(Error::NotFinite);
        }
        // SAFETY: novas_set_redshift writes to the cat_entry on a 0 return.
        let rc = unsafe { novas_set_redshift(&raw mut self.object.star, z) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(self)
    }

    /// Builder: attach radial velocity (positive = receding).
    ///
    /// Sets the `radialvelocity` field of the underlying `cat_entry` directly.
    /// For catalogs that specify which reference frame the velocity is in,
    /// prefer [`with_ssb_velocity`](Self::with_ssb_velocity) (SSB-relative)
    /// or [`with_lsr_velocity`](Self::with_lsr_velocity) (LSR-relative).
    pub fn with_radial_velocity(self, rv: ScalarVelocity) -> Result<Self> {
        let star = self.object.star;
        Self::make(
            cstr_to_str(&star.starname).unwrap_or(""),
            TimeAngle::from_hours(star.ra)?,
            Angle::from_degrees(star.dec)?,
            star.promora,
            star.promodec,
            star.parallax,
            rv.km_per_s(),
        )
    }

    // ── Accessors ─────────────────────────────────────────────────────────

    /// ICRS right ascension.
    #[must_use]
    pub fn ra(&self) -> TimeAngle {
        TimeAngle::from_hours(self.object.star.ra).expect("RA stored finite by construction")
    }

    /// ICRS declination.
    #[must_use]
    pub fn dec(&self) -> Angle {
        Angle::from_degrees(self.object.star.dec).expect("Dec stored finite by construction")
    }

    /// Borrow the underlying C `object`, for passing to FFI functions that
    /// take a `*const object`.
    #[allow(dead_code)]
    pub(crate) fn as_object_inner(&self) -> &object {
        &self.object
    }

    /// Compute the apparent place of this source in the given [`crate::Frame`] and
    /// [`crate::ReferenceSystem`].
    ///
    /// The returned [`crate::Apparent`] carries RA/Dec in `system` and can
    /// be converted to horizontal (az/el) coordinates via
    /// [`crate::Apparent::to_horizontal`].
    pub fn apparent_in(
        &self,
        frame: &crate::Frame,
        system: crate::ReferenceSystem,
    ) -> Result<crate::Apparent> {
        crate::apparent::apparent_of_source_in(self, frame, system)
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    fn make(
        name: &str,
        ra: TimeAngle,
        dec: Angle,
        pm_ra: f64,
        pm_dec: f64,
        parallax_mas: f64,
        radial_v_km_per_s: f64,
    ) -> Result<Self> {
        let entry = make_raw_cat_entry(
            name,
            ra,
            dec,
            pm_ra,
            pm_dec,
            parallax_mas,
            radial_v_km_per_s,
        )?;
        let mut obj = MaybeUninit::<object>::zeroed();
        // SAFETY: make_cat_object copies entry into *obj on a zero return.
        let rc = unsafe { make_cat_object(&raw const entry, obj.as_mut_ptr()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(CatalogEntry {
            object: unsafe { obj.assume_init() },
        })
    }

    fn from_cat_entry_sys(entry: &cat_entry, system: CatalogSystem) -> Result<Self> {
        let mut obj = MaybeUninit::<object>::zeroed();
        // SAFETY: make_cat_object_sys converts entry to ICRS and writes *obj on
        // a zero return. The system pointer is a static C string literal.
        let rc =
            unsafe { make_cat_object_sys(&raw const *entry, system.as_ptr(), obj.as_mut_ptr()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(CatalogEntry {
            object: unsafe { obj.assume_init() },
        })
    }
}

impl fmt::Display for CatalogEntry {
    /// Renders as `<name>: RA=<ra> Dec=<dec>` (ICRS).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = cstr_to_str(&self.object.star.starname).unwrap_or("?");
        write!(
            f,
            "{name}: RA={ra} Dec={dec}",
            ra = self.ra(),
            dec = self.dec()
        )
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn make_raw_cat_entry(
    name: &str,
    ra: TimeAngle,
    dec: Angle,
    pm_ra: f64,
    pm_dec: f64,
    parallax_mas: f64,
    radial_v_km_per_s: f64,
) -> Result<cat_entry> {
    let bytes = name.as_bytes();
    if bytes.contains(&0) || bytes.len() >= 50 {
        return Err(Error::Parse);
    }
    let mut name_buf = [0u8; 50];
    name_buf[..bytes.len()].copy_from_slice(bytes);
    let name_cs = CStr::from_bytes_with_nul(&name_buf[..=bytes.len()]).map_err(|_| Error::Parse)?;

    let mut entry = MaybeUninit::<cat_entry>::zeroed();
    // SAFETY: make_cat_entry initializes *entry on a zero return.
    let rc = unsafe {
        make_cat_entry(
            name_cs.as_ptr(),
            core::ptr::null(),
            0,
            ra.hours(),
            dec.deg(),
            pm_ra,
            pm_dec,
            parallax_mas,
            radial_v_km_per_s,
            entry.as_mut_ptr(),
        )
    };
    if rc != 0 {
        return Err(Error::ffi(rc));
    }
    Ok(unsafe { entry.assume_init() })
}

/// Convert a C-string-in-array (nul-terminated within the array, may also
/// reach the array boundary) into a Rust `&str`. Returns `None` if the
/// bytes aren't valid UTF-8.
fn cstr_to_str(arr: &[core::ffi::c_char]) -> Option<&str> {
    let bytes: &[u8] = unsafe { core::slice::from_raw_parts(arr.as_ptr().cast::<u8>(), arr.len()) };
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    core::str::from_utf8(&bytes[..end]).ok()
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;
    use crate::unit;

    #[test]
    fn icrs_round_trip() {
        // Vega: ICRS RA 18h36m56.34s, Dec +38°47′01.3″
        let ra = TimeAngle::from_hours(18.0 + 36.0 / 60.0 + 56.34 / 3600.0).unwrap();
        let dec = Angle::from_degrees(38.0 + 47.0 / 60.0 + 1.3 / 3600.0).unwrap();
        let vega = CatalogEntry::icrs("Vega", ra, dec).unwrap();
        assert_abs_diff_eq!(vega.ra(), ra, epsilon = unit::UAS);
        assert_abs_diff_eq!(vega.dec(), dec, epsilon = unit::UAS);
    }

    #[test]
    fn in_system_icrs_matches_icrs_constructor() {
        // ICRS → ICRS conversion should be nearly identical to the direct path.
        let ra = TimeAngle::from_hours(18.615_092).unwrap();
        let dec = Angle::from_degrees(38.783_692).unwrap();
        let direct = CatalogEntry::icrs("Vega", ra, dec).unwrap();
        let via_sys = CatalogEntry::in_system("Vega", ra, dec, CatalogSystem::Icrs).unwrap();
        assert_abs_diff_eq!(direct.ra(), via_sys.ra(), epsilon = unit::UAS);
        assert_abs_diff_eq!(direct.dec(), via_sys.dec(), epsilon = unit::UAS);
    }

    #[test]
    fn in_system_b1950_shifts_coordinates() {
        // Antares B1950 position from the doc example: RA 16h26m20.1918s Dec −26°19′23.138″.
        // After ICRS conversion the coordinates must differ (precession ~1 arcmin over 50 yr).
        let ra_b1950 = TimeAngle::from_hours(16.0 + 26.0 / 60.0 + 20.1918 / 3600.0).unwrap();
        let dec_b1950 = Angle::from_degrees(-26.0 - 19.0 / 60.0 - 23.138 / 3600.0).unwrap();
        let entry =
            CatalogEntry::in_system("Antares", ra_b1950, dec_b1950, CatalogSystem::B1950).unwrap();
        // ICRS RA should differ from B1950 RA by ~1 min of time or more.
        let delta_ra = (entry.ra().hours() - ra_b1950.hours()).abs();
        assert!(
            delta_ra > 1e-3,
            "expected precession shift in RA, got {delta_ra}"
        );
    }

    #[test]
    fn redshifted_icrs_constructs() {
        // 3C 273: ICRS RA 12.4851944h, Dec +2.0523883°, z=0.158339
        let ra = TimeAngle::from_hours(12.485_194_4).unwrap();
        let dec = Angle::from_degrees(2.052_388_3).unwrap();
        let q = CatalogEntry::redshifted_icrs("3c273", ra, dec, 0.158_339).unwrap();
        assert_abs_diff_eq!(q.ra(), ra, epsilon = unit::UAS);
        assert_abs_diff_eq!(q.dec(), dec, epsilon = unit::UAS);
    }

    #[test]
    fn with_ssb_velocity_does_not_error() {
        let ra = TimeAngle::from_hours(18.0).unwrap();
        let dec = Angle::from_degrees(38.0).unwrap();
        let rv = ScalarVelocity::from_km_per_s(-13.9).unwrap();
        let _ = CatalogEntry::icrs("Vega", ra, dec)
            .unwrap()
            .with_ssb_velocity(rv)
            .unwrap();
    }

    #[test]
    fn with_lsr_velocity_does_not_error() {
        let ra = TimeAngle::from_hours(18.0).unwrap();
        let dec = Angle::from_degrees(38.0).unwrap();
        let rv = ScalarVelocity::from_km_per_s(10.0).unwrap();
        // J2000 epoch
        let _ = CatalogEntry::icrs("Star", ra, dec)
            .unwrap()
            .with_lsr_velocity(rv, 2_451_545.0)
            .unwrap();
    }

    #[test]
    fn with_redshift_does_not_error() {
        let ra = TimeAngle::from_hours(12.5).unwrap();
        let dec = Angle::from_degrees(2.0).unwrap();
        let _ = CatalogEntry::icrs("Quasar", ra, dec)
            .unwrap()
            .with_redshift(0.158)
            .unwrap();
    }

    #[test]
    fn with_distance_does_not_error() {
        let ra = TimeAngle::from_hours(18.0).unwrap();
        let dec = Angle::from_degrees(38.0).unwrap();
        let d = Coordinate::from_pc(237.0).unwrap(); // Vega ~8 pc, 237 pc is far but valid
        let _ = CatalogEntry::icrs("Vega", ra, dec)
            .unwrap()
            .with_distance(d)
            .unwrap();
    }

    #[test]
    fn name_with_interior_nul_is_rejected() {
        let ra = TimeAngle::from_hours(0.0).unwrap();
        let dec = Angle::from_degrees(0.0).unwrap();
        assert!(matches!(
            CatalogEntry::icrs("bad\0name", ra, dec),
            Err(Error::Parse)
        ));
    }

    #[test]
    fn name_too_long_is_rejected() {
        let ra = TimeAngle::from_hours(0.0).unwrap();
        let dec = Angle::from_degrees(0.0).unwrap();
        let long_name = "x".repeat(60);
        assert!(matches!(
            CatalogEntry::icrs(&long_name, ra, dec),
            Err(Error::Parse)
        ));
    }

    #[test]
    fn with_proper_motion_rejects_non_finite() {
        let ra = TimeAngle::from_hours(0.0).unwrap();
        let dec = Angle::from_degrees(0.0).unwrap();
        let entry = CatalogEntry::icrs("Star", ra, dec).unwrap();
        assert!(matches!(
            entry.with_proper_motion_mas_per_yr(f64::NAN, 0.0),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn display_contains_name_and_coords() {
        let ra = TimeAngle::from_hours(18.0).unwrap();
        let dec = Angle::from_degrees(38.0).unwrap();
        let entry = CatalogEntry::icrs("Vega", ra, dec).unwrap();
        let s = format!("{entry}");
        assert!(s.contains("Vega"), "got: {s}");
        assert!(s.contains("RA="), "got: {s}");
        assert!(s.contains("Dec="), "got: {s}");
    }

    #[test]
    fn debug_format_shows_name() {
        let ra = TimeAngle::from_hours(18.0).unwrap();
        let dec = Angle::from_degrees(38.0).unwrap();
        let entry = CatalogEntry::icrs("Vega", ra, dec).unwrap();
        let s = format!("{entry:?}");
        assert!(s.contains("Vega"), "got: {s}");
    }

    #[test]
    fn builder_attaches_optional_fields() {
        let ra = TimeAngle::from_hours(18.0).unwrap();
        let dec = Angle::from_degrees(38.0).unwrap();
        let parallax = Angle::from_mas(130.23).unwrap();
        let rv = ScalarVelocity::from_km_per_s(-13.5).unwrap();
        let entry = CatalogEntry::icrs("Vega", ra, dec)
            .unwrap()
            .with_proper_motion_mas_per_yr(200.94, 286.23)
            .unwrap()
            .with_parallax(parallax)
            .unwrap()
            .with_radial_velocity(rv)
            .unwrap();
        // Round-trip RA/Dec survived the rebuilds.
        assert_abs_diff_eq!(entry.ra(), ra, epsilon = unit::UAS);
        assert_abs_diff_eq!(entry.dec(), dec, epsilon = unit::UAS);
    }
}
