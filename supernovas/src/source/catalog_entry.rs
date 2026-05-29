//! Catalog (sidereal) source: a fixed direction on the sky in ICRS, with
//! optional proper motion, parallax, and radial velocity.

use core::{ffi::CStr, fmt, mem::MaybeUninit};

use supernovas_ffi::{cat_entry, make_cat_entry, make_cat_object, object};

use crate::{
    Angle, ScalarVelocity, TimeAngle,
    error::{Error, Result},
};

/// A catalog (sidereal) sky source.
///
/// Stored ICRS RA / Dec plus, optionally, proper motion, parallax, and
/// radial velocity. Constructed via [`Self::icrs`] for the simple
/// fixed-position case; refine via the `with_*` builder methods to add
/// proper motion, parallax, or radial velocity.
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
    /// Manual `Debug` impl — the underlying C `object` contains an `orbit`
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
        Self::new(name, ra, dec, 0.0, 0.0, 0.0, 0.0)
    }

    /// Builder: attach proper motion in RA and Dec (mas/yr).
    pub fn with_proper_motion_mas_per_yr(self, pm_ra: f64, pm_dec: f64) -> Result<Self> {
        if !pm_ra.is_finite() || !pm_dec.is_finite() {
            return Err(Error::NotFinite);
        }
        let star = self.object.star;
        Self::new(
            cstr_to_str(&star.starname).unwrap_or(""),
            TimeAngle::from_hours(star.ra)?,
            Angle::from_degrees(star.dec)?,
            pm_ra,
            pm_dec,
            star.parallax,
            star.radialvelocity,
        )
    }

    /// Builder: attach trigonometric parallax.
    pub fn with_parallax(self, parallax: Angle) -> Result<Self> {
        let star = self.object.star;
        Self::new(
            cstr_to_str(&star.starname).unwrap_or(""),
            TimeAngle::from_hours(star.ra)?,
            Angle::from_degrees(star.dec)?,
            star.promora,
            star.promodec,
            parallax.mas(),
            star.radialvelocity,
        )
    }

    /// Builder: attach radial velocity (positive = receding).
    pub fn with_radial_velocity(self, rv: ScalarVelocity) -> Result<Self> {
        let star = self.object.star;
        Self::new(
            cstr_to_str(&star.starname).unwrap_or(""),
            TimeAngle::from_hours(star.ra)?,
            Angle::from_degrees(star.dec)?,
            star.promora,
            star.promodec,
            star.parallax,
            rv.km_per_s(),
        )
    }

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

    fn new(
        name: &str,
        ra: TimeAngle,
        dec: Angle,
        pm_ra: f64,
        pm_dec: f64,
        parallax_mas: f64,
        radial_v_km_per_s: f64,
    ) -> Result<Self> {
        let bytes = name.as_bytes();
        if bytes.contains(&0) || bytes.len() >= 50 {
            return Err(Error::Parse);
        }
        let mut name_buf = [0u8; 50];
        name_buf[..bytes.len()].copy_from_slice(bytes);
        let name_cs =
            CStr::from_bytes_with_nul(&name_buf[..=bytes.len()]).map_err(|_| Error::Parse)?;

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
        let entry = unsafe { entry.assume_init() };

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
