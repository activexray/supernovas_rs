//! End-to-end test of the high-precision astrometry path using the pure-Rust
//! ANISE backend ([`crate::AniseEphemeris`]) instead of CALCEPH.
//!
//! Requires the `anise` cargo feature and the same `de440s.bsp` file the
//! `calceph` test uses, at `supernovas/tests/data/de440s.bsp`. The file is
//! gitignored; download with:
//!
//! ```text
//! curl -L -o supernovas/tests/data/de440s.bsp \
//!     http://public-data.nyxspace.com/anise/de440s.bsp
//! ```
//!
//! `AniseEphemeris::install` is process-global and install-once, so we keep
//! all assertions in a single `#[test]`.

#![cfg(feature = "anise")]

use std::path::PathBuf;

use supernovas::{
    Accuracy, AniseEphemeris, CatalogEntry, EphemObject, Ephemeris, Frame, Observer, Planet,
    ReferenceSystem, Site, SolarBody, Source, Time,
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

    Ephemeris::from_provider(AniseEphemeris::open(&path).expect("ephemeris file readable"))
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

    let eq_reduced = app_reduced.equatorial();
    let eq_full = app_full.equatorial();
    let sep_mas = eq_reduced.distance_to(eq_full).mas();
    assert!(
        sep_mas < 50.0,
        "Reduced vs Full apparent positions disagree by {sep_mas} mas - \
         expected sub-mas to ~tens of mas for a sidereal source"
    );

    // Sanity: matches the value our standard example prints.
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

    // EphemObject NAIF IDs must be honored literally. NAIF 3 is the
    // Earth–Moon barycenter; the old planet_ephem_provider funnel
    // misread small ids as novas_planet discriminants and remapped 3 to
    // Earth (NAIF 399), ~4700 km away. The EphemObject path must agree
    // with the Planet path for the same body.
    let emb_planet = Planet::new(SolarBody::EarthMoonBarycenter).unwrap();
    let emb_ephem = EphemObject::new("EMB", 3).unwrap();
    let app_planet = emb_planet
        .apparent_in(&frame_full, ReferenceSystem::Cirs)
        .unwrap();
    let app_ephem = emb_ephem
        .apparent_in(&frame_full, ReferenceSystem::Cirs)
        .unwrap();
    let sep_mas = app_planet
        .equatorial()
        .distance_to(app_ephem.equatorial())
        .mas();
    assert!(
        sep_mas < 1.0,
        "EphemObject(NAIF 3) is {sep_mas} mas from Planet(EMB) - \
         NAIF id was not honored literally"
    );

    // A Voyager 1 query (NAIF -31) without its kernel loaded must fail
    // cleanly rather than panic across the FFI boundary.
    let voyager = EphemObject::new("VOYAGER 1", -31).unwrap();
    assert!(
        voyager
            .apparent_in(&frame_full, ReferenceSystem::Cirs)
            .is_err()
    );
}
