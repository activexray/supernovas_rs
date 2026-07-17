use std::{
    ffi::c_char,
    os::raw::{c_int, c_long, c_short},
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

/// NAIF ID of the Solar System Barycenter.
const NAIF_SSB: i32 = 0;
/// NAIF ID of the Sun.
const NAIF_SUN: i32 = 10;

/// Process-global almanac. Populated by [`EphemerisProvider::install`]; read
/// by the `extern "C"` callbacks registered with `SuperNOVAS`.
static ALMANAC: OnceLock<Almanac> = OnceLock::new();

/// An ANISE-backed planetary ephemeris.
///
/// Load a JPL DE-series SPK file and install it as the process-global
/// `SuperNOVAS` planet provider via [`EphemerisProvider::install`] or the
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
    /// Registers the ANISE callbacks with `SuperNOVAS`, mirroring the
    /// structure of the C `solsys-calceph` plugin:
    ///
    /// - `planet_provider` / `planet_provider_hp` handle major-planet
    ///   queries, which arrive with `novas_planet` IDs (0–13) and map them
    ///   to NAIF IDs internally.
    /// - `ephem_provider` handles [`crate::EphemObject`] queries, whose
    ///   `id` is always the NAIF ID the object was constructed with.
    ///
    /// Keeping the two interfaces separate (rather than funneling planet
    /// calls through the generic provider via the `planet_ephem_provider`
    /// built-ins) means the generic provider never has to guess whether a
    /// small integer is a `novas_planet` discriminant or a NAIF ID - e.g.
    /// NAIF 3 (Earth–Moon barycenter) resolves correctly instead of being
    /// remapped to Earth.
    fn install(self) -> Result<()> {
        ALMANAC.set(self.almanac).map_err(|_| Error::Ephemeris)?;
        // SAFETY: the callbacks have C-compatible ABI and live for the rest
        // of the process. All registration calls return 0 on success.
        let rc1 = unsafe { sys::set_planet_provider(Some(planet_provider)) };
        let rc2 = unsafe { sys::set_planet_provider_hp(Some(planet_provider_hp)) };
        let rc3 = unsafe { sys::set_ephem_provider(Some(ephem_provider)) };
        if rc1 != 0 || rc2 != 0 || rc3 != 0 {
            return Err(Error::Ephemeris);
        }
        Ok(())
    }
}

/// Translate `naif` relative to `center` at the split TDB Julian date and
/// write the state into the NOVAS output buffers (position in AU, velocity
/// in AU/day, both in the BCRS/ICRF J2000 frame). Either output pointer may
/// be NULL when the caller doesn't need that component, per the NOVAS
/// provider contract.
///
/// Returns `false` (with a provider-error message set) if ANISE cannot
/// produce the state.
fn write_state(
    almanac: &Almanac,
    naif: i32,
    center: i32,
    jd_tdb_high: f64,
    jd_tdb_low: f64,
    pos: *mut f64,
    vel: *mut f64,
) -> bool {
    let epoch = Epoch::from_jde_tdb(jd_tdb_high) + Duration::from_days(jd_tdb_low);
    let target = AniseFrame::from_ephem_j2000(naif);
    let observer = AniseFrame::from_ephem_j2000(center);

    let state = match almanac.translate(target, observer, epoch, Aberration::NONE) {
        Ok(s) => s,
        Err(e) => {
            crate::error::set_provider_error(format_args!(
                "ANISE could not translate NAIF {naif} relative to NAIF {center}: {e}"
            ));
            return false;
        }
    };

    // SAFETY: when non-NULL, NOVAS guarantees `pos` and `vel` each point to
    // a 3-element `double` array. ANISE returns km / km·s⁻¹; NOVAS expects
    // AU / AU·day⁻¹.
    unsafe {
        if !pos.is_null() {
            *pos.add(0) = state.radius_km.x / KM_PER_AU;
            *pos.add(1) = state.radius_km.y / KM_PER_AU;
            *pos.add(2) = state.radius_km.z / KM_PER_AU;
        }
        if !vel.is_null() {
            *vel.add(0) = state.velocity_km_s.x * SEC_PER_DAY / KM_PER_AU;
            *vel.add(1) = state.velocity_km_s.y * SEC_PER_DAY / KM_PER_AU;
            *vel.add(2) = state.velocity_km_s.z * SEC_PER_DAY / KM_PER_AU;
        }
    }
    true
}

/// Shared implementation of the regular and high-precision planet
/// providers. Return codes follow the `novas_planet_provider` convention:
/// 1 for an invalid body, 3 for an ephemeris-data error.
fn planet_state(
    jd_tdb_high: f64,
    jd_tdb_low: f64,
    body: sys::novas_planet,
    origin: sys::novas_origin,
    pos: *mut f64,
    vel: *mut f64,
) -> c_short {
    // Rust panics across FFI boundaries are UB. Catch and convert to a
    // non-zero return so SuperNOVAS surfaces it as a normal error.
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| -> c_short {
        let Some(almanac) = ALMANAC.get() else {
            crate::error::set_provider_error("ANISE almanac not installed");
            return 3;
        };

        // novas_to_dexxx_planet returns the NAIF IDs that DE-series SPK
        // files (de440s.bsp, etc.) actually contain: barycenters 1–9 for
        // the planets, 399 for Earth, 301 for the Moon, 10 for the Sun,
        // 0 for the SSB. novas_to_naif_planet would return planet-center
        // IDs (e.g. 599 for Jupiter) which are absent from short-form DE
        // files, silently breaking gravitational deflection.
        let naif = unsafe { sys::novas_to_dexxx_planet(body) };
        if naif < 0 {
            crate::error::set_provider_error(format_args!(
                "no NAIF mapping for NOVAS planet id {}",
                body as u32
            ));
            return 1;
        }

        let center = match origin {
            sys::novas_origin::NOVAS_BARYCENTER => NAIF_SSB,
            sys::novas_origin::NOVAS_HELIOCENTER => NAIF_SUN,
        };

        if write_state(
            almanac,
            naif as i32,
            center,
            jd_tdb_high,
            jd_tdb_low,
            pos,
            vel,
        ) {
            0
        } else {
            3
        }
    }));
    result.unwrap_or(99)
}

/// Major-planet provider (`set_planet_provider`): `body` is a
/// `novas_planet` discriminant; the state is returned relative to the
/// requested `origin` (SSB or heliocenter).
unsafe extern "C" fn planet_provider(
    jd_tdb: f64,
    body: sys::novas_planet,
    origin: sys::novas_origin,
    pos: *mut f64,
    vel: *mut f64,
) -> c_short {
    planet_state(jd_tdb, 0.0, body, origin, pos, vel)
}

/// High-precision major-planet provider (`set_planet_provider_hp`); same as
/// [`planet_provider`] but with a split TDB Julian date.
unsafe extern "C" fn planet_provider_hp(
    jd_tdb: *const f64,
    body: sys::novas_planet,
    origin: sys::novas_origin,
    pos: *mut f64,
    vel: *mut f64,
) -> c_short {
    // SAFETY: NOVAS guarantees a `double[2]` at jd_tdb.
    let (high, low) = unsafe { (*jd_tdb, *jd_tdb.add(1)) };
    planet_state(high, low, body, origin, pos, vel)
}

/// Generic ephemeris callback for [`crate::EphemObject`] sources
/// (`set_ephem_provider`).
///
/// `id` is the NAIF body ID the object was constructed with (e.g. −31 for
/// Voyager 1, 3 for the Earth–Moon barycenter). The NOVAS convention of
/// `id == -1` meaning "look the body up by name" is not supported by this
/// backend - ANISE SPK lookups are ID-based - so it returns an error with a
/// message saying as much. The state is always returned relative to the
/// Solar System Barycenter and `*origin` is set accordingly.
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
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| -> c_int {
        let Some(almanac) = ALMANAC.get() else {
            crate::error::set_provider_error("ANISE almanac not installed");
            return 1;
        };

        if id == -1 {
            crate::error::set_provider_error(
                "the ANISE backend does not support name-based lookup (id = -1); \
                 construct the EphemObject with an explicit NAIF ID",
            );
            return 1;
        }

        // Always report relative to the Solar System Barycenter.
        if !origin.is_null() {
            // SAFETY: non-NULL origin points to a valid novas_origin.
            unsafe { *origin = sys::novas_origin::NOVAS_BARYCENTER };
        }

        if write_state(
            almanac,
            id as i32,
            NAIF_SSB,
            jd_tdb_high,
            jd_tdb_low,
            pos,
            vel,
        ) {
            0
        } else {
            3
        }
    }));
    result.unwrap_or(99)
}
