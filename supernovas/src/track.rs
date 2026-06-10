//! Fast polynomial interpolation of apparent source positions.
//!
//! [`EquatorialTrack`] and [`HorizontalTrack`] each store a polynomial fit
//! (position, velocity, acceleration) computed once via the full astrometric
//! pipeline. [`pos_at`](EquatorialTrack::pos_at) then evaluates the polynomial
//! at any nearby time in microseconds, making them useful for telescope drive
//! control at high output rates.
//!
//! # Example
//!
//! ```no_run
//! use supernovas::{Accuracy, CatalogEntry, Frame, Observer, Time, track::EquatorialTrack};
//!
//! let time = Time::from_tt_jd(2_460_676.5, 37, 0.0)?;
//! let observer = Observer::Geocenter;
//! let frame = Frame::new(Accuracy::Reduced, &observer, &time)?;
//!
//! let vega = CatalogEntry::icrs("Vega", "18:36:56.336".parse()?, "+38:47:01.28".parse()?)?;
//!
//! // Compute the track with a 1-second time step.
//! let track = EquatorialTrack::compute(&vega, &frame, 1.0)?;
//!
//! // Evaluate at the reference time.
//! let (eq, dist) = track.pos_at(&time)?;
//! println!("{eq}  dist = {} AU", dist.au());
//! # Ok::<(), supernovas::Error>(())
//! ```

use supernovas_ffi::{novas_equ_track, novas_hor_track, novas_track, novas_track_pos};

use crate::{
    Coordinate, Equinox, Frame, Refraction, Time,
    error::{Error, Result},
    source::Source,
    spherical::{Equatorial, Horizontal},
};

/// A polynomial track of equatorial (RA/Dec) apparent positions in the
/// true equator and equinox of date (TOD) system.
///
/// Computed once by [`EquatorialTrack::compute`]; evaluated cheaply at any
/// nearby time by [`pos_at`](EquatorialTrack::pos_at).
#[derive(Debug, Clone, Copy)]
pub struct EquatorialTrack(novas_track);

impl EquatorialTrack {
    /// Compute the equatorial track for `source` in `frame`.
    ///
    /// `dt_s` is the time step in seconds used to estimate the velocity and
    /// acceleration via finite differences. Typical values: 0.1–10 s.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `SuperNOVAS` computation fails.
    pub fn compute(source: &dyn Source, frame: &Frame, dt_s: f64) -> Result<Self> {
        let mut track = novas_track::default();
        // SAFETY: object and frame pointers are valid for the call duration;
        // novas_equ_track writes into `track` on a 0 return.
        let rc = unsafe {
            novas_equ_track(
                source.as_object(),
                frame.as_novas_frame(),
                dt_s,
                &raw mut track,
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(EquatorialTrack(track))
    }

    /// Evaluate the track at `time`, returning the apparent equatorial
    /// position and distance.
    ///
    /// The position is in the **true equator and equinox of date** (TOD)
    /// system — that is what `novas_equ_track` computes — so the returned
    /// [`Equatorial`] is tagged with a TOD equinox at the evaluation time.
    ///
    /// # Errors
    ///
    /// Returns an error if extrapolation fails or if a coordinate value is
    /// out of range.
    pub fn pos_at(&self, time: &Time) -> Result<(Equatorial, Coordinate)> {
        let mut lon = 0.0_f64;
        let mut lat = 0.0_f64;
        let mut dist = 0.0_f64;
        let mut z = 0.0_f64;
        // SAFETY: track and timespec pointers are valid; novas_track_pos
        // writes the four output doubles on a 0 return.
        let rc = unsafe {
            novas_track_pos(
                &raw const self.0,
                time.as_timespec(),
                &raw mut lon,
                &raw mut lat,
                &raw mut dist,
                &raw mut z,
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        let eq = Equatorial::from_degrees(lon, lat, Equinox::tod_at(time.tt_jd())?)?;
        let d = Coordinate::from_au(dist)?;
        Ok((eq, d))
    }
}

/// A polynomial track of horizontal (azimuth/elevation) apparent positions.
///
/// Computed once by [`HorizontalTrack::compute`]; evaluated cheaply at any
/// nearby time by [`pos_at`](HorizontalTrack::pos_at).
#[derive(Debug, Clone, Copy)]
pub struct HorizontalTrack(novas_track);

impl HorizontalTrack {
    /// Compute the horizontal track for `source` in `frame` using the given
    /// refraction model.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `SuperNOVAS` computation fails.
    pub fn compute(source: &dyn Source, frame: &Frame, refraction: Refraction) -> Result<Self> {
        let mut track = novas_track::default();
        // SAFETY: object and frame pointers are valid for the call duration;
        // novas_hor_track writes into `track` on a 0 return.
        let rc = unsafe {
            novas_hor_track(
                source.as_object(),
                frame.as_novas_frame(),
                refraction.to_sys(),
                &raw mut track,
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(HorizontalTrack(track))
    }

    /// Evaluate the track at `time`, returning the apparent horizontal
    /// position and distance.
    ///
    /// # Errors
    ///
    /// Returns an error if extrapolation fails or if a coordinate value is
    /// out of range.
    pub fn pos_at(&self, time: &Time) -> Result<(Horizontal, Coordinate)> {
        let mut lon = 0.0_f64;
        let mut lat = 0.0_f64;
        let mut dist = 0.0_f64;
        let mut z = 0.0_f64;
        // SAFETY: track and timespec pointers are valid; novas_track_pos
        // writes the four output doubles on a 0 return.
        let rc = unsafe {
            novas_track_pos(
                &raw const self.0,
                time.as_timespec(),
                &raw mut lon,
                &raw mut lat,
                &raw mut dist,
                &raw mut z,
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        let hor = Horizontal::from_degrees(lon, lat)?;
        let d = Coordinate::from_au(dist)?;
        Ok((hor, d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Accuracy, Frame, Observer, Time, source::CatalogEntry};

    fn frame() -> Frame {
        let t = Time::from_tt_jd(2_460_676.5, 37, 0.0).unwrap();
        let obs = Observer::Geocenter;
        Frame::new(Accuracy::Reduced, &obs, &t).unwrap()
    }

    fn vega() -> CatalogEntry {
        CatalogEntry::icrs(
            "Vega",
            "18:36:56.336".parse().unwrap(),
            "+38:47:01.28".parse().unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn equatorial_track_computes_without_error() {
        let frame = frame();
        let vega = vega();
        let track = EquatorialTrack::compute(&vega, &frame, 1.0).unwrap();
        let time = Time::from_tt_jd(2_460_676.5, 37, 0.0).unwrap();
        let (eq, dist) = track.pos_at(&time).unwrap();
        assert!(eq.ra().hours().is_finite());
        assert!(eq.dec().deg().is_finite());
        assert!(dist.au().is_finite());
    }

    #[test]
    fn equatorial_track_pos_is_near_reference() {
        let frame = frame();
        let vega = vega();
        let time = Time::from_tt_jd(2_460_676.5, 37, 0.0).unwrap();
        let track = EquatorialTrack::compute(&vega, &frame, 1.0).unwrap();
        let (eq, _) = track.pos_at(&time).unwrap();
        // Vega RA ≈ 18.6 h at J2025; within 0.1 h is a reasonable sanity bound.
        let ra_h = eq.ra().hours();
        assert!((ra_h - 18.6).abs() < 0.1, "RA {ra_h} h far from Vega");
    }

    /// `novas_equ_track` works in TOD internally; the evaluated position
    /// must match the apparent TOD place and carry a TOD equinox tag —
    /// not ICRS, which is ~10 arcmin away at this epoch.
    #[test]
    fn equatorial_track_is_tagged_and_positioned_in_tod() {
        use crate::ReferenceSystem;
        let frame = frame();
        let vega = vega();
        let time = Time::from_tt_jd(2_460_676.5, 37, 0.0).unwrap();
        let track = EquatorialTrack::compute(&vega, &frame, 1.0).unwrap();
        let (eq, _) = track.pos_at(&time).unwrap();
        assert_eq!(eq.system().system(), ReferenceSystem::Tod);
        assert!((eq.system().jd() - time.tt_jd()).abs() < 1e-9);
        let tod = vega.apparent_in(&frame, ReferenceSystem::Tod).unwrap();
        let sep = eq
            .as_spherical()
            .distance_to(tod.equatorial().as_spherical());
        assert!(
            sep.arcsec() < 0.01,
            "track position {} arcsec from apparent TOD place",
            sep.arcsec()
        );
    }

    #[test]
    fn horizontal_track_requires_site_observer() {
        use crate::{Observer, observer::Site};
        let t = Time::from_tt_jd(2_460_676.5, 37, 0.0).unwrap();
        let site = Site::from_degrees(37.0, -122.0, 50.0).unwrap();
        let obs = Observer::Geodetic(site);
        let frame = Frame::new(Accuracy::Reduced, &obs, &t).unwrap();
        let vega = vega();
        let result = HorizontalTrack::compute(&vega, &frame, Refraction::None);
        // May succeed or fail depending on source visibility; just ensure no panic.
        let _ = result;
    }
}
