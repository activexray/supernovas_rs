//! Error type for the safe wrapper.

// Thread-local storage for the most recent human-readable error context.
// Populated by two sources:
//   1. Rust ephemeris callbacks (ANISE, CALCEPH): call set_provider_error()
//      before returning a non-zero code to bridge the FFI boundary.
//   2. SuperNOVAS C library: the capture handler in debug.rs intercepts
//      novas_error() descriptions when debug mode is non-Off.
// Both sources are drained by Error::ffi() and take_provider_error().
#[cfg(feature = "std")]
std::thread_local! {
    static PROVIDER_ERROR: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

/// Store a descriptive error message.
///
/// Called either from a Rust ephemeris provider callback before returning a
/// non-zero code, or from the SuperNOVAS C error capture handler.  In either
/// case the message is retrievable via [`take_provider_error`].
///
/// Only available under the `std` feature. On `no_std` targets, this is a
/// no-op (the message is discarded).
#[inline]
pub fn set_provider_error(msg: impl Into<String>) {
    #[cfg(feature = "std")]
    PROVIDER_ERROR.with(|cell| *cell.borrow_mut() = Some(msg.into()));
    #[cfg(not(feature = "std"))]
    let _ = msg;
}

/// Retrieve and clear the most recent error description, if any.
///
/// Under `std` this is drained automatically by every [`Error::Ffi`]
/// construction (via `Error::ffi`), so calling this manually is only
/// necessary when you need the raw string separately from the error value.
///
/// Returns `None` if no context was recorded or if it has already been taken.
/// Only meaningful under the `std` feature; always returns `None` otherwise.
#[inline]
pub fn take_provider_error() -> Option<String> {
    #[cfg(feature = "std")]
    {
        PROVIDER_ERROR.with(|cell| cell.borrow_mut().take())
    }
    #[cfg(not(feature = "std"))]
    {
        None
    }
}

/// Errors produced by safe-wrapper constructors and parsers.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A finite real value was required, but the input was NaN or infinite.
    #[error("value was not finite")]
    NotFinite,

    /// A string or name could not be parsed into the requested type.
    #[error("parse error")]
    Parse,

    /// A SuperNOVAS or supporting C library call returned a non-zero status.
    ///
    /// Under `std`, the display text is the human-readable description
    /// captured from the C library or from a Rust ephemeris callback.
    /// Enable [`crate::enable_debug_mode`] before the failing call to ensure
    /// C-side descriptions are captured.
    ///
    /// Under `no_std`, the display text is `"FFI error (code N)"`.
    #[cfg(feature = "std")]
    #[error("{0}")]
    Ffi(String),

    /// A SuperNOVAS or supporting C library call returned a non-zero status.
    #[cfg(not(feature = "std"))]
    #[error("FFI error (code {0})")]
    Ffi(i32),

    /// The requested operation is not supported for the given coordinate
    /// system (e.g. converting ITRS coordinates to ecliptic).
    #[error("unsupported coordinate system for this operation")]
    UnsupportedSystem,

    /// Loading or installing a planetary ephemeris failed.
    ///
    /// Triggered by ephemeris file-open errors, unsupported formats, or
    /// failed SuperNOVAS provider-registration calls. Reachable via the
    /// `calceph` feature, the `anise` feature, or a custom `PlanetProvider`
    /// whose process-global `OnceLock` is already occupied.
    #[error("ephemeris error")]
    Ephemeris,
}

impl Error {
    /// Build an `Ffi` error.
    ///
    /// Under `std`, drains the thread-local error description captured from
    /// either a Rust ephemeris callback or the SuperNOVAS C error handler.
    /// Falls back to `"FFI error (code N)"` if nothing was captured.
    #[inline]
    pub(crate) fn ffi(code: impl Into<i32>) -> Self {
        #[cfg(feature = "std")]
        {
            let code = code.into();
            let desc = take_provider_error().unwrap_or_else(|| format!("FFI error (code {code})"));
            Error::Ffi(desc)
        }
        #[cfg(not(feature = "std"))]
        {
            Error::Ffi(code.into())
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_take_provider_error() {
        set_provider_error("something went wrong");
        let msg = take_provider_error();
        assert_eq!(msg.as_deref(), Some("something went wrong"));
        // Second take clears the slot.
        assert!(take_provider_error().is_none());
    }

    #[test]
    fn ffi_drains_captured_description() {
        set_provider_error("bad ephemeris call");
        let err = Error::ffi(3);
        assert!(format!("{err}").contains("bad ephemeris call"));
        // Slot is now empty — next ffi() falls back to numeric.
        let err2 = Error::ffi(7);
        assert!(format!("{err2}").contains("7"));
    }

    #[test]
    fn not_finite_display() {
        let s = format!("{}", Error::NotFinite);
        assert_eq!(s, "value was not finite");
    }

    #[test]
    fn parse_display() {
        let s = format!("{}", Error::Parse);
        assert!(!s.is_empty());
    }

    #[test]
    fn unsupported_system_display() {
        let s = format!("{}", Error::UnsupportedSystem);
        assert!(!s.is_empty());
    }

    #[test]
    fn ephemeris_display() {
        let s = format!("{}", Error::Ephemeris);
        assert!(!s.is_empty());
    }

    #[test]
    fn error_is_clone() {
        let e = Error::NotFinite;
        let _e2 = e.clone();
    }
}
