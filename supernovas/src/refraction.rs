//! Atmospheric refraction models.
//!
//! Refraction lifts the apparent elevation of sources above their geometric
//! position. `SuperNOVAS` exposes a few built-in models; this module wraps
//! them as a simple enum, converting at the FFI boundary into the
//! C-side function-pointer typedef.

use supernovas_ffi::{
    RefractionModel, novas_optical_refraction, novas_radio_refraction, novas_standard_refraction,
};

/// Atmospheric refraction model used when converting between apparent
/// equatorial and horizontal coordinates.
///
/// All models are most accurate well above the horizon (≳ 5° elevation).
/// Near the horizon every model becomes unreliable; treat refraction-
/// corrected elevations there as indicative only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Refraction {
    /// No refraction correction; the result is the geometric direction
    /// (what you'd see in a vacuum).
    #[default]
    None,

    /// `SuperNOVAS`' standard-atmosphere model. Independent of any per-site
    /// weather supplied through the [`crate::Frame`]'s observer — useful as
    /// a quick approximation when you don't have local measurements.
    Standard,

    /// Optical-wavelength refraction using the per-site temperature and
    /// pressure stored in the observer's [`crate::Weather`]. The standard
    /// choice for visible-band telescopes. Weather fields left unset fall
    /// back to `SuperNOVAS`'s mean annual estimate for the site location.
    Optical,

    /// Radio-wavelength refraction (Berman & Rockwell 1976) using the
    /// per-site weather; includes water-vapor effects so the humidity
    /// field of [`crate::Weather`] matters here. Weather fields left unset
    /// fall back to `SuperNOVAS`'s mean annual estimate for the site
    /// location.
    Radio,
}

impl Refraction {
    /// Convert to the C-side `RefractionModel` function-pointer typedef.
    ///
    /// `None` maps to a null callback (no refraction); the other variants
    /// map to the corresponding built-in `SuperNOVAS` function. The fn-item
    /// → fn-pointer coercion happens automatically in the `Some(_)` arms
    /// because the target type (`RefractionModel`) pins the signature.
    pub(crate) fn to_sys(self) -> RefractionModel {
        match self {
            Refraction::None => None,
            Refraction::Standard => Some(novas_standard_refraction),
            Refraction::Optical => Some(novas_optical_refraction),
            Refraction::Radio => Some(novas_radio_refraction),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_variants_produce_a_model() {
        // None → null; others → Some(fn ptr).
        assert!(Refraction::None.to_sys().is_none());
        assert!(Refraction::Standard.to_sys().is_some());
        assert!(Refraction::Optical.to_sys().is_some());
        assert!(Refraction::Radio.to_sys().is_some());
    }

    #[test]
    fn default_is_none() {
        assert_eq!(Refraction::default(), Refraction::None);
    }
}
