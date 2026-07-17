# SuperNOVAS C API Coverage

Tracks which parts of the SuperNOVAS v1.7.x C API are wrapped by the `supernovas` crate.

- `[x]` - implemented
- `[ ]` - planned
- plain bullet - not planned: either an internal building block best accessed via
  `supernovas::sys`, redundant with a Rust-native equivalent, or superseded by the
  frame-based API

---

## Time (`novas_timespec`)

- [x] `novas_set_time` / `novas_set_split_time` → `Time::from_jd`, `Time::from_split_jd`
- [x] `novas_set_unix_time` → `Time::from_unix`
- [ ] `novas_set_str_time` - parse ISO/calendar string into `Time`
- [x] `novas_set_current_time` → `Time::now` (std only)
- [x] `novas_get_time` / `novas_get_split_time` → `Time::jd`, `Time::split_jd`
- [x] `novas_offset_time` → `Time + Interval` / `Time - Interval`
- [x] `novas_timescale_offset` → `Time::timescale_offset`
- [x] `novas_time_leap` → `Time::leap_seconds`
- [ ] `novas_time_gst` / `novas_time_lst` - GST/LST from a `Time`
- `novas_diff_time` / `novas_diff_time_scale` - superseded by `Time - Time → Interval` arithmetic
- `novas_timestamp` / `novas_iso_timestamp` - superseded by `Display` / `format!`
- `tt2tdb` / `tdb2tt` - internal scalar TT↔TDB; accessible via `sys`
- `get_ut1_to_tt` / `get_utc_to_tt` - superseded by `novas_timescale_offset`

---

## Observers

- [x] `make_observer_at_geocenter` → `Observer::Geocenter`
- [x] `make_observer_on_surface` → `Observer::Geodetic(Site)`
- [ ] `make_observer_in_space` / `make_solar_system_observer` → satellite / solar-system observer
- [ ] `make_airborne_observer` → airborne observer
- [ ] `make_gps_observer` / `make_itrf_observer` → GPS/ITRF observers

### Site construction

- [x] `make_observer_on_surface` (lat/lon/height/T/P) → `Site::from_degrees`
- [ ] `make_itrf_site` / `make_gps_site` - ITRF/GPS coordinate constructors
- [ ] `make_xyz_site` - ECEF Cartesian site
- [ ] `novas_geodetic_to_cartesian` / `novas_cartesian_to_geodetic` - geodetic ↔ ECEF
- `make_on_surface` - redundant with `Site::new`
- `novas_set_default_weather` - internal C helper
- [x] `novas_site_gcrs_posvel` → `Frame::site_gcrs_posvel` (see also Interferometry section)

---

## Sources

### Catalog (sidereal) entries

- [x] `make_cat_entry` + `make_cat_object` → `CatalogEntry::icrs`
- [x] proper motion, parallax, radial velocity → `CatalogEntry::with_*` builders
- [x] `make_cat_object_sys` → `CatalogEntry::in_system(name, ra, dec, CatalogSystem)`
- [x] `make_redshifted_object_sys` → `CatalogEntry::redshifted_icrs(name, ra, dec, z)`
- [x] `novas_set_ssb_vel` → `CatalogEntry::with_ssb_velocity`
- [x] `novas_set_lsr_vel` → `CatalogEntry::with_lsr_velocity(rv, epoch_jd)`
- [x] `novas_set_redshift` → `CatalogEntry::with_redshift`
- [x] `novas_set_distance` → `CatalogEntry::with_distance`
- [ ] `transform_cat` - transform a `cat_entry` between catalog epochs
- [ ] `transform_hip` - Hipparcos catalog → FK5 J2000
- `novas_init_cat_entry` - internal zero-init helper
- `novas_set_parallax` / `novas_set_proper_motion` - covered by `with_parallax` / `with_proper_motion_mas_per_yr`

### Planets and solar-system bodies

- [x] `make_planet` → `Planet` / `SolarBody`
- [x] `make_ephem_object` → `EphemObject` (arbitrary NAIF body by name/number)
- [ ] `novas_approx_sky_pos` - fast approximate sky position via built-in VSOP/ELP (no external ephemeris)
- [ ] `novas_approx_heliocentric` - low-accuracy heliocentric position

### Keplerian orbitals

- [x] `make_orbital_object` → `OrbitalObject` / `OrbitalElements`
- [ ] `novas_make_planet_orbit` / `novas_make_moon_orbit` / `novas_make_moon_mean_orbit` - build `novas_orbital` from built-in models
- `novas_orbit_posvel` / `novas_orbit_native_posvel` - internal Keplerian evaluator; accessible via `sys`

---

## Frame and observation pipeline

- [x] `novas_make_frame` → `Frame::new` / `Frame::with_polar_motion` / `Frame::with_auto_polar_motion` (`eop` feature)
- [x] `novas_sky_pos` → `Source::apparent_in` → `Apparent` (all source types)
- [x] `novas_app_to_hor` → `Apparent::to_horizontal` / `to_horizontal_with_refraction`
- [x] `novas_change_observer` → `Frame::update_observer`
- [x] `novas_hor_to_app` → `Frame::horizontal_to_apparent` (partial `Apparent`: `dis`/`rv` zeroed)
- [x] `novas_geom_posvel` → `Source::geometric_in` → `Geometric`
- [x] `novas_make_transform` / `novas_transform_sky_pos` / `novas_transform_vector` → `Transform::new` / `apply_sky_pos` / `apply_vector` (also `Frame::transform`)
- [x] `novas_invert_transform` → `Transform::invert` (swaps system tags the C call leaves stale)
- [x] `novas_frame_lst` → `Frame::lst` (`TimeAngle`, `[0, 24h)`)
- `novas_geom_to_app` / `novas_app_to_geom` - internal unit-vector ↔ sky-pos conversions; accessible via `sys`
- `novas_frame_is_initialized` - unnecessary: Rust construction guarantees a valid frame

---

## Coordinate transforms (vector / angle level)

- [x] `equ2gal` / `gal2equ` → `Equatorial::to_galactic` / `Galactic::to_equatorial_icrs`
- [x] `equ2ecl` / `ecl2equ` → `Equatorial::to_ecliptic` / `Ecliptic::to_equatorial`
- [x] `novas_sys_to_icrs` / `novas_icrs_to_sys` → `Equatorial::to_system`
- [x] `radec2vector` / `vector2radec` - internal to `Equatorial::to_system`
- [x] `cirs_to_itrs` / `itrs_to_cirs` → `Frame::cirs_to_itrs` / `Frame::itrs_to_cirs` (IAU 2000 path)
- [ ] `tod_to_itrs` / `itrs_to_tod` - **not wrapped**: the pre-IAU-2000 legacy path. `Transform` covers any system pair (including TOD) when needed, and CIRS is the modern default.
- [x] `hor_to_itrs` / `itrs_to_hor` → `Frame::horizontal_to_itrs` / `Frame::itrs_to_horizontal`
- `frame_tie` - internal ICRS ↔ dynamical-frame tie rotation; accessible via `sys`
- `gcrs_to_cirs` / `cirs_to_gcrs` and the full pairwise family (`gcrs_to_j2000`, `gcrs_to_mod`,
  `gcrs_to_tod`, `j2000_to_gcrs`, `j2000_to_tod`, `tod_to_cirs`, `tod_to_gcrs`, `tod_to_j2000`,
  `cirs_to_tod`, `mod_to_gcrs`) - internal frame-rotation building blocks; accessible via `sys`
  (the public `Transform` API builds the same rotation chains as needed)
- `wobble` / `nutation` / `precession` - internal rotation steps; accessible via `sys`

---

## Refraction

- [x] `novas_standard_refraction` → `Refraction::Standard`
- [x] `novas_optical_refraction` → `Refraction::Optical`
- [x] `novas_radio_refraction` → `Refraction::Radio`
- [ ] `novas_wave_refraction` - wavelength-specific refraction model
- [ ] `novas_refract_wavelength` - set the wavelength for `novas_wave_refraction`
- [ ] `novas_inv_refract` - inverse refraction (observed elevation → apparent elevation)
- `refract` / `refract_astro` (**legacy**) - superseded by `novas_optical_refraction`

---

## Ephemeris backends

- [x] `set_planet_provider` / `set_planet_provider_hp` → `PlanetProvider` blanket impl / `EphemerisProvider`
- [x] `novas_use_calceph` → `CalcephEphemeris`
- [x] `novas_to_naif_planet` - available via `sys` for custom `PlanetProvider` impls
- [x] `novas_to_dexxx_planet` - available via `sys`
- [x] `set_ephem_provider` → called by `AniseEphemeris::install` to register the ephem-object (non-planet) provider
- `get_ephem_provider` - process-global state readback; not useful to expose
- `set_nutation_lp_provider` - custom low-precision nutation hook; won't wrap
- `novas_calceph_use_ids` / `novas_use_calceph_planets` / `novas_calceph_is_thread_safe` - internal CALCEPH tuning; accessible via `sys`

---

## Earth Orientation Parameters (EOP)

Requires the `eop` crate feature (enables CURL in the vendored build).

- [x] `novas_fetch_eop` → `eop::fetch_eop` (`eop` feature)
- [x] `novas_fetch_eop_unix` → `eop::fetch_eop_unix` (`eop` feature)
- [x] `novas_reset_eop` → `eop::reset_eop` (`eop` feature)
- [x] `novas_set_auto_fetch_eop` / `novas_is_auto_fetch_eop` → `eop::set_auto_fetch_eop` / `eop::is_auto_fetch_eop` (`eop` feature)
- [x] `novas_set_eop_url` / `novas_get_eop_url` / `novas_get_eop_itrf_year` → `eop::set_eop_url` / `eop::get_eop_url` / `eop::get_eop_itrf_year` (`eop` feature)
- [x] `novas_diurnal_eop_at_time` → `eop::diurnal_eop_at_time` (`eop` feature)
- [x] `novas_itrf_transform_eop` → `eop::itrf_transform_eop` (`eop` feature)
- [x] `novas_set_leap_list` → `eop::set_leap_list` (`eop` feature)
- [ ] `novas_lookup_leap` - leap-second lookup by Unix timestamp
- `novas_diurnal_eop` - takes raw GMST + `novas_delaunay_args`; accessible via `sys`

---

## String parsing and formatting

- [x] `novas_str_degrees` → `Angle::from_str` (DMS)
- [x] `novas_str_hours` → `TimeAngle::from_str` (HMS)
- [x] `novas_print_dms` → `Angle::fmt`
- [x] `novas_print_hms` → `TimeAngle::fmt`
- [ ] `novas_parse_date` / `novas_parse_iso_date` / `novas_parse_date_format` - calendar date string → JD
- [ ] `novas_date` / `novas_date_scale` - convenience date string → JD
- [ ] `novas_epoch` - parse an epoch string (`"J2000"`, `"B1950"`, …) → JD
- `novas_parse_dms` / `novas_parse_hms` / `novas_parse_degrees` / `novas_parse_hours` - trailing-pointer C idiom; `FromStr` covers this
- `novas_dms_degrees` / `novas_hms_hours` - non-validating variants; we validate at construction
- `novas_print_decimal` / `novas_print_timescale` - superseded by Rust `Display`

---

## Angular and spherical utilities

- [x] `novas_sep` → `Spherical::distance_to` / `Horizontal::distance_to`
- [x] `novas_offset_by` → `Horizontal::offset` (great-circle arc; also `Horizontal::offset_by_sky` for sky-frame offsets)
- `novas_equ_sep` - `novas_sep(15.0 * ra, dec, ...)` thin wrapper. Redundant: `Equatorial` already has `as_spherical()` → `Spherical::distance_to` (and we'd need only a `Distance` impl to avoid duplicating the method). Not worth wrapping.
- [ ] `novas_object_sep` - angular separation between two `object`s in a frame
- [ ] `novas_moon_angle` / `novas_sun_angle` - proximity to Moon / Sun from a frame
- [ ] `novas_e2h_offset` / `novas_h2e_offset` - equatorial ↔ horizontal small-angle offset
- [ ] `novas_epa` / `novas_hpa` - equatorial / horizontal parallactic angle
- `novas_norm_ang` - superseded by `Angle`, which normalises at construction

---

## Interferometry (UVW baselines)

- [x] `novas_uvw` → `uvw::uvw` (generic; any shared coordinate system, station positions relative to the array reference)
- [ ] `novas_site_uvw` - **not wrapped**: it is the geocentric-only convenience for `novas_uvw` (it calls `novas_site_gcrs_posvel` + `novas_uvw` internally, referenced to the geocenter). The Rust API exposes those two building blocks directly so callers get the higher-precision array-reference model the C docs recommend, without the geocentric limitation or the TOD-only input contract.
- [x] `novas_uvw_to_xyz` / `novas_xyz_to_uvw` → `uvw::uvw_to_xyz` / `uvw::xyz_to_uvw`
- [x] `novas_enu_to_itrs` / `novas_itrs_to_enu` → `Site::enu_to_itrs` / `Site::itrs_to_enu`
- [x] `novas_los_to_xyz` / `novas_xyz_to_los` → `uvw::los_to_xyz` / `uvw::xyz_to_los`
- [x] `novas_site_gcrs_posvel` → `Frame::site_gcrs_posvel` (GCRS position+velocity of a geodetic site; the building block that turns a ground station into the GCRS station vector `uvw::uvw` consumes)

---

## Rise / set / transit

- [ ] `novas_rises_above` / `novas_sets_below` - next time a source crosses a given elevation
- [ ] `novas_transit_time` - next upper transit time

---

## Moon and solar illumination

- [ ] `novas_moon_phase` / `novas_next_moon_phase` - Moon phase angle
- [ ] `novas_moon_elp_sky_pos` / `novas_moon_elp_posvel` - Moon position via ELP2000 (no external ephemeris)
- [ ] `novas_solar_illum` - fraction of source disk illuminated by the Sun
- [ ] `novas_solar_power` - solar flux at a body
- [ ] `novas_helio_dist` - heliocentric distance and rate

---

## Tracking

- [x] `novas_equ_track` → `track::EquatorialTrack::compute`
- [x] `novas_hor_track` → `track::HorizontalTrack::compute`
- [x] `novas_track_pos` → `EquatorialTrack::pos_at` / `HorizontalTrack::pos_at`

---

## Calendar and date utilities

- [ ] `julian_date` / `cal_date` - scalar JD ↔ Gregorian calendar
- [ ] `novas_jd_to_date` / `novas_jd_from_date` - JD ↔ Gregorian or Julian calendar
- [ ] `novas_day_of_week` / `novas_day_of_year` - calendar helpers

---

## Redshift / velocity utilities

- [ ] `novas_v2z` / `novas_z2v` - recession velocity ↔ redshift
- [ ] `novas_z_add` / `novas_z_inv` - compose / invert redshifts
- [ ] `redshift_vrad` / `unredshift_vrad` - apply / remove Doppler redshift to radial velocity
- [ ] `novas_lsr_to_ssb_vel` / `novas_ssb_to_lsr_vel` - LSR ↔ SSB velocity frame
- [ ] `novas_add_vel` / `novas_add_beta` - relativistic velocity addition

---

## ITRF / EOP transforms

- [ ] `novas_itrf_transform` / `novas_itrf_transform_site` - transform between ITRF realisations
- [x] `novas_itrf_transform_eop` → `eop::itrf_transform_eop` (`eop` feature)
- [ ] `novas_geodetic_transform_site` - transform between reference ellipsoids

---

## Internal / low-level (not planned)

These are building blocks used internally by the frame-based pipeline. Direct use is rarely
needed; they are accessible via `supernovas::sys` for advanced callers.

- `aberration` - stellar aberration correction
- `bary2obs` - barycentric → observer position
- `d_light` - light-travel distance
- `e_tilt` - Earth's obliquity and nutation angles
- `ee_ct` - equation of the equinoxes complementary terms
- `era` - Earth Rotation Angle
- `fund_args` - Delaunay fundamental arguments
- `grav_def` / `grav_planets` / `grav_undef` / `grav_vec` / `grav_undo_planets` - gravitational deflection
- `iau2000a` / `iau2000b` / `nu2000k` / `nutation_angles` - nutation algorithms
- `ira_equinox` - RA of the true equinox
- `light_time` / `light_time2` - light-time iteration
- `limb_angle` - limb and nadir angles
- `mean_obliq` - mean obliquity of the ecliptic
- `nutation` - apply nutation rotation to a vector
- `novas_Rx` / `novas_Ry` / `novas_Rz` / `novas_tiny_rotate` - rotation matrix helpers
- `novas_vdot` / `novas_vlen` / `novas_vdist` / `novas_vdist2` - vector math
- `novas_cio_gcrs_ra` / `cio_location` / `cio_basis` / `cio_ra` / `cio_array` - CIO calculations
- `novas_clock_skew` / `novas_mean_clock_skew` - relativistic clock corrections
- `novas_diurnal_eop` / `novas_diurnal_libration` / `novas_diurnal_ocean_tides` - diurnal EOP corrections (note: `novas_diurnal_eop_at_time` is wrapped as `eop::diurnal_eop_at_time`)
- `novas_gast` / `novas_gmst` / `novas_gmst_prec` - Greenwich sidereal/mean time
- `obs_posvel` / `obs_planets` - observer position and planet bundle
- `planet_lon` / `accum_prec` - planetary longitude / accumulated precession
- `polar_dxdy_to_dpsideps` - EOP pole offset conversion
- `proper_motion` - apply proper motion to a position vector
- `rad_vel` / `rad_vel2` - radial velocity corrections
- `spin` - diurnal rotation
- `starvectors` - catalog entry → position + motion vectors
- `tdb2tt` / `tt2tdb_hp` / `tt2tdb_fp` - TDB ↔ TT variants
- `terra` - site position/velocity relative to geocenter

---

## Legacy NOVAS API (will not be wrapped)

These are the pre-frame NOVAS interfaces. The frame-based API (`novas_make_frame` + `novas_sky_pos`) supersedes them entirely.

- `app_star` / `virtual_star` / `astro_star` / `local_star` / `topo_star`
- `app_planet` / `virtual_planet` / `astro_planet` / `local_planet` / `topo_planet` / `radec_planet`
- `place` / `place_star` / `place_cirs` / `place_gcrs` / `place_icrs` / `place_j2000` / `place_mod` / `place_tod`
- `gcrs2equ` / `geo_posvel` / `radec_star` / `mean_star`
- `equ2hor` - old horizontal conversion
- `sidereal_time` / `cel2ter` / `ter2cel` - old sidereal time and frame rotation
- `cel_pole` / `set_cio_locator_file` - old CIO file and pole-offset interface
- `cio_array` / `cio_basis` / `cio_location` / `cio_ra` - old CIO interface
- `earth_sun_calc` / `earth_sun_calc_hp` / `planet_ephem_provider` / `planet_ephem_provider_hp` - old ephemeris hooks
- `enable_earth_sun_hp` - legacy Earth/Sun precision toggle
- `readeph` - old Fortran-era ephemeris reader
- `grav_redshift` - scalar gravitational redshift (superseded by `novas_clock_skew`)
