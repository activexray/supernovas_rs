# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] — 2026-05-20

### Added

- `supernovas`: optional `hifitime` feature — adds `From<hifitime::Epoch> for Time`,
  `Time::to_epoch() -> hifitime::Epoch`, and `Time::from_epoch_with_dut1(epoch, dut1)`
  for interoperability with the [hifitime](https://crates.io/crates/hifitime) time library.
  Enabling both `hifitime` and `std` automatically propagates `std` to hifitime.

## [0.2.0] — 2026-05-20

### Breaking

- `supernovas`: `Error::Parse(String)` is now a unit variant `Error::Parse` with no
  payload. Callers matching `Error::Parse(_)` must be updated to `Error::Parse`.

### Added

- `supernovas`: `std` feature — opt-in to link the standard library. The crate is
  `no_std` by default and no longer requires a global allocator.
- `supernovas`: `Error` now derives `Copy`.

### Changed

- `supernovas`: removed the `alloc` dependency entirely. Stack-allocated
  null-terminated buffers replace `CString` in `FromStr` for `Angle` /
  `TimeAngle` and in `CatalogEntry::icrs`; `Weather`'s `Display` impl writes
  directly to the formatter instead of building intermediate `String`s.

## [0.1.1] — 2026-05-20

### Fixed

- `supernovas-ffi`: correct crate-level docs that incorrectly described `vendored` as the default feature.

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
- `vendored` feature: builds the bundled C library statically via CMake (opt-in).
- System-library path via `pkg-config` or `SUPERNOVAS_INCLUDE_DIR` /
  `SUPERNOVAS_LIB_DIR` env vars (default, when `vendored` is not enabled).

[Unreleased]: https://github.com/kiranshila/supernovas_rs/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/kiranshila/supernovas_rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/kiranshila/supernovas_rs/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/kiranshila/supernovas_rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/kiranshila/supernovas_rs/releases/tag/v0.1.0
