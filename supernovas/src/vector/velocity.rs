//! 3-D velocity vector in meters per second.

use core::{
    fmt,
    ops::{Add, Mul, Neg, Sub},
};

use super::Position;
use crate::{
    Interval, ScalarVelocity,
    error::{Error, Result},
    unit,
};

/// A 3-D velocity vector, stored internally as meters per second.
///
/// Constructors reject non-finite components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Velocity([f64; 3]);

impl Velocity {
    /// Construct from x, y, z in meters per second.
    pub fn from_mps(x: f64, y: f64, z: f64) -> Result<Self> {
        Self::from_mps_array([x, y, z])
    }

    /// Construct from an `[x, y, z]` array in meters per second.
    pub fn from_mps_array(c: [f64; 3]) -> Result<Self> {
        if c.iter().any(|v| !v.is_finite()) {
            return Err(Error::NotFinite);
        }
        Ok(Velocity(c))
    }

    /// Construct from x, y, z in kilometers per second.
    pub fn from_km_per_s(x: f64, y: f64, z: f64) -> Result<Self> {
        Self::from_mps_array([x * unit::KM_PER_S, y * unit::KM_PER_S, z * unit::KM_PER_S])
    }

    /// Construct from x, y, z in astronomical units per day.
    pub fn from_au_per_day(x: f64, y: f64, z: f64) -> Result<Self> {
        Self::from_mps_array([
            x * unit::AU_PER_DAY,
            y * unit::AU_PER_DAY,
            z * unit::AU_PER_DAY,
        ])
    }

    /// Construct from three typed [`ScalarVelocity`] components. Infallible:
    /// the inputs are already validated as finite.
    #[must_use]
    pub fn from_components(c: [ScalarVelocity; 3]) -> Self {
        Velocity([c[0].m_per_s(), c[1].m_per_s(), c[2].m_per_s()])
    }

    /// The x component.
    #[must_use]
    pub fn x(self) -> ScalarVelocity {
        ScalarVelocity::from_m_per_s(self.0[0])
            .expect("Velocity components are finite by construction")
    }

    /// The y component.
    #[must_use]
    pub fn y(self) -> ScalarVelocity {
        ScalarVelocity::from_m_per_s(self.0[1])
            .expect("Velocity components are finite by construction")
    }

    /// The z component.
    #[must_use]
    pub fn z(self) -> ScalarVelocity {
        ScalarVelocity::from_m_per_s(self.0[2])
            .expect("Velocity components are finite by construction")
    }

    /// The three components as typed [`ScalarVelocity`] values.
    #[must_use]
    pub fn components(self) -> [ScalarVelocity; 3] {
        [self.x(), self.y(), self.z()]
    }

    /// The raw `[x, y, z]` array in meters per second. Useful for FFI.
    #[must_use]
    pub fn as_mps(self) -> [f64; 3] {
        self.0
    }

    /// Speed (vector magnitude).
    #[must_use]
    pub fn speed(self) -> ScalarVelocity {
        let [x, y, z] = self.0;
        ScalarVelocity::from_m_per_s((x * x + y * y + z * z).sqrt())
            .expect("magnitude of a finite vector is finite")
    }

    /// Dot product with another velocity, in (m/s)².
    #[must_use]
    pub fn dot(self, other: Velocity) -> f64 {
        let a = self.0;
        let b = other.0;
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    /// Negate (returns the velocity pointing the opposite direction).
    #[must_use]
    pub fn inv(self) -> Velocity {
        -self
    }

    /// The position reached after traveling for the given interval.
    #[must_use]
    pub fn travel(self, dt: Interval) -> Position {
        self * dt
    }
}

impl Add for Velocity {
    type Output = Velocity;
    fn add(self, rhs: Velocity) -> Velocity {
        let a = self.0;
        let b = rhs.0;
        Velocity([a[0] + b[0], a[1] + b[1], a[2] + b[2]])
    }
}

impl Sub for Velocity {
    type Output = Velocity;
    fn sub(self, rhs: Velocity) -> Velocity {
        let a = self.0;
        let b = rhs.0;
        Velocity([a[0] - b[0], a[1] - b[1], a[2] - b[2]])
    }
}

impl Neg for Velocity {
    type Output = Velocity;
    fn neg(self) -> Velocity {
        let a = self.0;
        Velocity([-a[0], -a[1], -a[2]])
    }
}

impl Mul<f64> for Velocity {
    type Output = Velocity;
    fn mul(self, factor: f64) -> Velocity {
        let a = self.0;
        Velocity([a[0] * factor, a[1] * factor, a[2] * factor])
    }
}

impl Mul<Velocity> for f64 {
    type Output = Velocity;
    fn mul(self, v: Velocity) -> Velocity {
        v * self
    }
}

impl Mul<Interval> for Velocity {
    type Output = Position;

    /// Velocity multiplied by an interval gives the displacement vector.
    fn mul(self, dt: Interval) -> Position {
        let s = dt.seconds();
        let a = self.0;
        Position::from_meters_array([a[0] * s, a[1] * s, a[2] * s])
            .expect("finite components × finite interval -> finite position")
    }
}

impl Mul<Velocity> for Interval {
    type Output = Position;

    /// Symmetric overload so `dt * v` reads as naturally as `v * dt`.
    fn mul(self, v: Velocity) -> Position {
        v * self
    }
}

impl fmt::Display for Velocity {
    /// Renders as `[x, y, z]` with each component as km/s.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [x, y, z] = self.components();
        if let Some(p) = f.precision() {
            write!(f, "[{x:.p$}, {y:.p$}, {z:.p$}]")
        } else {
            write!(f, "[{x}, {y}, {z}]")
        }
    }
}

impl approx::AbsDiffEq for Velocity {
    type Epsilon = f64;

    /// Default tolerance: 1 mm/s, matching [`ScalarVelocity`]'s default.
    fn default_epsilon() -> Self::Epsilon {
        unit::MM / unit::SEC
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.0
            .iter()
            .zip(other.0.iter())
            .all(|(a, b)| (a - b).abs() <= epsilon)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;
    use crate::Timescale;

    #[test]
    fn rejects_non_finite() {
        assert!(matches!(
            Velocity::from_mps(f64::NAN, 0.0, 0.0),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn typed_accessors_round_trip() {
        let v = Velocity::from_mps(1.0, 2.0, 3.0).unwrap();
        assert!((v.x().m_per_s() - 1.0).abs() < 1e-12);
        assert!((v.y().m_per_s() - 2.0).abs() < 1e-12);
        assert!((v.z().m_per_s() - 3.0).abs() < 1e-12);
        assert_eq!(v.as_mps(), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn speed_is_magnitude() {
        let v = Velocity::from_mps(6.0, 8.0, 0.0).unwrap();
        assert!((v.speed().m_per_s() - 10.0).abs() < 1e-12);
    }

    #[test]
    fn arithmetic() {
        let v = Velocity::from_mps(1.0, 2.0, 3.0).unwrap();
        let w = Velocity::from_mps(4.0, 5.0, 6.0).unwrap();
        assert_abs_diff_eq!(
            v + w,
            Velocity::from_mps(5.0, 7.0, 9.0).unwrap(),
            epsilon = 1e-12
        );
        assert_abs_diff_eq!(
            -v,
            Velocity::from_mps(-1.0, -2.0, -3.0).unwrap(),
            epsilon = 1e-12
        );
        assert_abs_diff_eq!(
            v * 2.0,
            Velocity::from_mps(2.0, 4.0, 6.0).unwrap(),
            epsilon = 1e-12
        );
    }

    #[test]
    fn multiplying_by_interval_gives_position() {
        let v = Velocity::from_mps(5.0, -2.5, 1.0).unwrap();
        let dt = Interval::from_seconds(60.0, Timescale::Tt).unwrap();
        let p = v * dt;
        assert!((p.x().m() - 300.0).abs() < 1e-12);
        assert!((p.y().m() - -150.0).abs() < 1e-12);
        assert!((p.z().m() - 60.0).abs() < 1e-12);
        // And the reverse multiplication also works.
        let p2 = dt * v;
        assert_abs_diff_eq!(p, p2, epsilon = 1e-12);
        // And v.travel(dt) is equivalent.
        assert_abs_diff_eq!(p, v.travel(dt), epsilon = 1e-12);
    }

    #[test]
    fn round_trips_through_position() {
        let v = Velocity::from_mps(5.0, -2.5, 1.0).unwrap();
        let dt = Interval::from_seconds(60.0, Timescale::Tt).unwrap();
        let p = v * dt;
        let back = p / dt;
        assert_abs_diff_eq!(v, back, epsilon = 1e-12);
    }

    #[test]
    fn from_km_per_s_round_trip() {
        let v = Velocity::from_km_per_s(1.0, 2.0, 3.0).unwrap();
        assert!((v.x().km_per_s() - 1.0).abs() < 1e-12);
        assert!((v.y().km_per_s() - 2.0).abs() < 1e-12);
        assert!((v.z().km_per_s() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn from_au_per_day_round_trip() {
        let v = Velocity::from_au_per_day(1.0, 0.0, 0.0).unwrap();
        assert!((v.x().au_per_day() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn from_components_round_trip() {
        let cx = ScalarVelocity::from_km_per_s(1.0).unwrap();
        let cy = ScalarVelocity::from_km_per_s(2.0).unwrap();
        let cz = ScalarVelocity::from_km_per_s(3.0).unwrap();
        let v = Velocity::from_components([cx, cy, cz]);
        assert!((v.x().km_per_s() - 1.0).abs() < 1e-12);
        let comps = v.components();
        assert!((comps[1].km_per_s() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn dot_product() {
        let a = Velocity::from_mps(1.0, 0.0, 0.0).unwrap();
        let b = Velocity::from_mps(0.0, 1.0, 0.0).unwrap();
        assert!((a.dot(b)).abs() < 1e-12);

        let c = Velocity::from_mps(2.0, 3.0, 4.0).unwrap();
        let d = Velocity::from_mps(1.0, 1.0, 1.0).unwrap();
        assert!((c.dot(d) - 9.0).abs() < 1e-12);
    }

    #[test]
    fn inv_is_neg() {
        let v = Velocity::from_mps(1.0, -2.0, 3.0).unwrap();
        let neg = -v;
        assert_abs_diff_eq!(v.inv(), neg, epsilon = 1e-12);
    }

    #[test]
    fn subtraction() {
        let a = Velocity::from_mps(5.0, 3.0, 1.0).unwrap();
        let b = Velocity::from_mps(2.0, 1.0, 0.0).unwrap();
        assert_abs_diff_eq!(
            a - b,
            Velocity::from_mps(3.0, 2.0, 1.0).unwrap(),
            epsilon = 1e-12
        );
    }

    #[test]
    fn scalar_mul_commutes() {
        let v = Velocity::from_mps(1.0, 2.0, 3.0).unwrap();
        assert_abs_diff_eq!(v * 3.0, 3.0 * v, epsilon = 1e-12);
    }

    #[test]
    fn display_shows_km_per_s() {
        let v = Velocity::from_km_per_s(1.0, 2.0, 3.0).unwrap();
        let s = format!("{v}");
        assert!(s.contains("km/s"), "got: {s}");
    }
}
