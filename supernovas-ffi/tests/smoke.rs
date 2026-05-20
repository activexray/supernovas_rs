#[test]
fn julian_date_j2000() {
    let jd = unsafe { supernovas_ffi::julian_date(2000, 1, 1, 12.0) };
    assert!((jd - 2_451_545.0).abs() < 1e-9, "got JD = {jd}");
}
