//! Astronomical sources: catalog stars, planets, ephemeris bodies, etc.
//!
//! Currently only [`CatalogEntry`] (sidereal sources) is implemented; other
//! source kinds (planets, orbital, ephemeris-driven) follow as their use
//! cases come up.

mod catalog_entry;

pub use catalog_entry::CatalogEntry;
