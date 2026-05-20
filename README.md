# SuperNOVAS (Rust)

[![CI](https://github.com/kiranshila/supernovas_rs/actions/workflows/ci.yml/badge.svg)](https://github.com/kiranshila/supernovas_rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/supernovas.svg)](https://crates.io/crates/supernovas)
[![docs.rs](https://docs.rs/supernovas/badge.svg)](https://docs.rs/supernovas)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

Safe Rust bindings to the [SuperNOVAS](https://github.com/sigmyne/supernovas) astrometry C library.

SuperNOVAS is a high-precision astrometry library based on NOVAS (Naval Observatory Vector Astrometry Software). This workspace provides two crates:

| Crate | Description |
|---|---|
| [`supernovas-ffi`](supernovas-ffi/) | Raw FFI bindings (auto-generated via bindgen) |
| [`supernovas`](supernovas/) | Safe, idiomatic Rust wrapper |

## Features

- ICRS catalog source → apparent horizontal (az/el) via `Frame::observe`
- Dimensioned scalar types: `Angle`, `TimeAngle`, `Coordinate`, `Interval`, `Pressure`, `Temperature`, `ScalarVelocity`
- 3-D vector types: `Position`, `Velocity`, with cross-type arithmetic
- Spherical coordinate types: `Horizontal`, `Galactic`
- `Time` from UTC/TT Julian dates and Unix epoch
- Geodetic and geocentric `Observer`; `Site` + `Weather`
- `CatalogEntry` with proper motion, parallax, and radial velocity
- Full and reduced accuracy modes (reduced requires no external ephemeris)
- `no_std` + alloc-free by default; opt-in `std` feature available
- Optional `vendored` feature: bundles SuperNOVAS v1.6.0 statically (no system library required)

## Quick start

Add the wrapper crate to your `Cargo.toml`. Enable the `vendored` feature to build the bundled SuperNOVAS v1.6.0 statically — no system library required:

```toml
[dependencies]
supernovas = { version = "0.2", features = ["vendored"] }
```

If you already have SuperNOVAS ≥ 1.6.0 installed system-wide (e.g. via Nix or a distro package), omit the feature and it will be found via `pkg-config`:

```toml
[dependencies]
supernovas = "0.2"
```

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

## Remaining work

These are known gaps that will be addressed in future releases:

- **Coordinate types**: `Equatorial` (ICRS / GCRS / J2000) and `Ecliptic` — the `Equinox` machinery and associated frame conversions are not yet implemented.
- **Refraction correction**: `Frame::observe` currently returns the unrefracted geometric direction. A refraction-enabled path (feeding `Site::weather` into SuperNOVAS's built-in refraction model) is the next planned addition.
- **Observer variants**: airborne and near-Earth (satellite) observers are not yet wrapped.
- **Source types**: planets, solar-system bodies, and ephemeris-driven targets (requires a configured ephemeris provider via `novas_use_calceph` or equivalent).
- **Full-accuracy mode**: `Accuracy::Full` needs an external ephemeris provider configured before use. The reduced path works out of the box.
- **Error type**: FFI call failures and parse errors both map to the unit variant `Error::Parse`. A richer error type (with context strings) is deferred until the `std` feature is a reasonable default for the target use cases.
- **`Interval` timescale**: `Interval::from_seconds` takes a raw `novas_timescale` FFI enum directly; this will be replaced by a safe `Timescale` newtype.

## Note on AI Usage

Generative AI was used in the production of some portions of this crate, mainly in the generation of unit tests and getting `build.rs` correct for the ffi layer.
All code in this crate was at the very least validated manually by the author, if not written by them.

## Upstream attribution

This project wraps [SuperNOVAS](https://github.com/sigmyne/supernovas), a C astrometry library authored by **Attila Kovács** ([@sigmyne](https://github.com/sigmyne)), itself derived from the original NOVAS library by the U.S. Naval Observatory.

SuperNOVAS is released into the **public domain** under [The Unlicense](https://unlicense.org). The vendored copy in `supernovas-ffi/vendor/supernovas` is pinned to upstream v1.6.0 and its full license text is in `supernovas-ffi/vendor/supernovas/LICENSE`.

## License

The Rust source code in this repository (`supernovas-ffi` and `supernovas` crates, excluding the vendored C library) is licensed under either of

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
