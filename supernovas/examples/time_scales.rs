//! Time construction, timescale conversion, and arithmetic.
//!
//! Demonstrates the `Time` and `Interval` API:
//!
//! - Constructing `Time` from a UTC Julian date.
//! - Reading the same instant in multiple timescales (`TT`, `TAI`, `TDB`).
//! - Querying clock offsets (`TDB − TT`, `TAI − UTC`).
//! - Shifting a `Time` forward or backward with `+`/`-` `Interval`.
//! - Computing the duration between two `Time` values with `Time - Time`.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example time_scales
//! ```

use supernovas::{Interval, Time, Timescale};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    // ── Construct from UTC ────────────────────────────────────────────────
    // 2025-01-01 00:00:00 UTC = JD 2 460 676.5 UTC.
    // TAI − UTC = 37 leap seconds; DUT1 (UT1 − UTC) ≈ 0.0 s (simplified).
    let t = Time::from_utc_jd(2_460_676.5, 37, 0.0)?;
    println!("── Constructed time ──────────────────────────────────────────");
    println!("  {t}");

    // ── Same instant in multiple timescales ───────────────────────────────
    println!("\n── Julian date in each timescale ─────────────────────────────");
    for scale in [
        Timescale::Utc,
        Timescale::Tai,
        Timescale::Tt,
        Timescale::Tdb,
        Timescale::Ut1,
    ] {
        println!("  {scale}  JD {:.9}", t.jd(scale));
    }

    // ── Clock offsets ─────────────────────────────────────────────────────
    // `timescale_offset(A, B)` returns A − B in seconds.
    println!("\n── Clock offsets ─────────────────────────────────────────────");
    let tai_minus_utc = t.timescale_offset(Timescale::Tai, Timescale::Utc);
    let tt_minus_tai = t.timescale_offset(Timescale::Tt, Timescale::Tai);
    let tdb_minus_tt = t.timescale_offset(Timescale::Tdb, Timescale::Tt);
    println!(
        "  TAI − UTC = {tai_minus_utc:+.3} s  (= {} leap seconds)",
        t.leap_seconds()
    );
    println!("  TT  − TAI = {tt_minus_tai:+.3} s  (fixed 32.184 s offset)");
    println!("  TDB − TT  = {tdb_minus_tt:+.6} s  (periodic, ≲ 1.7 ms)");

    // ── Arithmetic ────────────────────────────────────────────────────────
    println!("\n── Time arithmetic ───────────────────────────────────────────");
    let one_day = Interval::from_days(1.0)?;
    let one_year = Interval::from_julian_years(1.0)?;

    let t_plus_1d = t + one_day;
    let t_plus_1y = t + one_year;

    println!("  t            JD(UTC) = {:.6}", t.jd(Timescale::Utc));
    println!(
        "  t + 1 day    JD(UTC) = {:.6}",
        t_plus_1d.jd(Timescale::Utc)
    );
    println!(
        "  t + 1 year   JD(UTC) = {:.6}",
        t_plus_1y.jd(Timescale::Utc)
    );

    // Time − Time gives an Interval (TT-second difference).
    let gap: Interval = t_plus_1y - t;
    println!(
        "\n  1 Julian year = {:.3} days = {:.3} seconds",
        gap.days(),
        gap.seconds(),
    );

    // ── Ordering ──────────────────────────────────────────────────────────
    println!("\n── Ordering ──────────────────────────────────────────────────");
    println!("  t < t + 1 year? {}", t < t_plus_1y);
    println!("  t > t + 1 day?  {}", t > t_plus_1d);

    Ok(())
}
