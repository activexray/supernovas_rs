//! Error type for the safe wrapper.

use alloc::string::String;

/// Errors produced by safe-wrapper constructors and parsers.
///
/// # Known limitations
///
/// FFI call failures are currently reported as `Parse(message)` rather than a
/// dedicated variant. A proper `FfiError(String)` variant will be added before
/// the API stabilises; callers should not match on `Parse` to detect FFI
/// failures.
// FIXME: add FfiError(String) variant; update all Error::Parse("… failed") call
//        sites in frame.rs, observer.rs, source/catalog_entry.rs to use it.
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
