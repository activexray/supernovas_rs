//! Parity tests: the safe wrapper must reproduce a direct raw-FFI computation
//! to full precision.
//!
//! These guard against the wrapper's pre-C arithmetic (unit conversions,
//! `Angle`/`TimeAngle` normalization round-trips, scalar wrapping) silently
//! degrading a result relative to calling the C library directly. Every path
//! here must agree with the equivalent raw FFI call to far below the C
//! library's own accuracy floor — we assert sub-µas (1e-6 arcsec), while the
//! scalar round-trip noise is ~1e-10 arcsec.
//!
//! Note the wrapper folds longitudes/azimuth into (-180°, 180°] (via `Angle`)
//! whereas the C functions return [0°, 360°); comparisons therefore use a
//! circular difference rather than a raw subtraction.
//!
//! No ephemeris provider is required: a sidereal source at `Accuracy::Reduced`
//! exercises the full frame → `sky_pos` → horizontal pipeline without one.

#![cfg(feature = "std")]

use core::mem::MaybeUninit;

use supernovas::{
    Accuracy, Apparent, CatalogEntry, Equatorial, Equinox, Frame, Observer, ReferenceSystem,
    Refraction, Site, Time, sys,
};

// Shared scenario: Vega from OVRO at 2026-07-15 06:00 UTC.
const RA_H: f64 = 18.0 + 36.0 / 60.0 + 56.336 / 3600.0;
const DEC_D: f64 = 38.0 + 47.0 / 60.0 + 1.28 / 3600.0;
const LAT_D: f64 = 37.234;
const LON_D: f64 = -118.282;
const HEIGHT_M: f64 = 1222.0;
const JD_UTC: f64 = 2_461_236.75;
const LEAP: i32 = 37;

/// Sub-µas: 1e-6 arcsec, expressed in degrees.
const TOL_DEG: f64 = 1e-6 / 3600.0;

/// Absolute circular separation between two angles in degrees, in `[0, 180]`.
/// Robust to the wrapper's (-180, 180] vs the C library's [0, 360) convention.
fn deg_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    if d > 180.0 { 360.0 - d } else { d }
}

fn wrapper_apparent() -> Apparent {
    let vega = CatalogEntry::icrs(
        "Vega",
        "18:36:56.336".parse().unwrap(),
        "+38:47:01.28".parse().unwrap(),
    )
    .unwrap();
    let observer = Observer::Geodetic(Site::from_degrees(LAT_D, LON_D, HEIGHT_M).unwrap());
    let time = Time::from_utc_jd(JD_UTC, LEAP, 0.0).unwrap();
    let frame = Frame::new(Accuracy::Reduced, &observer, &time).unwrap();
    vega.apparent_in(&frame, ReferenceSystem::Cirs).unwrap()
}

/// Build the same frame + `sky_pos` the wrapper builds, but via raw FFI.
unsafe fn raw_frame_and_sky() -> (sys::novas_frame, sys::sky_pos) {
    let name = c"Vega";
    let mut entry = MaybeUninit::<sys::cat_entry>::zeroed();
    assert_eq!(
        unsafe {
            sys::make_cat_entry(
                name.as_ptr(),
                core::ptr::null(),
                0,
                RA_H,
                DEC_D,
                0.0,
                0.0,
                0.0,
                0.0,
                entry.as_mut_ptr(),
            )
        },
        0
    );
    let entry = unsafe { entry.assume_init() };

    let mut obj = MaybeUninit::<sys::object>::zeroed();
    assert_eq!(
        unsafe { sys::make_cat_object(&raw const entry, obj.as_mut_ptr()) },
        0
    );
    let obj = unsafe { obj.assume_init() };

    let mut robs = MaybeUninit::<sys::observer>::zeroed();
    assert_eq!(
        unsafe {
            sys::make_observer_on_surface(
                LAT_D,
                LON_D,
                HEIGHT_M,
                f64::NAN,
                f64::NAN,
                robs.as_mut_ptr(),
            )
        },
        0
    );
    let robs = unsafe { robs.assume_init() };

    // Mirror Time::from_utc_jd's integer/fraction split exactly.
    let ijd = JD_UTC.floor() as i64;
    let fjd = JD_UTC - ijd as f64;
    let mut ts = MaybeUninit::<sys::novas_timespec>::zeroed();
    assert_eq!(
        unsafe {
            sys::novas_set_split_time(
                sys::novas_timescale::NOVAS_UTC,
                ijd as _,
                fjd,
                LEAP,
                0.0,
                ts.as_mut_ptr(),
            )
        },
        0
    );
    let ts = unsafe { ts.assume_init() };

    let mut frame = MaybeUninit::<sys::novas_frame>::zeroed();
    assert_eq!(
        unsafe {
            sys::novas_make_frame(
                sys::novas_accuracy::NOVAS_REDUCED_ACCURACY,
                &raw const robs,
                &raw const ts,
                0.0,
                0.0,
                frame.as_mut_ptr(),
            )
        },
        0
    );
    let frame = unsafe { frame.assume_init() };

    let mut sky = MaybeUninit::<sys::sky_pos>::zeroed();
    assert_eq!(
        unsafe {
            sys::novas_sky_pos(
                &raw const obj,
                &raw const frame,
                sys::novas_reference_system::NOVAS_CIRS,
                sky.as_mut_ptr(),
            )
        },
        0
    );
    let sky = unsafe { sky.assume_init() };
    (frame, sky)
}

#[test]
fn apparent_ra_dec_matches_raw_sky_pos() {
    let app = wrapper_apparent();
    let (_frame, sky) = unsafe { raw_frame_and_sky() };
    // sky.ra is hours, sky.dec is degrees; the wrapper exposes them through
    // TimeAngle / Angle and the normalized round-trip must not move them.
    assert!(
        deg_diff(app.ra().deg(), sky.ra * 15.0) < TOL_DEG,
        "RA: wrapper {} h vs raw {} h",
        app.ra().hours(),
        sky.ra
    );
    assert!(
        deg_diff(app.dec().deg(), sky.dec) < TOL_DEG,
        "Dec: wrapper {} deg vs raw {} deg",
        app.dec().deg(),
        sky.dec
    );
}

#[test]
fn horizontal_no_refraction_matches_raw_ffi() {
    let app = wrapper_apparent();
    let h = app.to_horizontal().unwrap();

    let (frame, sky) = unsafe { raw_frame_and_sky() };
    let (mut az, mut el) = (0.0_f64, 0.0_f64);
    assert_eq!(
        unsafe {
            sys::novas_app_to_hor(
                &raw const frame,
                sys::novas_reference_system::NOVAS_CIRS,
                sky.ra,
                sky.dec,
                None,
                &raw mut az,
                &raw mut el,
            )
        },
        0
    );
    assert!(
        deg_diff(h.azimuth().deg(), az) < TOL_DEG,
        "azimuth: wrapper {} vs raw {}",
        h.azimuth().deg(),
        az
    );
    assert!(
        deg_diff(h.elevation().deg(), el) < TOL_DEG,
        "elevation: wrapper {} vs raw {}",
        h.elevation().deg(),
        el
    );
}

#[test]
fn horizontal_standard_refraction_matches_raw_ffi() {
    let app = wrapper_apparent();
    let h = app
        .to_horizontal_with_refraction(Refraction::Standard)
        .unwrap();

    let (frame, sky) = unsafe { raw_frame_and_sky() };
    let (mut az, mut el) = (0.0_f64, 0.0_f64);
    assert_eq!(
        unsafe {
            sys::novas_app_to_hor(
                &raw const frame,
                sys::novas_reference_system::NOVAS_CIRS,
                sky.ra,
                sky.dec,
                Some(sys::novas_standard_refraction),
                &raw mut az,
                &raw mut el,
            )
        },
        0
    );
    assert!(deg_diff(h.azimuth().deg(), az) < TOL_DEG);
    assert!(deg_diff(h.elevation().deg(), el) < TOL_DEG);
}

#[test]
fn to_galactic_matches_raw_equ2gal() {
    // Wrapper path: Equatorial(ICRS) -> Galactic. RA/Dec round-trip through
    // the typed scalars before reaching equ2gal.
    let eq = Equatorial::from_hours_and_degrees(RA_H, DEC_D, Equinox::ICRS).unwrap();
    let g = eq.to_galactic(Accuracy::Reduced).unwrap();

    let (mut glon, mut glat) = (0.0_f64, 0.0_f64);
    assert_eq!(
        unsafe { sys::equ2gal(RA_H, DEC_D, &raw mut glon, &raw mut glat) },
        0
    );

    assert!(
        deg_diff(g.l().deg(), glon) < TOL_DEG,
        "galactic l: wrapper {} vs raw {}",
        g.l().deg(),
        glon
    );
    assert!(
        deg_diff(g.b().deg(), glat) < TOL_DEG,
        "galactic b: wrapper {} vs raw {}",
        g.b().deg(),
        glat
    );
}

#[test]
fn to_ecliptic_matches_raw_equ2ecl() {
    let eq = Equatorial::from_hours_and_degrees(RA_H, DEC_D, Equinox::ICRS).unwrap();
    let ecl = eq.to_ecliptic(Accuracy::Reduced).unwrap();

    let (mut elon, mut elat) = (0.0_f64, 0.0_f64);
    assert_eq!(
        unsafe {
            sys::equ2ecl(
                sys::NOVAS_JD_J2000,
                sys::novas_equator_type::NOVAS_GCRS_EQUATOR,
                sys::novas_accuracy::NOVAS_REDUCED_ACCURACY,
                RA_H,
                DEC_D,
                &raw mut elon,
                &raw mut elat,
            )
        },
        0
    );

    assert!(
        deg_diff(ecl.longitude().deg(), elon) < TOL_DEG,
        "ecliptic λ: wrapper {} vs raw {}",
        ecl.longitude().deg(),
        elon
    );
    assert!(
        deg_diff(ecl.latitude().deg(), elat) < TOL_DEG,
        "ecliptic β: wrapper {} vs raw {}",
        ecl.latitude().deg(),
        elat
    );
}
