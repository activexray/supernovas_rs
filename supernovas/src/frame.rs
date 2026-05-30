//! Observing frame: an observer × instant × Earth-orientation snapshot.
//!
//! A [`Frame`] bundles everything `SuperNOVAS` needs to compute apparent
//! positions: where the observer is, when, and (optionally) Earth's polar
//! motion at that instant. Sky positions are then evaluated against the
//! frame.

use core::mem::MaybeUninit;

use supernovas_ffi::{
    novas_accuracy::{NOVAS_FULL_ACCURACY, NOVAS_REDUCED_ACCURACY},
    novas_frame, novas_make_frame,
};

use crate::{
    Angle, Horizontal, Observer, Time,
    apparent::ReferenceSystem,
    error::{Error, Result},
    source::Source,
};

/// Calculation accuracy. `SuperNOVAS` distinguishes a full-precision path
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
    pub(crate) fn to_sys(self) -> supernovas_ffi::novas_accuracy {
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

    /// Construct with polar motion automatically fetched from IERS.
    ///
    /// Passes `NAN` for both `xp` and `yp`, which tells `novas_make_frame` to
    /// query the IERS servers for interpolated polar offsets. Requires the
    /// `eop` feature (CURL support) and network access.
    ///
    /// Pass raw IERS Bulletin values if you supply them manually via
    /// [`with_polar_motion`](Self::with_polar_motion); do **not**
    /// pre-apply libration or ocean-tide corrections — `novas_make_frame`
    /// handles those internally for `Accuracy::Full` frames.
    #[cfg(feature = "eop")]
    pub fn with_auto_polar_motion(
        accuracy: Accuracy,
        observer: &Observer,
        time: &Time,
    ) -> Result<Self> {
        Self::make_frame(accuracy, observer, time, f64::NAN, f64::NAN)
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
        Self::make_frame(accuracy, observer, time, xp_mas, yp_mas)
    }

    fn make_frame(
        accuracy: Accuracy,
        observer: &Observer,
        time: &Time,
        xp_mas: f64,
        yp_mas: f64,
    ) -> Result<Self> {
        let obs = observer.as_novas_observer()?;
        let mut frame = MaybeUninit::<novas_frame>::zeroed();
        // SAFETY: novas_make_frame fully initializes `*frame` on a zero
        // return, which we check before assuming initialization.
        // NAN xp/yp are the auto-fetch sentinel and are valid C API inputs.
        let rc = unsafe {
            novas_make_frame(
                accuracy.to_sys(),
                &raw const obs,
                time.as_timespec(),
                xp_mas,
                yp_mas,
                frame.as_mut_ptr(),
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(Frame(unsafe { frame.assume_init() }))
    }

    /// The accuracy mode this frame was built with.
    #[must_use]
    pub fn accuracy(&self) -> Accuracy {
        match self.0.accuracy {
            NOVAS_FULL_ACCURACY => Accuracy::Full,
            NOVAS_REDUCED_ACCURACY => Accuracy::Reduced,
        }
    }

    /// The frame's instant as a TT-based Julian date.
    #[must_use]
    pub fn tt_jd(&self) -> f64 {
        // novas_timespec stores integer + fractional TT JD parts.
        // `c_long` is `i64` on 64-bit Unix and `i32` on 32-bit / Windows;
        // `i64::from` is identity on the former and a widening on the latter.
        #[allow(clippy::useless_conversion)]
        let ijd: i64 = i64::from(self.0.time.ijd_tt);
        ijd as f64 + self.0.time.fjd_tt
    }

    /// Borrow the underlying C `novas_frame` for FFI calls inside the
    /// safe-wrapper crate.
    pub(crate) fn as_novas_frame(&self) -> &novas_frame {
        &self.0
    }

    /// Compute the apparent horizontal (azimuth, elevation) of a source as
    /// seen from this frame's observer at this frame's time.
    ///
    /// Accepts any [`Source`]: [`crate::CatalogEntry`], [`crate::Planet`],
    /// [`crate::EphemObject`], or [`crate::OrbitalObject`].
    ///
    /// This is the convenience shortcut for the full pipeline:
    ///
    /// ```text
    ///   source.apparent_in(frame, ReferenceSystem::Cirs)?.to_horizontal()
    /// ```
    ///
    /// No atmospheric refraction is applied. Use [`crate::Apparent`]
    /// directly if you need intermediate RA/Dec, a different
    /// [`ReferenceSystem`], or atmospheric refraction.
    pub fn observe(&self, source: &impl Source) -> Result<Horizontal> {
        source
            .apparent_in(self, ReferenceSystem::Cirs)?
            .to_horizontal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Timescale;

    fn j2000() -> Time {
        Time::from_jd(Timescale::Tt, 2_451_545.0, 32, 0.0).unwrap()
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
