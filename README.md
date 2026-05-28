# SuperNOVAS (Rust)

[![CI](https://github.com/kiranshila/supernovas_rs/actions/workflows/ci.yml/badge.svg)](https://github.com/kiranshila/supernovas_rs/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/kiranshila/supernovas_rs/graph/badge.svg)](https://codecov.io/gh/kiranshila/supernovas_rs)
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

- **Observer variants**: airborne and near-Earth (satellite) observers are not yet wrapped.
- **Solar-system body sources**: planets and other ephemeris-driven source types are not yet
  exposed as first-class `Source` objects (though a raw `PlanetProvider` can be installed and
  `Accuracy::Full` works end-to-end for stellar sources).
- **`Interval` timescale**: `Interval::from_seconds` takes a raw `novas_timescale` FFI enum
  directly; this will be replaced by a safe `Timescale` newtype.

## Note on LLM Usage

LLMs were used in the production of some portions of this crate, mainly in the generation of unit tests and getting `build.rs` correct for the ffi layer.
All code in this crate was at the very least validated manually by the author, if not written by them.

## Upstream attribution

This project wraps [SuperNOVAS](https://github.com/sigmyne/supernovas), a C astrometry library authored by **Attila Kovács** ([@sigmyne](https://github.com/sigmyne)), itself derived from the original NOVAS library by the U.S. Naval Observatory.

SuperNOVAS is released into the **public domain** under [The Unlicense](https://unlicense.org). The vendored copy in `supernovas-ffi/vendor/supernovas` is pinned to upstream v1.6.0 and its full license text is in `supernovas-ffi/vendor/supernovas/LICENSE`.

## License

The Rust source code in this repository (`supernovas-ffi` and `supernovas` crates, excluding the vendored C library) is licensed under either of

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
