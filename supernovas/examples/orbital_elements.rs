//! Observe a solar-system small body from Keplerian orbital elements.
//!
//! `OrbitalObject` computes apparent positions from classical orbital elements
//! without requiring any external SPK/BSP ephemeris file. The propagator is
//! analytic (no N-body perturbations), so accuracy degrades for strongly
//! perturbed orbits, but it is ideal for newly discovered objects or any body
//! whose MPC/JPL Horizons elements are available.
//!
//! This example uses Halley's Comet near its 1986 perihelion and Ceres (the
//! largest main-belt asteroid), showing how the same pipeline works for
//! bodies with very different orbital shapes.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example orbital_elements
//! ```

use supernovas::{Accuracy, Frame, Observer, OrbitalElements, Site, Time};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    // ── Observer ──────────────────────────────────────────────────────────
    // European Southern Observatory, La Silla, Chile.
    let site = Site::from_degrees(-29.2584, -70.7345, 2400.0)?;
    let observer = Observer::Geodetic(site);

    // ── Halley's Comet ────────────────────────────────────────────────────
    // Osculating elements from the 1986 perihelion passage.
    // Epoch: 1986 Feb 19.0 TDB = JD 2 446 495.5 TDB.
    // Source: Yeomans & Kiang (1981) with updates for the 1986 apparition.
    let halley = OrbitalElements {
        epoch_jd_tdb: 2_446_467.4,
        semi_major_axis_au: 17.834,
        eccentricity: 0.9673,
        arg_of_perihelion_deg: 111.33,
        ascending_node_deg: 58.42,
        inclination_deg: 162.26,
        mean_anomaly_at_epoch_deg: 38.38,
        ..Default::default()
    }
    .into_source("1P/Halley", 1_000_012)?;

    // ── Ceres ─────────────────────────────────────────────────────────────
    // Mean orbital elements at epoch 2025-Jan-17 TDB (JD 2 460 692.5).
    // Source: JPL Horizons small-body database.
    let ceres = OrbitalElements {
        epoch_jd_tdb: 2_460_692.5,
        semi_major_axis_au: 2.767,
        eccentricity: 0.0789,
        arg_of_perihelion_deg: 73.597,
        ascending_node_deg: 80.305,
        inclination_deg: 10.594,
        mean_anomaly_at_epoch_deg: 130.72,
        ..Default::default()
    }
    .into_source("1 Ceres", 2_000_001)?;

    // ── Two observation times ─────────────────────────────────────────────
    // 1986-04-10 02:00 UTC — Halley ~1 month past perihelion, near opposition.
    // JD 2 446 525.583 UTC; leap seconds = 24 (correct for 1986).
    let t_halley = Time::from_utc_jd(2_446_525.583, 24, 0.0)?;

    // 2025-06-15 22:00 UTC — Ceres near opposition in 2025.
    // JD 2 460 841.417 UTC; leap seconds = 37.
    let t_ceres = Time::from_utc_jd(2_460_841.417, 37, 0.0)?;

    // ── Observe ───────────────────────────────────────────────────────────
    println!("── Halley's Comet  (1986-04-10 02:00 UTC, La Silla) ──────────");
    let frame_halley = Frame::new(Accuracy::Reduced, &observer, &t_halley)?;
    let h_halley = frame_halley.observe(&halley)?;
    println!("  {h_halley}");
    println!(
        "  az {:.2}°  el {:.2}°  zenith angle {:.2}°",
        h_halley.azimuth().deg().rem_euclid(360.0),
        h_halley.elevation().deg(),
        h_halley.zenith_angle().deg(),
    );

    println!("\n── Ceres  (2025-06-15 22:00 UTC, La Silla) ───────────────────");
    let frame_ceres = Frame::new(Accuracy::Reduced, &observer, &t_ceres)?;
    let h_ceres = frame_ceres.observe(&ceres)?;
    println!("  {h_ceres}");
    println!(
        "  az {:.2}°  el {:.2}°  zenith angle {:.2}°",
        h_ceres.azimuth().deg().rem_euclid(360.0),
        h_ceres.elevation().deg(),
        h_ceres.zenith_angle().deg(),
    );

    Ok(())
}
