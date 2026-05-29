//! 3-D position vector in meters.

use core::{
    fmt,
    ops::{Add, Div, Mul, Neg, Sub},
};

use super::Velocity;
use crate::{
    Coordinate, Interval,
    error::{Error, Result},
    unit,
};

/// A 3-D position vector, stored internally as meters in a Cartesian frame.
///
/// Constructors reject non-finite components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position([f64; 3]);

impl Position {
    /// Construct from x, y, z in meters.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if any component is not finite.
    pub fn from_meters(x: f64, y: f64, z: f64) -> Result<Self> {
        Self::from_meters_array([x, y, z])
    }

    /// Construct from an `[x, y, z]` array in meters.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if any component is not finite.
    pub fn from_meters_array(c: [f64; 3]) -> Result<Self> {
        if c.iter().any(|v| !v.is_finite()) {
            return Err(Error::NotFinite);
        }
        Ok(Position(c))
    }

    /// Construct from x, y, z in astronomical units.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if any resulting component is not finite.
    pub fn from_au(x: f64, y: f64, z: f64) -> Result<Self> {
        Self::from_meters_array([x * unit::AU, y * unit::AU, z * unit::AU])
    }

    /// Construct from x, y, z in kilometers.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFinite`] if any resulting component is not finite.
    pub fn from_km(x: f64, y: f64, z: f64) -> Result<Self> {
        Self::from_meters_array([x * unit::KM, y * unit::KM, z * unit::KM])
    }

    /// Construct from three typed [`Coordinate`] components. Infallible: the
    /// inputs are already validated as finite.
    #[must_use]
    pub fn from_components(c: [Coordinate; 3]) -> Self {
        Position([c[0].m(), c[1].m(), c[2].m()])
    }

    /// The x component.
    ///
    /// # Panics
    ///
    /// Does not panic: a `Position`'s components are always finite (enforced
    /// at construction).
    #[must_use]
    pub fn x(self) -> Coordinate {
        Coordinate::from_meters(self.0[0]).expect("Position components are finite by construction")
    }

    /// The y component.
    ///
    /// # Panics
    ///
    /// Does not panic: a `Position`'s components are always finite (enforced
    /// at construction).
    #[must_use]
    pub fn y(self) -> Coordinate {
        Coordinate::from_meters(self.0[1]).expect("Position components are finite by construction")
    }

    /// The z component.
    ///
    /// # Panics
    ///
    /// Does not panic: a `Position`'s components are always finite (enforced
    /// at construction).
    #[must_use]
    pub fn z(self) -> Coordinate {
        Coordinate::from_meters(self.0[2]).expect("Position components are finite by construction")
    }

    /// The three components as typed [`Coordinate`] values.
    #[must_use]
    pub fn components(self) -> [Coordinate; 3] {
        [self.x(), self.y(), self.z()]
    }

    /// The raw `[x, y, z]` array in meters. Useful for FFI.
    #[must_use]
    pub fn as_meters(self) -> [f64; 3] {
        self.0
    }

    /// Distance from the origin (vector magnitude).
    ///
    /// # Panics
    ///
    /// Does not panic: the magnitude of a finite vector is finite.
    #[must_use]
    pub fn distance(self) -> Coordinate {
        let [x, y, z] = self.0;
        Coordinate::from_meters((x * x + y * y + z * z).sqrt())
            .expect("magnitude of a finite vector is finite")
    }

    /// Dot product with another position, in meters².
    #[must_use]
    pub fn dot(self, other: Position) -> f64 {
        let a = self.0;
        let b = other.0;
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    /// Negate (returns the position vector pointing the opposite direction).
    #[must_use]
    pub fn inv(self) -> Position {
        -self
    }
}

impl Add for Position {
    type Output = Position;
    fn add(self, rhs: Position) -> Position {
        let a = self.0;
        let b = rhs.0;
        Position([a[0] + b[0], a[1] + b[1], a[2] + b[2]])
    }
}

impl Sub for Position {
    type Output = Position;
    fn sub(self, rhs: Position) -> Position {
        let a = self.0;
        let b = rhs.0;
        Position([a[0] - b[0], a[1] - b[1], a[2] - b[2]])
    }
}

impl Neg for Position {
    type Output = Position;
    fn neg(self) -> Position {
        let a = self.0;
        Position([-a[0], -a[1], -a[2]])
    }
}

impl Mul<f64> for Position {
    type Output = Position;
    fn mul(self, factor: f64) -> Position {
        let a = self.0;
        Position([a[0] * factor, a[1] * factor, a[2] * factor])
    }
}

impl Mul<Position> for f64 {
    type Output = Position;
    fn mul(self, p: Position) -> Position {
        p * self
    }
}

impl Div<Interval> for Position {
    type Output = Velocity;

    /// Position divided by an interval gives an average velocity.
    fn div(self, dt: Interval) -> Velocity {
        let s = dt.seconds();
        let a = self.0;
        Velocity::from_mps_array([a[0] / s, a[1] / s, a[2] / s])
            .expect("finite components / finite interval -> finite velocity")
    }
}

impl fmt::Display for Position {
    /// Renders as `[x, y, z]` using each component's [`Coordinate`] display,
    /// which auto-scales its unit.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [x, y, z] = self.components();
        if let Some(p) = f.precision() {
            write!(f, "[{x:.p$}, {y:.p$}, {z:.p$}]")
        } else {
            write!(f, "[{x}, {y}, {z}]")
        }
    }
}

impl approx::AbsDiffEq for Position {
    type Epsilon = f64;

    /// Default tolerance: 1 meter, matching [`Coordinate`]'s default.
    fn default_epsilon() -> Self::Epsilon {
        1.0
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

    use crate::Timescale;

    use super::*;

    #[test]
    fn rejects_non_finite() {
        assert!(matches!(
            Position::from_meters(f64::NAN, 0.0, 0.0),
            Err(Error::NotFinite)
        ));
        assert!(matches!(
            Position::from_meters_array([0.0, f64::INFINITY, 0.0]),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn typed_accessors_round_trip() {
        let p = Position::from_meters(1.0, 2.0, 3.0).unwrap();
        assert!((p.x().m() - 1.0).abs() < 1e-12);
        assert!((p.y().m() - 2.0).abs() < 1e-12);
        assert!((p.z().m() - 3.0).abs() < 1e-12);
        assert_eq!(p.as_meters(), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn distance_is_magnitude() {
        let p = Position::from_meters(3.0, 4.0, 0.0).unwrap();
        assert!((p.distance().m() - 5.0).abs() < 1e-12);
    }

    #[test]
    fn arithmetic() {
        let p = Position::from_meters(1.0, 2.0, 3.0).unwrap();
        let q = Position::from_meters(4.0, 5.0, 6.0).unwrap();
        assert_abs_diff_eq!(
            p + q,
            Position::from_meters(5.0, 7.0, 9.0).unwrap(),
            epsilon = 1e-12
        );
        assert_abs_diff_eq!(
            p - q,
            Position::from_meters(-3.0, -3.0, -3.0).unwrap(),
            epsilon = 1e-12
        );
        assert_abs_diff_eq!(
            -p,
            Position::from_meters(-1.0, -2.0, -3.0).unwrap(),
            epsilon = 1e-12
        );
        assert_abs_diff_eq!(
            p * 2.0,
            Position::from_meters(2.0, 4.0, 6.0).unwrap(),
            epsilon = 1e-12
        );
        assert_abs_diff_eq!(
            3.0 * p,
            Position::from_meters(3.0, 6.0, 9.0).unwrap(),
            epsilon = 1e-12
        );
    }

    #[test]
    fn dot_product() {
        let p = Position::from_meters(1.0, 2.0, 3.0).unwrap();
        let q = Position::from_meters(4.0, -5.0, 6.0).unwrap();
        // 1·4 + 2·(-5) + 3·6 = 4 - 10 + 18 = 12
        assert!((p.dot(q) - 12.0).abs() < 1e-12);
    }

    #[test]
    fn divides_by_interval_to_velocity() {
        let p = Position::from_meters(300.0, -150.0, 60.0).unwrap();
        let dt = Interval::from_seconds(60.0, Timescale::Tt).unwrap();
        let v = p / dt;
        assert!((v.x().m_per_s() - 5.0).abs() < 1e-12);
        assert!((v.y().m_per_s() - -2.5).abs() < 1e-12);
        assert!((v.z().m_per_s() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn from_au_and_km() {
        let p = Position::from_au(1.0, 0.0, 0.0).unwrap();
        assert!((p.x().au() - 1.0).abs() < 1e-12);

        let p = Position::from_km(1.0, 2.0, 3.0).unwrap();
        assert!((p.x().km() - 1.0).abs() < 1e-12);
        assert!((p.z().km() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn from_components_and_components_getter() {
        let cx = Coordinate::from_km(1.0).unwrap();
        let cy = Coordinate::from_km(2.0).unwrap();
        let cz = Coordinate::from_km(3.0).unwrap();
        let p = Position::from_components([cx, cy, cz]);
        let out = p.components();
        assert!((out[0].km() - 1.0).abs() < 1e-12);
        assert!((out[1].km() - 2.0).abs() < 1e-12);
        assert!((out[2].km() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn inv_is_neg() {
        let p = Position::from_meters(1.0, -2.0, 3.0).unwrap();
        let neg = -p;
        assert_abs_diff_eq!(p.inv(), neg, epsilon = 1e-12);
    }

    #[test]
    fn display_shows_components() {
        // 10 000 km crosses the km display threshold (>= 1e9 m = 1e6 km).
        // Stay in AU range: 1 AU = ~1.496e11 m.
        let p = Position::from_au(1.0, 2.0, 3.0).unwrap();
        let s = format!("{p}");
        assert!(s.contains("AU"), "got: {s}");
    }
}
