//! Error type for the safe wrapper.

use alloc::string::String;

/// Errors produced by safe-wrapper constructors and parsers.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A finite real value was required, but the input was NaN or infinite.
    #[error("value was not finite")]
    NotFinite,

    /// A string could not be parsed into the requested type.
    #[error("could not parse {0:?}")]
    Parse(String),
}

pub type Result<T> = core::result::Result<T, Error>;
