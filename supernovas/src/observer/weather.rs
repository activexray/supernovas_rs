//! Local weather data, used for refraction at an observing site.
//!
//! All three fields are independently optional (any may be `None`); a `None`
//! field disables the refraction contribution that depends on it.

use core::fmt;

use crate::{
    Pressure, Temperature,
    error::{Error, Result},
};

/// Local atmospheric conditions for refraction calculations.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Weather {
    temperature: Option<Temperature>,
    pressure: Option<Pressure>,
    humidity_percent: Option<f64>,
}

impl Weather {
    /// Construct from optional temperature, pressure, and relative humidity
    /// (in percent, 0–100). Returns [`Error::NotFinite`] if a supplied
    /// humidity isn't finite.
    pub fn new(
        temperature: Option<Temperature>,
        pressure: Option<Pressure>,
        humidity_percent: Option<f64>,
    ) -> Result<Self> {
        if let Some(h) = humidity_percent
            && !h.is_finite()
        {
            return Err(Error::NotFinite);
        }
        Ok(Weather {
            temperature,
            pressure,
            humidity_percent,
        })
    }

    /// "Standard" atmosphere often used as a starting default: 15 °C,
    /// 1013.25 hPa, 50 % relative humidity.
    pub fn standard() -> Self {
        Weather {
            temperature: Some(Temperature::from_celsius(15.0).expect("15 °C is finite")),
            pressure: Some(Pressure::from_hpa(1013.25).expect("1013.25 hPa is finite")),
            humidity_percent: Some(50.0),
        }
    }

    /// Temperature at the site, if known.
    pub fn temperature(self) -> Option<Temperature> {
        self.temperature
    }

    /// Atmospheric pressure at the site, if known.
    pub fn pressure(self) -> Option<Pressure> {
        self.pressure
    }

    /// Relative humidity in percent (0–100), if known.
    pub fn humidity_percent(self) -> Option<f64> {
        self.humidity_percent
    }
}

impl fmt::Display for Weather {
    /// Renders fields that are set; "—" for fields that aren't.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let t = self.temperature.map_or_else(
            || alloc::string::String::from("—"),
            |t| alloc::format!("{t}"),
        );
        let p = self.pressure.map_or_else(
            || alloc::string::String::from("—"),
            |p| alloc::format!("{p}"),
        );
        let h = self.humidity_percent.map_or_else(
            || alloc::string::String::from("—"),
            |h| alloc::format!("{h:.0} %"),
        );
        write!(f, "T={t} P={p} RH={h}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_has_expected_values() {
        let w = Weather::standard();
        assert!((w.temperature().unwrap().celsius() - 15.0).abs() < 1e-12);
        assert!((w.pressure().unwrap().hpa() - 1013.25).abs() < 1e-12);
        assert!((w.humidity_percent().unwrap() - 50.0).abs() < 1e-12);
    }

    #[test]
    fn default_is_all_none() {
        let w = Weather::default();
        assert!(w.temperature().is_none());
        assert!(w.pressure().is_none());
        assert!(w.humidity_percent().is_none());
    }

    #[test]
    fn rejects_non_finite_humidity() {
        assert!(matches!(
            Weather::new(None, None, Some(f64::NAN)),
            Err(Error::NotFinite)
        ));
    }
}
