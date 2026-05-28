use std::{
    ffi::c_char,
    os::raw::{c_int, c_long},
    panic::AssertUnwindSafe,
    path::Path,
    sync::OnceLock,
};

use ::anise::{
    almanac::Almanac,
    astro::Aberration,
    frames::Frame as AniseFrame,
    prelude::{Duration, Epoch},
};
use supernovas_ffi as sys;

use super::EphemerisProvider;
use crate::error::{Error, Result};

// NOVAS_AU_KM = 1e-3 * NOVAS_AU (novas.h). NOVAS_AU is the IAU 2012 nominal
// value of the astronomical unit: 1.495978707×10¹¹ m (exact by definition).
// Bindgen cannot evaluate the computed #define, so we replicate it here.
const KM_PER_AU: f64 = 1.495_978_707e8;
const SEC_PER_DAY: f64 = sys::NOVAS_DAY;

/// Process-global almanac. Populated by [`Backend::install`]; read by the
/// `extern "C"` callbacks registered with SuperNOVAS.
static ALMANAC: OnceLock<Almanac> = OnceLock::new();

/// An ANISE-backed planetary ephemeris.
///
/// Load a JPL DE-series SPK file and install it as the process-global
/// SuperNOVAS planet provider via [`EphemerisProvider::install`] or the
/// [`super::Ephemeris`] wrapper.
///
/// # Example
///
/// ```no_run
/// use supernovas::{AniseEphemeris, Ephemeris};
///
/// // Single-backend shortcut:
/// Ephemeris::open("/path/to/de440s.bsp")?.install()?;
///
/// // Or name the backend explicitly (e.g. when both features are active):
/// Ephemeris::from_provider(AniseEphemeris::open("/path/to/de440s.bsp")?).install()?;
/// # Ok::<(), supernovas::Error>(())
/// ```
pub struct AniseEphemeris {
    almanac: Almanac,
}

impl AniseEphemeris {
    /// Open an ANISE almanac from an SPK file (e.g. `de440s.bsp`).
    ///
    /// Returns [`Error::Ephemeris`] on any ANISE error (missing file,
    /// unsupported format, etc.).
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path_str = path.as_ref().to_str().ok_or(Error::Ephemeris)?;
        let almanac = Almanac::new(path_str).map_err(|_| Error::Ephemeris)?;
        Ok(AniseEphemeris { almanac })
    }

    /// Load an additional SPK / BPC / PCK / FK file into this almanac.
    ///
    /// Useful for stacking JPL DE files with extra body kernels before
    /// calling [`EphemerisProvider::install`].
    pub fn with(self, path: impl AsRef<Path>) -> Result<Self> {
        let path_str = path.as_ref().to_str().ok_or(Error::Ephemeris)?;
        let almanac = self.almanac.load(path_str).map_err(|_| Error::Ephemeris)?;
        Ok(AniseEphemeris { almanac })
    }
}

impl EphemerisProvider for AniseEphemeris {
    fn install(self) -> Result<()> {
        ALMANAC.set(self.almanac).map_err(|_| Error::Ephemeris)?;
        // SAFETY: The function pointers have C-compatible ABI and outlive the
        // process. planet_ephem_provider[_hp] are SuperNOVAS built-ins that
        // delegate to whatever ephem_provider is registered. All return 0 on
        // success.
        let rc1 = unsafe { sys::set_planet_provider(Some(sys::planet_ephem_provider)) };
        let rc2 = unsafe { sys::set_planet_provider_hp(Some(sys::planet_ephem_provider_hp)) };
        let rc3 = unsafe { sys::set_ephem_provider(Some(ephem_provider)) };
        if rc1 != 0 || rc2 != 0 || rc3 != 0 {
            return Err(Error::Ephemeris);
        }
        Ok(())
    }
}

/// Unified ephemeris callback for all solar-system bodies (`set_ephem_provider`).
///
/// Handles two call sites:
/// - **Planet calls** (forwarded by the built-in `planet_ephem_provider`):
///   `id` is a `novas_planet` discriminant (0–13); converted to the NAIF body
///   ID via `novas_to_naif_planet`.
/// - **Ephem-object calls** (`EphemObject` sources, e.g. spacecraft):
///   `id` is the NAIF body ID directly (e.g. −31 for Voyager 1).
///
/// In both cases the state is returned relative to the Solar System Barycenter
/// (NAIF 0) and `*origin` is set to `NOVAS_BARYCENTER`.
unsafe extern "C" fn ephem_provider(
    _name: *const c_char,
    id: c_long,
    jd_tdb_high: f64,
    jd_tdb_low: f64,
    origin: *mut sys::novas_origin,
    pos: *mut f64,
    vel: *mut f64,
) -> c_int {
    // Rust panics across FFI boundaries are UB. Catch and convert to a
    // non-zero return so SuperNOVAS surfaces it as a normal error.
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| -> i32 {
        let Some(almanac) = ALMANAC.get() else {
            crate::error::set_provider_error("ANISE almanac not installed");
            return 1;
        };

        // novas_planet discriminants are 0–13 (NOVAS_SSB … NOVAS_PLUTO_BARYCENTER).
        // Any id outside that range is a direct NAIF body ID.
        let naif = if (0..14).contains(&id) {
            // SAFETY: id is in [0, 14), which covers every valid novas_planet
            // discriminant (repr u32).
            let planet: sys::novas_planet = unsafe { std::mem::transmute(id as u32) };
            // novas_to_dexxx_planet returns the barycenter NAIF IDs that DE-series
            // SPK files (de440s.bsp, etc.) actually contain. novas_to_naif_planet
            // would return center IDs (e.g. 599 for Jupiter) which are absent from
            // short-form DE files, silently breaking gravitational deflection.
            let n = unsafe { sys::novas_to_dexxx_planet(planet) };
            if n < 0 {
                crate::error::set_provider_error(format!(
                    "novas_to_dexxx_planet returned -1 for novas_planet id {id}"
                ));
                return 2;
            }
            n as i32
        } else {
            id as i32 // direct NAIF ID: spacecraft, minor planets, etc.
        };

        let epoch = Epoch::from_jde_tdb(jd_tdb_high) + Duration::from_days(jd_tdb_low);
        let target = AniseFrame::from_ephem_j2000(naif);
        let observer = AniseFrame::from_ephem_j2000(0); // SSB = NAIF 0

        let state = match almanac.translate(target, observer, epoch, Aberration::NONE) {
            Ok(s) => s,
            Err(e) => {
                crate::error::set_provider_error(format!(
                    "ANISE could not translate NAIF {naif}: {e}"
                ));
                return 3;
            }
        };

        // SAFETY: NOVAS guarantees `origin`, `pos`, and `vel` are non-null.
        // Set `*origin` to indicate we computed relative to the Solar System
        // Barycenter. ANISE returns km / km·s⁻¹; NOVAS expects AU / AU·day⁻¹.
        unsafe {
            *origin = sys::novas_origin::NOVAS_BARYCENTER;
            *pos.add(0) = state.radius_km.x / KM_PER_AU;
            *pos.add(1) = state.radius_km.y / KM_PER_AU;
            *pos.add(2) = state.radius_km.z / KM_PER_AU;
            *vel.add(0) = state.velocity_km_s.x * SEC_PER_DAY / KM_PER_AU;
            *vel.add(1) = state.velocity_km_s.y * SEC_PER_DAY / KM_PER_AU;
            *vel.add(2) = state.velocity_km_s.z * SEC_PER_DAY / KM_PER_AU;
        }
        0
    }));
    result.unwrap_or(99)
}
