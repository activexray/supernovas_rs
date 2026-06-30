//! Geometric (astrometric) position and velocity of a source.
//!
//! A [`Geometric`] is the raw geometric place of a source — proper motion
//! and light-time are applied, but **no** stellar aberration and **no**
//! gravitational deflection. It's the input to `novas_geom_to_app`; for the
//! observed (apparent) place that includes those corrections, use
//! [`crate::Apparent`] via [`crate::Source::apparent_in`].

use supernovas_ffi::novas_geom_posvel;

use crate::{
    Frame, Position, ReferenceSystem, Velocity,
    error::{Error, Result},
    source::Source,
    unit,
};

/// The geometric place of a source: position and velocity in a chosen
/// [`ReferenceSystem`], anchored to a [`Frame`].
///
/// Produced by [`Source::geometric_in`]. Unlike [`crate::Apparent`], this
/// carries no aberration or gravitational-deflection correction — it's the
/// "where the source physically is" vector, useful for interferometric
/// baseline projections and for callers that want to apply their own
/// aberration model.
#[derive(Debug, Clone, Copy)]
pub struct Geometric {
    frame: Frame,
    system: ReferenceSystem,
    pos: Position,
    vel: Velocity,
}

impl Geometric {
    /// The position vector (meters) in [`Self::reference_system`].
    #[must_use]
    pub fn position(self) -> Position {
        self.pos
    }

    /// The velocity vector (meters/second) in [`Self::reference_system`].
    #[must_use]
    pub fn velocity(self) -> Velocity {
        self.vel
    }

    /// The reference system the position/velocity are expressed in.
    #[must_use]
    pub fn reference_system(self) -> ReferenceSystem {
        self.system
    }

    /// The frame this geometric place was computed in.
    #[must_use]
    pub fn frame(self) -> Frame {
        self.frame
    }
}

/// Compute the geometric position+velocity of any [`Source`] for the given
/// frame and reference system.
///
/// Used internally by [`Source::geometric_in`]. Wraps `novas_geom_posvel`;
/// converts the C-side AU / AU-per-day outputs to meters / meters-per-second.
pub(crate) fn geometric_of_source_in(
    source: &(impl Source + ?Sized),
    frame: &Frame,
    system: ReferenceSystem,
) -> Result<Geometric> {
    let mut pos_au = [0.0_f64; 3];
    let mut vel_au_per_day = [0.0_f64; 3];
    // SAFETY: novas_geom_posvel writes 6 doubles (pos then vel) on a zero
    // return. The two output slices are distinct (different stack arrays).
    let rc = unsafe {
        novas_geom_posvel(
            source.as_object(),
            frame.as_novas_frame(),
            system.to_sys(),
            pos_au.as_mut_ptr(),
            vel_au_per_day.as_mut_ptr(),
        )
    };
    if rc != 0 {
        return Err(Error::ffi(rc));
    }
    // AU -> meters, AU/day -> m/s.
    let pos_m = [
        pos_au[0] * unit::AU,
        pos_au[1] * unit::AU,
        pos_au[2] * unit::AU,
    ];
    let vel_mps = [
        vel_au_per_day[0] * unit::AU_PER_DAY,
        vel_au_per_day[1] * unit::AU_PER_DAY,
        vel_au_per_day[2] * unit::AU_PER_DAY,
    ];
    Ok(Geometric {
        frame: *frame,
        system,
        pos: Position::from_meters_array(pos_m)?,
        vel: Velocity::from_mps_array(vel_mps)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Accuracy, CatalogEntry, Observer, Planet, SolarBody, Time};

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
    fn geometric_differs_from_apparent_at_arcsec_scale() {
        // Aberration is ~20 arcsec; geometric (no aberration) and apparent
        // should differ by that order on the sky direction. Both vectors
        // are unit-ish (catalog source with no parallax), so normalise and
        // compare directions.
        let f = frame();
        let g = vega().geometric_in(&f, ReferenceSystem::Cirs).unwrap();
        let a = vega().apparent_in(&f, ReferenceSystem::Cirs).unwrap();

        let gp = g.position().as_meters();
        let gm = (gp[0] * gp[0] + gp[1] * gp[1] + gp[2] * gp[2]).sqrt();
        let ghat = [gp[0] / gm, gp[1] / gm, gp[2] / gm];
        let rhat = a.as_sky_pos().r_hat;
        let dot = ghat[0] * rhat[0] + ghat[1] * rhat[1] + ghat[2] * rhat[2];
        let sep_rad = dot.clamp(-1.0, 1.0).acos();
        let sep_arcsec = sep_rad / unit::ARCSEC;
        assert!(
            sep_arcsec > 0.1,
            "geometric vs apparent sep = {sep_arcsec} arcsec (too small; expected aberration signal)"
        );
    }

    #[test]
    fn getters_round_trip() {
        let f = frame();
        let g = vega().geometric_in(&f, ReferenceSystem::Cirs).unwrap();
        assert_eq!(g.reference_system(), ReferenceSystem::Cirs);
        assert!((g.frame().tt_jd() - f.tt_jd()).abs() < 1e-9);
        assert!(g.position().as_meters().iter().all(|x| x.is_finite()));
        assert!(g.velocity().as_mps().iter().all(|x| x.is_finite()));
    }

    #[test]
    fn geometric_for_planet_is_finite() {
        // Exercises the non-catalog (light_time2) branch and the AU/day
        // velocity conversion. Use the Sun from a geocentric observer so
        // the position is ~1 AU away (non-zero magnitude).
        let f = Frame::new(
            Accuracy::Reduced,
            &Observer::Geocenter,
            &Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap(),
        )
        .unwrap();
        let g = Planet::new(SolarBody::Sun)
            .unwrap()
            .geometric_in(&f, ReferenceSystem::Icrs)
            .unwrap();
        let p = g.position().as_meters();
        // Sun is ~1 AU from Earth; in meters that's ~1.5e11.
        let mag = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        assert!(mag > 1e10 && mag < 1e12, "Sun distance {mag} m looks wrong");
        let v = g.velocity().as_mps();
        assert!(v.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn itrs_output_is_supported() {
        // Covers the earth-rotating output branch of icrs_to_sys inside
        // novas_geom_posvel.
        let f = frame();
        let g = vega().geometric_in(&f, ReferenceSystem::Itrs).unwrap();
        assert_eq!(g.reference_system(), ReferenceSystem::Itrs);
        assert!(g.position().as_meters().iter().all(|x| x.is_finite()));
    }
}
