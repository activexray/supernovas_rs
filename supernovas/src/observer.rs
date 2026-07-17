//! Observers: who or where the observation is taken from.
//!
//! Currently exposes ground-based ([`Observer::Geodetic`]) and geocentric
//! ([`Observer::Geocenter`]) observers. Airborne and near-Earth (satellite)
//! variants are not yet implemented.

use core::{fmt, mem::MaybeUninit};

use supernovas_ffi::{make_observer_at_geocenter, make_observer_at_site, observer};

use crate::error::{Error, Result};

mod site;
mod weather;

pub use site::Site;
pub use weather::Weather;

/// Where the observation is taken from.
///
/// Construct via the variant directly or via [`Observer::geodetic`]. Pass
/// to [`crate::Frame`] to combine with a [`crate::Time`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Observer {
    /// A fixed ground-based location, with optional local weather.
    Geodetic(Site),

    /// At the geocenter (the center of the Earth). Useful for theoretical
    /// calculations and as a coarse stand-in when site precision doesn't
    /// matter.
    Geocenter,
}

impl Observer {
    /// Shortcut: build a [`Observer::Geodetic`] from latitude / longitude
    /// in degrees and height in meters.
    pub fn geodetic(latitude_deg: f64, longitude_deg: f64, height_m: f64) -> Result<Self> {
        Ok(Observer::Geodetic(Site::from_degrees(
            latitude_deg,
            longitude_deg,
            height_m,
        )?))
    }

    /// Build the C-side `observer` representation, ready to pass to
    /// `novas_make_frame`. Returns an error if the C side reports a
    /// problem (e.g. malformed coordinates).
    pub(crate) fn as_novas_observer(&self) -> Result<observer> {
        let mut obs = MaybeUninit::<observer>::zeroed();
        let rc = match self {
            Observer::Geodetic(site) => {
                // make_observer_at_site copies the whole on_surface struct -
                // including humidity, which make_observer_on_surface has no
                // parameter for and would silently replace with a location
                // default.
                let on_surf = site.as_on_surface()?;
                // SAFETY: make_observer_at_site fully initializes *obs on a
                // zero return; the site struct is valid for the call.
                unsafe { make_observer_at_site(&raw const on_surf, obs.as_mut_ptr()) }
            }
            Observer::Geocenter => unsafe { make_observer_at_geocenter(obs.as_mut_ptr()) },
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        // SAFETY: rc == 0 guarantees obs has been initialized.
        Ok(unsafe { obs.assume_init() })
    }
}

impl fmt::Display for Observer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Observer::Geodetic(site) => write!(f, "Geodetic({site})"),
            Observer::Geocenter => f.write_str("Geocenter"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geodetic_shortcut_round_trips() {
        let obs = Observer::geodetic(34.0, -118.0, 100.0).unwrap();
        match obs {
            Observer::Geodetic(site) => {
                assert!((site.latitude().deg() - 34.0).abs() < 1e-12);
                assert!((site.longitude().deg() - -118.0).abs() < 1e-12);
                assert!((site.height().m() - 100.0).abs() < 1e-12);
            }
            _ => panic!("expected Geodetic"),
        }
    }

    #[test]
    fn geodetic_builds_a_novas_observer() {
        let obs = Observer::geodetic(34.0, -118.0, 100.0).unwrap();
        let raw = obs.as_novas_observer().unwrap();
        assert!((raw.on_surf.latitude - 34.0).abs() < 1e-12);
        assert!((raw.on_surf.longitude - -118.0).abs() < 1e-12);
    }

    #[test]
    fn geocenter_builds_a_novas_observer() {
        let raw = Observer::Geocenter.as_novas_observer().unwrap();
        // The on_surf and near_earth substructures should be zeroed; what
        // matters is just that the FFI returned 0.
        let _ = raw;
    }

    #[test]
    fn display_geodetic() {
        let obs = Observer::geodetic(34.0, -118.0, 100.0).unwrap();
        let s = format!("{obs}");
        assert!(s.contains("Geodetic"), "got: {s}");
    }

    #[test]
    fn display_geocenter() {
        let s = format!("{}", Observer::Geocenter);
        assert_eq!(s, "Geocenter");
    }
}
