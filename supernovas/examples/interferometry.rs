//! F-engine delay tracking for a 10-antenna compact array.
//!
//! Computes the per-antenna geometric delays and delay rates (fringe rates)
//! that an F-engine FPGA would apply to phase up the array on a target
//! source. Demonstrates the array-reference UVW pipeline:
//!
//! 1. Define 10 geodetic antenna sites (a ~km-scale E-W / N-S grid).
//! 2. Build a `Frame` at a chosen epoch with antenna 0 as the array
//!    reference observer.
//! 3. Get the GCRS apparent position of the target source (Cygnus A) via
//!    `Frame::source_gcrs_position`.
//! 4. For each antenna, compute the GCRS position+velocity relative to the
//!    array reference via `Frame::site_gcrs_posvel`, then feed the relative
//!    position and the source position to `uvw::uvw` to get the baseline
//!    projection `Uvw`.
//! 5. Read the geometric delay and delay rate directly from the `Uvw`:
//!    `delay()` and `delay_rate()`.
//! 6. Express each delay as integer ADC samples (coarse delay, applied by
//!    a FIFO) plus a fractional remainder (fine delay, applied by a
//!    per-subband phase rotation in the polyphase filterbank).
//!
//! These are the numbers a correlator's delay tracker locks onto: the
//! integer-sample delays go to the F-engine FIFOs, the fractional remainders
//! drive the per-subband fringe rotators, and the delay rates update them
//! once per integration.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example interferometry
//! ```

use std::f64::consts::PI;

use supernovas::{Accuracy, CatalogEntry, Frame, Observer, Site, Time, Weather, uvw};

/// Array reference: Ely
const REF_LAT: f64 = 39.802;
const REF_LON: f64 = -114.842;
const REF_HEIGHT: f64 = 1909.0;

/// ADC sample clock (samples per second)
const SAMPLE_RATE_HZ: f64 = 2.4e9;
const SAMPLE_PERIOD: f64 = 1.0 / SAMPLE_RATE_HZ;

/// Subband bandwidth (Hz) for the fringe-rotator output. The fractional
/// delay translates to a per-subband phase rotation of
/// `2π · f_sub · τ_frac`.
const SUBBAND_BW_HZ: f64 = 1e6;

#[allow(clippy::similar_names, clippy::items_after_statements)]
fn main() -> Result<(), Box<dyn core::error::Error>> {
    const EARTH_R: f64 = 6_378_137.0;

    // ── 10 antennas: a compact E-W / N-S grid around the reference ───────
    let ref_site =
        Site::from_degrees(REF_LAT, REF_LON, REF_HEIGHT)?.with_weather(Weather::standard());

    // (east_m, north_m) offsets from the reference for antennas 1..9.
    let offsets_m: [(f64, f64); 9] = [
        (100.0, 0.0),
        (200.0, 0.0),
        (300.0, 0.0),
        (0.0, 100.0),
        (0.0, 200.0),
        (100.0, 100.0),
        (-100.0, 0.0),
        (-200.0, 0.0),
        (0.0, -100.0),
    ];

    let lat_rad = REF_LAT.to_radians();
    let m_to_deg_lon = (180.0 / PI) / (EARTH_R * lat_rad.cos());
    let m_to_deg_lat = (180.0 / PI) / EARTH_R;

    let mut sites = Vec::with_capacity(10);
    sites.push(ref_site);
    for (e, n) in offsets_m {
        sites.push(Site::from_degrees(
            REF_LAT + n * m_to_deg_lat,
            REF_LON + e * m_to_deg_lon,
            REF_HEIGHT,
        )?);
    }

    // ── Frame at the reference antenna ───────────────────────────────────
    let observer = Observer::Geodetic(sites[0]);
    let time = Time::from_utc_jd(2_461_236.75, 37, 0.0)?;
    let frame = Frame::new(Accuracy::Reduced, &observer, &time)?;

    // ── Source: Cygnus A (a bright radio calibrator) ─────────────────────
    let cyg_a = CatalogEntry::icrs("Cyg A", "19:59:28.36".parse()?, "+40:44:02.1".parse()?)?;

    // GCRS apparent position of the source (phase center).
    let source_pos = frame.source_gcrs_position(&cyg_a)?;

    // ── Reference antenna GCRS pos/vel ──────────────────────────────────
    let (ref_pos, ref_vel) = frame.site_gcrs_posvel(&sites[0])?;

    // ── Per-antenna delays and delay rates ──────────────────────────────
    println!("F-engine delay tracking - 10-antenna array");
    println!("  reference : antenna 0 at ({REF_LAT}°, {REF_LON}°, {REF_HEIGHT} m)");
    println!("  source    : Cygnus A");
    println!("  epoch     : JD 2461236.750 UTC");
    println!("  ADC clock : {SAMPLE_RATE_HZ:.1e} Hz  ({SAMPLE_PERIOD:.3e} s/sample)");
    println!("  subband   : {SUBBAND_BW_HZ:.0} Hz");
    println!();
    println!("ant  τ [ns]        rate [ns/s]    samples  frac    φ_sub [°]");
    println!("---  -------------  -------------  -------  ------  ----------");

    for (i, site) in sites.iter().enumerate() {
        let (pos, vel) = frame.site_gcrs_posvel(site)?;

        // Position/velocity relative to the array reference (antenna 0).
        let rel_pos = pos - ref_pos;
        let rel_vel = vel - ref_vel;

        // Geometric delay (seconds) and delay rate (s/s) from the UVW
        // baseline projection.
        let u = uvw::uvw(&rel_pos, Some(&rel_vel), &source_pos)?;
        let tau_s = u.delay();
        let rate_s_per_s = u.delay_rate(&source_pos, &rel_vel);

        // Split into coarse (integer-sample) and fine (fractional) delay.
        let n_samples = tau_s / SAMPLE_PERIOD;
        let n_int = n_samples.floor();
        let frac = n_samples - n_int;
        let phase_deg = 360.0 * SUBBAND_BW_HZ * (frac * SAMPLE_PERIOD);

        // Display in nanoseconds for readability.
        let tau_ns = tau_s * 1e9;
        let rate_ns_per_s = rate_s_per_s * 1e9;
        println!(
            " {i:>2}  {tau_ns:>13.3}  {rate_ns_per_s:>13.4}  {n_int:>7.0}  {frac:>6.3}  {phase_deg:>10.3}"
        );
    }

    println!();
    println!("FPGA load per antenna:");
    println!("  coarse delay (FIFO)  : integer `samples` column");
    println!(
        "  fine delay (rotator) : `φ_sub` applied per subband at {SUBBAND_BW_HZ:.0} Hz spacing"
    );
    println!("  update rate           : `rate` ns/s drives the once-per-integration delay walk");

    Ok(())
}
