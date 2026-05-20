# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-05-20

### Added

#### `supernovas`

- `Frame` — observer × time snapshot; `Frame::observe` converts a `CatalogEntry`
  to an apparent `Horizontal` (az/el) position.
- `Frame::with_polar_motion` — accepts IERS polar-motion offsets for higher-accuracy frames.
- `Observer` — `Geodetic(Site)` and `Geocenter` variants.
- `Site` — geodetic location (latitude, longitude, height) with optional `Weather`.
- `Weather` — temperature, pressure, humidity for refraction; `Weather::standard()` preset.
- `Time` — UTC/TT Julian dates, split Julian dates, Unix epoch construction.
- `CatalogEntry` — ICRS sidereal source; builder methods for proper motion,
  parallax, and radial velocity.
- `Horizontal`, `Galactic`, `Spherical` — typed spherical-coordinate wrappers.
- Scalar dimensioned types: `Angle`, `TimeAngle`, `Coordinate`, `Interval`,
  `Pressure`, `Temperature`, `ScalarVelocity`.
- Vector types: `Position`, `Velocity` with arithmetic operators and
  cross-type ops (`Position / Interval → Velocity`, `Velocity × Interval → Position`).
- `unit` module — named conversion constants (radians, degrees, mas, AU, pc, …).
- `FromStr` / `Display` implementations on `Angle` (DMS) and `TimeAngle` (HMS).
- `approx::AbsDiffEq` on all scalar and aggregate types.

#### `supernovas-ffi`

- Raw bindgen FFI bindings to SuperNOVAS v1.6.0.
- `vendored` feature (default): builds the bundled C library statically via CMake.
- System-library path via `pkg-config` or `SUPERNOVAS_INCLUDE_DIR` /
  `SUPERNOVAS_LIB_DIR` env vars when `vendored` is disabled.

[Unreleased]: https://github.com/kiranshila/supernovas_rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/kiranshila/supernovas_rs/releases/tag/v0.1.0
