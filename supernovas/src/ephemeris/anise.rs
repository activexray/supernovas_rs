use std::{os::raw::c_short, panic::AssertUnwindSafe, path::Path, sync::OnceLock};

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
        // SAFETY: set_planet_provider[_hp] stores the function pointer in a
        // SuperNOVAS global. Our callbacks have C-compatible ABI and outlive
        // the process. Both return 0 on success.
        let rc1 = unsafe { sys::set_planet_provider(Some(planet_provider)) };
        let rc2 = unsafe { sys::set_planet_provider_hp(Some(planet_provider_hp)) };
        if rc1 != 0 || rc2 != 0 {
            return Err(Error::Ephemeris);
        }
        Ok(())
    }
}

/// Map a SuperNOVAS planet enum to a NAIF SPK body ID using the
/// SuperNOVAS-provided `novas_to_naif_planet` function.
///
/// Returns `None` for bodies SuperNOVAS considers out-of-range (EMB,
/// PLUTO_BARYCENTER, etc.), matching the upstream planet-provider contract.
fn naif_id(body: sys::novas_planet) -> Option<i32> {
    // SAFETY: novas_to_naif_planet is a pure mapping function with no
    // side effects. It returns -1 for unsupported bodies.
    let id = unsafe { sys::novas_to_naif_planet(body) };
    if id < 0 { None } else { Some(id as i32) }
}

/// Map a SuperNOVAS origin enum to the corresponding NAIF body ID.
///
/// NAIF 0 = Solar System Barycenter, NAIF 10 = Sun; these are fixed by
/// the NAIF standard and match SuperNOVAS's `NOVAS_BARYCENTER` /
/// `NOVAS_HELIOCENTER` semantics.
fn origin_naif_id(origin: sys::novas_origin) -> i32 {
    use sys::novas_origin::*;
    match origin {
        NOVAS_BARYCENTER => 0,
        NOVAS_HELIOCENTER => 10,
    }
}

/// Single-precision JD planet provider (`set_planet_provider`).
unsafe extern "C" fn planet_provider(
    jd_tdb: f64,
    body: sys::novas_planet,
    origin: sys::novas_origin,
    position: *mut f64,
    velocity: *mut f64,
) -> c_short {
    provider_impl(jd_tdb, 0.0, body, origin, position, velocity)
}

/// High-precision JD planet provider (`set_planet_provider_hp`).
/// NOVAS passes `jd_tdb` as a 2-element array (high word + low word).
unsafe extern "C" fn planet_provider_hp(
    jd_tdb: *const f64,
    body: sys::novas_planet,
    origin: sys::novas_origin,
    position: *mut f64,
    velocity: *mut f64,
) -> c_short {
    // SAFETY: NOVAS guarantees a `double[2]` at jd_tdb.
    let (high, low) = unsafe { (*jd_tdb, *jd_tdb.add(1)) };
    provider_impl(high, low, body, origin, position, velocity)
}

fn provider_impl(
    jd_high: f64,
    jd_low: f64,
    body: sys::novas_planet,
    origin: sys::novas_origin,
    position: *mut f64,
    velocity: *mut f64,
) -> c_short {
    // Rust panics across FFI boundaries are UB. Catch and convert to a
    // non-zero return so SuperNOVAS surfaces it as a normal error.
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let Some(almanac) = ALMANAC.get() else {
            return 1;
        };
        let Some(target_id) = naif_id(body) else {
            return 2;
        };
        let observer_id = origin_naif_id(origin);

        // hifitime's i128-ns Epoch + Duration preserves the split JD
        // better than a naive f64 add.
        let epoch = Epoch::from_jde_tdb(jd_high) + Duration::from_days(jd_low);
        let target = AniseFrame::from_ephem_j2000(target_id);
        let observer = AniseFrame::from_ephem_j2000(observer_id);

        let state = match almanac.translate(target, observer, epoch, Aberration::NONE) {
            Ok(s) => s,
            Err(_) => return 3,
        };

        // ANISE returns km / km·s⁻¹ in ICRF; NOVAS expects AU / AU·day⁻¹.
        // SAFETY: NOVAS guarantees `position` and `velocity` each point to
        // a 3-element `double` array.
        unsafe {
            *position.add(0) = state.radius_km.x / KM_PER_AU;
            *position.add(1) = state.radius_km.y / KM_PER_AU;
            *position.add(2) = state.radius_km.z / KM_PER_AU;
            *velocity.add(0) = state.velocity_km_s.x * SEC_PER_DAY / KM_PER_AU;
            *velocity.add(1) = state.velocity_km_s.y * SEC_PER_DAY / KM_PER_AU;
            *velocity.add(2) = state.velocity_km_s.z * SEC_PER_DAY / KM_PER_AU;
        }
        0
    }));
    result.unwrap_or(99)
}
