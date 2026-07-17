//! CALCEPH backend for [`super::EphemerisProvider`].

use core::{mem, ptr::NonNull};
use std::{ffi::CString, path::Path};

use supernovas_ffi::{calceph_close, calceph_open, novas_use_calceph, t_calcephbin};

use super::EphemerisProvider;
use crate::error::{Error, Result};

/// A CALCEPH-backed planetary ephemeris.
///
/// Load a JPL DE-series SPK file and install it as the process-global
/// `SuperNOVAS` planet provider via [`EphemerisProvider::install`] or the
/// [`super::Ephemeris`] wrapper.
///
/// # Example
///
/// ```no_run
/// use supernovas::{CalcephEphemeris, Ephemeris};
///
/// // Single-backend shortcut:
/// Ephemeris::open("/path/to/de440s.bsp")?.install()?;
///
/// // Or name the backend explicitly (e.g. when both features are active):
/// Ephemeris::from_provider(CalcephEphemeris::open("/path/to/de440s.bsp")?).install()?;
/// # Ok::<(), supernovas::Error>(())
/// ```
pub struct CalcephEphemeris {
    handle: NonNull<t_calcephbin>,
}

impl CalcephEphemeris {
    /// Open a CALCEPH ephemeris file (e.g. `de440s.bsp`).
    ///
    /// Returns [`Error::Ephemeris`] if the file is missing, unreadable,
    /// or not a valid CALCEPH-compatible ephemeris.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let path_str = path.to_str().ok_or(Error::Ephemeris)?;
        let cstr = CString::new(path_str).map_err(|_| Error::Ephemeris)?;
        // SAFETY: `cstr` is a valid nul-terminated C string for the duration
        // of the call. `calceph_open` returns NULL on failure.
        let raw = unsafe { calceph_open(cstr.as_ptr()) };
        let handle = NonNull::new(raw).ok_or(Error::Ephemeris)?;
        Ok(CalcephEphemeris { handle })
    }
}

impl EphemerisProvider for CalcephEphemeris {
    fn install(self) -> Result<()> {
        // SAFETY: self.handle was returned by calceph_open and is non-null.
        let rc = unsafe { novas_use_calceph(self.handle.as_ptr()) };
        if rc != 0 {
            return Err(Error::Ephemeris);
        }
        // SuperNOVAS now owns the handle for the rest of the process.
        // Skip Drop so we don't close it out from under SuperNOVAS.
        mem::forget(self);
        Ok(())
    }
}

impl Drop for CalcephEphemeris {
    fn drop(&mut self) {
        // Only runs if `install()` was not called - we still own the handle.
        // SAFETY: self.handle came from calceph_open and was not yet closed.
        unsafe { calceph_close(self.handle.as_ptr()) }
    }
}

// SAFETY: calceph_open returns a handle that is safe to move between threads
// as long as it is not used concurrently. CalcephEphemeris is consumed by
// install() (via mem::forget), so it is never used after crossing a thread
// boundary; the Send impl is therefore sound.
unsafe impl Send for CalcephEphemeris {}
