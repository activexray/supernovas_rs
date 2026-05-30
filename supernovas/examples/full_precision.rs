//! Sub-microarcsecond astrometry with `Accuracy::Full` and live IERS EOP data.
//!
//! Demonstrates the complete high-precision pipeline:
//!
//! 1. Install the ANISE/DE440s ephemeris (required for full-accuracy
//!    planet positions).
//! 2. Construct a [`Time`] with automatic IERS dut1/leap-seconds lookup via
//!    [`Time::from_tt_jd_auto_eop`].
//! 3. Build a [`Frame`] with [`Accuracy::Full`], automatically fetching live
//!    IERS polar-motion offsets (x_p, y_p) via [`Frame::with_auto_polar_motion`].
//! 4. Compute ICRS apparent positions for catalog stars and solar-system
//!    bodies.
//! 5. Repeat at [`Accuracy::Reduced`] and report the angular separation.
//!
//! Both [`Time::from_tt_jd_auto_eop`] and [`Frame::with_auto_polar_motion`]
//! pass sentinel `NAN` values to the underlying SuperNOVAS C functions, which
//! then query the IERS servers automatically. The library applies diurnal
//! libration and ocean-tide corrections to the fetched xp/yp internally — you
//! should never pre-apply those corrections yourself.
//!
//! ## Full vs Reduced accuracy
//!
//! | Feature                       | Full              | Reduced        |
//! |-------------------------------|-------------------|----------------|
//! | Nutation model                | IAU 2000A (1300+) | truncated (~10) |
//! | Gravitational deflection      | complete          | simplified     |
//! | Typical position error        | < 1 μas           | ~1 mas         |
//!
//! Run with:
//!
//! ```sh
//! cargo run --example full_precision --features eop,anise
//! ```

use std::path::PathBuf;

use supernovas::{
    Accuracy, AniseEphemeris, CatalogEntry, EphemerisProvider, Frame, Observer, Planet,
    ReferenceSystem, Source, Time,
};

fn ephemeris_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("de440s.bsp")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Ephemeris ─────────────────────────────────────────────────────────
    // Accuracy::Full requires an ephemeris provider for all solar-system
    // body positions and for the Earth–Sun vector used in aberration.
    AniseEphemeris::open(ephemeris_path())?.install()?;

    // ── Time ──────────────────────────────────────────────────────────────
    // J2025.0 = 2025-01-01 12:00 TT (JD 2 460 676.5 TT).
    //
    // from_tt_jd_auto_eop passes NAN for dut1 and -1 for leap_seconds.
    // novas_set_time fetches the IERS values automatically and applies
    // diurnal libration/ocean-tide corrections internally.
    let jd = 2_460_676.5_f64;
    let time = Time::from_tt_jd_auto_eop(jd)?;

    let observer = Observer::Geocenter;

    // ── Frames ────────────────────────────────────────────────────────────
    // with_auto_polar_motion passes NAN/NAN for xp/yp; novas_make_frame
    // fetches and interpolates IERS polar offsets automatically.
    // Do NOT pre-correct xp/yp for libration/tides — the C library handles
    // that for Accuracy::Full frames.
    let frame_full = Frame::with_auto_polar_motion(Accuracy::Full, &observer, &time)?;
    let frame_red = Frame::new(Accuracy::Reduced, &observer, &time)?;

    // ── Catalog stars ─────────────────────────────────────────────────────
    let stars: &[(&str, CatalogEntry)] = &[
        (
            "Vega",
            CatalogEntry::icrs("Vega", "18:36:56.336".parse()?, "+38:47:01.28".parse()?)?,
        ),
        (
            "Sirius",
            CatalogEntry::icrs("Sirius", "06:45:08.917".parse()?, "-16:42:58.02".parse()?)?,
        ),
        (
            "Arcturus",
            CatalogEntry::icrs("Arcturus", "14:15:39.672".parse()?, "+19:10:56.67".parse()?)?,
        ),
    ];

    println!("── Catalog stars (ICRS apparent, Accuracy::Full, J2025.0) ───────────────");
    for (name, star) in stars {
        let app_full = star.apparent_in(&frame_full, ReferenceSystem::Icrs)?;
        let app_red = star.apparent_in(&frame_red, ReferenceSystem::Icrs)?;
        let sep_mas = app_full
            .equatorial()
            .distance_to(app_red.equatorial())
            .mas();
        let eq_full = app_full.equatorial();
        println!("  {name:<10}  {eq_full}");
        println!("             Full–Reduced: {sep_mas:.4} mas");
    }

    // ── Solar-system bodies ───────────────────────────────────────────────
    let planets: &[(&str, Planet)] = &[
        ("Mars", Planet::mars()?),
        ("Jupiter", Planet::jupiter()?),
        ("Saturn", Planet::saturn()?),
    ];

    println!("\n── Solar-system bodies (ICRS apparent, Accuracy::Full, J2025.0) ─────────");
    for (name, planet) in planets {
        let app_full = planet.apparent_in(&frame_full, ReferenceSystem::Icrs)?;
        let app_red = planet.apparent_in(&frame_red, ReferenceSystem::Icrs)?;
        let sep_mas = app_full
            .equatorial()
            .distance_to(app_red.equatorial())
            .mas();
        let dist_au = app_full.distance().au();
        let light_s = dist_au * 499.004_848;
        let eq_full = app_full.equatorial();
        println!("  {name:<10}  {eq_full}");
        println!(
            "             dist {dist_au:.4} AU  ({light_s:.1} s light time)  \
             Full–Reduced: {sep_mas:.4} mas"
        );
    }

    Ok(())
}
