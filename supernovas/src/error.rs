//! Error type for the safe wrapper.

/// Errors produced by safe-wrapper constructors and parsers.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A finite real value was required, but the input was NaN or infinite.
    #[error("value was not finite")]
    NotFinite,

    /// A string or name could not be parsed into the requested type, **or**
    /// an FFI call returned a non-zero status.
    ///
    /// # Cleanup wanted
    ///
    /// This variant is currently overloaded: most call sites that hit a C
    /// failure code return `Error::Parse` for lack of a better fit. That
    /// makes the name misleading. A future refactor should split this into:
    ///
    /// - `Error::Parse` — genuine string/name parse failures
    /// - `Error::Ffi(&'static str)` (or similar) — FFI call returned non-zero,
    ///   with the function name attached for diagnosability
    ///
    /// Keeping the unit-variant shape for now so the type remains `Copy`;
    /// the split lands when we audit all the `return Err(Error::Parse)`
    /// sites in one go.
    #[error("parse or FFI error")]
    Parse,

    /// Loading or installing a planetary ephemeris failed.
    ///
    /// Triggered by ephemeris file-open errors, unsupported formats, or
    /// failed SuperNOVAS provider-registration calls. Reachable via the
    /// `calceph` feature, the `anise` feature, or a custom `PlanetProvider`
    /// whose process-global `OnceLock` is already occupied.
    #[error("ephemeris error")]
    Ephemeris,
}

pub type Result<T> = core::result::Result<T, Error>;
