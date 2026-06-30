//! Observing frame: an observer × instant × Earth-orientation snapshot.
//!
//! A [`Frame`] bundles everything `SuperNOVAS` needs to compute apparent
//! positions: where the observer is, when, and (optionally) Earth's polar
//! motion at that instant. Sky positions are then evaluated against the
//! frame.

use core::mem::MaybeUninit;

use supernovas_ffi::{
    novas_accuracy::{NOVAS_FULL_ACCURACY, NOVAS_REDUCED_ACCURACY},
    novas_change_observer, novas_frame, novas_hor_to_app, novas_make_frame, radec2vector, sky_pos,
};

use crate::{
    Angle, Horizontal, Observer, Refraction, Time,
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

    /// Updates the [`Observer`] for this [`Frame`]
    pub fn update_observer(&mut self, obs: &Observer) -> Result<()> {
        let c_obs = obs.as_novas_observer()?;
        // SAFETY: novas_change_observer explicitly handles orig==out aliasing
        // (it skips the `*out = *orig` copy when the pointers are equal).
        let rc =
            unsafe { novas_change_observer(&raw const self.0, &raw const c_obs, &raw mut self.0) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(())
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

    /// Wrap a raw `novas_frame` produced by a `SuperNOVAS` call.
    ///
    /// Used inside the crate to embed a frame snapshot in derived types
    /// (e.g. [`crate::Transform`]); not part of the public API because the
    /// caller must guarantee the struct is fully initialized by `novas_make_frame`.
    #[allow(clippy::large_types_passed_by_value)]
    pub(crate) fn from_novas(frame: novas_frame) -> Self {
        Frame(frame)
    }

    /// The observer-place tag of this frame's observer (mirrors the C
    /// `novas_observer_place` enum). Used by the UVW helpers to gate on
    /// Earth-bound observers.
    pub(crate) fn observer_place(&self) -> supernovas_ffi::novas_observer_place {
        self.0.observer.where_
    }

    /// The polar-wobble offsets `dx`, `dy` converted to **arcseconds** (the
    /// C `cirs_to_itrs` / `novas_site_gcrs_posvel` family wants arcsec; the
    /// frame stores them in milliarcseconds).
    pub(crate) fn polar_motion_arcsec(&self) -> (f64, f64) {
        (self.0.dx / 1000.0, self.0.dy / 1000.0)
    }

    /// The frame's accuracy as the raw C enum value.
    pub(crate) fn accuracy_sys(&self) -> supernovas_ffi::novas_accuracy {
        self.0.accuracy
    }

    /// Borrow the embedded `novas_timespec` as a raw pointer, for FFI calls
    /// that take a `*const novas_timespec` (e.g. `novas_site_gcrs_posvel`).
    pub(crate) fn as_timespec_ptr(&self) -> *const supernovas_ffi::novas_timespec {
        &raw const self.0.time
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

    /// Inverse of [`crate::Apparent::to_horizontal_with_refraction`]: convert
    /// an observed horizontal direction to an apparent equatorial place in
    /// the requested [`ReferenceSystem`].
    ///
    /// Wraps `novas_hor_to_app`. The returned [`crate::Apparent`] carries
    /// the requested system tag; its `distance` and `radial_velocity` are
    /// **left at zero** (the C function only computes RA/Dec) and the unit
    /// direction `r_hat` is reconstructed from RA/Dec via `radec2vector`. If
    /// you need the full `(ra, dec, distance, rv)` tuple, recompute with
    /// [`Source::apparent_in`].
    ///
    /// Requires an Earth-bound observer; a geocentric frame returns
    /// [`Error::UnsupportedObserver`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedObserver`] if the frame's observer is not
    /// on Earth, or [`Error::Ffi`] if the C call fails.
    pub fn horizontal_to_apparent(
        &self,
        h: Horizontal,
        refraction: Refraction,
        system: ReferenceSystem,
    ) -> Result<crate::Apparent> {
        let on_surf = self.require_on_surface()?;
        let mut ra = 0.0_f64;
        let mut dec = 0.0_f64;
        // SAFETY: novas_hor_to_app writes the two output doubles on a zero
        // return. The refraction callback matches the RefractionModel ABI.
        let rc = unsafe {
            novas_hor_to_app(
                &raw const self.0,
                h.azimuth().deg(),
                h.elevation().deg(),
                refraction.to_sys(),
                system.to_sys(),
                &raw mut ra,
                &raw mut dec,
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        // The C call ignores on_surf beyond the observer type check, but
        // keeping the require_on_surface gate gives us a typed error path
        // instead of relying on the C-side EINVAL.
        let _ = on_surf;

        // Reconstruct r_hat from (ra, dec). distance and rv are not available
        // from novas_hor_to_app; leave them at the sky_pos default (0).
        let mut r_hat = [0.0_f64; 3];
        // SAFETY: radec2vector writes 3 doubles into r_hat on a zero return.
        let rc = unsafe { radec2vector(ra, dec, 1.0, r_hat.as_mut_ptr()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        let sky = sky_pos {
            r_hat,
            ra,
            dec,
            dis: 0.0,
            rv: 0.0,
        };
        Ok(crate::Apparent::from_parts(*self, system, sky))
    }

    /// Build a pre-computed [`crate::Transform`] between two reference
    /// systems valid at this frame's epoch and observer.
    ///
    /// Shorthand for [`crate::Transform::new`] with `self` as the anchoring
    /// frame. See that method for the full contract.
    pub fn transform(
        &self,
        from_system: ReferenceSystem,
        to_system: ReferenceSystem,
    ) -> Result<crate::Transform> {
        crate::Transform::new(self, from_system, to_system)
    }

    /// Rotate a Cartesian 3-vector from CIRS (celestial, IAU 2000) to ITRS
    /// (Earth-fixed) at this frame's epoch.
    ///
    /// Reads the split TT Julian date, the `UT1−TT` offset, the accuracy,
    /// and the polar-motion offsets (`dx`, `dy` in milliarcseconds) from
    /// the stored `novas_frame` and forwards them to `cirs_to_itrs`. Units
    /// pass through unchanged (this is a pure rotation).
    ///
    /// For *sky-position* (RA/Dec) conversions prefer
    /// [`crate::Equatorial::to_system`] or [`crate::Transform`] — this
    /// low-level rotation operates on bare 3-vectors and does not fold in
    /// aberration or gravitational deflection.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Ffi`] if the underlying C call fails.
    pub fn cirs_to_itrs(&self, v: [f64; 3]) -> Result<[f64; 3]> {
        let (ijd, fjd) = self.tt_split_parts();
        let mut out = [0.0_f64; 3];
        // SAFETY: cirs_to_itrs writes 3 doubles into out on a zero return.
        let rc = unsafe {
            supernovas_ffi::cirs_to_itrs(
                ijd as f64,
                fjd,
                self.0.time.ut1_to_tt,
                self.0.accuracy,
                self.0.dx / 1000.0,
                self.0.dy / 1000.0,
                v.as_ptr(),
                out.as_mut_ptr(),
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(out)
    }

    /// Rotate a Cartesian 3-vector from ITRS (Earth-fixed) to CIRS
    /// (celestial, IAU 2000) at this frame's epoch.
    ///
    /// Inverse of [`Self::cirs_to_itrs`]; see that method for the parameter
    /// sourcing and the recommended higher-level alternatives.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Ffi`] if the underlying C call fails.
    pub fn itrs_to_cirs(&self, v: [f64; 3]) -> Result<[f64; 3]> {
        let (ijd, fjd) = self.tt_split_parts();
        let mut out = [0.0_f64; 3];
        // SAFETY: itrs_to_cirs writes 3 doubles into out on a zero return.
        let rc = unsafe {
            supernovas_ffi::itrs_to_cirs(
                ijd as f64,
                fjd,
                self.0.time.ut1_to_tt,
                self.0.accuracy,
                self.0.dx / 1000.0,
                self.0.dy / 1000.0,
                v.as_ptr(),
                out.as_mut_ptr(),
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(out)
    }

    /// Convert an ITRS (Earth-fixed) 3-vector to local horizontal
    /// (azimuth, zenith angle) as seen by this frame's geodetic observer.
    ///
    /// Wraps `itrs_to_hor`. Requires an Earth-bound observer; pass a
    /// geocentric frame and you get [`crate::Error::UnsupportedObserver`].
    /// The zenith angle is converted to elevation internally so the result
    /// is a normal [`Horizontal`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::UnsupportedObserver`] if the frame's observer
    /// is not on Earth, or [`crate::Error::Ffi`] if the C call fails.
    pub fn itrs_to_horizontal(&self, itrs: [f64; 3]) -> Result<Horizontal> {
        let on_surf = self.require_on_surface()?;
        let mut az = 0.0_f64;
        let mut za = 0.0_f64;
        // SAFETY: itrs_to_hor writes the two output doubles on a zero return.
        let rc = unsafe {
            supernovas_ffi::itrs_to_hor(&raw const on_surf, itrs.as_ptr(), &raw mut az, &raw mut za)
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Horizontal::from_degrees(az, 90.0 - za)
    }

    /// Convert a local horizontal direction to an ITRS (Earth-fixed) 3-vector
    /// at this frame's geodetic observer.
    ///
    /// Inverse of [`Self::itrs_to_horizontal`]; same Earth-bound requirement.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::UnsupportedObserver`] if the frame's observer
    /// is not on Earth, or [`crate::Error::Ffi`] if the C call fails.
    pub fn horizontal_to_itrs(&self, h: Horizontal) -> Result<[f64; 3]> {
        let on_surf = self.require_on_surface()?;
        let mut itrs = [0.0_f64; 3];
        // SAFETY: hor_to_itrs writes 3 doubles into itrs on a zero return.
        // Horizontal stores azimuth in (-180,180] via Angle; the C side wants
        // [0,360) but accepts any finite value (it only uses sin/cos).
        let rc = unsafe {
            supernovas_ffi::hor_to_itrs(
                &raw const on_surf,
                h.azimuth().deg(),
                h.zenith_angle().deg(),
                itrs.as_mut_ptr(),
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(itrs)
    }

    /// The Local (apparent) Sidereal Time of this frame's observer, in
    /// `[0, 24h)`.
    ///
    /// Wraps `novas_frame_lst`. Requires an Earth-bound observer; a
    /// geocentric frame has no local horizon and no LST, so it returns
    /// [`crate::Error::UnsupportedObserver`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::UnsupportedObserver`] if the frame's observer
    /// is not on Earth.
    pub fn lst(&self) -> Result<crate::TimeAngle> {
        // Pre-check the observer type so we can return a typed error rather
        // than a NotFinite from the NaN that novas_frame_lst returns for
        // non-Earth-bound observers.
        if self.0.observer.where_ != supernovas_ffi::novas_observer_place::NOVAS_OBSERVER_ON_EARTH
            && self.0.observer.where_
                != supernovas_ffi::novas_observer_place::NOVAS_AIRBORNE_OBSERVER
        {
            return Err(Error::UnsupportedObserver);
        }
        // SAFETY: read-only FFI call; self.0 is a valid initialized frame.
        let hours = unsafe { supernovas_ffi::novas_frame_lst(&raw const self.0) };
        crate::TimeAngle::from_hours(hours).map_err(|_| {
            // A NaN return here means the frame wasn't initialized or the
            // observer check above missed an edge case; both are bugs in
            // our invariant, but surface them as a typed error anyway.
            Error::UnsupportedObserver
        })
    }

    /// The `(integer_tt_jd, fractional_tt_jd)` parts of this frame's time.
    ///
    /// The polar-wobble and rotation parameters in the C `novas_frame` are
    /// referenced to TT, so the ITRS↔CIRS rotations read these directly.
    fn tt_split_parts(&self) -> (i64, f64) {
        #[allow(clippy::useless_conversion)]
        let ijd: i64 = i64::from(self.0.time.ijd_tt);
        (ijd, self.0.time.fjd_tt)
    }

    /// Return the frame's `on_surface` struct if the observer is Earth-bound,
    /// else an [`Error::UnsupportedObserver`].
    fn require_on_surface(&self) -> Result<supernovas_ffi::on_surface> {
        if self.0.observer.where_ != supernovas_ffi::novas_observer_place::NOVAS_OBSERVER_ON_EARTH {
            return Err(Error::UnsupportedObserver);
        }
        Ok(self.0.observer.on_surf)
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

    #[test]
    fn update_observer_replaces_location() {
        use supernovas_ffi::novas_observer_place::NOVAS_OBSERVER_ON_EARTH;

        let t = j2000();
        let geodetic = Observer::geodetic(34.0, -118.0, 100.0).unwrap();
        let mut frame = Frame::new(Accuracy::Reduced, &Observer::Geocenter, &t).unwrap();

        // After construction the internal observer is at geocenter.
        assert_ne!(frame.0.observer.where_, NOVAS_OBSERVER_ON_EARTH);

        frame.update_observer(&geodetic).unwrap();

        // After the update the observer should be the on-surface type.
        assert_eq!(frame.0.observer.where_, NOVAS_OBSERVER_ON_EARTH);
        // Accuracy and time must be unchanged.
        assert_eq!(frame.accuracy(), Accuracy::Reduced);
        assert!((frame.tt_jd() - 2_451_545.0).abs() < 1e-9);
    }

    /// Polaris sits ~0.74° from the true celestial pole, so from any
    /// northern-hemisphere site at geographic latitude L, its elevation is
    /// `L ± 0.74°`. This is the classic "Polaris altitude = your latitude"
    /// trick — and a tight end-to-end smoke test for the whole ICRS → az/el
    /// pipeline (`Frame::new` + `sky_pos` + `app_to_hor`).
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

    // ── ITRS ↔ CIRS vector rotations ──────────────────────────────────────

    fn ovro_frame() -> Frame {
        let obs = Observer::geodetic(37.234, -118.282, 1222.0).unwrap();
        let t = Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap();
        Frame::new(Accuracy::Reduced, &obs, &t).unwrap()
    }

    #[test]
    fn cirs_itrs_vector_round_trip() {
        let f = ovro_frame();
        let v = [0.6, -0.2, 0.8];
        let itrs = f.cirs_to_itrs(v).unwrap();
        let back = f.itrs_to_cirs(itrs).unwrap();
        let max_err = v
            .iter()
            .zip(back.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(max_err < 1e-12, "CIRS↔ITRS round-trip max err = {max_err}");
    }

    #[test]
    fn cirs_itrs_agrees_with_transform() {
        // The Frame method and the Transform path must compute the same rotation.
        let f = ovro_frame();
        let v = [0.3, 0.7, -0.4];
        let via_frame = f.cirs_to_itrs(v).unwrap();
        let via_transform = f
            .transform(crate::ReferenceSystem::Cirs, crate::ReferenceSystem::Itrs)
            .unwrap()
            .apply_vector(v)
            .unwrap();
        let max_err = via_frame
            .iter()
            .zip(via_transform.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(
            max_err < 1e-12,
            "frame vs transform CIRS→ITRS max err = {max_err}"
        );
    }

    #[test]
    fn cirs_itrs_polar_motion_changes_result() {
        // Nonzero polar motion must produce a small but finite difference
        // vs. the zero-polar-motion frame at the same epoch.
        let obs = Observer::geodetic(37.234, -118.282, 1222.0).unwrap();
        let t = Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap();
        let zero = Frame::with_polar_motion(
            Accuracy::Reduced,
            &obs,
            &t,
            Angle::from_mas(0.0).unwrap(),
            Angle::from_mas(0.0).unwrap(),
        )
        .unwrap();
        let nonzero = Frame::with_polar_motion(
            Accuracy::Reduced,
            &obs,
            &t,
            Angle::from_mas(120.0).unwrap(),
            Angle::from_mas(-85.0).unwrap(),
        )
        .unwrap();
        let v = [0.5, 0.5, 0.707];
        let a = zero.cirs_to_itrs(v).unwrap();
        let b = nonzero.cirs_to_itrs(v).unwrap();
        let max_diff = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f64::max);
        // 120 mas polar motion rotates a unit vector by ~120 mas ≈ 6e-7 rad.
        assert!(
            max_diff > 1e-9,
            "polar motion had no effect: max_diff = {max_diff}"
        );
        assert!(
            max_diff < 1e-5,
            "polar motion effect too large: max_diff = {max_diff}"
        );
    }

    // ── LST ───────────────────────────────────────────────────────────────

    #[test]
    fn lst_is_in_range_for_geodetic_observer() {
        let f = ovro_frame();
        let lst = f.lst().unwrap();
        let h = lst.hours();
        assert!((0.0..24.0).contains(&h), "LST {h} out of [0,24)");
    }

    #[test]
    fn lst_increases_with_time() {
        let obs = Observer::geodetic(37.234, -118.282, 1222.0).unwrap();
        let t0 = Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap();
        let t1 = t0 + crate::Interval::from_hours(1.0).unwrap();
        let f0 = Frame::new(Accuracy::Reduced, &obs, &t0).unwrap();
        let f1 = Frame::new(Accuracy::Reduced, &obs, &t1).unwrap();
        let h0 = f0.lst().unwrap().hours();
        let h1 = f1.lst().unwrap().hours();
        // 1 hour later → LST advances by ~1.0027 h (sidereal rate).
        let dh = (h1 - h0).rem_euclid(24.0);
        assert!(
            dh > 0.9 && dh < 1.1,
            "LST advance {dh} h doesn't look sidereal"
        );
    }

    #[test]
    fn lst_refuses_geocenter() {
        let f = Frame::new(
            Accuracy::Reduced,
            &Observer::Geocenter,
            &Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap(),
        )
        .unwrap();
        assert!(matches!(f.lst(), Err(crate::Error::UnsupportedObserver)));
    }

    #[test]
    fn lst_longitude_offset_is_one_hour() {
        let t = Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap();
        let west = Frame::new(
            Accuracy::Reduced,
            &Observer::geodetic(0.0, 0.0, 0.0).unwrap(),
            &t,
        )
        .unwrap();
        let east = Frame::new(
            Accuracy::Reduced,
            &Observer::geodetic(0.0, 15.0, 0.0).unwrap(),
            &t,
        )
        .unwrap();
        let dh = (east.lst().unwrap().hours() - west.lst().unwrap().hours()).rem_euclid(24.0);
        // 15° east → LST +1 h.
        assert!(
            (dh - 1.0).abs() < 1e-6,
            "LST longitude offset = {dh} h, expected 1.0"
        );
    }

    // ── Horizontal ↔ ITRS ─────────────────────────────────────────────────

    #[test]
    fn itrs_horizontal_round_trip() {
        let f = ovro_frame();
        let h = crate::Horizontal::from_degrees(123.4, 56.7).unwrap();
        let itrs = f.horizontal_to_itrs(h).unwrap();
        let back = f.itrs_to_horizontal(itrs).unwrap();
        assert!(
            (back.azimuth().deg().rem_euclid(360.0) - 123.4).abs() < 1e-6,
            "az round-trip"
        );
        assert!(
            (back.elevation().deg() - 56.7).abs() < 1e-6,
            "el round-trip"
        );
    }

    #[test]
    fn itrs_horizontal_refuses_geocenter() {
        let f = Frame::new(
            Accuracy::Reduced,
            &Observer::Geocenter,
            &Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap(),
        )
        .unwrap();
        let h = crate::Horizontal::from_degrees(0.0, 45.0).unwrap();
        assert!(matches!(
            f.itrs_to_horizontal([1.0, 0.0, 0.0]),
            Err(crate::Error::UnsupportedObserver)
        ));
        assert!(matches!(
            f.horizontal_to_itrs(h),
            Err(crate::Error::UnsupportedObserver)
        ));
    }

    #[test]
    fn itrs_horizontal_cross_checks_apparent_path() {
        // A CIRS apparent's to_horizontal() should agree (to <1 arcsec) with
        // cirs_to_itrs(r_hat) → itrs_to_horizontal for the same source.
        let f = ovro_frame();
        let vega = crate::CatalogEntry::icrs(
            "Vega",
            "18:36:56.336".parse().unwrap(),
            "+38:47:01.28".parse().unwrap(),
        )
        .unwrap();
        let app = vega.apparent_in(&f, crate::ReferenceSystem::Cirs).unwrap();
        let via_app = app.to_horizontal().unwrap();
        let r_hat = app.as_sky_pos().r_hat;
        let itrs = f.cirs_to_itrs(r_hat).unwrap();
        let via_vec = f.itrs_to_horizontal(itrs).unwrap();
        let daz = (via_app.azimuth().deg() - via_vec.azimuth().deg()).abs();
        let del = (via_app.elevation().deg() - via_vec.elevation().deg()).abs();
        assert!(daz < 1.0 / 3600.0, "az mismatch {daz}°");
        assert!(del < 1.0 / 3600.0, "el mismatch {del}°");
    }

    // ── horizontal_to_apparent ────────────────────────────────────────────

    #[test]
    fn horizontal_to_apparent_round_trips_without_refraction() {
        let f = ovro_frame();
        let vega = crate::CatalogEntry::icrs(
            "Vega",
            "18:36:56.336".parse().unwrap(),
            "+38:47:01.28".parse().unwrap(),
        )
        .unwrap();
        let app = vega.apparent_in(&f, crate::ReferenceSystem::Cirs).unwrap();
        let h = app.to_horizontal().unwrap();
        let back = f
            .horizontal_to_apparent(h, crate::Refraction::None, crate::ReferenceSystem::Cirs)
            .unwrap();
        let back_h = back.to_horizontal().unwrap();
        let daz = (h.azimuth().deg() - back_h.azimuth().deg()).abs();
        let del = (h.elevation().deg() - back_h.elevation().deg()).abs();
        assert!(daz < 1.0 / 3600.0, "az round-trip {daz}°");
        assert!(del < 1.0 / 3600.0, "el round-trip {del}°");
    }

    #[test]
    fn horizontal_to_apparent_carries_system_tag() {
        let f = ovro_frame();
        let h = crate::Horizontal::from_degrees(100.0, 30.0).unwrap();
        let app = f
            .horizontal_to_apparent(h, crate::Refraction::None, crate::ReferenceSystem::Tod)
            .unwrap();
        assert_eq!(app.reference_system(), crate::ReferenceSystem::Tod);
    }

    #[test]
    fn horizontal_to_apparent_refuses_geocenter() {
        let f = Frame::new(
            Accuracy::Reduced,
            &Observer::Geocenter,
            &Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap(),
        )
        .unwrap();
        let h = crate::Horizontal::from_degrees(0.0, 45.0).unwrap();
        assert!(matches!(
            f.horizontal_to_apparent(h, crate::Refraction::None, crate::ReferenceSystem::Cirs),
            Err(crate::Error::UnsupportedObserver)
        ));
    }
}
