# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking

- **`Error` is no longer `Copy`** — it now derives `Clone` only. The `Ffi` variant under `std`
  carries a `String`, making `Copy` impossible. Code that relied on implicitly copying an `Error`
  must be updated to `.clone()` explicitly.
- **`Error::Ffi` shape changed** — previously `Ffi { code: i32, os_error: OsError }` (both
  features), now `Ffi(String)` under `std` and `Ffi(i32)` under `no_std`. Match arms and
  struct-update syntax referencing the old fields will fail. `OsError` is removed entirely.
- **`supernovas-ffi` now requires SuperNOVAS ≥ 1.7.0** (up from ≥ 1.6.0) for system builds.
  Vendored builds automatically use the bundled v1.7 submodule.
- **`Timescale` enum replaces `novas_timescale` in the public API** — `Time::from_jd`,
  `Time::from_split_jd`, `Interval::from_seconds`, and `Interval::timescale` now use
  `supernovas::Timescale` instead of the raw `supernovas_ffi::novas_timescale` type. Code
  passing `NOVAS_TT`, `NOVAS_UTC`, etc. must switch to `Timescale::Tt`, `Timescale::Utc`, etc.
- **`Interval::timescale()` returns `Timescale`** instead of `novas_timescale`.
- **`Time::PartialEq` now compares only the TT Julian date** — the previous impl also compared
  `tt2tdb` and `ut1_to_tt` fields, so two `Time` values at the same instant but different
  `dut1` were considered unequal. The fixed impl matches the documented semantics.
- **`Time` is now `Eq + PartialOrd + Ord`** — ordering is by TT Julian date.

### Added

- **`Timescale` enum** — `Tcb`, `Tdb`, `Tcg`, `Tt`, `Tai`, `Gps`, `Utc`, `Ut1`. Replaces raw
  `novas_timescale` FFI constants throughout the public API. Implements `Display` (e.g. `"TT"`,
  `"UTC"`), `Copy`, `Eq`, and `Hash`.
- **`Time::jd(Timescale) -> f64`** — Julian date in any timescale (wraps `novas_get_time`).
- **`Time::split_jd(Timescale) -> (i64, f64)`** — split Julian date preserving sub-nanosecond
  precision (wraps `novas_get_split_time`).
- **`Time::now(leap_seconds, dut1)` (std only)** — construct from the system clock
  (wraps `novas_set_current_time`).
- **`Time::leap_seconds() -> i32`** — returns the TAI − UTC leap-second count stored in the
  timespec (wraps `novas_time_leap`).
- **`Time::timescale_offset(scale, reference) -> f64`** — clock difference `scale − reference`
  in seconds at this instant (wraps `novas_timescale_offset`). Useful for TDB−TT, TAI−UTC, etc.
- **`Time + Interval` / `Time - Interval`** — shift a `Time` forward or backward in TT seconds
  (wraps `novas_offset_time`).
- **`Time - Time → Interval`** — TT-second difference between two instants.

- **`Error::Ffi` now carries a human-readable description** (under `std`). The display text is
  the captured error description rather than a bare numeric code; e.g.
  `"ANISE could not translate NAIF -31: …"` instead of `"FFI call failed (code 83): no errno"`.
  The description is captured automatically from two sources:
  - Rust ephemeris callbacks (ANISE, CALCEPH) via `set_provider_error` — always available.
  - The SuperNOVAS C library via the new `novas_set_error_handler` hook — available when
    `enable_debug_mode(DebugMode::On)` is called before the failing operation.
- **`enable_debug_mode(DebugMode)` / `get_debug_mode() -> DebugMode`** — new public API in
  `supernovas`. Calling `enable_debug_mode(DebugMode::On)` installs a silent capture handler
  (via SuperNOVAS 1.7's `novas_set_error_handler`) so that `novas_error()` descriptions are
  routed into `Error::Ffi` rather than written to `stderr`.
- **`DebugMode` enum** — `Off` (default), `On`, `Extra`.
- **`EphemObject` support fixed** — `AniseEphemeris::install()` now also calls
  `set_ephem_provider`, enabling `NOVAS_EPHEM_OBJECT` sources (spacecraft, minor planets, etc.).
  Previously only planet providers were registered, causing any `EphemObject` observation to
  fail with an opaque `Error::Ffi`.
- **Correct DE-series planet NAIF IDs** — `AniseEphemeris` now uses `novas_to_dexxx_planet`
  (barycenter IDs present in DE440s) instead of `novas_to_naif_planet` (center IDs absent from
  short-form DE files). The prior bug silently disabled gravitational deflection for
  Jupiter and Saturn.
- **`supernovas-ffi`: new `libc` feature** — controls `WITHOUT_LIBC` in the CMake build.
  The `std` feature of `supernovas` automatically implies `supernovas-ffi/libc`; no-std builds
  produce a freestanding C library with no libc calls.
- **`supernovas-ffi`: vendor updated to SuperNOVAS v1.7** — picks up `novas_set_error_handler`,
  `novas_offset_by`, `novas_equ_offset_by`, and other v1.7 additions. The vendored build
  passes `WITHOUT_CURL=ON` (no libcurl dependency) and respects `WITHOUT_LIBC` via the new
  `libc` feature.
- **`track_ephem` example** — corrected observation date to 2020-01-01 (MJD 58849), within the
  bundled Voyager 1 SPK coverage window (1977–2020).

### Changed

- **Default features** — `vendored` and `anise` are now on by default. Out-of-the-box builds
  compile SuperNOVAS from the bundled submodule (no system library required) and include the
  ANISE ephemeris backend. Disable with `default-features = false` for no-std / custom setups.
- **CI / coverage** — flake.nix updated to build SuperNOVAS from the vendored submodule rather
  than the nixpkgs-packaged system library. The `nixpkgs-master` input is removed; calceph is
  still provided by nix for the optional `calceph` feature tests. The coverage derivation now
  exercises the default feature set (vendored + anise + std).

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
