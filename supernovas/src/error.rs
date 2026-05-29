//! Error type for the safe wrapper.

use core::fmt::{self, Display, Write as _};

/// Maximum length, in bytes, of a captured FFI error description.
///
/// Messages longer than this are truncated at a UTF-8 character boundary. The
/// value comfortably holds every `SuperNOVAS` `novas_error()` description (the
/// longest reach ~110 bytes once `%p`/`%d` placeholders are expanded).
pub const FFI_MSG_CAP: usize = 128;

/// An inline, heap-free error description stored directly inside [`Error`].
///
/// Backed by a fixed-capacity [`heapless::String`] so an [`Error`] carries its
/// message without allocating — identical under `std` and `no_std`.
pub type FfiMessage = heapless::String<FFI_MSG_CAP>;

/// Render a `Display` value into a fresh [`FfiMessage`], truncating at a UTF-8
/// boundary if it would exceed [`FFI_MSG_CAP`]. Never allocates, never fails.
fn build_message(msg: impl Display) -> FfiMessage {
    let mut buf = FfiMessage::new();
    // `Truncating` swallows overflow, so this write cannot fail.
    let _ = write!(Truncating(&mut buf), "{msg}");
    buf
}

/// A `fmt::Write` adapter that appends only what fits into the backing
/// [`FfiMessage`] (cut to a UTF-8 boundary) and silently drops the rest, so a
/// long message truncates instead of failing the whole format operation.
struct Truncating<'a>(&'a mut FfiMessage);

impl fmt::Write for Truncating<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let remaining = FFI_MSG_CAP - self.0.len();
        if remaining == 0 {
            return Ok(());
        }
        let mut end = s.len().min(remaining);
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        // `&s[..end]` is at most `remaining` bytes, so this push always fits.
        let _ = self.0.push_str(&s[..end]);
        Ok(())
    }
}

// Thread-local storage for the most recent human-readable error context.
// Populated by two sources:
//   1. Rust ephemeris callbacks (ANISE, CALCEPH): call set_provider_error()
//      before returning a non-zero code to bridge the FFI boundary.
//   2. SuperNOVAS C library: the capture handler in debug.rs intercepts
//      novas_error() descriptions when debug mode is non-Off.
// Both sources are drained by Error::ffi() and take_provider_error().
//
// Both sources require `std` (the ANISE/CALCEPH providers and the debug
// capture handler are all std-gated), so under `no_std` there is nothing to
// capture and the slot does not exist.
#[cfg(feature = "std")]
std::thread_local! {
    static PROVIDER_ERROR: core::cell::RefCell<Option<FfiMessage>> =
        const { core::cell::RefCell::new(None) };
}

/// Store a descriptive error message for the next [`Error::Ffi`].
///
/// Called from a Rust ephemeris provider callback before returning a non-zero
/// code, or from the `SuperNOVAS` C error capture handler. The message is copied
/// into a fixed-capacity inline buffer (truncated past [`FFI_MSG_CAP`] bytes),
/// so it never allocates; retrieve it via [`take_provider_error`].
///
/// Accepts any [`Display`] value — including [`format_args!`] output — so
/// callers can attach runtime context without building a `String`.
///
/// Effective only under the `std` feature. On `no_std` targets this is a no-op
/// (there are no capture sources there), matching where the message is read.
#[inline]
pub fn set_provider_error(msg: impl Display) {
    #[cfg(feature = "std")]
    PROVIDER_ERROR.with(|cell| *cell.borrow_mut() = Some(build_message(msg)));
    #[cfg(not(feature = "std"))]
    let _ = msg;
}

/// Retrieve and clear the most recent error description, if any.
///
/// Drained automatically by every [`Error::Ffi`] construction (via
/// `Error::ffi`), so calling this manually is only necessary when you want the
/// description separately from the error value.
///
/// Returns `None` if no context was recorded or it has already been taken;
/// always `None` under `no_std`.
#[inline]
#[must_use]
pub fn take_provider_error() -> Option<FfiMessage> {
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

    /// A value was finite but outside the valid range for the quantity it
    /// represents (e.g. a geodetic latitude or declination beyond ±90°).
    /// The payload names the offending quantity.
    #[error("{0} is out of its valid range")]
    OutOfRange(&'static str),

    /// A `SuperNOVAS` or supporting C library call returned a non-zero status.
    ///
    /// `code` is the raw C return code; `message` is a best-effort, inline
    /// (heap-free) human description. Under `std` it is the text captured from
    /// the C library (enable [`crate::enable_debug_mode`] first) or from a Rust
    /// ephemeris callback; otherwise a generic fallback. Same shape under
    /// `std` and `no_std`.
    #[error("{message} (code {code})")]
    Ffi {
        /// The raw non-zero return code reported by the C library.
        code: i32,
        /// Best-effort human-readable description (see [`FfiMessage`]).
        message: FfiMessage,
    },

    /// The requested operation is not supported for the given coordinate
    /// system (e.g. converting ITRS coordinates to ecliptic).
    #[error("unsupported coordinate system for this operation")]
    UnsupportedSystem,

    /// Loading or installing a planetary ephemeris failed.
    ///
    /// Triggered by ephemeris file-open errors, unsupported formats, or
    /// failed `SuperNOVAS` provider-registration calls. Reachable via the
    /// `calceph` feature, the `anise` feature, or a custom `PlanetProvider`
    /// whose process-global `OnceLock` is already occupied.
    #[error("ephemeris error")]
    Ephemeris,
}

impl Error {
    /// Build an [`Error::Ffi`] from a C return code.
    ///
    /// Drains any description captured via [`set_provider_error`] (from a Rust
    /// ephemeris callback or the `SuperNOVAS` C error handler); falls back to a
    /// generic description when nothing was captured (always, under `no_std`).
    #[inline]
    pub(crate) fn ffi(code: impl Into<i32>) -> Self {
        let code = code.into();
        let message = take_provider_error()
            .unwrap_or_else(|| build_message(format_args!("FFI call returned an error")));
        Error::Ffi { code, message }
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
    fn ffi_exposes_code_and_message() {
        set_provider_error("boom");
        let err = Error::ffi(42);
        match err {
            Error::Ffi { code, message } => {
                assert_eq!(code, 42);
                assert_eq!(message.as_str(), "boom");
            }
            other => panic!("expected Ffi, got {other:?}"),
        }
    }

    #[test]
    fn set_provider_error_accepts_format_args() {
        // No allocation: runtime context is rendered straight into the buffer.
        let id = 499;
        set_provider_error(format_args!("NAIF {id} not covered"));
        assert_eq!(
            take_provider_error().as_deref(),
            Some("NAIF 499 not covered")
        );
    }

    #[test]
    fn long_message_truncates_without_panicking() {
        let long = "x".repeat(FFI_MSG_CAP * 2);
        set_provider_error(&long);
        let msg = take_provider_error().unwrap();
        assert_eq!(msg.len(), FFI_MSG_CAP, "should fill exactly to capacity");
        assert!(msg.chars().all(|c| c == 'x'));
    }

    #[test]
    fn truncation_respects_utf8_boundaries() {
        // One ASCII byte makes the remaining capacity odd, so a following
        // stream of 2-byte 'é' chars must hit a point where the last char
        // can't fit and has to be dropped whole rather than split. The result
        // must stay valid UTF-8 (guaranteed: `as_str()` would otherwise be UB)
        // and stop one byte short of capacity.
        let s: String = core::iter::once('a')
            .chain(core::iter::repeat('é').take(FFI_MSG_CAP))
            .collect();
        set_provider_error(&s);
        let msg = take_provider_error().unwrap();
        assert_eq!(
            msg.len(),
            FFI_MSG_CAP - 1,
            "should stop one byte short, not split 'é'"
        );
        assert!(msg.starts_with('a'));
        assert!(msg.chars().skip(1).all(|c| c == 'é'));
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
