//! Error type for the safe wrapper.

/// Errors produced by safe-wrapper constructors and parsers.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A finite real value was required, but the input was NaN or infinite.
    #[error("value was not finite")]
    NotFinite,

    /// A string or name could not be parsed into the requested type.
    #[error("parse error")]
    Parse,

    /// A SuperNOVAS or supporting C library call returned a non-zero status.
    #[error("FFI call returned an error")]
    Ffi,

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

pub type Result<T> = core::result::Result<T, Error>;
