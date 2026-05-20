//! Error type for the safe wrapper.

/// Errors produced by safe-wrapper constructors and parsers.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A finite real value was required, but the input was NaN or infinite.
    #[error("value was not finite")]
    NotFinite,

    /// A string or name could not be parsed into the requested type, or an FFI
    /// call returned a failure code.
    #[error("parse error")]
    Parse,
}

pub type Result<T> = core::result::Result<T, Error>;
