# SuperNOVAS (Rust)

[![CI](https://github.com/activexray/supernovas_rs/actions/workflows/ci.yml/badge.svg)](https://github.com/activexray/supernovas_rs/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/activexray/supernovas_rs/graph/badge.svg)](https://codecov.io/gh/activexray/supernovas_rs)
[![crates.io](https://img.shields.io/crates/v/supernovas.svg)](https://crates.io/crates/supernovas)
[![docs.rs](https://docs.rs/supernovas/badge.svg)](https://docs.rs/supernovas)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

Safe Rust bindings to the [SuperNOVAS](https://github.com/sigmyne/supernovas) astrometry C library.

SuperNOVAS is a high-precision astrometry library based on NOVAS (Naval Observatory Vector Astrometry Software).

## Quick start

### Example — ICRS to horizontal

Compute the az/el of Vega as seen from Owens Valley Radio Observatory:

```rust
use supernovas::{Accuracy, CatalogEntry, Frame, Observer, Site, Time, Weather};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let vega = CatalogEntry::icrs("Vega", "18:36:56.336".parse()?, "+38:47:01.28".parse()?)?;

    let site = Site::from_degrees(37.234, -118.282, 1222.0)?.with_weather(Weather::standard());
    let observer = Observer::Geodetic(site);

    // JD 2461236.75 UTC, 37 leap seconds
    let time = Time::from_utc_jd(2_461_236.75, 37, 0.0)?;

    let frame = Frame::new(Accuracy::Reduced, &observer, &time)?;
    let horizontal = frame.observe(&vega)?;
    println!("{horizontal}");
    Ok(())
}
```

Run the bundled example:

```sh
cargo run --example icrs_to_horizontal
```

## Building

Requires a C compiler and CMake (for the vendored build). With [Nix](https://nixos.org/) and `direnv`, the dev environment is provided automatically via `flake.nix`.

For system-library builds (i.e. `--no-default-features`, without `vendored`), SuperNOVAS ≥ 1.7.0 must be installed and discoverable via `pkg-config`, or the `SUPERNOVAS_INCLUDE_DIR` / `SUPERNOVAS_LIB_DIR` environment variables must point to a local install.

```sh
cargo build
cargo test
```

## Workspace layout

```
supernovas-ffi/       # -sys crate (bindgen + optional cmake build)
  vendor/supernovas/  # git submodule: upstream SuperNOVAS C source
  build.rs
  wrapper.h
supernovas/           # safe wrapper crate
  src/
  examples/
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `vendored` | yes | Build SuperNOVAS from the bundled C submodule via CMake |
| `std` | yes | Link the standard library (required by `anise`, `eop`) |
| `anise` | yes | Pure-Rust ANISE/SPK ephemeris backend for `Accuracy::Full` |
| `calceph` | no | CALCEPH C library ephemeris backend (alternative to `anise`) |
| `eop` | no | Live IERS EOP data fetch via CURL; exposes the `eop` module |
| `hifitime` | no | `hifitime::Epoch` ↔ `Time` conversions |

With `eop` enabled, pass `Time::from_tt_jd_auto_eop(jd)` and
`Frame::with_auto_polar_motion(accuracy, observer, time)` to have SuperNOVAS fetch
leap seconds, UT1−UTC, and polar offsets from IERS automatically.

## Remaining work

Known gaps that will be addressed in future releases:

- **Frame and observation pipeline**: `Frame::source_gcrs_direction` for interferometry; `novas_app_to_hor` / `novas_hor_to_app` round-trip; `novas_sky_pos` output access for intermediate CIRS/GCRS positions.
- **Angular and spherical utilities**: `FromStr` for `Equatorial`, `Horizontal`, `Ecliptic`, `Galactic`; typed proper-motion newtype (mas/yr instead of bare `f64`); `Angle::great_circle_distance` wrapping `novas_sep`; more between-system conversion helpers.
- **Interferometry**: `novas_site_uvw` convenience; per-baseline F-engine FIFO/rotator splits (currently demonstrated in the `interferometry` example).
- **Observer variants**: airborne and near-Earth (satellite) observers are not yet wrapped.
- **Rise / set / transit**: `novas_rises_above`, `novas_sets_below`, `novas_transit_time` are not yet exposed.
- **ITRS transforms**: `cirs_to_itrs`, `itrs_to_cirs`, `hor_to_itrs`, and related functions are not yet wrapped.

## Note on LLM Usage

LLMs were used in the production of some portions of this crate, mainly in the generation of unit tests and getting `build.rs` correct for the ffi layer.
All code in this crate was at the very least validated manually by the author, if not written by them.

## Upstream attribution

This project wraps [SuperNOVAS](https://github.com/sigmyne/supernovas), a C astrometry library authored by **Attila Kovács** ([@sigmyne](https://github.com/sigmyne)), itself derived from the original NOVAS library by the U.S. Naval Observatory.

SuperNOVAS is released into the **public domain** under [The Unlicense](https://unlicense.org). The vendored copy in `supernovas-ffi/vendor/supernovas` is pinned to upstream v1.7 and its full license text is in `supernovas-ffi/vendor/supernovas/LICENSE`.

## License

The Rust source code in this repository (`supernovas-ffi` and `supernovas` crates, excluding the vendored C library) is licensed under either of

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
