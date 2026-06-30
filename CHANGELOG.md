# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### `supernovas`

- `Transform` — pre-computed coordinate-transform matrix between two
  `ReferenceSystem`s anchored to a `Frame`; `Transform::new` /
  `invert` / `apply_vector` / `apply_sky_pos`. `Frame::transform` is a
  convenience constructor. `invert` swaps the system tags that the C
  `novas_invert_transform` leaves stale.
- `Frame::cirs_to_itrs` / `Frame::itrs_to_cirs` — IAU 2000 ITRS ↔ CIRS
  3-vector rotations (polar motion and `UT1−TT` sourced from the frame).
  The legacy TOD path (`tod_to_itrs` / `itrs_to_tod`) is intentionally
  not wrapped; `Transform` covers any system pair when needed.
- `Frame::itrs_to_horizontal` / `Frame::horizontal_to_itrs` — local
  horizontal ↔ ITRS for an Earth-bound observer.
- `Frame::lst` — local apparent sidereal time as a `TimeAngle` in
  `[0, 24h)`; refuses non-Earth-bound observers.
- `Geometric` + `Source::geometric_in` — geometric (astrometric, no
  aberration / no deflection) position+velocity of a source; distinct
  from `Apparent`.
- `Frame::horizontal_to_apparent` — inverse of
  `Apparent::to_horizontal_with_refraction`; returns a partial `Apparent`
  (`dis` / `rv` zeroed, `r_hat` reconstructed).
- `Apparent::r_hat` — public unit-direction accessor.
- `Error::UnsupportedObserver` — typed error for operations that require
  an Earth-bound observer (LST, ITRS ↔ horizontal, site UVW).
- `uvw` module — interferometry: `Uvw` (meters, with `delay()` =
  `w / c`, `delay_ns()`, and `delay_rate_ns_per_s()`), `uvw::uvw` (generic,
  array-reference, phase centre as `[f64; 3]` direction), `Frame::site_gcrs_posvel`
  (the building block that turns a ground station into the GCRS station
  vector `uvw::uvw` consumes), and the low-level `xyz_to_uvw` /
  `uvw_to_xyz` / `los_to_xyz` / `xyz_to_los` helpers — all taking/returning
  typed `Position` / `Uvw` instead of raw `[f64; 3]`. The geocentric
  `novas_site_uvw` is intentionally not wrapped — `site_gcrs_posvel` +
  `uvw::uvw` give the higher-precision array-reference model the C docs
  recommend, without the geocentric limitation or TOD-only input contract.
- `Frame::source_gcrs_direction` — one-call convenience to get the GCRS
  unit-direction to a source for UVW projections.
- `Site::itrs_to_enu` / `Site::enu_to_itrs` — ENU ↔ ITRS at a site,
  with `Position` in/out.
- `Interval::from_nanos` / `Interval::nanos` — nanosecond constructor and
  accessor.
- `examples/interferometry.rs` — F-engine delay tracker for a 10-antenna
  array (Ely, NV, 2.4 GSPS): per-antenna coarse/fine delays and delay
  rates (fringe rates) via the array-reference `site_gcrs_posvel` +
  `uvw::uvw` path.
- `unit::C` — speed of light in m/s.

## [0.5.0] — 2026-06-19

### Breaking

- **`Error` is no longer `Copy`** — it now derives `Clone` only. The `Ffi` variant carries a
  `FfiMessage` (`heapless::String<128>`), which is not `Copy`. Code that relied on implicitly
  copying an `Error` must be updated to `.clone()` explicitly.
- **`Error::Ffi` shape changed** — previously `Ffi { code: i32, os_error: OsError }` (both
  features), now `Ffi { code: i32, message: FfiMessage }` for both `std` and `no_std`. Match
  arms and struct-update syntax referencing the old fields will fail. `OsError` is removed
  entirely.
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

- **`Error::Ffi` now carries a human-readable description**. The `message` field is a
  heap-free `FfiMessage` inline string, identical under `std` and `no_std`. The display text is
  the captured error description rather than a bare numeric code; e.g.
  `"ANISE could not translate NAIF -31: …"` instead of `"FFI call returned an error (code 83)"`.
  Under `std`, the description is captured automatically from two sources:
  - Rust ephemeris callbacks (ANISE, CALCEPH) via `set_provider_error` — always available.
  - The SuperNOVAS C library via the new `novas_set_error_handler` hook — available when
    `enable_debug_mode(DebugMode::On)` is called before the failing operation.
  Under `no_std`, the message falls back to a generic description (no capture sources exist).
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
- **`eop` feature** — enables CURL in the vendored CMake build (removes `WITHOUT_CURL=ON`) and
  exposes the `eop` module with Rust wrappers for live IERS data fetch and EOP correction
  utilities: `eop::fetch_eop`, `eop::reset_eop`, `eop::set_auto_fetch_eop`,
  `eop::diurnal_eop_at_time`, `eop::itrf_transform_eop`, `eop::set_leap_list`, and related
  functions. Types: `Eop`, `EopSeries`. Polar-offset getters (`xp`, `xp_err`, `yp`, `yp_err`)
  return typed `Angle` values; `diurnal_eop_at_time` and `itrf_transform_eop` likewise use
  `Angle` for the polar-motion components.
- **`eop::set_eop_file`** — convenience wrapper that configures an IERS data series to be read
  from a pre-downloaded local file via a `file://` URL (CURL supports this natively), avoiding
  network access while still using the full IERS parsing pipeline.
- **`supernovas-ffi`: new `curl` feature** — signals the vendored CMake build to remove
  `WITHOUT_CURL=ON`, compiling in the IERS EOP fetch functions. Implied by `supernovas/eop`.
- **Track interpolation** — `track::EquatorialTrack` and `track::HorizontalTrack` provide fast
  polynomial position evaluation for telescope drive control. `EquatorialTrack::compute` /
  `HorizontalTrack::compute` run the full astrometric pipeline once; `pos_at(&Time)` evaluates
  the stored polynomial in microseconds. Both types are also re-exported at the crate root.
- **`track_ephem` example** — corrected observation date to 2020-01-01 (MJD 58849), within the
  bundled Voyager 1 SPK coverage window (1977–2020).
- **`full_precision` example** — end-to-end `Accuracy::Full` demonstration using auto-IERS EOP
  fetch (`Time::from_tt_jd_auto_eop`, `Frame::with_auto_polar_motion`) to achieve
  sub-microarcsecond apparent positions for catalog stars and solar-system bodies.
- **`CatalogSystem` enum** — `Icrs`, `J2000`, `B1950`, `Fk4`, `Fk5`; selects the input
  coordinate system for catalog entry construction, with automatic conversion to ICRS.
- **`CatalogEntry::in_system(name, ra, dec, CatalogSystem)`** — construct a source from
  legacy catalog coordinates (B1950/FK4/FK5/J2000); SuperNOVAS converts to ICRS internally
  via `make_cat_object_sys`.
- **`CatalogEntry::redshifted_icrs(name, ra, dec, z)`** — construct a cosmological source
  (quasar, galaxy) from ICRS coordinates and a spectroscopic redshift `z` via
  `make_redshifted_object_sys`.
- **`CatalogEntry::with_ssb_velocity(rv)`** — set the Solar System Barycenter radial velocity
  (preferred for modern stellar catalogs such as Gaia/APOGEE); wraps `novas_set_ssb_vel`.
- **`CatalogEntry::with_lsr_velocity(rv, epoch_jd)`** — set a Local Standard of Rest radial
  velocity with an epoch Julian date; wraps `novas_set_lsr_vel`.
- **`CatalogEntry::with_redshift(z)`** — set spectroscopic redshift on an existing entry;
  wraps `novas_set_redshift`.
- **`CatalogEntry::with_distance(d)`** — set distance in parsecs; wraps `novas_set_distance`.
- **`Time::from_jd_auto_eop(scale, jd)`** / **`Time::from_tt_jd_auto_eop(jd_tt)`** /
  **`Time::now_auto_eop()`** (`eop` feature) — auto-IERS constructors that pass the SuperNOVAS
  sentinel values (`leap = -1`, `dut1 = NAN`) so that leap seconds and UT1−UTC are fetched from
  IERS automatically. Do **not** pre-apply diurnal libration/ocean-tide corrections — the C
  library handles that internally.
- **`Frame::with_auto_polar_motion(accuracy, observer, time)`** (`eop` feature) — construct a
  frame with polar offsets fetched automatically from IERS by passing `NAN`/`NAN` for `xp`/`yp`
  to `novas_make_frame`.
- **`Frame::update_observer(&mut self, obs: &Observer)`** — swap the observer in an existing
  frame while keeping the time and accuracy unchanged (wraps `novas_change_observer`). The
  underlying C function handles the in-place aliased case (`orig == out`) correctly, so no
  intermediate allocation is needed.
- **`Error::OutOfRange(&'static str)`** — new public variant for values that are finite but
  outside the physically valid range of a quantity (e.g. geodetic latitude beyond ±90°,
  declination beyond ±90°). The payload names the offending quantity.
- **`FFI_MSG_CAP: usize`** and **`FfiMessage`** — public constant and type alias
  (`heapless::String<FFI_MSG_CAP>`) representing the inline, heap-free error message carried
  inside `Error::Ffi`. Exposed so callers can allocate matching buffers or inspect the capacity.
- **`set_provider_error(impl Display)`** / **`take_provider_error() -> Option<FfiMessage>`** —
  public API for wiring Rust ephemeris callbacks into `Error::Ffi`. Call `set_provider_error`
  inside a provider callback before returning a non-zero code; `Error::ffi` drains it
  automatically. `take_provider_error` is also available for callers that need the description
  separately from the error value.

### Changed

- **Default features** — `vendored` and `anise` are now on by default. Out-of-the-box builds
  compile SuperNOVAS from the bundled submodule (no system library required) and include the
  ANISE ephemeris backend. Disable with `default-features = false` for no-std / custom setups.
- **CI / coverage** — flake.nix updated to build SuperNOVAS from the vendored submodule rather
  than the nixpkgs-packaged system library. The `nixpkgs-master` input is removed; calceph is
  still provided by nix for the optional `calceph` feature tests. The coverage derivation now
  exercises the default feature set (vendored + anise + std).

### Fixed

- **`EquatorialTrack::pos_at` mislabeled its output as ICRS** — `novas_equ_track` computes
  true-of-date (TOD) positions, but the returned `Equatorial` was tagged `Equinox::ICRS`,
  off from real ICRS by the full precession since J2000 (~10 arcmin in 2026, growing
  ~50″/yr) for anyone converting onward via the tag. The result is now tagged with a TOD
  equinox at the evaluation time.
- **`Weather` humidity never reached the observer** — `Observer::as_novas_observer` went
  through `make_observer_on_surface`, which has no humidity parameter; the C side substituted
  a location-based default, so `Refraction::Radio` silently ignored the user's humidity
  (RH 0% and 100% gave bit-identical elevations). The observer is now built via
  `make_observer_at_site` with the fully populated `on_surface` struct.
- **Unset weather fields poisoned weather-dependent refraction with NaN** — fields left
  `None` were passed to C as `NAN`, which slipped past the C range checks and made
  `Refraction::Optical`/`Radio` fail with an opaque `Error::NotFinite` (the docs incorrectly
  claimed a `None` field "disables the refraction contribution"). Unset fields now fall back
  to SuperNOVAS's mean annual weather estimate for the site location (the C library's own
  `make_itrf_site` defaults). User-supplied values are range-checked
  (temperature `[-120, 70]` °C, pressure `[0, 1200]` mbar, humidity `[0, 100]` %) and
  rejected with `Error::OutOfRange`, matching the checks `make_on_surface` performed.
- **TIRS/ITRS apparent places were re-tagged as TOD** — `Apparent::equinox()` mapped the
  Earth-rotating systems to a TOD equinox, so `equatorial()`/`ecliptic()` on a TIRS/ITRS
  apparent produced coordinates wrong by the Earth rotation angle while looking valid
  (and `Apparent::ecliptic`'s documented `UnsupportedSystem` error for ITRS was unreachable).
  TIRS/ITRS apparents now keep their own system tag, and ecliptic conversion of either
  system returns `Error::UnsupportedSystem` instead of garbage.
- **ANISE backend misread small `EphemObject` NAIF IDs as planet numbers** — the backend
  registered SuperNOVAS's `planet_ephem_provider` built-ins, which funnel major-planet
  queries into the generic ephemeris callback with `novas_planet` IDs (0–13). The callback
  therefore had to guess whether a small `id` was a planet discriminant or a NAIF ID, and
  guessed planet: an `EphemObject` with NAIF 3 (Earth–Moon barycenter) was silently remapped
  to Earth (NAIF 399), ~4700 km away. The backend now mirrors the structure of the C
  `solsys-calceph` plugin: dedicated planet-provider callbacks handle `novas_planet` IDs
  (mapped via `novas_to_dexxx_planet`), and the generic callback treats `id` strictly as a
  NAIF ID. The NOVAS `id == -1` name-lookup convention now returns a descriptive error
  (ANISE lookups are ID-based) instead of querying NAIF −1, and the callbacks honor the
  provider contract's allowance for NULL position/velocity output pointers instead of
  writing unconditionally.

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

[Unreleased]: https://github.com/kiranshila/supernovas_rs/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/kiranshila/supernovas_rs/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/kiranshila/supernovas_rs/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/kiranshila/supernovas_rs/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/kiranshila/supernovas_rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/kiranshila/supernovas_rs/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/kiranshila/supernovas_rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/kiranshila/supernovas_rs/releases/tag/v0.1.0
