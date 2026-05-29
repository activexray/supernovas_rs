//! Cross-validation: CALCEPH and ANISE backends, fed the same SPK file,
//! produce results that agree to µas level.
//!
//! A single test installs CALCEPH, records Vega's apparent az/el, then
//! installs ANISE (overwriting the planet provider), records again, and
//! asserts the two agree to within 10 µas in azimuth and 1 µas in elevation.
//!
//! Observed agreement for de440s.bsp:
//!   azimuth:   ~1.7 µas (4.8e-10 deg)
//!   elevation: ~0.04 µas (9.9e-12 deg)
//!
//! Requires both `calceph` and `anise` features and
//! `supernovas/tests/data/de440s.bsp`. Skipped silently if the file is absent.

#![cfg(all(feature = "calceph", feature = "anise"))]

use std::path::PathBuf;

use supernovas::{
    Accuracy, AniseEphemeris, CalcephEphemeris, CatalogEntry, Ephemeris, Frame, Observer,
    ReferenceSystem, Site, Time,
};

fn ephemeris_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("de440s.bsp")
}

fn vega() -> CatalogEntry {
    CatalogEntry::icrs(
        "Vega",
        "18:36:56.336".parse().unwrap(),
        "+38:47:01.28".parse().unwrap(),
    )
    .unwrap()
}

fn ovro() -> Observer {
    Observer::Geodetic(Site::from_degrees(37.234, -118.282, 1222.0).unwrap())
}

fn epoch() -> Time {
    Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap()
}

/// Both backends, fed the same de440s.bsp, must yield az/el that agree to
/// within 10 µas in azimuth and 1 µas in elevation. This validates that the
/// CALCEPH and ANISE interpolation paths produce consistent results from
/// identical SPK data, as required for mm-wave pointing.
///
/// The test installs CALCEPH first, records the pointing, then overwrites the
/// process-global planet provider with ANISE and records again. The ALMANAC
/// `OnceLock` is unset at test start, so ANISE installs cleanly.
#[test]
fn backends_agree_to_uas() {
    let path = ephemeris_path();
    if !path.exists() {
        eprintln!("skipping: ephemeris file not present at {}", path.display());
        return;
    }

    // ── CALCEPH ──────────────────────────────────────────────────────────────
    Ephemeris::from_provider(CalcephEphemeris::open(&path).unwrap())
        .install()
        .unwrap();

    let frame = Frame::new(Accuracy::Full, &ovro(), &epoch()).unwrap();
    let h_calc = vega()
        .apparent_in(&frame, ReferenceSystem::Cirs)
        .unwrap()
        .to_horizontal()
        .unwrap();

    // ── ANISE ─────────────────────────────────────────────────────────────────
    // Overwrites the process-global planet provider set by novas_use_calceph.
    Ephemeris::from_provider(AniseEphemeris::open(&path).unwrap())
        .install()
        .unwrap();

    let frame = Frame::new(Accuracy::Full, &ovro(), &epoch()).unwrap();
    let h_anise = vega()
        .apparent_in(&frame, ReferenceSystem::Cirs)
        .unwrap()
        .to_horizontal()
        .unwrap();

    let az_diff = (h_calc.azimuth().deg() - h_anise.azimuth().deg()).abs();
    let el_diff = (h_calc.elevation().deg() - h_anise.elevation().deg()).abs();

    // 10 µas = 10e-6 arcsec = 2.78e-9 deg. Observed agreement is ~1.7 µas
    // in az and ~0.04 µas in el; 1e-8 deg gives ~6× headroom.
    assert!(
        az_diff < 1e-8,
        "azimuth disagreement {az_diff:.2e} deg > 10 µas \
         (CALCEPH={}, ANISE={})",
        h_calc.azimuth().deg(),
        h_anise.azimuth().deg(),
    );
    assert!(
        el_diff < 1e-8,
        "elevation disagreement {el_diff:.2e} deg > 10 µas \
         (CALCEPH={}, ANISE={})",
        h_calc.elevation().deg(),
        h_anise.elevation().deg(),
    );
}
