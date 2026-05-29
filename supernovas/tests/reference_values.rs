//! Absolute-accuracy tests against implementation-independent ground truth.
//!
//! Unlike the parity tests (wrapper vs raw FFI) and the `full_accuracy` tests
//! (Reduced vs Full, backend vs backend) — both of which are *self-consistency*
//! checks — these pin the wrapper's output to values defined outside any C
//! computation:
//!
//!   * the IAU galactic-coordinate definition (the north galactic pole and the
//!     galactic-centre direction are fixed ICRS points), and
//!   * the physical magnitude of annual aberration (bounded by the aberration
//!     constant κ ≈ 20.4955″).
//!
//! These would catch a wrong rotation matrix or a mis-scaled aberration step
//! that a self-consistency test cannot.

#![cfg(feature = "std")]

use supernovas::{
    Accuracy, CatalogEntry, Equatorial, Equinox, Frame, Observer, ReferenceSystem, Time,
};

/// IAU 1958 north galactic pole, J2000/ICRS: α = 192.85948°, δ = +27.12825°.
const NGP_RA_DEG: f64 = 192.859_48;
const NGP_DEC_DEG: f64 = 27.128_25;
/// IAU galactic-centre direction (l = 0, b = 0), J2000/ICRS.
const GC_RA_DEG: f64 = 266.404_99;
const GC_DEC_DEG: f64 = -28.936_17;

fn deg_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    if d > 180.0 { 360.0 - d } else { d }
}

#[test]
fn north_galactic_pole_maps_to_b_plus_90() {
    let ngp = Equatorial::from_degrees(NGP_RA_DEG, NGP_DEC_DEG, Equinox::ICRS).unwrap();
    let g = ngp.to_galactic(Accuracy::Reduced).unwrap();
    // A source at the NGP has galactic latitude +90° by definition.
    let off_arcsec = (90.0 - g.b().deg()).abs() * 3600.0;
    assert!(
        off_arcsec < 5.0,
        "NGP should map to b=+90°, got b={}° ({off_arcsec}″ off)",
        g.b().deg()
    );
}

#[test]
fn galactic_centre_maps_to_origin() {
    let gc = Equatorial::from_degrees(GC_RA_DEG, GC_DEC_DEG, Equinox::ICRS).unwrap();
    let g = gc.to_galactic(Accuracy::Reduced).unwrap();
    // The galactic centre is l=0, b=0 by definition. l is sensitive here (the
    // point is on the galactic equator), so this exercises both the rotation
    // and the longitude zero-point.
    let l_off = deg_diff(g.l().deg(), 0.0);
    let b_off = g.b().deg().abs();
    assert!(
        l_off < 0.05,
        "galactic centre should map to l=0, got l={}° ({l_off}° off)",
        g.l().deg()
    );
    assert!(
        b_off < 0.05,
        "galactic centre should map to b=0, got b={}°",
        g.b().deg()
    );
}

#[test]
fn annual_aberration_magnitude_is_physical() {
    // GCRS keeps ICRS orientation (no precession/nutation), so a zero-proper-
    // motion catalog star's GCRS apparent place differs from its ICRS catalog
    // place only by annual aberration (+ negligible solar deflection well away
    // from the Sun). That offset is bounded by the aberration constant,
    // κ = 20.4955″, and is clearly non-zero at a generic date.
    let ra_h = 18.0 + 36.0 / 60.0 + 56.336 / 3600.0; // Vega
    let dec_d = 38.0 + 47.0 / 60.0 + 1.28 / 3600.0;
    let catalog = Equatorial::from_hours_and_degrees(ra_h, dec_d, Equinox::ICRS).unwrap();

    // Geocentric observer isolates *annual* aberration (no diurnal term).
    let frame = Frame::new(
        Accuracy::Reduced,
        &Observer::Geocenter,
        &Time::from_utc_jd(2_461_236.75, 37, 0.0).unwrap(),
    )
    .unwrap();
    let apparent = CatalogEntry::icrs("Vega", catalog.ra(), catalog.dec())
        .unwrap()
        .apparent_in(&frame, ReferenceSystem::Gcrs)
        .unwrap();

    let sep_arcsec = catalog.distance_to(apparent.equatorial()).arcsec();
    assert!(
        (1.0..=20.6).contains(&sep_arcsec),
        "GCRS aberration offset {sep_arcsec}″ outside the physical (1″, κ=20.4955″] range"
    );
}
