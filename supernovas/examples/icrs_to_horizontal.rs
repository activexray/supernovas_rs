//! ICRS → horizontal (az/el) for a catalog star.
//!
//! Walks through the end-to-end pipeline:
//!
//!     ICRS RA/Dec  →  CatalogEntry
//!     Site + Time  →  Observer + Frame
//!                  →  Frame::observe  →  Horizontal (az/el)
//!
//! Run with:
//!
//! ```sh
//! cargo run --example icrs_to_horizontal
//! ```

use supernovas::{Accuracy, CatalogEntry, Frame, Observer, Site, Time, Weather};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    // ── Source ────────────────────────────────────────────────────────────
    // Vega (α Lyr), ICRS J2000 position. RA parses as HMS, Dec as DMS via
    // the `FromStr` impls.
    let vega = CatalogEntry::icrs("Vega", "18:36:56.336".parse()?, "+38:47:01.28".parse()?)?;
    println!("Source     {vega}");

    // ── Site / Observer ───────────────────────────────────────────────────
    // Owens Valley Radio Observatory, with a "standard" atmosphere on top.
    let site = Site::from_degrees(37.234, -118.282, 1222.0)?.with_weather(Weather::standard());
    let observer = Observer::Geodetic(site);
    println!("Observer   {observer}");
    println!("Weather    {}", site.weather());

    // ── Time ──────────────────────────────────────────────────────────────
    // 2026-07-15 06:00:00 UTC (≈ 23:00 local on Jul 14) - Vega is high
    // overhead at OVRO.  JD 2461236.75 UTC.
    // Current leap seconds (Jul 2026): TAI − UTC = 37 s.
    let time = Time::from_utc_jd(2_461_236.75, 37, 0.0)?;
    println!("Time       {time}");

    // ── Frame ─────────────────────────────────────────────────────────────
    // Reduced accuracy is plenty for az/el at the arcsecond level, and
    // doesn't need a high-precision ephemeris provider configured.
    let frame = Frame::new(Accuracy::Reduced, &observer, &time)?;

    // ── Observe ───────────────────────────────────────────────────────────
    let horizontal = frame.observe(&vega)?;
    println!();
    println!("→ {horizontal}");
    println!("  zenith angle: {}", horizontal.zenith_angle());

    Ok(())
}
