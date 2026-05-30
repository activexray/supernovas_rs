//! Convert an apparent position between equatorial, ecliptic, and galactic
//! frames, and compute the angular separation between two sources.
//!
//! Starting from two catalog stars (Vega and Altair) the example:
//!
//! 1. Computes apparent positions (CIRS) via `CatalogEntry::apparent_in`.
//! 2. Reads the equatorial (RA/Dec) result.
//! 3. Converts Vega's position to ecliptic and galactic coordinates.
//! 4. Computes the on-sky angular separation between Vega and Altair.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example coordinate_conversion
//! ```

use supernovas::{Accuracy, CatalogEntry, Frame, Observer, ReferenceSystem, Time};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    // ── Sources ───────────────────────────────────────────────────────────
    // ICRS J2000 positions from the Yale Bright Star Catalogue.
    let vega = CatalogEntry::icrs("Vega", "18:36:56.336".parse()?, "+38:47:01.28".parse()?)?;
    let altair = CatalogEntry::icrs("Altair", "19:50:46.999".parse()?, "+08:52:05.96".parse()?)?;

    // ── Frame ─────────────────────────────────────────────────────────────
    // Geocentric observer at J2025.0 (2025-01-01 12:00 TT, JD 2 460 676.5 TT).
    // Reduced accuracy: no ephemeris file needed.
    let time = Time::from_tt_jd(2_460_676.5, 37, 0.0)?;
    let frame = Frame::new(Accuracy::Reduced, &Observer::Geocenter, &time)?;

    // ── Apparent CIRS positions ───────────────────────────────────────────
    let vega_app = vega.apparent_in(&frame, ReferenceSystem::Cirs)?;
    let altair_app = altair.apparent_in(&frame, ReferenceSystem::Cirs)?;

    let vega_eq = vega_app.equatorial();
    let altair_eq = altair_app.equatorial();

    // ── Equatorial (CIRS) ─────────────────────────────────────────────────
    println!("── Equatorial (CIRS at J2025.0) ─────────────────────────────");
    println!("  Vega   {vega_eq}");
    println!("  Altair {altair_eq}");

    // ── Angular separation ────────────────────────────────────────────────
    let sep = vega_eq.distance_to(altair_eq);
    println!("\n── Angular separation ────────────────────────────────────────");
    println!(
        "  Vega ↔ Altair: {:.4}°  ({:.2} arcmin)",
        sep.deg(),
        sep.arcmin()
    );

    // ── Ecliptic ──────────────────────────────────────────────────────────
    // apparent_in with CIRS tags coordinates as TOD (true equator/equinox
    // of date), so the resulting ecliptic is also TOD.  For J2000 ecliptic,
    // compute against the J2000 reference system instead.
    let vega_app_j2000 = vega.apparent_in(&frame, ReferenceSystem::J2000)?;
    let vega_ecl = vega_app_j2000.ecliptic(Accuracy::Reduced)?;
    println!("\n── Ecliptic (J2000) ──────────────────────────────────────────");
    println!("  Vega {vega_ecl}");

    // ── Galactic ──────────────────────────────────────────────────────────
    let vega_gal = vega_app.galactic(Accuracy::Reduced)?;
    println!("\n── Galactic ──────────────────────────────────────────────────");
    println!("  Vega {vega_gal}");

    Ok(())
}
