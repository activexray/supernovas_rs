use core::{
    fmt::{Debug, Formatter},
    mem::MaybeUninit,
};

use supernovas_ffi::{
    make_planet,
    novas_planet::{
        NOVAS_EARTH, NOVAS_EMB, NOVAS_JUPITER, NOVAS_MARS, NOVAS_MERCURY, NOVAS_MOON,
        NOVAS_NEPTUNE, NOVAS_PLUTO, NOVAS_PLUTO_BARYCENTER, NOVAS_SATURN, NOVAS_SSB, NOVAS_SUN,
        NOVAS_URANUS, NOVAS_VENUS,
    },
    object,
};

use crate::error::{Error, Result};

/// Identifies a major solar-system body (planet, Sun, Moon, or barycenter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SolarBody {
    Mercury,
    Venus,
    Earth,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
    Pluto,
    Sun,
    Moon,
    SolarSystemBarycenter,
    EarthMoonBarycenter,
    PlutoBarycenter,
}

impl SolarBody {
    fn to_ffi(self) -> supernovas_ffi::novas_planet {
        match self {
            SolarBody::SolarSystemBarycenter => NOVAS_SSB,
            SolarBody::Mercury => NOVAS_MERCURY,
            SolarBody::Venus => NOVAS_VENUS,
            SolarBody::Earth => NOVAS_EARTH,
            SolarBody::Mars => NOVAS_MARS,
            SolarBody::Jupiter => NOVAS_JUPITER,
            SolarBody::Saturn => NOVAS_SATURN,
            SolarBody::Uranus => NOVAS_URANUS,
            SolarBody::Neptune => NOVAS_NEPTUNE,
            SolarBody::Pluto => NOVAS_PLUTO,
            SolarBody::Sun => NOVAS_SUN,
            SolarBody::Moon => NOVAS_MOON,
            SolarBody::EarthMoonBarycenter => NOVAS_EMB,
            SolarBody::PlutoBarycenter => NOVAS_PLUTO_BARYCENTER,
        }
    }
}

/// A major solar-system body as a sky source.
///
/// Apparent positions are computed via the installed planet-ephemeris
/// provider. Configure one before observing at `Accuracy::Full`:
/// - `CalcephEphemeris` (`calceph` feature)
/// - `AniseEphemeris` (`anise` feature)
///
/// At `Accuracy::Reduced` `SuperNOVAS` uses built-in low-precision
/// approximations and no external provider is needed.
#[derive(Clone, Copy)]
#[allow(clippy::missing_fields_in_debug)]
pub struct Planet {
    body: SolarBody,
    object: object,
}

impl Debug for Planet {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Planet")
            .field("body", &self.body)
            .finish_non_exhaustive()
    }
}

impl super::sealed::Sealed for Planet {}
impl super::Source for Planet {
    fn as_object(&self) -> &object {
        &self.object
    }
}

impl Planet {
    /// Construct a source for the given solar-system body.
    pub fn new(body: SolarBody) -> Result<Self> {
        let mut obj = MaybeUninit::<object>::zeroed();
        // SAFETY: make_planet fully initializes *obj on a zero return.
        let rc = unsafe { make_planet(body.to_ffi(), obj.as_mut_ptr()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(Planet {
            body,
            object: unsafe { obj.assume_init() },
        })
    }

    /// The solar-system body this source represents.
    #[must_use]
    pub fn body(&self) -> SolarBody {
        self.body
    }

    pub fn mercury() -> Result<Self> {
        Self::new(SolarBody::Mercury)
    }
    pub fn venus() -> Result<Self> {
        Self::new(SolarBody::Venus)
    }
    pub fn earth() -> Result<Self> {
        Self::new(SolarBody::Earth)
    }
    pub fn mars() -> Result<Self> {
        Self::new(SolarBody::Mars)
    }
    pub fn jupiter() -> Result<Self> {
        Self::new(SolarBody::Jupiter)
    }
    pub fn saturn() -> Result<Self> {
        Self::new(SolarBody::Saturn)
    }
    pub fn uranus() -> Result<Self> {
        Self::new(SolarBody::Uranus)
    }
    pub fn neptune() -> Result<Self> {
        Self::new(SolarBody::Neptune)
    }
    pub fn pluto() -> Result<Self> {
        Self::new(SolarBody::Pluto)
    }
    pub fn sun() -> Result<Self> {
        Self::new(SolarBody::Sun)
    }
    pub fn moon() -> Result<Self> {
        Self::new(SolarBody::Moon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_convenience_constructors_succeed() {
        Planet::mercury().unwrap();
        Planet::venus().unwrap();
        Planet::earth().unwrap();
        Planet::mars().unwrap();
        Planet::jupiter().unwrap();
        Planet::saturn().unwrap();
        Planet::uranus().unwrap();
        Planet::neptune().unwrap();
        Planet::pluto().unwrap();
        Planet::sun().unwrap();
        Planet::moon().unwrap();
    }

    #[test]
    fn new_covers_all_solar_bodies() {
        for body in [
            SolarBody::SolarSystemBarycenter,
            SolarBody::EarthMoonBarycenter,
            SolarBody::PlutoBarycenter,
        ] {
            Planet::new(body).unwrap();
        }
    }

    #[test]
    fn body_getter_round_trips() {
        let p = Planet::new(SolarBody::Mars).unwrap();
        assert_eq!(p.body(), SolarBody::Mars);
    }

    #[test]
    fn debug_format_shows_body() {
        let p = Planet::new(SolarBody::Jupiter).unwrap();
        let s = format!("{p:?}");
        assert!(s.contains("Jupiter"), "got: {s}");
    }
}
