//! 1-D distance coordinate (meters).

use core::fmt;

use super::Angle;
use crate::{
    error::{Error, Result},
    unit,
};

/// A 1-D distance, stored internally as meters.
///
/// Used for things like distance to a star, or a single-axis displacement.
/// Constructors reject non-finite inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coordinate(f64);

impl Coordinate {
    /// Construct from meters.
    pub fn from_meters(m: f64) -> Result<Self> {
        if !m.is_finite() {
            return Err(Error::NotFinite);
        }
        Ok(Coordinate(m))
    }

    /// Construct from kilometers.
    pub fn from_km(km: f64) -> Result<Self> {
        Self::from_meters(km * unit::KM)
    }

    /// Construct from astronomical units.
    pub fn from_au(au: f64) -> Result<Self> {
        Self::from_meters(au * unit::AU)
    }

    /// Construct from light-years.
    pub fn from_lyr(lyr: f64) -> Result<Self> {
        Self::from_meters(lyr * unit::LYR)
    }

    /// Construct from parsecs.
    pub fn from_pc(pc: f64) -> Result<Self> {
        Self::from_meters(pc * unit::PC)
    }

    /// Construct from kiloparsecs.
    pub fn from_kpc(kpc: f64) -> Result<Self> {
        Self::from_meters(kpc * unit::KPC)
    }

    /// Construct from megaparsecs.
    pub fn from_mpc(mpc: f64) -> Result<Self> {
        Self::from_meters(mpc * unit::MPC)
    }

    /// Construct from gigaparsecs.
    pub fn from_gpc(gpc: f64) -> Result<Self> {
        Self::from_meters(gpc * unit::GPC)
    }

    /// Construct from an annual parallax angle: `d = 1 AU / tan(parallax)`.
    pub fn from_parallax(parallax: Angle) -> Result<Self> {
        let p = parallax.rad();
        if p == 0.0 {
            return Err(Error::NotFinite);
        }
        Self::from_meters(unit::AU / p.tan())
    }

    /// The distance in meters.
    pub fn m(self) -> f64 {
        self.0
    }

    /// The distance in kilometers.
    pub fn km(self) -> f64 {
        self.0 / unit::KM
    }

    /// The distance in astronomical units.
    pub fn au(self) -> f64 {
        self.0 / unit::AU
    }

    /// The distance in light-years.
    pub fn lyr(self) -> f64 {
        self.0 / unit::LYR
    }

    /// The distance in parsecs.
    pub fn pc(self) -> f64 {
        self.0 / unit::PC
    }

    /// The distance in kiloparsecs.
    pub fn kpc(self) -> f64 {
        self.0 / unit::KPC
    }

    /// The distance in megaparsecs.
    pub fn mpc(self) -> f64 {
        self.0 / unit::MPC
    }

    /// The distance in gigaparsecs.
    pub fn gpc(self) -> f64 {
        self.0 / unit::GPC
    }

    /// Absolute magnitude.
    pub fn abs(self) -> Coordinate {
        Coordinate(self.0.abs())
    }

    /// The annual parallax angle implied by this distance.
    pub fn parallax(self) -> Result<Angle> {
        if self.0 == 0.0 {
            return Err(Error::NotFinite);
        }
        Angle::from_radians((unit::AU / self.0).atan())
    }
}

impl fmt::Display for Coordinate {
    /// Auto-scales the unit by magnitude: m → km → AU → pc → kpc → Mpc → Gpc.
    /// Use `{:.N}` to control decimal places (default 3).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = f.precision().unwrap_or(3);
        let d = self.0.abs();
        let (value, unit_label) = if d < 1e4 {
            (self.0, "m")
        } else if d < 1e9 {
            (self.km(), "km")
        } else if d < 1e3 * unit::AU {
            (self.au(), "AU")
        } else if d < 1e3 * unit::PC {
            (self.pc(), "pc")
        } else if d < 1e6 * unit::PC {
            (self.kpc(), "kpc")
        } else if d < 1e9 * unit::PC {
            (self.mpc(), "Mpc")
        } else {
            (self.gpc(), "Gpc")
        };
        write!(f, "{value:.decimals$} {unit_label}")
    }
}

impl approx::AbsDiffEq for Coordinate {
    type Epsilon = f64;

    /// Default tolerance: 1 meter.
    fn default_epsilon() -> Self::Epsilon {
        1.0
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        (self.0 - other.0).abs() <= epsilon
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_finite() {
        assert!(matches!(
            Coordinate::from_meters(f64::NAN),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn round_trip_au() {
        let d = Coordinate::from_au(1.0).unwrap();
        assert!((d.m() - 1.495978707e11).abs() < 1.0);
        assert!((d.au() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn parsec_definition() {
        // By definition, 1 pc subtends 1 arc-second at 1 AU baseline.
        let one_pc = Coordinate::from_pc(1.0).unwrap();
        let p = one_pc.parallax().unwrap();
        assert!((p.arcsec() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn parallax_round_trip() {
        let arcsec = Angle::from_arcsec(0.5).unwrap();
        let d = Coordinate::from_parallax(arcsec).unwrap();
        assert!((d.pc() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn all_unit_constructors_and_getters() {
        let d = Coordinate::from_km(1.0).unwrap();
        assert!((d.km() - 1.0).abs() < 1e-12);

        let d = Coordinate::from_lyr(1.0).unwrap();
        assert!((d.lyr() - 1.0).abs() < 1e-9);

        let d = Coordinate::from_pc(1.0).unwrap();
        assert!((d.pc() - 1.0).abs() < 1e-12);

        let d = Coordinate::from_kpc(1.0).unwrap();
        assert!((d.kpc() - 1.0).abs() < 1e-12);

        let d = Coordinate::from_mpc(1.0).unwrap();
        assert!((d.mpc() - 1.0).abs() < 1e-12);

        let d = Coordinate::from_gpc(1.0).unwrap();
        assert!((d.gpc() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn abs_of_negative() {
        let d = Coordinate::from_meters(-100.0).unwrap();
        assert!((d.abs().m() - 100.0).abs() < 1e-12);
    }

    #[test]
    fn parallax_zero_distance_is_err() {
        let zero = Coordinate::from_meters(0.0).unwrap();
        assert!(matches!(zero.parallax(), Err(Error::NotFinite)));
    }

    #[test]
    fn display_auto_scales() {
        let m = Coordinate::from_meters(500.0).unwrap();
        assert!(format!("{m}").contains('m'));

        let km = Coordinate::from_km(1000.0).unwrap();
        assert!(format!("{km}").contains("km"));

        let au = Coordinate::from_au(10.0).unwrap();
        assert!(format!("{au}").contains("AU"));

        let pc = Coordinate::from_pc(5.0).unwrap();
        assert!(format!("{pc}").contains("pc"));

        let kpc = Coordinate::from_kpc(5.0).unwrap();
        assert!(format!("{kpc}").contains("kpc"));

        let mpc = Coordinate::from_mpc(5.0).unwrap();
        assert!(format!("{mpc}").contains("Mpc"));

        let gpc = Coordinate::from_gpc(5.0).unwrap();
        assert!(format!("{gpc}").contains("Gpc"));
    }
}
