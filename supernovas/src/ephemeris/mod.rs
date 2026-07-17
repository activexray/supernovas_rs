//! Planetary ephemeris providers for high-precision astrometry.
//!
//! Two built-in backends are available as optional features:
//!
//! - `calceph` - wraps the C CALCEPH library; fast, mature, requires a
//!   system `libcalceph`. Exposes `CalcephEphemeris`.
//! - `anise` - pure-Rust ANISE/SPK reader ([nyx-space/anise]); no C
//!   dependency beyond `SuperNOVAS` itself. Exposes [`AniseEphemeris`].
//!
//! Both features may be enabled simultaneously; each backend is an
//! independent type implementing [`EphemerisProvider`].
//!
//! When exactly one feature is enabled, [`Ephemeris::open`] is available as
//! a convenience that delegates to that backend. When both are enabled,
//! construct via [`Ephemeris::from_provider`] with the backend of your
//! choice.
//!
//! ## Custom backends
//!
//! Implement [`PlanetProvider`] on your data type to add a custom ephemeris
//! source without writing any `unsafe` code. The framework handles all C
//! callback registration and panic catching for you; a blanket impl
//! automatically gives your type [`EphemerisProvider`].
//!
//! ## Backend agreement
//!
//! CALCEPH and ANISE both evaluate the same Chebyshev polynomials from the
//! SPK binary, but as independent C and Rust implementations they accumulate
//! floating-point rounding differently. For a typical stellar pointing
//! (de440s.bsp, ~50–100 polynomial terms per body), the two backends agree to
//! within **~2 µas** in azimuth and **~0.05 µas** in elevation - well inside
//! `SuperNOVAS`'s sub-µas full-accuracy guarantee and negligible for mm-wave
//! pointing. The residual divergence is irreducible rounding noise, not a bug.
//!
//! ```no_run
//! use supernovas::Ephemeris;
//!
//! Ephemeris::open("/path/to/de440s.bsp")
//!     .expect("ephemeris file present")
//!     .install()
//!     .expect("install succeeded");
//! ```

#[cfg(feature = "anise")]
pub mod anise;
#[cfg(feature = "calceph")]
pub mod calceph;

// Path is only needed for the Ephemeris::open conveniences, available when
// exactly one backend feature is enabled.
#[cfg(any(
    all(feature = "calceph", not(feature = "anise")),
    all(feature = "anise", not(feature = "calceph")),
))]
use std::path::Path;
use std::{os::raw::c_short, panic::AssertUnwindSafe, sync::OnceLock};

#[cfg(feature = "anise")]
pub use anise::AniseEphemeris;
#[cfg(feature = "calceph")]
pub use calceph::CalcephEphemeris;
use supernovas_ffi as sys;

use crate::error::{Error, Result};

// ── PlanetProvider ────────────────────────────────────────────────────────────

/// The interface `SuperNOVAS` requires from a planetary ephemeris backend.
///
/// Implement this on your data type to bridge any ephemeris source to
/// `SuperNOVAS` without writing `unsafe` C callbacks or managing process-global
/// state yourself. The framework handles all of that, and a blanket impl
/// automatically gives your type [`EphemerisProvider`].
///
/// # Units and frame
///
/// - **Position**: AU in the BCRS J2000.0 (ICRF) frame.
/// - **Velocity**: AU/day in the same frame.
///
/// # Arguments
///
/// - `body`: which solar-system body `SuperNOVAS` needs.
///   Map to a NAIF integer ID with
///   `unsafe { supernovas::sys::novas_to_naif_planet(body) }`.
/// - `origin`: [`sys::novas_origin::NOVAS_BARYCENTER`] (Solar System
///   Barycenter, NAIF 0) or [`sys::novas_origin::NOVAS_HELIOCENTER`]
///   (Sun, NAIF 10).
/// - `jd_high` + `jd_low`: split TDB Julian date. Adding them as `f64` is
///   fine for most purposes; use both parts for sub-µs precision.
///
/// # Return value
///
/// Return `None` if `body` is not covered by your ephemeris - `SuperNOVAS` will
/// surface this as an ephemeris error. Return `Some(([x, y, z], [vx, vy, vz]))`.
///
/// # Panics
///
/// Panics inside `state` are caught at the C boundary and converted to a
/// non-zero error code so `SuperNOVAS` can surface them cleanly. Nevertheless,
/// prefer returning `None` over panicking for unsupported bodies.
///
/// # Example
///
/// ```no_run
/// use supernovas::{sys, Ephemeris, PlanetProvider};
///
/// struct LinearEphemeris;
///
/// impl PlanetProvider for LinearEphemeris {
///     fn state(
///         &self,
///         body: sys::novas_planet,
///         origin: sys::novas_origin,
///         jd_high: f64,
///         jd_low: f64,
///     ) -> Option<([f64; 3], [f64; 3])> {
///         let _naif = unsafe { sys::novas_to_naif_planet(body) };
///         // look up position and velocity from your ephemeris source...
///         Some(([1.0, 0.0, 0.0], [0.0, 0.017, 0.0]))
///     }
/// }
///
/// Ephemeris::from_provider(LinearEphemeris).install().unwrap();
/// ```
pub trait PlanetProvider: Send + Sync + 'static {
    /// State vector of `body` relative to `origin` at `jd_high + jd_low` TDB.
    fn state(
        &self,
        body: sys::novas_planet,
        origin: sys::novas_origin,
        jd_high: f64,
        jd_low: f64,
    ) -> Option<([f64; 3], [f64; 3])>;
}

// ── Generic dispatch (used by the PlanetProvider blanket impl) ────────────────

static GENERIC_PROVIDER: OnceLock<Box<dyn PlanetProvider>> = OnceLock::new();

unsafe extern "C" fn generic_planet_provider(
    jd_tdb: f64,
    body: sys::novas_planet,
    origin: sys::novas_origin,
    pos: *mut f64,
    vel: *mut f64,
) -> c_short {
    generic_dispatch(jd_tdb, 0.0, body, origin, pos, vel)
}

unsafe extern "C" fn generic_planet_provider_hp(
    jd_tdb: *const f64,
    body: sys::novas_planet,
    origin: sys::novas_origin,
    pos: *mut f64,
    vel: *mut f64,
) -> c_short {
    // SAFETY: NOVAS guarantees a `double[2]` at jd_tdb.
    let (high, low) = unsafe { (*jd_tdb, *jd_tdb.add(1)) };
    generic_dispatch(high, low, body, origin, pos, vel)
}

fn generic_dispatch(
    jd_high: f64,
    jd_low: f64,
    body: sys::novas_planet,
    origin: sys::novas_origin,
    pos: *mut f64,
    vel: *mut f64,
) -> c_short {
    // Panics across FFI boundaries are UB; catch and convert to a non-zero
    // return code so SuperNOVAS can surface them cleanly.
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let Some(provider) = GENERIC_PROVIDER.get() else {
            return 1;
        };
        let Some(([px, py, pz], [vx, vy, vz])) = provider.state(body, origin, jd_high, jd_low)
        else {
            return 2;
        };
        // SAFETY: NOVAS guarantees `pos` and `vel` each point to a
        // 3-element `double` array.
        unsafe {
            *pos.add(0) = px;
            *pos.add(1) = py;
            *pos.add(2) = pz;
            *vel.add(0) = vx;
            *vel.add(1) = vy;
            *vel.add(2) = vz;
        }
        0
    }));
    result.unwrap_or(99)
}

/// Any [`PlanetProvider`] is automatically an [`EphemerisProvider`].
///
/// The blanket impl stores the provider in a process-global [`OnceLock`] and
/// registers the C dispatch callbacks with `SuperNOVAS`. At most one
/// `PlanetProvider` can be installed per process via this path; a second call
/// returns [`Error::Ephemeris`].
///
/// Built-in backends (`CalcephEphemeris`, [`AniseEphemeris`]) implement
/// [`EphemerisProvider`] directly and do not go through this dispatch layer.
impl<T: PlanetProvider> EphemerisProvider for T {
    fn install(self) -> Result<()> {
        GENERIC_PROVIDER
            .set(Box::new(self))
            .map_err(|_| Error::Ephemeris)?;
        // SAFETY: the callbacks have C-compatible ABI and live for the rest
        // of the process. Both return 0 on success.
        let rc1 = unsafe { sys::set_planet_provider(Some(generic_planet_provider)) };
        let rc2 = unsafe { sys::set_planet_provider_hp(Some(generic_planet_provider_hp)) };
        if rc1 != 0 || rc2 != 0 {
            return Err(Error::Ephemeris);
        }
        Ok(())
    }
}

// ── EphemerisProvider ─────────────────────────────────────────────────────────

/// A planetary ephemeris backend that can be installed as the process-global
/// `SuperNOVAS` planet provider.
///
/// Most users should implement [`PlanetProvider`] instead - it exposes the
/// natural Rust interface (return a state vector) and the blanket impl here
/// handles all C callback wiring automatically.
///
/// Implement `EphemerisProvider` directly only when you need a non-standard
/// registration path, as CALCEPH does via `novas_use_calceph`.
pub trait EphemerisProvider: Send + 'static {
    /// Install this provider as the process-global `SuperNOVAS` planet source.
    ///
    /// Must call [`sys::set_planet_provider`] and/or
    /// [`sys::set_planet_provider_hp`] (for split-JD high-accuracy queries) to
    /// register C-compatible callbacks. The callbacks receive a TDB Julian date,
    /// a [`sys::novas_planet`] body enum, and a [`sys::novas_origin`] enum, and
    /// must write position (AU) and velocity (AU/day) in BCRS J2000.0 into the
    /// provided output buffers, returning 0 on success.
    ///
    /// Any state the callbacks need must live in a process-global static (e.g.
    /// [`OnceLock`]), since C function pointers cannot
    /// close over Rust values. See [`AniseEphemeris`] for a complete reference
    /// implementation, or implement [`PlanetProvider`] to avoid all of this.
    fn install(self) -> Result<()>;
}

// ── Ephemeris wrapper ─────────────────────────────────────────────────────────

/// A loaded planetary ephemeris, ready to install as the process-global
/// `SuperNOVAS` planet provider.
///
/// Construct via [`Ephemeris::open`] (when exactly one backend feature is
/// enabled) or [`Ephemeris::from_provider`] (with any [`EphemerisProvider`],
/// including the named backend types when both features are active).
pub struct Ephemeris {
    install_fn: Box<dyn FnOnce() -> Result<()> + Send>,
}

impl Ephemeris {
    /// Wrap any [`EphemerisProvider`] in an [`Ephemeris`].
    ///
    /// Use this when you need to name the backend explicitly - either because
    /// both `calceph` and `anise` features are enabled, or to use a custom
    /// provider.
    pub fn from_provider(provider: impl EphemerisProvider) -> Self {
        Ephemeris {
            install_fn: Box::new(move || provider.install()),
        }
    }

    /// Open an ephemeris file using the active backend.
    ///
    /// Available when exactly one of `calceph` or `anise` is enabled.
    /// When both features are active, use
    /// [`Ephemeris::from_provider`] with [`CalcephEphemeris`] or
    /// [`AniseEphemeris`] explicitly.
    ///
    /// Returns [`crate::Error::Ephemeris`] if the file is missing,
    /// unreadable, or not in a format the backend understands.
    #[cfg(all(feature = "calceph", not(feature = "anise")))]
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Ephemeris::from_provider(CalcephEphemeris::open(path)?))
    }

    /// Open an ephemeris file using the active backend.
    ///
    /// Available when exactly one of `calceph` or `anise` is enabled.
    /// When both features are active, use
    /// [`Ephemeris::from_provider`] with `CalcephEphemeris` or
    /// [`AniseEphemeris`] explicitly.
    ///
    /// Returns [`crate::Error::Ephemeris`] if the file is missing,
    /// unreadable, or not in a format the backend understands.
    #[cfg(all(feature = "anise", not(feature = "calceph")))]
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Ephemeris::from_provider(AniseEphemeris::open(path)?))
    }

    /// Install as the process-global `SuperNOVAS` planet provider.
    ///
    /// Call once at process start, before any
    /// [`crate::Frame::new`] with [`crate::Accuracy::Full`].
    ///
    /// *CALCEPH:* a second call replaces the previous handle, which is then
    /// leaked because `SuperNOVAS` may still reference it.
    ///
    /// *ANISE:* a second call returns [`crate::Error::Ephemeris`]; the
    /// almanac is stored in a `OnceLock`. Restart the process to switch
    /// SPK files.
    pub fn install(self) -> Result<()> {
        (self.install_fn)()
    }
}
