//! Interferometry: UVW baseline projections and ENU / line-of-sight helpers.
//!
//! The headline type is [`Uvw`], the `(u, v, w)` projection of a station's
//! position relative to an array reference along the line of sight to a
//! source. Build it with [`uvw`] (generic, for any coordinate system), using
//! station positions relative to the array reference; obtain those via
//! [`Frame::site_gcrs_posvel`](crate::Frame::site_gcrs_posvel) for ground
//! stations. The lower-level [`xyz_to_uvw`] / [`xyz_to_los`] family exposes
//! the raw building blocks for callers that already have hour-angle /
//! declination or a local ENU frame.
//!
//! All UVW coordinates are in **meters**. The `xyz`/`los` helpers pass units
//! through unchanged - they're pure rotations.

use supernovas_ffi::{
    novas_enu_to_itrs, novas_itrs_to_enu, novas_los_to_xyz,
    novas_observer_place::NOVAS_OBSERVER_ON_EARTH, novas_site_gcrs_posvel, novas_uvw,
    novas_uvw_to_xyz, novas_xyz_to_los, novas_xyz_to_uvw,
};

use crate::{
    Angle, Coordinate, Frame, Position, ReferenceSystem, Site, Source, TimeAngle, Velocity,
    error::{Error, Result},
    unit,
};

/// A `(u, v, w)` baseline projection in **meters**.
///
/// `u` and `v` are the projections of the baseline along the local East and
/// North directions as seen from the source; `w` is the projected distance
/// along the line of sight (the geometric delay divided by the speed of
/// light). Build via [`uvw`] or the example `interferometry.rs`. Obtain the
/// source position via
/// [`Frame::source_gcrs_position`](crate::Frame::source_gcrs_position) and
/// station positions via
/// [`Frame::site_gcrs_posvel`](crate::Frame::site_gcrs_posvel).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Uvw([f64; 3]);

impl Uvw {
    /// Construct from a `[u, v, w]` array in meters.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if any component is not finite.
    pub fn from_meters_array(c: [f64; 3]) -> Result<Self> {
        if c.iter().any(|v| !v.is_finite()) {
            return Err(Error::NotFinite);
        }
        Ok(Uvw(c))
    }

    /// The `u` component (East projection), in meters.
    #[must_use]
    pub fn u(self) -> Coordinate {
        Coordinate::from_meters(self.0[0]).expect("Uvw components are finite by construction")
    }

    /// The `v` component (North projection), in meters.
    #[must_use]
    pub fn v(self) -> Coordinate {
        Coordinate::from_meters(self.0[1]).expect("Uvw components are finite by construction")
    }

    /// The `w` component (line-of-sight projection), in meters.
    #[must_use]
    pub fn w(self) -> Coordinate {
        Coordinate::from_meters(self.0[2]).expect("Uvw components are finite by construction")
    }

    /// The raw `[u, v, w]` array in meters. Useful for FFI.
    #[must_use]
    pub fn as_meters(self) -> [f64; 3] {
        self.0
    }

    /// The geometric delay `τ = w / c` along the line of sight, in seconds.
    ///
    /// `w` is the projected baseline along the source direction; dividing by
    /// the speed of light gives the arrival-time difference between the two
    /// ends of the baseline (the quantity a correlator's delay tracker locks
    /// onto).
    #[must_use]
    pub fn delay(self) -> f64 {
        self.0[2] / unit::C
    }

    /// The geometric delay `τ = w / c` in nanoseconds.
    #[must_use]
    pub fn delay_ns(self) -> f64 {
        self.delay() * 1e9
    }

    /// The delay rate `dτ/dt = (v · ŝ) / c` (dimensionless, s/s).
    ///
    /// `phase_center` is the same [`Position`] passed to [`uvw`] (obtain
    /// via [`Frame::source_gcrs_position`](crate::Frame::source_gcrs_position));
    /// it is normalized internally. `station_vel` is the station's GCRS
    /// velocity relative to the array reference.
    ///
    /// This is the plane-wave (far-field) delay rate - the rate of change of
    /// the geometric delay assuming the source is effectively at infinity.
    /// For near-field sources (spacecraft VLBI) the true delay rate includes
    /// additional terms from the changing source distance.
    #[must_use]
    pub fn delay_rate(self, phase_center: &Position, station_vel: &Velocity) -> f64 {
        let [pc_x, pc_y, pc_z] = phase_center.as_meters();
        let norm = (pc_x * pc_x + pc_y * pc_y + pc_z * pc_z).sqrt();
        let vel = station_vel.as_mps();
        let v_dot_s = (vel[0] * pc_x + vel[1] * pc_y + vel[2] * pc_z) / norm;
        v_dot_s / unit::C
    }
}

/// Compute `(u, v, w)` for a station relative to an array reference, given
/// the station's position/velocity and the source's apparent position.
///
/// Wraps `novas_uvw`. All three inputs must be expressed in the **same**
/// reference system (typically GCRS). `phase_center` is the source's
/// apparent position - obtain it via
/// [`Frame::source_gcrs_position`](crate::Frame::source_gcrs_position).
/// `station_pos` and `station_vel` are relative to the array reference
/// (e.g. via differencing [`Frame::site_gcrs_posvel`] results).
///
/// # Errors
///
/// Returns [`Error::Ffi`] if the C call fails.
///
/// # Examples
///
/// See `examples/interferometry.rs` for a 10-antenna delay calculation.
///
/// [`Frame::site_gcrs_posvel`]: crate::Frame::site_gcrs_posvel
pub fn uvw(
    station_pos: &Position,
    station_vel: Option<&Velocity>,
    phase_center: &Position,
) -> Result<Uvw> {
    let pos_au = station_pos.components().map(Coordinate::au);
    let pc_au = phase_center.components().map(Coordinate::au);
    let vel_au_per_day =
        station_vel.map(|v| v.components().map(|c| c.m_per_s() / unit::AU_PER_DAY));
    let mut out = [0.0_f64; 3];
    // SAFETY: novas_uvw writes 3 doubles into out on a zero return. The
    // velocity pointer is NULL when station_vel is None.
    let rc = unsafe {
        novas_uvw(
            pos_au.as_ptr(),
            vel_au_per_day
                .as_ref()
                .map_or(core::ptr::null(), |v| v.as_ptr()),
            pc_au.as_ptr(),
            out.as_mut_ptr(),
        )
    };
    if rc != 0 {
        return Err(Error::ffi(rc));
    }
    Uvw::from_meters_array(out)
}

/// Convert a position vector in the Pseudo-Earth-Frame (PEF) to a `(u, v, w)`
/// projection for a given hour angle and declination.
///
/// Wraps `novas_xyz_to_uvw`. Units pass through unchanged - the input
/// [`Position`] and returned [`Uvw`] are in the same units. For
/// arcsecond-level work, PEF and ITRS positions are interchangeable; for
/// higher precision, de-wobble ITRS to PEF first.
///
/// # Errors
///
/// Returns [`Error::Ffi`] if the C call fails.
pub fn xyz_to_uvw(pos: Position, ha: TimeAngle, dec: Angle) -> Result<Uvw> {
    let xyz = pos.as_meters();
    let mut out = [0.0_f64; 3];
    // SAFETY: novas_xyz_to_uvw writes 3 doubles into out on a zero return.
    let rc = unsafe { novas_xyz_to_uvw(xyz.as_ptr(), ha.hours(), dec.deg(), out.as_mut_ptr()) };
    if rc != 0 {
        return Err(Error::ffi(rc));
    }
    Uvw::from_meters_array(out)
}

/// Convert a `(u, v, w)` projection to a PEF position vector for a given
/// hour angle and declination. Inverse of [`xyz_to_uvw`].
///
/// # Errors
///
/// Returns [`Error::Ffi`] if the C call fails.
pub fn uvw_to_xyz(uvw: Uvw, ha: TimeAngle, dec: Angle) -> Result<Position> {
    let u = uvw.as_meters();
    let mut out = [0.0_f64; 3];
    // SAFETY: novas_uvw_to_xyz writes 3 doubles into out on a zero return.
    let rc = unsafe { novas_uvw_to_xyz(u.as_ptr(), ha.hours(), dec.deg(), out.as_mut_ptr()) };
    if rc != 0 {
        return Err(Error::ffi(rc));
    }
    Position::from_meters_array(out)
}

/// Convert a `(u, v, w)` line-of-sight vector to a rectangular equatorial
/// [`Position`] vector along the given direction. Units pass through
/// unchanged.
///
/// The three components are the Cartesian projections onto the local East,
/// North, and line-of-sight axes defined by `(lon, lat)` — the same
/// representation used by [`xyz_to_uvw`], just parameterized by
/// longitude/latitude instead of hour angle/declination. The position
/// vector and `(lon, lat)` must be in the same coordinate frame.
///
/// # Errors
///
/// Returns [`Error::Ffi`] if the C call fails.
pub fn los_to_xyz(los: [f64; 3], lon: Angle, lat: Angle) -> Result<Position> {
    let mut out = [0.0_f64; 3];
    // SAFETY: novas_los_to_xyz writes 3 doubles into out on a zero return.
    let rc = unsafe { novas_los_to_xyz(los.as_ptr(), lon.deg(), lat.deg(), out.as_mut_ptr()) };
    if rc != 0 {
        return Err(Error::ffi(rc));
    }
    Position::from_meters_array(out)
}

/// Convert a rectangular equatorial [`Position`] vector to a `(u, v, w)`
/// line-of-sight vector along the given direction. Inverse of [`los_to_xyz`].
/// Units pass through unchanged.
///
/// The position vector and `(lon, lat)` must be in the same coordinate
/// frame.
///
/// # Errors
///
/// Returns [`Error::Ffi`] if the C call fails.
pub fn xyz_to_los(pos: Position, lon: Angle, lat: Angle) -> Result<[f64; 3]> {
    let xyz = pos.as_meters();
    let mut out = [0.0_f64; 3];
    // SAFETY: novas_xyz_to_los writes 3 doubles into out on a zero return.
    let rc = unsafe { novas_xyz_to_los(xyz.as_ptr(), lon.deg(), lat.deg(), out.as_mut_ptr()) };
    if rc != 0 {
        return Err(Error::ffi(rc));
    }
    Ok(out)
}

impl Frame {
    /// The GCRS position and velocity of a geodetic [`Site`] at this frame's
    /// time, in meters and meters-per-second.
    ///
    /// Wraps `novas_site_gcrs_posvel`. Useful as the building block for the
    /// generic [`uvw`] projection when you want the array-reference station
    /// expressed in the same system as a GCRS phase center, or for
    /// interferometric delay calculations that need the geocentric station
    /// motion.
    ///
    /// Requires an Earth-bound observer; a geocentric frame returns
    /// [`Error::UnsupportedObserver`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedObserver`] if the frame's observer is not
    /// on Earth, or [`Error::Ffi`] if the C call fails.
    pub fn site_gcrs_posvel(&self, site: &Site) -> Result<(Position, Velocity)> {
        if self.observer_place() != NOVAS_OBSERVER_ON_EARTH {
            return Err(Error::UnsupportedObserver);
        }
        let on_surf = site.as_on_surface()?;
        let mut pos_au = [0.0_f64; 3];
        let mut vel_au_per_day = [0.0_f64; 3];
        let (xp, yp) = self.polar_motion_arcsec();
        // SAFETY: novas_site_gcrs_posvel writes 6 doubles (pos then vel) on
        // a zero return; the v_itrs pointer is NULL (no extra site motion).
        let rc = unsafe {
            novas_site_gcrs_posvel(
                self.as_timespec_ptr(),
                &raw const on_surf,
                core::ptr::null(),
                xp,
                yp,
                self.accuracy_sys(),
                pos_au.as_mut_ptr(),
                vel_au_per_day.as_mut_ptr(),
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok((
            Position::from_au(pos_au[0], pos_au[1], pos_au[2])?,
            Velocity::from_au_per_day(vel_au_per_day[0], vel_au_per_day[1], vel_au_per_day[2])?,
        ))
    }

    /// The GCRS apparent [`Position`] of a source at this frame's epoch.
    ///
    /// For solar-system sources the actual geometric distance is used; for
    /// sidereal sources (stars) the NOVAS convention of 1 gigaparsec is
    /// used, placing the source effectively at infinity. Pass the result as
    /// the `phase_center` argument to [`uvw`] and to
    /// [`Uvw::delay_rate`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Ffi`] if the underlying C calls fail.
    pub fn source_gcrs_position<S>(&self, source: &S) -> Result<Position>
    where
        S: Source + ?Sized,
    {
        let app = source.apparent_in(self, ReferenceSystem::Tod)?;
        let tod_rhat = app.r_hat();
        let dist_m = app.distance().m();
        let dist_m = if dist_m > 0.0 { dist_m } else { unit::GPC };
        let gcrs_dir = self
            .transform(ReferenceSystem::Tod, ReferenceSystem::Gcrs)?
            .apply_vector(tod_rhat)?;
        let n = (gcrs_dir[0].powi(2) + gcrs_dir[1].powi(2) + gcrs_dir[2].powi(2)).sqrt();
        Position::from_meters(
            gcrs_dir[0] / n * dist_m,
            gcrs_dir[1] / n * dist_m,
            gcrs_dir[2] / n * dist_m,
        )
    }
}

impl Site {
    /// Convert an ITRS [`Position`] to a local East-North-Up (ENU)
    /// [`Position`] at this site's longitude/latitude.
    ///
    /// Wraps `novas_itrs_to_enu`. Units pass through unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Ffi`] if the C call fails.
    pub fn itrs_to_enu(&self, itrs: Position) -> Result<Position> {
        let xyz = itrs.as_meters();
        let mut enu = [0.0_f64; 3];
        // SAFETY: novas_itrs_to_enu writes 3 doubles into enu on a zero return.
        let rc = unsafe {
            novas_itrs_to_enu(
                xyz.as_ptr(),
                self.longitude().deg(),
                self.latitude().deg(),
                enu.as_mut_ptr(),
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Position::from_meters_array(enu)
    }

    /// Convert a local East-North-Up (ENU) [`Position`] at this site to an
    /// ITRS [`Position`]. Inverse of [`Self::itrs_to_enu`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Ffi`] if the C call fails.
    pub fn enu_to_itrs(&self, enu: Position) -> Result<Position> {
        let xyz = enu.as_meters();
        let mut itrs = [0.0_f64; 3];
        // SAFETY: novas_enu_to_itrs writes 3 doubles into itrs on a zero return.
        let rc = unsafe {
            novas_enu_to_itrs(
                xyz.as_ptr(),
                self.longitude().deg(),
                self.latitude().deg(),
                itrs.as_mut_ptr(),
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Position::from_meters_array(itrs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Accuracy, CatalogEntry, Observer, Time, unit};

    /// Stellar distance in AU (1 GPC, matching NOVAS's convention for stars
    /// without parallax).
    const STAR_AU: f64 = unit::GPC / unit::AU;

    fn frame() -> Frame {
        let obs = Observer::geodetic(37.234, -118.282, 1222.0).unwrap();
        let t = Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap();
        Frame::new(Accuracy::Reduced, &obs, &t).unwrap()
    }

    fn site() -> Site {
        Site::from_degrees(37.234, -118.282, 1222.0).unwrap()
    }

    #[test]
    fn uvw_source_at_pole_ew_baseline_is_v() {
        let b = 100.0;
        let station = Position::from_meters(b / 2.0, 0.0, 0.0).unwrap();
        let pc = Position::from_au(0.0, 0.0, STAR_AU).unwrap();
        let out = uvw(&station, None, &pc).unwrap();
        assert!(
            (out.v().m() - b / 2.0).abs() < 1e-3,
            "v = {} m, expected {}",
            out.v().m(),
            b / 2.0
        );
        assert!(
            out.u().m().abs() < 1e-3,
            "u should be ~0, got {}",
            out.u().m()
        );
        assert!(
            out.w().m().abs() < 1e-3,
            "w should be ~0, got {}",
            out.w().m()
        );
    }

    #[test]
    fn uvw_source_on_x_axis_baseline_along_x_is_w() {
        let b = 100.0;
        let station = Position::from_meters(b / 2.0, 0.0, 0.0).unwrap();
        let pc = Position::from_au(STAR_AU, 0.0, 0.0).unwrap();
        let out = uvw(&station, None, &pc).unwrap();
        assert!(out.u().m().abs() < 1e-3, "u = {}", out.u().m());
        assert!(out.v().m().abs() < 1e-3, "v = {}", out.v().m());
        assert!(
            (out.w().m() - b / 2.0).abs() < 1e-3,
            "w = {} m, expected {}",
            out.w().m(),
            b / 2.0
        );
    }

    #[test]
    fn uvw_source_on_x_axis_baseline_along_y_is_u() {
        let b = 100.0;
        let station = Position::from_meters(0.0, b / 2.0, 0.0).unwrap();
        let pc = Position::from_au(STAR_AU, 0.0, 0.0).unwrap();
        let out = uvw(&station, None, &pc).unwrap();
        assert!(out.v().m().abs() < 1e-3, "v = {}", out.v().m());
        assert!(out.w().m().abs() < 1e-3, "w = {}", out.w().m());
        assert!(
            (out.u().m() - b / 2.0).abs() < 1e-3,
            "u = {} m, expected {}",
            out.u().m(),
            b / 2.0
        );
    }

    #[test]
    fn uvw_long_baseline_has_negligible_parallax() {
        // VLBI-scale baseline perpendicular to the source direction. With
        // the source at 1 GPC (stellar distance), w must be ~0.
        let b = 8.0e6;
        let station = Position::from_meters(0.0, b, 0.0).unwrap();
        let pc = Position::from_au(STAR_AU, 0.0, 0.0).unwrap();
        let out = uvw(&station, None, &pc).unwrap();
        assert!(
            out.w().m().abs() < 1.0e-3,
            "w = {} m, expected ~0 for {} m baseline",
            out.w().m(),
            b
        );
        assert!(
            (out.u().m() - b).abs() < 1.0e-3,
            "u = {} m, expected {} m",
            out.u().m(),
            b
        );
    }

    #[test]
    fn uvw_unit_vector_fails_on_long_baseline() {
        // Same geometry, but with a 1 AU position instead of a stellar
        // distance - demonstrates the parallax bug.
        let b = 8.0e6;
        let station = Position::from_meters(0.0, b, 0.0).unwrap();
        let pc = Position::from_au(1.0, 0.0, 0.0).unwrap();
        let out = uvw(&station, None, &pc).unwrap();
        assert!(
            out.w().m().abs() > 100.0,
            "w = {} m, expected >100 m with 1 AU position",
            out.w().m()
        );
    }

    #[test]
    fn uvw_station_vel_none_runs() {
        let station = Position::from_meters(10.0, 0.0, 0.0).unwrap();
        let pc = Position::from_au(0.0, 0.0, STAR_AU).unwrap();
        assert!(uvw(&station, None, &pc).is_ok());
    }

    #[test]
    fn uvw_rejects_non_finite() {
        let station = Position::from_meters(f64::NAN, 0.0, 0.0);
        assert!(station.is_err());
    }

    #[test]
    fn uvw_accessors_and_as_meters_round_trip() {
        let u = Uvw::from_meters_array([1.0, 2.0, 3.0]).unwrap();
        assert!((u.u().m() - 1.0).abs() < 1e-12);
        assert!((u.v().m() - 2.0).abs() < 1e-12);
        assert!((u.w().m() - 3.0).abs() < 1e-12);
        assert_eq!(u.as_meters(), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn uvw_delay_is_w_over_c() {
        let u = Uvw::from_meters_array([0.0, 0.0, unit::C]).unwrap();
        assert!((u.delay() - 1.0).abs() < 1e-6, "delay = {} s", u.delay());
        assert!(
            (u.delay_ns() - 1e9).abs() < 1e3,
            "delay_ns = {} ns",
            u.delay_ns()
        );
    }

    #[test]
    fn uvw_delay_rate_matches_manual_dot() {
        // A unit-direction scaled to stellar distance - normalization
        // inside delay_rate must recover the original direction.
        let star_au = unit::GPC / unit::AU;
        let phase_center = Position::from_au(0.6 * star_au, 0.8 * star_au, 0.0).unwrap();
        let vel = Velocity::from_mps(100.0, 0.0, 0.0).unwrap();
        let uvw = Uvw::from_meters_array([1.0, 2.0, 3.0]).unwrap();
        let rate = uvw.delay_rate(&phase_center, &vel);
        let expected = (100.0 * 0.6 + 0.0 * 0.8 + 0.0 * 0.0) / unit::C;
        assert!((rate - expected).abs() < 1e-15);
    }

    #[test]
    fn uvw_with_position_direction_works() {
        let b = 100.0;
        let station = Position::from_meters(0.0, b / 2.0, 0.0).unwrap();
        let pc = Position::from_au(0.0, 0.0, STAR_AU).unwrap();
        let out = uvw(&station, None, &pc).unwrap();
        assert!(out.u().m().abs() < 1e-3, "u = {}", out.u().m());
        assert!((out.v().m() - b / 2.0).abs() < 1e-3, "v = {}", out.v().m());
        assert!(out.w().m().abs() < 1e-3, "w = {}", out.w().m());
    }

    #[test]
    fn xyz_to_uvw_round_trips() {
        let pos = Position::from_meters(3.0, -4.0, 5.0).unwrap();
        let ha = TimeAngle::from_hours(2.5).unwrap();
        let dec = Angle::from_degrees(30.0).unwrap();
        let uvw = xyz_to_uvw(pos, ha, dec).unwrap();
        let back = uvw_to_xyz(uvw, ha, dec).unwrap();
        let a = pos.as_meters();
        let b = back.as_meters();
        for i in 0..3 {
            assert!((a[i] - b[i]).abs() < 1e-9, "round-trip mismatch");
        }
    }

    #[test]
    fn los_to_xyz_round_trips() {
        let los = [0.1, 0.2, 0.3];
        let lon = Angle::from_degrees(45.0).unwrap();
        let lat = Angle::from_degrees(-10.0).unwrap();
        let pos = los_to_xyz(los, lon, lat).unwrap();
        let back = xyz_to_los(pos, lon, lat).unwrap();
        for (a, b) in los.iter().zip(back.iter()) {
            assert!((a - b).abs() < 1e-9, "round-trip mismatch");
        }
    }

    #[test]
    fn los_to_xyz_at_lon0_lat0_r_axis_is_x() {
        let los = [0.0, 0.0, 5.0];
        let pos = los_to_xyz(
            los,
            Angle::from_degrees(0.0).unwrap(),
            Angle::from_degrees(0.0).unwrap(),
        )
        .unwrap();
        let xyz = pos.as_meters();
        assert!((xyz[0] - 5.0).abs() < 1e-9, "x = {}", xyz[0]);
        assert!(xyz[1].abs() < 1e-9);
        assert!(xyz[2].abs() < 1e-9);
    }

    #[test]
    fn site_itrs_enu_round_trip() {
        let s = site();
        let itrs = Position::from_meters(1000.0, -500.0, 200.0).unwrap();
        let enu = s.itrs_to_enu(itrs).unwrap();
        let back = s.enu_to_itrs(enu).unwrap();
        for (a, b) in itrs.as_meters().iter().zip(back.as_meters().iter()) {
            assert!((a - b).abs() < 1e-6, "ENU round-trip mismatch");
        }
    }

    #[test]
    fn site_itrs_to_enu_up_vector() {
        let s = site();
        let lat = s.latitude().deg().to_radians();
        let lon = s.longitude().deg().to_radians();
        let up_m = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
        let up = Position::from_meters_array(up_m).unwrap();
        let enu = s.itrs_to_enu(up).unwrap();
        let e = enu.as_meters();
        assert!(e[0].abs() < 1e-9, "E = {}", e[0]);
        assert!(e[1].abs() < 1e-9, "N = {}", e[1]);
        assert!((e[2] - 1.0).abs() < 1e-9, "U = {}", e[2]);
    }

    #[test]
    fn site_gcrs_posvel_is_finite_and_plausible() {
        let f = frame();
        let (pos, vel) = f.site_gcrs_posvel(&site()).unwrap();
        let mag = {
            let p = pos.as_meters();
            (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt()
        };
        assert!(
            mag > 6.0e6 && mag < 7.5e6,
            "geocentric site magnitude {mag} m"
        );
        assert!(vel.as_mps().iter().all(|v| v.is_finite()));
    }

    #[test]
    fn site_gcrs_posvel_refuses_geocenter() {
        let f = Frame::new(
            Accuracy::Reduced,
            &Observer::Geocenter,
            &Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            f.site_gcrs_posvel(&site()),
            Err(Error::UnsupportedObserver)
        ));
    }

    #[test]
    fn site_gcrs_posvel_feeds_generic_uvw() {
        let f = frame();
        let vega = CatalogEntry::icrs(
            "Vega",
            "18:36:56.336".parse().unwrap(),
            "+38:47:01.28".parse().unwrap(),
        )
        .unwrap();
        let (pos, vel) = f.site_gcrs_posvel(&site()).unwrap();
        let source_pos = f.source_gcrs_position(&vega).unwrap();
        let out = uvw(&pos, Some(&vel), &source_pos).unwrap();
        let m = {
            let v = out.as_meters();
            (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
        };
        assert!(m > 1.0e6 && m < 1.0e7, "geocentric UVW magnitude {m} m");
    }

    #[test]
    fn source_gcrs_position_is_stellar_distance() {
        let f = frame();
        let vega = CatalogEntry::icrs(
            "Vega",
            "18:36:56.336".parse().unwrap(),
            "+38:47:01.28".parse().unwrap(),
        )
        .unwrap();
        let pos = f.source_gcrs_position(&vega).unwrap();
        let dist = pos.distance().au();
        // Stars use the 1-GPC convention.
        let gpc_au = unit::GPC / unit::AU;
        assert!(
            (dist - gpc_au).abs() / gpc_au < 1e-9,
            "dist = {dist} AU, expected {gpc_au}"
        );
    }
}
