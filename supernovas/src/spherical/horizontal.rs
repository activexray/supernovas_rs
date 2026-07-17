//! Horizontal coordinates: azimuth and elevation as seen from a site.

use core::{f64::consts::FRAC_PI_2, fmt};

use supernovas_ffi::novas_offset_by;

use super::Spherical;
use crate::{
    Angle, Coordinate, Position,
    error::{Error, Result},
    unit,
};

/// A direction on the local sky, expressed as azimuth and elevation.
///
/// Azimuth is measured eastward from north along the horizon. Elevation is
/// measured upward from the horizon (positive above, negative below).
///
/// Obtain a `Horizontal` from a catalog source via [`crate::Frame::observe`]
/// (no refraction) or via [`crate::Apparent::to_horizontal_with_refraction`]
/// for atmosphere-corrected coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Horizontal(Spherical);

impl Horizontal {
    /// Construct from typed azimuth and elevation.
    #[must_use]
    pub fn new(azimuth: Angle, elevation: Angle) -> Self {
        Horizontal(Spherical::new(azimuth, elevation))
    }

    /// Construct from azimuth and elevation in radians.
    pub fn from_radians(azimuth: f64, elevation: f64) -> Result<Self> {
        Ok(Horizontal(Spherical::from_radians(azimuth, elevation)?))
    }

    /// Construct from azimuth and elevation in degrees.
    pub fn from_degrees(azimuth: f64, elevation: f64) -> Result<Self> {
        Ok(Horizontal(Spherical::from_degrees(azimuth, elevation)?))
    }

    /// Azimuth, measured eastward from north.
    ///
    /// The underlying [`Angle`] is stored in `(-180°, 180°]` (the invariant of
    /// `Angle`), so values in `(180°, 360°)` appear as negative. For the
    /// conventional `[0°, 360°)` display, use:
    ///
    /// ```
    /// # use supernovas::Horizontal;
    /// # let h = Horizontal::from_degrees(270.0, 45.0).unwrap();
    /// let az_deg = h.azimuth().deg().rem_euclid(360.0);
    /// ```
    #[must_use]
    pub fn azimuth(self) -> Angle {
        self.0.longitude()
    }

    /// Elevation (latitude analog).
    #[must_use]
    pub fn elevation(self) -> Angle {
        self.0.latitude()
    }

    /// Zenith angle: complement of the elevation, in `[0, π]`. Zero looks
    /// straight up, π/2 is on the horizon.
    #[must_use]
    pub fn zenith_angle(self) -> Angle {
        Angle::from_radians(FRAC_PI_2 - self.elevation().rad())
            .expect("FRAC_PI_2 minus a finite angle is finite")
    }

    /// The bare [`Spherical`] view for cases that don't care about the
    /// reference system.
    #[must_use]
    pub fn as_spherical(self) -> Spherical {
        self.0
    }

    /// Great-circle angular separation between this direction and `other`.
    #[must_use]
    pub fn distance_to(self, other: Horizontal) -> Angle {
        self.0.distance_to(other.0)
    }

    /// Offset along a great-circle arc.
    ///
    /// `direction` is measured from north through east (the standard position
    /// angle convention). `distance` is the great-circle angular distance to
    /// travel.
    ///
    /// Zero distance returns `self` unchanged. Returns an error at the poles
    /// (longitude is undefined) or on internal failure.
    pub fn offset(self, direction: Angle, distance: Angle) -> Result<Horizontal> {
        if distance.rad() == 0.0 {
            return Ok(self);
        }
        let mut out_lon = 0.0f64;
        let mut out_lat = 0.0f64;
        let rc = unsafe {
            novas_offset_by(
                self.azimuth().deg(),
                self.elevation().deg(),
                direction.deg(),
                distance.deg(),
                &raw mut out_lon,
                &raw mut out_lat,
            )
        };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Horizontal::from_degrees(out_lon, out_lat)
    }

    /// Offset by cross-elevation and elevation amounts in the sky frame.
    ///
    /// Converts `(xel, el)` to a great-circle `(direction, distance)` and
    /// delegates to [`Self::offset`]. The mapping is:
    ///
    /// ```text
    /// distance = acos(cos(xel) * cos(el))
    /// direction = atan2(sin(xel) * cos(el), sin(el))
    /// ```
    ///
    /// This makes it a drop-in replacement for the common ``offset_by_sky``
    /// / ``apply_sky_offset`` pattern used for telescope raster scans and
    /// pointing-model corrections.
    pub fn offset_by_sky(self, xel: Angle, el: Angle) -> Result<Horizontal> {
        let (x, y) = (xel.rad(), el.rad());
        let cos_d = (x.cos() * y.cos()).clamp(-1.0, 1.0);
        let distance = Angle::from_radians(cos_d.acos())
            .expect("acos of a clamped value in [-1, 1] is always finite");
        let direction = Angle::from_radians((y.cos() * x.sin()).atan2(y.sin()))
            .expect("atan2 of finite values is finite");
        self.offset(direction, distance)
    }

    /// Cartesian position at the given distance along this direction, in
    /// horizon-aligned axes (x toward north, y toward east, z toward zenith;
    /// but be careful: the underlying transform uses the spherical
    /// convention with x toward `lon=0, lat=0`).
    #[must_use]
    pub fn xyz(self, distance: Coordinate) -> Position {
        self.0.xyz(distance)
    }
}

impl fmt::Display for Horizontal {
    /// Renders azimuth (normalized to `[0°, 360°)`) and elevation as decimal
    /// degrees - the convention for observing logs and telescope control. Use
    /// `{:.N}` to control decimal places (default 3).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = f.precision().unwrap_or(3);
        write!(
            f,
            "az={:.decimals$}° el={:.decimals$}°",
            self.azimuth().deg().rem_euclid(360.0),
            self.elevation().deg()
        )
    }
}

impl approx::AbsDiffEq for Horizontal {
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        unit::UAS
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.0.abs_diff_eq(&other.0, epsilon)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn round_trip_degrees() {
        let h = Horizontal::from_degrees(180.0, 45.0).unwrap();
        assert!((h.azimuth().deg() - 180.0).abs() < 1e-12);
        assert!((h.elevation().deg() - 45.0).abs() < 1e-12);
    }

    #[test]
    fn zenith_angle_relates_to_elevation() {
        let zenith = Horizontal::from_degrees(0.0, 90.0).unwrap();
        assert!(zenith.zenith_angle().deg().abs() < 1e-12);

        let horizon = Horizontal::from_degrees(0.0, 0.0).unwrap();
        assert!((horizon.zenith_angle().deg() - 90.0).abs() < 1e-12);
    }

    #[test]
    fn approx_eq_across_azimuth_wrap() {
        let a = Horizontal::from_degrees(359.9999, 30.0).unwrap();
        let b = Horizontal::from_degrees(-0.0001, 30.0).unwrap();
        assert_abs_diff_eq!(a, b, epsilon = unit::ARCSEC);
    }

    #[test]
    fn from_radians_round_trip() {
        use core::f64::consts::PI;
        let h = Horizontal::from_radians(PI, PI / 4.0).unwrap();
        assert!((h.azimuth().rad() - PI).abs() < 1e-12);
        assert!((h.elevation().rad() - PI / 4.0).abs() < 1e-12);
    }

    #[test]
    fn as_spherical_gives_same_coords() {
        let h = Horizontal::from_degrees(90.0, 45.0).unwrap();
        let s = h.as_spherical();
        assert!((s.longitude().deg() - 90.0).abs() < 1e-12);
        assert!((s.latitude().deg() - 45.0).abs() < 1e-12);
    }

    #[test]
    fn xyz_has_finite_components() {
        let h = Horizontal::from_degrees(0.0, 45.0).unwrap();
        let d = crate::Coordinate::from_au(1.0).unwrap();
        let p = h.xyz(d);
        assert!(p.x().m().is_finite());
        assert!(p.y().m().is_finite());
        assert!(p.z().m().is_finite());
    }

    #[test]
    fn display_contains_az_el() {
        let h = Horizontal::from_degrees(270.0, 30.0).unwrap();
        let s = format!("{h}");
        assert!(s.contains("az=270"), "got: {s}");
        assert!(s.contains("el=30"), "got: {s}");
    }

    #[test]
    fn display_normalizes_azimuth_to_0_360() {
        // SuperNOVAS returns azimuths in [0°, 360°); Angle wraps to (-180°, 180°].
        // Display must re-normalize so west shows as 270°, not -90°.
        let h = Horizontal::from_degrees(-90.0, 10.0).unwrap(); // stored as -90°
        let s = format!("{h}");
        assert!(s.contains("az=270"), "expected az=270, got: {s}");
    }

    // ---------- offset tests ----------

    #[test]
    fn offset_zero_distance_is_identity() {
        let h = Horizontal::from_degrees(45.0, 30.0).unwrap();
        let h2 = h
            .offset(
                Angle::from_degrees(0.0).unwrap(),
                Angle::from_degrees(0.0).unwrap(),
            )
            .unwrap();
        assert_abs_diff_eq!(h, h2, epsilon = unit::UAS);
    }

    #[test]
    fn offset_pure_elevation() {
        let h = Horizontal::from_degrees(45.0, 30.0).unwrap();
        let h2 = h
            .offset(
                Angle::from_degrees(0.0).unwrap(),
                Angle::from_degrees(10.0).unwrap(),
            )
            .unwrap();
        assert_abs_diff_eq!(h2.azimuth().deg(), 45.0, epsilon = unit::UAS);
        assert_abs_diff_eq!(h2.elevation().deg(), 40.0, epsilon = unit::UAS);
    }

    #[test]
    fn offset_pure_cross_elevation() {
        let h = Horizontal::from_degrees(0.0, 45.0).unwrap();
        let h2 = h
            .offset(
                Angle::from_degrees(90.0).unwrap(),
                Angle::from_degrees(5.0).unwrap(),
            )
            .unwrap();
        // At az=0, el=45, a 5° eastward offset gives some azimuth east
        assert!(h2.azimuth().deg() > 0.0);
        // Elevation should decrease slightly (moving east from north at 45° el
        // curves down toward the horizon for large offsets)
        assert!(h2.elevation().deg() < 45.0);
        assert!(h2.elevation().deg() > 40.0);
    }

    #[test]
    fn offset_by_sky_zero_is_identity() {
        let h = Horizontal::from_degrees(45.0, 30.0).unwrap();
        let zero = Angle::from_degrees(0.0).unwrap();
        let h2 = h.offset_by_sky(zero, zero).unwrap();
        assert_abs_diff_eq!(h, h2, epsilon = unit::UAS);
    }

    #[test]
    fn offset_by_sky_pure_el() {
        let h = Horizontal::from_degrees(0.0, 30.0).unwrap();
        let h2 = h
            .offset_by_sky(
                Angle::from_degrees(0.0).unwrap(),
                Angle::from_degrees(5.0).unwrap(),
            )
            .unwrap();
        assert_abs_diff_eq!(h2.azimuth().deg(), 0.0, epsilon = unit::UAS);
        assert_abs_diff_eq!(h2.elevation().deg(), 35.0, epsilon = unit::UAS);
    }

    #[test]
    fn offset_by_sky_pure_xel() {
        let h = Horizontal::from_degrees(0.0, 30.0).unwrap();
        let h2 = h
            .offset_by_sky(
                Angle::from_degrees(5.0).unwrap(),
                Angle::from_degrees(0.0).unwrap(),
            )
            .unwrap();
        // 5° cross-elevation east from north at el=30°
        assert!(h2.azimuth().deg() > 0.0);
    }

    #[test]
    fn offset_by_sky_compound_matches_direct() {
        let h = Horizontal::from_degrees(0.0, 30.0).unwrap();
        // (xel, el) = (5°, 5°) should produce same result whether decomposed
        // via offset_by_sky or computed manually and fed to offset.
        let xel = Angle::from_degrees(5.0).unwrap();
        let el = Angle::from_degrees(5.0).unwrap();

        let via_sky = h.offset_by_sky(xel, el).unwrap();

        let (x, y) = (xel.rad(), el.rad());
        let cos_d = (x.cos() * y.cos()).clamp(-1.0, 1.0);
        let distance = Angle::from_radians(cos_d.acos()).unwrap();
        let direction = Angle::from_radians((y.cos() * x.sin()).atan2(y.sin())).unwrap();
        let via_manual = h.offset(direction, distance).unwrap();

        assert_abs_diff_eq!(via_sky, via_manual, epsilon = unit::UAS);
    }

    #[test]
    fn offset_by_sky_distance_correct() {
        let h = Horizontal::from_degrees(123.0, 45.0).unwrap();
        let xel = Angle::from_degrees(7.0).unwrap();
        let el = Angle::from_degrees(12.0).unwrap();

        let fwd = h.offset_by_sky(xel, el).unwrap();

        let expected_dist_rad = (xel.rad().cos() * el.rad().cos()).acos();
        assert_abs_diff_eq!(
            h.distance_to(fwd).rad(),
            expected_dist_rad,
            epsilon = unit::UAS,
        );
    }

    #[test]
    fn offset_by_sky_azimuth_direction() {
        // Starting from az=0 (north), pure cross-elevation offsets
        // should move eastward (positive azimuth).
        let h = Horizontal::from_degrees(0.0, 30.0).unwrap();
        let fwd = h
            .offset_by_sky(
                Angle::from_degrees(10.0).unwrap(),
                Angle::from_degrees(0.0).unwrap(),
            )
            .unwrap();
        assert!(fwd.azimuth().deg() > 0.0);
        // Pure elevation offset should NOT change azimuth.
        let fwd = h
            .offset_by_sky(
                Angle::from_degrees(0.0).unwrap(),
                Angle::from_degrees(10.0).unwrap(),
            )
            .unwrap();
        assert_abs_diff_eq!(fwd.azimuth().deg(), 0.0, epsilon = unit::UAS);
    }

    #[test]
    fn offset_near_pole_errors() {
        let h = Horizontal::from_degrees(0.0, 90.0).unwrap();
        let result = h.offset(
            Angle::from_degrees(0.0).unwrap(),
            Angle::from_degrees(1.0).unwrap(),
        );
        assert!(result.is_err());
    }
}
