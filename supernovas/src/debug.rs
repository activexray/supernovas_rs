//! Control `SuperNOVAS`'s built-in error-reporting verbosity.
//!
//! When debug mode is [`DebugMode::On`] or [`DebugMode::Extra`], `SuperNOVAS`
//! routes every `novas_error()` / `novas_set_errno()` description through a
//! custom handler.  The default behavior is to write to `stderr`; this crate
//! replaces that with silent capture into the same thread-local slot used by
//! [`crate::take_provider_error`], so all error context - whether from a Rust
//! ephemeris callback or from deep inside the C library - is retrievable with
//! a single call.
//!
//! Note: trace lines (`@ func [=> code]`) are printed via `fprintf(stderr,…)`
//! directly in the C source and are not intercepted by the handler.  They will
//! still appear on stderr when debug mode is non-Off.

use supernovas_ffi::novas_debug_mode;

/// Verbosity level for `SuperNOVAS`'s built-in error reporter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugMode {
    /// Silent - no error output, no captures (default).
    Off,
    /// Capture error descriptions via [`crate::take_provider_error`]; trace
    /// lines still appear on `stderr`.
    On,
    /// Same as `On` but also captures otherwise-acceptable conditions.
    Extra,
}

impl DebugMode {
    fn to_sys(self) -> novas_debug_mode {
        match self {
            DebugMode::Off => novas_debug_mode::NOVAS_DEBUG_OFF,
            DebugMode::On => novas_debug_mode::NOVAS_DEBUG_ON,
            DebugMode::Extra => novas_debug_mode::NOVAS_DEBUG_EXTRA,
        }
    }

    fn from_sys(m: novas_debug_mode) -> Self {
        match m {
            novas_debug_mode::NOVAS_DEBUG_ON => DebugMode::On,
            novas_debug_mode::NOVAS_DEBUG_EXTRA => DebugMode::Extra,
            novas_debug_mode::NOVAS_DEBUG_OFF => DebugMode::Off,
        }
    }
}

/// Error handler installed via `novas_set_error_handler` when debug mode is
/// non-Off.  Captures the description part of every `novas_error()` call into
/// the thread-local slot returned by [`crate::take_provider_error`].
///
/// `SuperNOVAS` routes four kinds of lines through the handler:
///   1. `"\n  ERROR! %s: "` (error prefix) - starts with `'\n'`, skipped
///   2. The actual description string - captured
///   3. `" [=> %d]\n"` (code suffix) - starts with `" [=>"`, skipped
///   4. `"       @ %s [=> %d]\n"` (`novas_trace` lines) - starts with
///      `"       @"`, skipped
///
/// All four forms pass the raw format string as `fmt`; the handler cannot
/// expand `%s`/`%d` from the attached `va_list` in safe Rust.  Real error
/// descriptions (form 2) are plain strings without format specifiers, so
/// capturing `fmt` directly gives the correct text.
#[cfg(feature = "std")]
#[cfg(all(target_vendor = "apple", target_arch = "aarch64"))]
unsafe extern "C" fn capture_handler(
    fmt: *const ::std::os::raw::c_char,
    _args: supernovas_ffi::va_list,
) {
    capture_error_description(fmt);
}

#[cfg(feature = "std")]
#[cfg(not(all(target_vendor = "apple", target_arch = "aarch64")))]
unsafe extern "C" fn capture_handler(
    fmt: *const ::std::os::raw::c_char,
    _args: *mut supernovas_ffi::__va_list_tag,
) {
    capture_error_description(fmt);
}

#[cfg(feature = "std")]
fn capture_error_description(fmt: *const ::std::os::raw::c_char) {
    use std::ffi::CStr;
    // SAFETY: SuperNOVAS guarantees fmt is a valid, non-NULL C string.
    let Ok(s) = unsafe { CStr::from_ptr(fmt) }.to_str() else {
        return;
    };
    // Skip the bracket lines and novas_trace lines; capture only the description.
    if s.starts_with('\n') || s.starts_with(" [=>") || s.starts_with("       @") {
        return;
    }
    crate::error::set_provider_error(s);
}

/// Set the `SuperNOVAS` error-reporting verbosity.
///
/// - [`DebugMode::Off`] (default): silent; [`crate::take_provider_error`]
///   still returns messages from Rust ephemeris callbacks (e.g. ANISE), but
///   not from the C library itself.
/// - [`DebugMode::On`] / [`DebugMode::Extra`]: installs a capture handler so
///   that `novas_error()` descriptions are silently stored rather than written
///   to `stderr`.  After any [`crate::Error::Ffi`] failure, call
///   [`crate::take_provider_error`] to retrieve the description.  Trace lines
///   (`@ func [=> code]`) are still printed to `stderr` via `fprintf`.
///
/// # Example
///
/// ```no_run
/// use supernovas::{DebugMode, enable_debug_mode, take_provider_error};
///
/// enable_debug_mode(DebugMode::On);
/// // … reproduce the failing call …
/// if let Some(desc) = take_provider_error() {
///     eprintln!("SuperNOVAS said: {desc}");
/// }
/// enable_debug_mode(DebugMode::Off);
/// ```
pub fn enable_debug_mode(mode: DebugMode) {
    // SAFETY: novas_debug and novas_set_error_handler are process-global; the
    // docs say novas_set_error_handler is not thread-safe but we accept that
    // (same contract as the C API).
    unsafe {
        if mode == DebugMode::Off {
            supernovas_ffi::novas_debug(novas_debug_mode::NOVAS_DEBUG_OFF);
            // Restore the default stderr handler.
            #[cfg(feature = "std")]
            supernovas_ffi::novas_set_error_handler(None);
        } else {
            // Install our silent capture handler before enabling debug
            // mode so no messages leak to stderr.
            #[cfg(feature = "std")]
            supernovas_ffi::novas_set_error_handler(Some(capture_handler));
            supernovas_ffi::novas_debug(mode.to_sys());
        }
    }
}

/// Return the current `SuperNOVAS` error-reporting verbosity.
#[must_use]
pub fn get_debug_mode() -> DebugMode {
    // SAFETY: read-only access to a process-global flag.
    DebugMode::from_sys(unsafe { supernovas_ffi::novas_get_debug_mode() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_all_modes() {
        for mode in [DebugMode::Off, DebugMode::On, DebugMode::Extra] {
            enable_debug_mode(mode);
            assert_eq!(get_debug_mode(), mode);
        }
        // Leave debug off so other tests are not affected by stderr output.
        enable_debug_mode(DebugMode::Off);
    }

    #[test]
    fn debug_mode_eq_and_clone() {
        assert_eq!(DebugMode::On, DebugMode::On);
        assert_ne!(DebugMode::On, DebugMode::Extra);
        let _ = DebugMode::Off;
    }

    #[test]
    fn debug_mode_debug_format() {
        assert_eq!(format!("{:?}", DebugMode::On), "On");
        assert_eq!(format!("{:?}", DebugMode::Extra), "Extra");
        assert_eq!(format!("{:?}", DebugMode::Off), "Off");
    }
}
