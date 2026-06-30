//! Pre-computed coordinate-transform matrices between reference systems.
//!
//! A [`Transform`] is a rotation matrix pre-computed by `SuperNOVAS` for a
//! specific [`Frame`] and a pair of [`ReferenceSystem`]s. Building it once
//! and re-applying it to many vectors or sky positions is cheaper than
//! recomputing the rotation chain each time — useful when transforming a
//! large catalog or a long time series through the same frame and system
//! pair.
//!
//! Note that, unlike a free-floating matrix, every coordinate transform
//! here is intrinsically tied to an observing [`Frame`] (it carries the
//! epoch, polar motion, and observer-dependent rotation chain), so there
//! is no `From<T>`/`Into<T>` between systems: the conversions are methods
//! that need the frame, not parameter-free conversions.

use core::mem::MaybeUninit;

use supernovas_ffi::{
    novas_invert_transform, novas_make_transform, novas_transform, novas_transform_sky_pos,
    novas_transform_vector,
};

use crate::{
    Apparent, Frame, ReferenceSystem,
    error::{Error, Result},
};

/// A pre-computed coordinate transform between two [`ReferenceSystem`]s at
/// a fixed [`Frame`].
///
/// Build with [`Transform::new`] (or [`Frame::transform`]) and apply with
/// [`Transform::apply_vector`] or [`Transform::apply_sky_pos`]. The inverse
/// is available via [`Transform::invert`].
///
/// `Transform` is `Copy` and cheap to pass by value (~1.4 KB); it owns the
/// full rotation matrix plus a snapshot of the originating frame, so it
/// stays valid even if the original [`Frame`] is later dropped or mutated.
#[derive(Debug, Clone, Copy)]
pub struct Transform(novas_transform);

impl Transform {
    /// Build a transform from `from_system` to `to_system` valid at `frame`.
    ///
    /// The transform snapshots the frame internally, so it remains valid
    /// regardless of later changes to the originating [`Frame`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Ffi`] if the underlying `novas_make_transform` call
    /// rejects the system pair or the frame.
    pub fn new(
        frame: &Frame,
        from_system: ReferenceSystem,
        to_system: ReferenceSystem,
    ) -> Result<Self> {
        let mut t = MaybeUninit::<novas_transform>::zeroed();
        // SAFETY: novas_make_transform fully initializes *transform on a
        // zero return, which we check before assuming initialization.
        let rc = unsafe {
            novas_make_transform(
                frame.as_novas_frame(),
                from_system.to_sys(),
                to_system.to_sys(),
                t.as_mut_ptr(),
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(Transform(unsafe { t.assume_init() }))
    }

    /// The reference system this transform converts **from**.
    #[must_use]
    pub fn from_system(self) -> ReferenceSystem {
        // from_system is the second field; map it back through the C enum.
        // The C side collapses ICRS↔GCRS internally, but the field keeps the
        // value we set, so the tag round-trips exactly.
        map_sys(self.0.from_system)
    }

    /// The reference system this transform converts **to**.
    #[must_use]
    pub fn to_system(self) -> ReferenceSystem {
        map_sys(self.0.to_system)
    }

    /// The observing frame this transform is anchored to.
    #[must_use]
    pub fn frame(self) -> Frame {
        // The frame is embedded by value in the transform; wrap it.
        Frame::from_novas(self.0.frame)
    }

    /// Produce the inverse transform (swap `from_system`/`to_system`).
    ///
    /// May be called with `self` (aliasing is handled by the C side, which
    /// copies the input before inverting).
    ///
    /// **Note:** `novas_invert_transform` inverts only the rotation matrix
    /// and leaves the `from_system`/`to_system` tags unchanged; this wrapper
    /// swaps them so the returned [`Transform`] is self-consistent (its
    /// `from_system` is `self.to_system` and vice versa).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Ffi`] if the C call fails.
    pub fn invert(&self) -> Result<Transform> {
        let mut inv = MaybeUninit::<novas_transform>::zeroed();
        // SAFETY: novas_invert_transform copies *transform into *inverse
        // before inverting in place, so &raw const self.0 / &raw mut same
        // is safe; it returns 0 on success.
        let rc = unsafe { novas_invert_transform(&raw const self.0, inv.as_mut_ptr()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        let mut inv = unsafe { inv.assume_init() };
        // The C call inverts the matrix but does not swap the system tags;
        // swap them here so the result is self-consistent.
        core::mem::swap(&mut inv.from_system, &mut inv.to_system);
        Ok(Transform(inv))
    }

    /// Apply the transform to a Cartesian 3-vector (position or velocity).
    ///
    /// The input is interpreted in [`Self::from_system`]; the output is in
    /// [`Self::to_system`]. Units pass through unchanged (this is a pure
    /// rotation, no scaling).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Ffi`] if the C call fails (in practice only on a
    /// null pointer, which this wrapper never passes).
    pub fn apply_vector(self, v: [f64; 3]) -> Result<[f64; 3]> {
        let mut out = [0.0_f64; 3];
        // SAFETY: novas_transform_vector writes 3 doubles into out on a zero
        // return. in/out may alias; the C side handles that.
        let rc = unsafe { novas_transform_vector(v.as_ptr(), &raw const self.0, out.as_mut_ptr()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(out)
    }

    /// Apply the transform to an [`Apparent`] sky position.
    ///
    /// Only the unit direction `r_hat` is rotated; the `distance` and
    /// `radial_velocity` fields are copied through unchanged from `app`.
    /// The returned [`Apparent`] is re-tagged with [`Self::to_system`] and
    /// anchored to [`Self::frame`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Ffi`] if the C call fails.
    pub fn apply_sky_pos(self, app: &Apparent) -> Result<Apparent> {
        let mut out = MaybeUninit::<supernovas_ffi::sky_pos>::zeroed();
        // SAFETY: novas_transform_sky_pos writes *out on a zero return; the
        // r_hat rotation is in-place-safe with separate in/out pointers.
        let rc = unsafe {
            novas_transform_sky_pos(app.as_sky_pos(), &raw const self.0, out.as_mut_ptr())
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        let sky = unsafe { out.assume_init() };
        Ok(Apparent::from_parts(
            self.frame(),
            map_sys(self.0.to_system),
            sky,
        ))
    }
}

/// Map the C-side `novas_reference_system` enum back to the Rust
/// [`ReferenceSystem`]. The C enum values are a dense i32 range, so this
/// is a total function over the values `to_sys()` can produce.
fn map_sys(s: supernovas_ffi::novas_reference_system) -> ReferenceSystem {
    use supernovas_ffi::novas_reference_system as sys;
    match s {
        sys::NOVAS_GCRS => ReferenceSystem::Gcrs,
        sys::NOVAS_TOD => ReferenceSystem::Tod,
        sys::NOVAS_CIRS => ReferenceSystem::Cirs,
        sys::NOVAS_ICRS => ReferenceSystem::Icrs,
        sys::NOVAS_J2000 => ReferenceSystem::J2000,
        sys::NOVAS_MOD => ReferenceSystem::Mod,
        sys::NOVAS_TIRS => ReferenceSystem::Tirs,
        sys::NOVAS_ITRS => ReferenceSystem::Itrs,
        // The C enum is a dense i32 range and the eight arms above cover
        // every documented value.
        #[allow(unreachable_patterns)]
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Accuracy, CatalogEntry, Observer, Time, Timescale};

    fn frame() -> Frame {
        let obs = Observer::geodetic(37.234, -118.282, 1222.0).unwrap();
        let t = Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap();
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
    fn new_and_accessors_round_trip() {
        let f = frame();
        let t = Transform::new(&f, ReferenceSystem::Cirs, ReferenceSystem::Itrs).unwrap();
        assert_eq!(t.from_system(), ReferenceSystem::Cirs);
        assert_eq!(t.to_system(), ReferenceSystem::Itrs);
        assert!((t.frame().tt_jd() - f.tt_jd()).abs() < 1e-9);
    }

    #[test]
    fn identity_transform_is_noop_on_vector() {
        let f = frame();
        let t = Transform::new(&f, ReferenceSystem::Cirs, ReferenceSystem::Cirs).unwrap();
        let v = [0.3, 0.4, 0.5];
        let out = t.apply_vector(v).unwrap();
        for (a, b) in v.iter().zip(out.iter()) {
            assert!(
                (a - b).abs() < 1e-12,
                "identity should leave vector unchanged"
            );
        }
    }

    #[test]
    fn cirs_itrs_round_trip_via_invert() {
        let f = frame();
        let fwd = Transform::new(&f, ReferenceSystem::Cirs, ReferenceSystem::Itrs).unwrap();
        let inv = fwd.invert().unwrap();
        let v = [0.6, -0.2, 0.8];
        let out = inv.apply_vector(fwd.apply_vector(v).unwrap()).unwrap();
        let max_err = v
            .iter()
            .zip(out.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            max_err < 1e-9,
            "round-trip should recover input, max err = {max_err}"
        );
        assert_eq!(inv.from_system(), ReferenceSystem::Itrs);
        assert_eq!(inv.to_system(), ReferenceSystem::Cirs);
    }

    #[test]
    fn invert_aliasing_safe() {
        let f = frame();
        let t = Transform::new(&f, ReferenceSystem::Cirs, ReferenceSystem::Itrs).unwrap();
        let _ = t.invert().unwrap();
        // Original still usable.
        let _ = t.apply_vector([1.0, 0.0, 0.0]).unwrap();
    }

    #[test]
    fn apply_sky_pos_retags_system() {
        let f = frame();
        let apparent = vega().apparent_in(&f, ReferenceSystem::Cirs).unwrap();
        let t = Transform::new(&f, ReferenceSystem::Cirs, ReferenceSystem::Itrs).unwrap();
        let out = t.apply_sky_pos(&apparent).unwrap();
        assert_eq!(out.reference_system(), ReferenceSystem::Itrs);
        // r_hat is a unit vector; the rotation must preserve its length.
        let rh = out.as_sky_pos().r_hat;
        let mag = (rh[0] * rh[0] + rh[1] * rh[1] + rh[2] * rh[2]).sqrt();
        assert!((mag - 1.0).abs() < 1e-9, "rotated r_hat must be unit");
    }

    #[test]
    fn apply_sky_pos_round_trip_via_inverse() {
        let f = frame();
        let apparent = vega().apparent_in(&f, ReferenceSystem::Cirs).unwrap();
        let fwd = Transform::new(&f, ReferenceSystem::Cirs, ReferenceSystem::J2000).unwrap();
        let inv = fwd.invert().unwrap();
        let out = fwd.apply_sky_pos(&apparent).unwrap();
        let back = inv.apply_sky_pos(&out).unwrap();
        // Round-trip CIRS -> J2000 -> CIRS closes to numerical precision.
        assert!((back.ra().hours() - apparent.ra().hours()).abs() < 1e-9);
        assert!((back.dec().deg() - apparent.dec().deg()).abs() < 1e-9);
    }

    #[test]
    fn j2000_icrs_equivalent_under_transform() {
        // novas_make_transform collapses ICRS <-> GCRS, and ICRS and J2000
        // differ by the frame-tie rotation (sub-arcsec). A J2000 -> ICRS
        // transform applied to a J2000 apparent should land within the
        // frame-tie tolerance of the ICRS apparent computed directly.
        let f = Frame::new(
            Accuracy::Reduced,
            &Observer::Geocenter,
            &Time::from_jd(Timescale::Tt, 2_451_545.0, 32, 0.0).unwrap(),
        )
        .unwrap();
        let j2000 = vega().apparent_in(&f, ReferenceSystem::J2000).unwrap();
        let icrs = vega().apparent_in(&f, ReferenceSystem::Icrs).unwrap();
        let t = Transform::new(&f, ReferenceSystem::J2000, ReferenceSystem::Icrs).unwrap();
        let via_t = t.apply_sky_pos(&j2000).unwrap();
        let sep = via_t.equatorial().distance_to(icrs.equatorial());
        assert!(
            sep.arcsec() < 0.1,
            "J2000->ICRS transform drift = {} arcsec",
            sep.arcsec()
        );
    }
}
