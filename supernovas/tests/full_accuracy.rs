//! End-to-end test of the high-precision astrometry path
//! ([`Accuracy::Full`]).
//!
//! Requires the `calceph` cargo feature and a planetary ephemeris file at
//! `supernovas/tests/data/de440s.bsp`. To bootstrap:
//!
//! ```text
//! curl -L -o supernovas/tests/data/de440s.bsp \
//!     http://public-data.nyxspace.com/anise/de440s.bsp
//! ```
//!
//! `de440s.bsp` is the "small" DE440 — sufficient for Sun/Moon/major
//! planets, ~32 MB. The file is gitignored.
//!
//! These tests **install a process-global ephemeris provider**, so we
//! keep all assertions in a single `#[test]` to avoid colliding with
//! parallel test runners.

#![cfg(feature = "calceph")]

use std::path::PathBuf;

use supernovas::{
    Accuracy, CalcephEphemeris, CatalogEntry, Ephemeris, Frame, Observer, ReferenceSystem, Site,
    Time,
};

fn ephemeris_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("de440s.bsp")
}

#[test]
fn full_accuracy_matches_reduced_within_mas() {
    let path = ephemeris_path();
    if !path.exists() {
        eprintln!("skipping: ephemeris file not present at {}", path.display());
        return;
    }

    Ephemeris::from_provider(CalcephEphemeris::open(&path).expect("ephemeris file readable"))
        .install()
        .expect("install succeeded");

    // Vega, ICRS J2000.
    let vega = CatalogEntry::icrs(
        "Vega",
        "18:36:56.336".parse().unwrap(),
        "+38:47:01.28".parse().unwrap(),
    )
    .unwrap();

    // OVRO at 2026-07-15 06:00 UTC, the same setup as the example.
    let observer = Observer::Geodetic(Site::from_degrees(37.234, -118.282, 1222.0).unwrap());
    let time = Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap();

    let frame_reduced = Frame::new(Accuracy::Reduced, &observer, &time).unwrap();
    let frame_full = Frame::new(Accuracy::Full, &observer, &time).unwrap();

    let app_reduced = vega
        .apparent_in(&frame_reduced, ReferenceSystem::Cirs)
        .unwrap();
    let app_full = vega
        .apparent_in(&frame_full, ReferenceSystem::Cirs)
        .unwrap();

    // The two methods agree to milliarcsecond level for a star at moderate
    // elevation; the difference is dominated by sub-mas precession /
    // nutation truncations in the Reduced path.
    let eq_reduced = app_reduced.equatorial();
    let eq_full = app_full.equatorial();
    let sep_mas = eq_reduced.distance_to(eq_full).mas();
    assert!(
        sep_mas < 50.0,
        "Reduced vs Full apparent positions disagree by {sep_mas} mas — \
         expected sub-mas to ~tens of mas for a sidereal source"
    );

    // Sanity: the horizontal of Vega at this site/time agrees with the
    // value our standard example prints. azimuth ≈ 77.7°, el ≈ 78.4°.
    let h = app_full.to_horizontal().unwrap();
    assert!(
        (h.azimuth().deg() - 77.75).abs() < 0.1,
        "azimuth {} should be near 77.75°",
        h.azimuth().deg()
    );
    assert!(
        (h.elevation().deg() - 78.37).abs() < 0.1,
        "elevation {} should be near 78.37°",
        h.elevation().deg()
    );
}
