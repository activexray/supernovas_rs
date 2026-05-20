//! Geodetic site: a fixed location on Earth's surface.
//!
//! A `Site` carries the geodetic (ITRS/GRS80) position of an observatory
//! plus optional local weather for refraction.

use core::fmt;

use supernovas_ffi::novas_on_surface;

use super::Weather;
use crate::{Angle, Coordinate, error::Result};

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
    pub fn new(latitude: Angle, longitude: Angle, height: Coordinate) -> Self {
        Site {
            latitude,
            longitude,
            height,
            weather: Weather::default(),
        }
    }

    /// Construct from latitude/longitude in degrees and height in meters.
    pub fn from_degrees(latitude_deg: f64, longitude_deg: f64, height_m: f64) -> Result<Self> {
        Ok(Site::new(
            Angle::from_degrees(latitude_deg)?,
            Angle::from_degrees(longitude_deg)?,
            Coordinate::from_meters(height_m)?,
        ))
    }

    /// Builder: attach local weather (for refraction).
    pub fn with_weather(mut self, weather: Weather) -> Self {
        self.weather = weather;
        self
    }

    /// Geodetic latitude (north positive).
    pub fn latitude(self) -> Angle {
        self.latitude
    }

    /// Geodetic longitude (east positive).
    pub fn longitude(self) -> Angle {
        self.longitude
    }

    /// Altitude above the GRS80 ellipsoid.
    pub fn height(self) -> Coordinate {
        self.height
    }

    /// Local weather. `Weather::default()` if none was set.
    pub fn weather(self) -> Weather {
        self.weather
    }

    /// Build the C-side `novas_on_surface` representation for FFI calls.
    ///
    /// Unset weather fields become `NAN`, matching the SuperNOVAS convention
    /// of "skip the refraction component that depends on this value".
    #[allow(dead_code)] // consumed by the Frame layer (next iteration)
    pub(crate) fn as_on_surface(self) -> novas_on_surface {
        novas_on_surface {
            latitude: self.latitude.deg(),
            longitude: self.longitude.deg(),
            height: self.height.m(),
            temperature: self.weather.temperature().map_or(f64::NAN, |t| t.celsius()),
            pressure: self.weather.pressure().map_or(f64::NAN, |p| p.mbar()),
            humidity: self.weather.humidity_percent().unwrap_or(f64::NAN),
        }
    }
}

impl fmt::Display for Site {
    /// Renders as `lat=<lat> lon=<lon> h=<height>`, no weather.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
