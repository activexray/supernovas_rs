# supernovas

Safe Rust bindings to the [SuperNOVAS](https://github.com/sigmyne/supernovas) astrometry C library.

SuperNOVAS is a high-precision astrometry library based on NOVAS (Naval Observatory Vector Astrometry Software). This workspace provides two crates:

| Crate | Description |
|---|---|
| [`supernovas-sys`](supernovas-sys/) | Raw FFI bindings (auto-generated via bindgen) |
| [`supernovas`](supernovas/) | Safe, idiomatic Rust wrapper |

## Features

- Convert celestial coordinates across reference frames (ICRS, GCRS, horizontal az/el, and more)
- Build observers from geodetic site coordinates with optional atmospheric refraction
- Handle time in UTC/TT/TDB Julian dates with leap-second correction
- Full/reduced accuracy modes — reduced accuracy works without an external ephemeris
- `no_std` compatible (`supernovas` crate)
- Vendored SuperNOVAS v1.6.0 by default (no system library required)

## Quick start

Add the wrapper crate to your `Cargo.toml`:

```toml
[dependencies]
supernovas = "0.1"
```

To use a system-installed SuperNOVAS instead of the vendored copy, disable the default `vendored` feature on the `-sys` crate:

```toml
[dependencies]
supernovas = { version = "0.1", default-features = false }
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
supernovas-sys/       # -sys crate (bindgen + optional cmake build)
  vendor/supernovas/  # git submodule: upstream SuperNOVAS C source
  build.rs
  wrapper.h
supernovas/           # safe wrapper crate
  src/
  examples/
```

## Upstream attribution

This project wraps [SuperNOVAS](https://github.com/sigmyne/supernovas), a C astrometry library authored by **Attila Kovács** ([@sigmyne](https://github.com/sigmyne)), itself derived from the original NOVAS library by the U.S. Naval Observatory.

SuperNOVAS is released into the **public domain** under [The Unlicense](https://unlicense.org). The vendored copy in `supernovas-sys/vendor/supernovas` is pinned to upstream v1.6.0 and its full license text is in `supernovas-sys/vendor/supernovas/LICENSE`.

## License

The Rust source code in this repository (`supernovas-sys` and `supernovas` crates, excluding the vendored C library) is licensed under either of

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
