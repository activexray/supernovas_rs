//! Apparent position of a high-proper-motion star at two epochs.
//!
//! Barnard's Star has the largest proper motion of any star (~10.3 arcsec/yr
//! in declination), making it an ideal sanity check: the apparent sky position
//! shifts by roughly half a degree per century, visible to the naked eye over
//! a human lifetime.
//!
//! The example builds a full astrometric `CatalogEntry` with proper motion,
//! parallax, and radial velocity, then computes the apparent RA/Dec at two
//! epochs 25 years apart and prints the drift.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example proper_motion
//! ```

use supernovas::{Accuracy, Angle, CatalogEntry, Frame, Interval, Observer, ScalarVelocity, Time};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    // ── Catalog entry ─────────────────────────────────────────────────────
    // Barnard's Star (GJ 699) - ICRS J2000 position from the Hipparcos
    // catalogue. Proper motion components are in mas/yr of arc (RA×cos δ, Dec).
    let barnard = CatalogEntry::icrs(
        "Barnard's Star",
        "17:57:48.498".parse()?, // RA  17 h 57 m 48.498 s
        "-04:41:36.19".parse()?, // Dec −04° 41' 36.19"  (south of equator)
    )?
    .with_proper_motion_mas_per_yr(-798.71, 10_337.77)? // mas/yr: μα·cos δ, μδ
    .with_parallax(Angle::from_mas(548.31)?)? // parallax → 1.82 pc
    .with_radial_velocity(ScalarVelocity::from_km_per_s(-110.51)?)?; // approaching

    // ── Observer ──────────────────────────────────────────────────────────
    // Geocentric observer removes the site-dependent diurnal aberration so
    // the comparison is purely about the star's secular motion.
    let observer = Observer::Geocenter;

    // ── Two epochs ───────────────────────────────────────────────────────
    // J2000.0 = 2000-01-01 12:00 TT, JD 2 451 545.0 TT.
    // J2025.0 ≈ 2025-01-01 12:00 TT, JD 2 460 676.5 TT (25 Julian years later).
    let epoch_2000 = Time::from_tt_jd(2_451_545.0, 32, 0.0)?;
    let epoch_2025 = epoch_2000 + Interval::from_julian_years(25.0)?;

    // ── Apparent positions ────────────────────────────────────────────────
    let frame_2000 = Frame::new(Accuracy::Reduced, &observer, &epoch_2000)?;
    let frame_2025 = Frame::new(Accuracy::Reduced, &observer, &epoch_2025)?;

    let app_2000 = barnard.apparent_in(&frame_2000, supernovas::ReferenceSystem::Icrs)?;
    let app_2025 = barnard.apparent_in(&frame_2025, supernovas::ReferenceSystem::Icrs)?;

    let eq_2000 = app_2000.equatorial();
    let eq_2025 = app_2025.equatorial();

    // ── Print results ─────────────────────────────────────────────────────
    println!("Barnard's Star - apparent ICRS position\n");
    println!(
        "  Epoch J2000  RA {:.6}  Dec {:+.5}°",
        eq_2000.ra(),
        eq_2000.dec().deg()
    );
    println!(
        "  Epoch J2025  RA {:.6}  Dec {:+.5}°",
        eq_2025.ra(),
        eq_2025.dec().deg()
    );

    let sep = eq_2000.distance_to(eq_2025);
    println!("\n  25-year drift: {:.1} arcsec", sep.arcsec());

    Ok(())
}
