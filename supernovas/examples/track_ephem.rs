use std::path::PathBuf;

use supernovas::{
    Accuracy, Angle, AniseEphemeris, CatalogEntry, EphemObject, EphemerisProvider, Frame, Observer,
    Site, Time, Weather,
};
use supernovas_ffi::NOVAS_JD_MJD0;

fn major_planets_ephemeris_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("de440s.bsp")
}

fn voyager_ephemeris_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("voyager1.bsp")
}

// IERS Bulletin B values for 2020-01-01 (MJD 58849).
// The Voyager 1 SPK bundled with this example covers 1977-09-05 to 2020-12-31;
// choose a date within that window.
const OBSTIME_MJD_UTC: f64 = 59849.0; // 2020-01-01
const LEAP_SECONDS: i32 = 37;
const DUT1: f64 = 0.0;
const POLAR_XY: f64 = 0.14;
const POLAR_DY: f64 = 0.43;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install the ephemeris provider
    AniseEphemeris::open(major_planets_ephemeris_path())?
        .with(voyager_ephemeris_path())?
        .install()?;

    // Create the site and subsequent observer at OVRO using the builder pattern
    let site = Site::from_degrees(37.234, -118.282, 1222.0)?.with_weather(Weather::standard());
    let observer = Observer::Geodetic(site);

    // Create an observation time
    let time = Time::from_utc_jd(OBSTIME_MJD_UTC + NOVAS_JD_MJD0, LEAP_SECONDS, DUT1)?;

    // Create the full accuracy observing frame
    let frame = Frame::with_polar_motion(
        Accuracy::Full,
        &observer,
        &time,
        Angle::from_mas(POLAR_XY)?,
        Angle::from_mas(POLAR_DY)?,
    )?;

    // Create the Vega catalog entry and observe the sidereal source
    println!(
        "{}",
        frame.observe(&CatalogEntry::icrs(
            "Vega",
            "18:36:56.336".parse()?,
            "+38:47:01.28".parse()?,
        )?)?
    );

    // Create a non-sidereal source for Voyager and compute the pointing
    println!("{}", frame.observe(&EphemObject::new("VOYAGER 1", -31)?)?);

    Ok(())
}
