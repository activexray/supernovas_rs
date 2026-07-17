//! Solar-system bodies in the local sky.
//!
//! Observe the Sun, Moon, and outer planets from a ground-based site.
//! The ANISE ephemeris (DE440s) is installed once at startup, enabling
//! `Accuracy::Reduced` planet positions - good to roughly an arcminute,
//! with no setup beyond the bundled BSP file.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example planet_positions
//! ```

use std::path::PathBuf;

use supernovas::{
    Accuracy, AniseEphemeris, EphemerisProvider, Frame, Observer, Planet, Site, Time, Timescale,
};

fn ephemeris_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("de440s.bsp")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Ephemeris ─────────────────────────────────────────────────────────
    // Planet positions require an installed ephemeris provider even at
    // reduced accuracy.  The bundled DE440s covers 1849–2150.
    AniseEphemeris::open(ephemeris_path())?.install()?;

    // ── Site / Observer ───────────────────────────────────────────────────
    // Kitt Peak National Observatory, Arizona.
    let site = Site::from_degrees(31.9583, -111.5967, 2096.0)?;
    let observer = Observer::Geodetic(site);

    // ── Time ──────────────────────────────────────────────────────────────
    // 2026-07-15 04:00 UTC - local midnight at Kitt Peak (UTC−7).
    // JD 2 461 236.667 UTC; TAI − UTC = 37 s (current leap-second count).
    let time = Time::from_utc_jd(2_461_236.667, 37, 0.0)?;
    println!(
        "UTC  JD {:.4}  (TT JD {:.4})",
        time.jd(Timescale::Utc),
        time.jd(Timescale::Tt),
    );
    println!(
        "Site lat {:.4}°  lon {:.4}°  alt {:.0} m\n",
        31.9583_f64, -111.5967_f64, 2096_f64,
    );

    // ── Frame ─────────────────────────────────────────────────────────────
    let frame = Frame::new(Accuracy::Reduced, &observer, &time)?;

    // ── Observe ───────────────────────────────────────────────────────────
    let bodies: &[(&str, Planet)] = &[
        ("Sun", Planet::sun()?),
        ("Moon", Planet::moon()?),
        ("Mercury", Planet::mercury()?),
        ("Venus", Planet::venus()?),
        ("Mars", Planet::mars()?),
        ("Jupiter", Planet::jupiter()?),
        ("Saturn", Planet::saturn()?),
    ];

    println!("{:<10}  {:>8}  {:>8}", "Body", "Az (°)", "El (°)");
    println!("{}", "-".repeat(32));
    for (name, planet) in bodies {
        match frame.observe(planet) {
            Ok(h) => println!(
                "{name:<10}  {:>8.2}  {:>8.2}",
                h.azimuth().deg().rem_euclid(360.0),
                h.elevation().deg(),
            ),
            Err(e) => println!("{name:<10}  (error: {e})"),
        }
    }

    Ok(())
}
