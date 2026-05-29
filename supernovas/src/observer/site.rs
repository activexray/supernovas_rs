//! Geodetic site: a fixed location on Earth's surface.
//!
//! A `Site` carries the geodetic (ITRS/GRS80) position of an observatory
//! plus optional local weather for refraction.

use core::fmt::{Display, Formatter};

use supernovas_ffi::novas_on_surface;

use super::Weather;
use crate::{
    Angle, Coordinate, Pressure, Temperature,
    error::{Error, Result},
};

/// A fixed observing location on Earth's surface (geodetic).
///
/// Latitude is north-positive; longitude is east-positive. `Site::new`
/// accepts already-validated typed angles; the `from_degrees` shortcut
/// accepts plain `f64` for convenience.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Site {
    latitude: Angle,
    longitude: Angle,
    height: Coordinate,
    weather: Weather,
}

impl Site {
    /// Construct from already-validated typed angles + height. Weather is
    /// initialised empty; add it via [`Self::with_weather`].
    #[must_use]
    pub fn new(latitude: Angle, longitude: Angle, height: Coordinate) -> Self {
        Site {
            latitude,
            longitude,
            height,
            weather: Weather::default(),
        }
    }

    /// Construct from latitude/longitude in degrees and height in meters.
    ///
    /// Latitude must lie in `[-90°, 90°]`. Longitude is unrestricted (it is
    /// inherently modular).
    ///
    /// # Errors
    /// Returns [`Error::OutOfRange`] for an out-of-range
    /// latitude and [`Error::NotFinite`] for non-finite inputs.
    pub fn from_degrees(latitude_deg: f64, longitude_deg: f64, height_m: f64) -> Result<Self> {
        // Validate the raw input before `Angle` folds it into (-180°, 180°],
        // which would silently turn an impossible latitude like 200° into a
        // plausible-looking -160°.
        if latitude_deg.is_finite() && !(-90.0..=90.0).contains(&latitude_deg) {
            return Err(Error::OutOfRange("geodetic latitude"));
        }
        Ok(Site::new(
            Angle::from_degrees(latitude_deg)?,
            Angle::from_degrees(longitude_deg)?,
            Coordinate::from_meters(height_m)?,
        ))
    }

    /// Builder: attach local weather (for refraction).
    #[must_use]
    pub fn with_weather(mut self, weather: Weather) -> Self {
        self.weather = weather;
        self
    }

    /// Geodetic latitude (north positive).
    #[must_use]
    pub fn latitude(self) -> Angle {
        self.latitude
    }

    /// Geodetic longitude (east positive).
    #[must_use]
    pub fn longitude(self) -> Angle {
        self.longitude
    }

    /// Altitude above the GRS80 ellipsoid.
    #[must_use]
    pub fn height(self) -> Coordinate {
        self.height
    }

    /// Local weather. `Weather::default()` if none was set.
    #[must_use]
    pub fn weather(self) -> Weather {
        self.weather
    }

    /// Build the C-side `novas_on_surface` representation for FFI calls.
    ///
    /// Unset weather fields become `NAN`, matching the `SuperNOVAS` convention
    /// of "skip the refraction component that depends on this value".
    pub(crate) fn as_on_surface(self) -> novas_on_surface {
        novas_on_surface {
            latitude: self.latitude.deg(),
            longitude: self.longitude.deg(),
            height: self.height.m(),
            temperature: self
                .weather
                .temperature()
                .map_or(f64::NAN, Temperature::celsius),
            pressure: self.weather.pressure().map_or(f64::NAN, Pressure::mbar),
            humidity: self.weather.humidity_percent().unwrap_or(f64::NAN),
        }
    }
}

impl Display for Site {
    /// Renders as `lat=<lat> lon=<lon> h=<height>`, no weather.
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "lat={} lon={} h={}",
            self.latitude, self.longitude, self.height
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_degrees_round_trip() {
        let s = Site::from_degrees(34.0, -118.0, 100.0).unwrap();
        assert!((s.latitude().deg() - 34.0).abs() < 1e-12);
        assert!((s.longitude().deg() - -118.0).abs() < 1e-12);
        assert!((s.height().m() - 100.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_out_of_range_latitude() {
        // 200° would silently fold to -160° if left to Angle's normalization.
        assert!(matches!(
            Site::from_degrees(200.0, 10.0, 0.0),
            Err(Error::OutOfRange("geodetic latitude"))
        ));
        // 95° stays 95° under Angle normalization, so it must be caught here.
        assert!(matches!(
            Site::from_degrees(95.0, 10.0, 0.0),
            Err(Error::OutOfRange(_))
        ));
        // The poles are valid.
        assert!(Site::from_degrees(90.0, 0.0, 0.0).is_ok());
        assert!(Site::from_degrees(-90.0, 0.0, 0.0).is_ok());
    }

    #[test]
    fn longitude_is_unrestricted() {
        // Longitude is modular; large values are accepted and folded.
        assert!(Site::from_degrees(0.0, 350.0, 0.0).is_ok());
        assert!(Site::from_degrees(0.0, -200.0, 0.0).is_ok());
    }

    #[test]
    fn weather_defaults_to_empty() {
        let s = Site::from_degrees(0.0, 0.0, 0.0).unwrap();
        assert!(s.weather().temperature().is_none());
    }

    #[test]
    fn with_weather_attaches_it() {
        let s = Site::from_degrees(34.0, -118.0, 100.0)
            .unwrap()
            .with_weather(Weather::standard());
        assert!((s.weather().temperature().unwrap().celsius() - 15.0).abs() < 1e-12);
    }

    #[test]
    fn as_on_surface_uses_nan_for_missing_weather() {
        let s = Site::from_degrees(34.0, -118.0, 100.0).unwrap();
        let raw = s.as_on_surface();
        assert!((raw.latitude - 34.0).abs() < 1e-12);
        assert!((raw.longitude - -118.0).abs() < 1e-12);
        assert!((raw.height - 100.0).abs() < 1e-12);
        assert!(raw.temperature.is_nan());
        assert!(raw.pressure.is_nan());
        assert!(raw.humidity.is_nan());
    }

    #[test]
    fn as_on_surface_includes_weather_when_set() {
        let s = Site::from_degrees(0.0, 0.0, 0.0)
            .unwrap()
            .with_weather(Weather::standard());
        let raw = s.as_on_surface();
        assert!((raw.temperature - 15.0).abs() < 1e-12);
        assert!((raw.pressure - 1013.25).abs() < 1e-12);
        assert!((raw.humidity - 50.0).abs() < 1e-12);
    }
}
