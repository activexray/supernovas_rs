pub(crate) mod sealed {
    pub trait Sealed {}
}

/// A sky source that can be observed via [`crate::Frame`].
///
/// Implemented by all concrete source types:
/// - [`CatalogEntry`] — ICRS sidereal source (fixed star, quasar, …)
/// - [`Planet`] — major solar-system body via the installed planet provider
/// - [`EphemObject`] — arbitrary body by name/NAIF ID from the installed
///   ephemeris provider
/// - [`OrbitalObject`] — Keplerian orbital-elements source (no external
///   provider required)
///
/// The trait is sealed; you cannot implement it outside this crate.
pub trait Source: sealed::Sealed {
    #[doc(hidden)]
    fn as_object(&self) -> &supernovas_ffi::object;

    /// Compute the apparent place of this source in the given [`crate::Frame`]
    /// and [`crate::ReferenceSystem`].
    fn apparent_in(
        &self,
        frame: &crate::Frame,
        system: crate::ReferenceSystem,
    ) -> crate::error::Result<crate::Apparent> {
        crate::apparent::apparent_of_source_in(self, frame, system)
    }
}

mod catalog_entry;
mod ephem_object;
mod orbital;
mod planet;

pub use catalog_entry::{CatalogEntry, CatalogSystem};
pub use ephem_object::EphemObject;
pub use orbital::{OrbitalElements, OrbitalObject};
pub use planet::{Planet, SolarBody};
