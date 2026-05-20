//! Observing frame: an observer × instant × Earth-orientation snapshot.
//!
//! A [`Frame`] bundles everything SuperNOVAS needs to compute apparent
//! positions: where the observer is, when, and (optionally) Earth's polar
//! motion at that instant. Sky positions are then evaluated against the
//! frame.

use core::mem::MaybeUninit;

use supernovas_sys::{
    novas_accuracy::{NOVAS_FULL_ACCURACY, NOVAS_REDUCED_ACCURACY},
    novas_app_to_hor, novas_frame, novas_make_frame,
    novas_reference_system::NOVAS_CIRS,
    novas_sky_pos, sky_pos,
};

use crate::{
    Angle, CatalogEntry, Horizontal, Observer, Time,
    error::{Error, Result},
};

/// Calculation accuracy. SuperNOVAS distinguishes a full-precision path
/// (sub-microarcsecond, computationally heavier) from a reduced path
/// (~milliarcsecond, faster).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accuracy {
    /// Full precision — microarcsecond-level, suitable for radio/sub-mm
    /// observatories and high-precision astrometry.
    Full,
    /// Reduced precision — milliarcsecond-level, faster.
    Reduced,
}

impl Accuracy {
    fn to_sys(self) -> supernovas_sys::novas_accuracy {
        match self {
            Accuracy::Full => NOVAS_FULL_ACCURACY,
            Accuracy::Reduced => NOVAS_REDUCED_ACCURACY,
        }
    }
}

/// An observing frame: observer + time (+ optional polar motion).
///
/// Construct via [`Self::new`] for the zero-polar-motion fast path, or
/// [`Self::with_polar_motion`] when you have IERS polar offsets.
#[derive(Debug, Clone, Copy)]
pub struct Frame(novas_frame);

impl Frame {
    /// Construct a frame with zero polar motion. Suitable for ~arcsecond
    /// accuracy or coarser.
    pub fn new(accuracy: Accuracy, observer: &Observer, time: &Time) -> Result<Self> {
        Self::with_polar_motion_mas(accuracy, observer, time, 0.0, 0.0)
    }

    /// Construct with explicit Earth-orientation polar offsets (typically
    /// from the IERS Bulletin A).
    pub fn with_polar_motion(
        accuracy: Accuracy,
        observer: &Observer,
        time: &Time,
        xp: Angle,
        yp: Angle,
    ) -> Result<Self> {
        Self::with_polar_motion_mas(accuracy, observer, time, xp.mas(), yp.mas())
    }

    fn with_polar_motion_mas(
        accuracy: Accuracy,
        observer: &Observer,
        time: &Time,
        xp_mas: f64,
        yp_mas: f64,
    ) -> Result<Self> {
        if !xp_mas.is_finite() || !yp_mas.is_finite() {
            return Err(Error::NotFinite);
        }
        let obs = observer.as_novas_observer()?;
        let mut frame = MaybeUninit::<novas_frame>::zeroed();
        // SAFETY: novas_make_frame fully initializes `*frame` on a zero
        // return, which we check before assuming initialization.
        let rc = unsafe {
            novas_make_frame(
                accuracy.to_sys(),
                &obs,
                time.as_timespec(),
                xp_mas,
                yp_mas,
                frame.as_mut_ptr(),
            )
        };
        if rc != 0 {
            return Err(Error::Parse("novas_make_frame failed".into()));
        }
        Ok(Frame(unsafe { frame.assume_init() }))
    }

    /// The accuracy mode this frame was built with.
    pub fn accuracy(&self) -> Accuracy {
        match self.0.accuracy {
            NOVAS_FULL_ACCURACY => Accuracy::Full,
            _ => Accuracy::Reduced,
        }
    }

    /// Compute the apparent horizontal (azimuth, elevation) of a catalog
    /// source as seen from this frame's observer at this frame's time.
    ///
    /// This pipeline currently applies **no refraction correction** — the
    /// result is the *astrometric* horizontal direction. Refraction-aware
    /// variants land alongside the full `Apparent` / `Refraction` layer.
    pub fn observe(&self, source: &CatalogEntry) -> Result<Horizontal> {
        use core::mem::MaybeUninit;

        let mut sky = MaybeUninit::<sky_pos>::zeroed();
        // SAFETY: novas_sky_pos initializes *sky on a zero return.
        let rc =
            unsafe { novas_sky_pos(source.as_object(), &self.0, NOVAS_CIRS, sky.as_mut_ptr()) };
        if rc != 0 {
            return Err(Error::Parse("novas_sky_pos failed".into()));
        }
        let sky = unsafe { sky.assume_init() };

        let mut az_deg: f64 = 0.0;
        let mut el_deg: f64 = 0.0;
        // SAFETY: novas_app_to_hor writes the two output doubles on zero
        // return. `None` for the refraction-model callback disables
        // refraction entirely.
        let rc = unsafe {
            novas_app_to_hor(
                &self.0,
                NOVAS_CIRS,
                sky.ra,
                sky.dec,
                None,
                &mut az_deg as *mut f64,
                &mut el_deg as *mut f64,
            )
        };
        if rc != 0 {
            return Err(Error::Parse("novas_app_to_hor failed".into()));
        }
        Horizontal::from_degrees(az_deg, el_deg)
    }
}

#[cfg(test)]
mod tests {
    use supernovas_sys::novas_timescale::NOVAS_TT;

    use super::*;

    fn j2000() -> Time {
        Time::from_jd(NOVAS_TT, 2_451_545.0, 32, 0.0).unwrap()
    }

    #[test]
    fn build_geocentric_frame() {
        let obs = Observer::Geocenter;
        let t = j2000();
        let f = Frame::new(Accuracy::Reduced, &obs, &t).unwrap();
        assert_eq!(f.accuracy(), Accuracy::Reduced);
    }

    #[test]
    fn build_geodetic_frame() {
        let obs = Observer::geodetic(34.0, -118.0, 100.0).unwrap();
        let t = j2000();
        let f = Frame::new(Accuracy::Reduced, &obs, &t).unwrap();
        assert_eq!(f.accuracy(), Accuracy::Reduced);
    }

    // Full accuracy needs a high-precision ephemeris provider configured
    // (via novas_use_calceph or equivalent). Not exercised in the unit
    // tests; covered in higher-level integration tests once that wiring is
    // in.

    #[test]
    fn with_polar_motion_is_finite_only() {
        let obs = Observer::Geocenter;
        let t = j2000();
        // Construct an Angle from mas via from_mas which validates finite.
        let xp = Angle::from_mas(120.5).unwrap();
        let yp = Angle::from_mas(-85.3).unwrap();
        let _ = Frame::with_polar_motion(Accuracy::Reduced, &obs, &t, xp, yp).unwrap();
    }

    /// Polaris sits ~0.74° from the true celestial pole, so from any
    /// northern-hemisphere site at geographic latitude L, its elevation is
    /// `L ± 0.74°`. This is the classic "Polaris altitude = your latitude"
    /// trick — and a tight end-to-end smoke test for the whole ICRS → az/el
    /// pipeline (Frame::new + sky_pos + app_to_hor).
    #[test]
    fn polaris_elevation_matches_observer_latitude() {
        let lat_deg = 34.0;

        // Approximate ICRS J2000 position of α UMi (Polaris).
        let polaris = crate::CatalogEntry::icrs(
            "Polaris",
            crate::TimeAngle::from_hours(2.530_301_5).unwrap(),
            Angle::from_degrees(89.264_109).unwrap(),
        )
        .unwrap();
        // Geodetic observer; longitude is irrelevant for Polaris.
        let obs = Observer::geodetic(lat_deg, 0.0, 0.0).unwrap();
        // Arbitrary recent epoch (2025-01-01 12:00 UTC, JD 2460676.5 UTC).
        let t = Time::from_utc_jd(2_460_676.5, 37, 0.0).unwrap();
        let frame = Frame::new(Accuracy::Reduced, &obs, &t).unwrap();

        let horizontal = frame.observe(&polaris).unwrap();
        let el = horizontal.elevation().deg();
        assert!(
            (el - lat_deg).abs() < 1.0,
            "Polaris elevation {el} should be within 1° of latitude {lat_deg}"
        );
    }
}
