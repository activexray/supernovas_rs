# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] — 2026-05-28

### Added

- **`Source` trait** (sealed) — common interface for all sky sources; provides a default
  `apparent_in(frame, system)` method. Implemented by `CatalogEntry`, `Planet`,
  `EphemObject`, and `OrbitalObject`.
- **`Planet`** — major solar-system body source (Sun, Moon, planets, barycenters) via
  `Planet::new(SolarBody::Mars)?` or convenience constructors (`Planet::mars()?`).
  Uses the installed planet-ephemeris provider at `Accuracy::Full`; built-in
  low-precision approximations suffice at `Accuracy::Reduced`.
- **`SolarBody`** enum — discriminant for `Planet`; variants cover all `novas_planet`
  entries including barycenters.
- **`EphemObject`** — arbitrary solar-system body by name and NAIF ID from the installed
  ephemeris provider (`EphemObject::new("Ceres", 2000001)?`).
- **`OrbitalObject`** + **`OrbitalElements`** — Keplerian orbital-elements source; no
  external provider required. Construct via `OrbitalElements { epoch_jd_tdb, semi_major_axis_au,
  eccentricity, … }.into_source("Halley", 1000012)?`.
- **`Frame::observe` is now generic** — accepts any `impl Source`, not only `CatalogEntry`.

## [0.3.0] — 2026-05-28

### Added

- **Apparent position types**: `Apparent` — a typed apparent position carrying a `ReferenceSystem`
  tag (ICRS, GCRS, CIRS, or equinox-of-date); `ReferenceSystem` enum.
- **`CatalogEntry::apparent_in(frame, reference_system)`** — compute an `Apparent` position for
  a catalog source in the requested reference frame.
- **Spherical coordinate types**: `Equatorial` (RA/Dec, tagged with `ReferenceSystem`) and
  `Ecliptic` (ecliptic longitude/latitude λ/β). Both support `Display` and `approx::AbsDiffEq`.
- **`Equinox`** — equinox representation for coordinate conversions between reference frames.
- **`Refraction`** — refraction model selector (`None`, `Standard`, or `Weather`-driven) passed
  to `Frame::observe` for apparent az/el corrections.
- **Ephemeris system** — planetary ephemeris backends for `Accuracy::Full`:
  - `EphemerisProvider` trait — low-level interface for installing a planet provider via C callbacks.
  - `PlanetProvider` trait — high-level safe interface; implement `state()` and a blanket impl
    handles all C callback registration, process-global `OnceLock`, and `catch_unwind` for you.
  - `Ephemeris` wrapper — `Ephemeris::open(path)` (single-backend) and `Ephemeris::from_provider(p)`.
  - `CalcephEphemeris` (feature `calceph`) — CALCEPH C library backend; wraps `novas_use_calceph`.
  - `AniseEphemeris` (feature `anise`) — pure-Rust ANISE/SPK reader; no extra C dependency.
  - When both features are enabled simultaneously the two backends agree to **≤2 µas** for a
    typical DE440s pointing — irreducible rounding noise from independent Chebyshev evaluators.
- **`Accuracy::Full` now works end-to-end**: install any `EphemerisProvider` (or `PlanetProvider`)
  once at process start and `Frame::new(Accuracy::Full, …)` produces sub-µas apparent positions.
- **Three new integration tests**: `full_accuracy` (CALCEPH), `full_accuracy_anise` (ANISE),
  `full_accuracy_backends_agree` (cross-validation of both backends against de440s.bsp).

### Fixed

- **`hifitime` feature — nanosecond-level precision**: `Time::to_epoch()` and
  `Time::from_epoch_with_dut1()` previously combined the split Julian date into a single `f64`,
  losing ~40–64 µs from ULP rounding on large integers. Both methods are now rewritten using
  integer `i128` nanosecond arithmetic (via `Duration::from_total_nanoseconds` /
  `Epoch::to_tai_duration().total_nanoseconds()`), achieving ≤1 ns round-trip precision.

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

[Unreleased]: https://github.com/kiranshila/supernovas_rs/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/kiranshila/supernovas_rs/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/kiranshila/supernovas_rs/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/kiranshila/supernovas_rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/kiranshila/supernovas_rs/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/kiranshila/supernovas_rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/kiranshila/supernovas_rs/releases/tag/v0.1.0
