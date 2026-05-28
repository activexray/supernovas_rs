# SuperNOVAS C API Coverage

Tracks which parts of the SuperNOVAS v1.6.0 C API are wrapped by the `supernovas` crate.
Items marked **legacy** are old NOVAS-style interfaces superseded by the frame-based API; they
will not be wrapped. Items marked **internal** are low-level building blocks unlikely to be
needed directly by users.

---

## Time (`novas_timespec`)

- [x] `novas_set_time` / `novas_set_split_time` → `Time::from_jd`, `Time::from_split_jd`
- [x] `novas_set_unix_time` → `Time::from_unix`
- [ ] `novas_set_str_time` — parse ISO/calendar string into `Time`
- [ ] `novas_set_current_time` — set to current system clock
- [ ] `novas_get_time` / `novas_get_split_time` — read back JD in any timescale
- [ ] `novas_diff_time` / `novas_diff_time_scale` — interval between two `Time`s
- [ ] `novas_offset_time` — shift a `Time` by seconds
- [ ] `novas_timescale_offset` — offset between two timescales at a given time
- [ ] `novas_time_leap` — check whether a `Time` falls in a leap second
- [ ] `novas_timestamp` / `novas_iso_timestamp` — format `Time` as a string
- [ ] `tt2tdb` / `tdb2tt` — TT ↔ TDB conversion (scalar, no `novas_timespec`)
- [ ] `get_ut1_to_tt` / `get_utc_to_tt` — convenience UT1/UTC→TT offsets
- [ ] `novas_time_gst` / `novas_time_lst` — GST/LST from a `Time`

---

## Observers

- [x] `make_observer_at_geocenter` → `Observer::Geocenter`
- [x] `make_observer_on_surface` → `Observer::Geodetic(Site)`
- [ ] `make_observer_in_space` / `make_solar_system_observer` → satellite / solar-system observer
- [ ] `make_airborne_observer` → airborne observer
- [ ] `make_gps_observer` / `make_itrf_observer` → GPS/ITRF observers

### Site construction

- [x] `make_observer_on_surface` (lat/lon/height/T/P) → `Site::from_degrees`
- [ ] `make_on_surface` (same, without creating an `observer`) → bare `Site`
- [ ] `make_itrf_site` / `make_gps_site` — ITRF/GPS coordinate constructors
- [ ] `make_xyz_site` — ECEF Cartesian site
- [ ] `novas_set_default_weather` — populate default weather on a site
- [ ] `novas_geodetic_to_cartesian` / `novas_cartesian_to_geodetic` — geodetic ↔ ECEF
- [ ] `novas_site_gcrs_posvel` — site position/velocity in GCRS

---

## Sources

### Catalog (sidereal) entries

- [x] `make_cat_entry` + `make_cat_object` → `CatalogEntry::icrs`
- [x] proper motion, parallax, radial velocity → `CatalogEntry::with_*` builders
- [ ] `novas_init_cat_entry` — zero-initialise a `cat_entry` (internal helper)
- [ ] `novas_set_parallax` / `novas_set_proper_motion` / `novas_set_redshift` — individual field setters (already covered by builders)
- [ ] `novas_set_distance` / `novas_set_lsr_vel` / `novas_set_ssb_vel` — LSR/distance setters
- [ ] `make_redshifted_cat_entry` / `make_redshifted_object` — cosmologically-redshifted point source
- [ ] `make_cat_object_sys` — catalog object in a named coordinate system other than FK5
- [ ] `transform_cat` — transform a `cat_entry` between catalog epochs
- [ ] `transform_hip` — Hipparcos catalog → FK5 J2000

### Planets and solar-system bodies

- [ ] `make_planet` → typed `Planet` source
- [ ] `make_ephem_object` → `EphemerisSource` (arbitrary NAIF body by name/number)
- [ ] `novas_approx_sky_pos` — fast approximate sky position using built-in VSOP/ELP models (no external ephemeris required)
- [ ] `novas_approx_heliocentric` — low-accuracy heliocentric position

### Keplerian orbitals

- [ ] `make_orbital_object` → `OrbitalSource`
- [ ] `novas_orbit_posvel` / `novas_orbit_native_posvel` — evaluate Keplerian orbit
- [ ] `novas_make_planet_orbit` / `novas_make_moon_orbit` / `novas_make_moon_mean_orbit` — build `novas_orbital` from built-in models

---

## Frame and observation pipeline

- [x] `novas_make_frame` → `Frame::new` / `Frame::with_polar_motion`
- [x] `novas_sky_pos` → `CatalogEntry::apparent_in` → `Apparent`
- [x] `novas_app_to_hor` → `Apparent::to_horizontal` / `to_horizontal_with_refraction`
- [ ] `novas_change_observer` — rebuild a frame with a different observer at the same time
- [ ] `novas_hor_to_app` — inverse: horizontal → apparent equatorial
- [ ] `novas_geom_posvel` — geometric (astrometric) position+velocity of a source
- [ ] `novas_geom_to_app` — geometric unit-vector → apparent sky position
- [ ] `novas_app_to_geom` — apparent sky position → geometric unit-vector
- [ ] `novas_make_transform` / `novas_transform_sky_pos` / `novas_transform_vector` — pre-built rotation matrix for fast repeated transforms
- [ ] `novas_invert_transform` — invert a `novas_transform`
- [ ] `novas_frame_is_initialized` — guard against using an uninitialised frame
- [ ] `novas_frame_lst` — local sidereal time of the frame's observer

---

## Coordinate transforms (vector / angle level)

- [x] `equ2gal` / `gal2equ` → `Equatorial::to_galactic` / `Galactic::to_equatorial_icrs`
- [x] `equ2ecl` / `ecl2equ` → `Equatorial::to_ecliptic` / `Ecliptic::to_equatorial`
- [x] `novas_sys_to_icrs` / `novas_icrs_to_sys` → `Equatorial::to_system`
- [x] `radec2vector` / `vector2radec` — internal to `Equatorial::to_system`
- [ ] `frame_tie` — ICRS ↔ dynamical-frame tie rotation
- [ ] `gcrs_to_cirs` / `cirs_to_gcrs` and the full family of pairwise transforms
  (`gcrs_to_j2000`, `gcrs_to_mod`, `gcrs_to_tod`, `j2000_to_gcrs`, `j2000_to_tod`,
  `tod_to_cirs`, `tod_to_gcrs`, `tod_to_j2000`, `cirs_to_tod`, `mod_to_gcrs`)
- [ ] `cirs_to_itrs` / `tod_to_itrs` / `itrs_to_cirs` / `itrs_to_tod` — ITRS ↔ celestial transforms
- [ ] `hor_to_itrs` / `itrs_to_hor` — horizontal ↔ ITRS
- [ ] `wobble` — polar motion rotation
- [ ] `nutation` — nutation rotation
- [ ] `precession` — precession rotation
- [ ] `gcrs2equ` (**legacy**) — GCRS → equatorial of date

---

## Refraction

- [x] `novas_standard_refraction` → `Refraction::Standard`
- [x] `novas_optical_refraction` → `Refraction::Optical`
- [x] `novas_radio_refraction` → `Refraction::Radio`
- [ ] `novas_wave_refraction` — wavelength-specific refraction model
- [ ] `novas_refract_wavelength` — set the wavelength for `novas_wave_refraction`
- [ ] `novas_inv_refract` — inverse refraction (observed elevation → apparent elevation)
- [ ] `refract` / `refract_astro` (**legacy**) — old refraction interface

---

## Ephemeris backends

- [x] `set_planet_provider` / `set_planet_provider_hp` → `PlanetProvider` blanket impl / `EphemerisProvider`
- [x] `novas_use_calceph` → `CalcephEphemeris`
- [x] `novas_to_naif_planet` — available via `sys` for custom `PlanetProvider` impls
- [x] `novas_to_dexxx_planet` — available via `sys`
- [ ] `set_ephem_provider` / `get_ephem_provider` — generic named-body ephemeris provider (not planet-specific)
- [ ] `set_nutation_lp_provider` — plug in a custom low-precision nutation model
- [ ] `novas_calceph_use_ids` / `novas_use_calceph_planets` / `novas_calceph_is_thread_safe` — CALCEPH tuning

---

## String parsing and formatting

- [x] `novas_str_degrees` → `Angle::from_str` (DMS)
- [x] `novas_str_hours` → `TimeAngle::from_str` (HMS)
- [x] `novas_print_dms` → `Angle::fmt`
- [x] `novas_print_hms` → `TimeAngle::fmt`
- [ ] `novas_parse_dms` / `novas_parse_hms` / `novas_parse_degrees` / `novas_parse_hours` — parse with a trailing-pointer (tail) for embedded strings
- [ ] `novas_parse_date` / `novas_parse_iso_date` / `novas_parse_date_format` — calendar date string → JD
- [ ] `novas_date` / `novas_date_scale` — convenience date string → JD
- [ ] `novas_dms_degrees` / `novas_hms_hours` — like `str_degrees/hours` but without validation
- [ ] `novas_epoch` — parse an epoch string (`"J2000"`, `"B1950"`, …) → JD
- [ ] `novas_print_decimal` — format a number to a fixed-decimal string
- [ ] `novas_print_timescale` — timescale enum → string label

---

## Angular and spherical utilities

- [ ] `novas_sep` — great-circle separation between two (lon, lat) pairs
- [ ] `novas_equ_sep` — great-circle separation between two (RA, Dec) pairs
- [ ] `novas_object_sep` — angular separation between two `object`s in a frame
- [ ] `novas_moon_angle` / `novas_sun_angle` — proximity to Moon / Sun from a frame
- [ ] `novas_e2h_offset` / `novas_h2e_offset` — equatorial ↔ horizontal small-angle offset
- [ ] `novas_epa` / `novas_hpa` — equatorial / horizontal parallactic angle
- [ ] `novas_norm_ang` — normalise an angle to (-π, π]

---

## Interferometry (UVW baselines)

- [ ] `novas_uvw` / `novas_site_uvw` — baseline UVW from station positions
- [ ] `novas_uvw_to_xyz` / `novas_xyz_to_uvw` — UVW ↔ XYZ
- [ ] `novas_enu_to_itrs` / `novas_itrs_to_enu` — ENU ↔ ITRS
- [ ] `novas_los_to_xyz` / `novas_xyz_to_los` — line-of-sight ↔ XYZ

---

## Rise / set / transit

- [ ] `novas_rises_above` / `novas_sets_below` — next time a source crosses a given elevation
- [ ] `novas_transit_time` — next upper transit time

---

## Moon and solar illumination

- [ ] `novas_moon_phase` / `novas_next_moon_phase` — Moon phase angle
- [ ] `novas_moon_elp_sky_pos` / `novas_moon_elp_posvel` — Moon position via ELP2000 (no external ephemeris)
- [ ] `novas_solar_illum` — fraction of source disk illuminated by the Sun
- [ ] `novas_solar_power` — solar flux at a body
- [ ] `novas_helio_dist` — heliocentric distance and rate

---

## Tracking

- [ ] `novas_equ_track` — interpolation track for equatorial coordinates
- [ ] `novas_hor_track` — interpolation track for horizontal coordinates
- [ ] `novas_track_pos` — evaluate a `novas_track` at a given time

---

## Calendar and date utilities

- [ ] `julian_date` / `cal_date` — scalar JD ↔ Gregorian calendar (**legacy**, simple wrappers)
- [ ] `novas_jd_to_date` / `novas_jd_from_date` — JD ↔ Gregorian or Julian calendar
- [ ] `novas_day_of_week` / `novas_day_of_year` — calendar helpers

---

## Redshift / velocity utilities

- [ ] `novas_v2z` / `novas_z2v` — recession velocity ↔ redshift
- [ ] `novas_z_add` / `novas_z_inv` — compose / invert redshifts
- [ ] `redshift_vrad` / `unredshift_vrad` — apply / remove Doppler redshift to radial velocity
- [ ] `novas_lsr_to_ssb_vel` / `novas_ssb_to_lsr_vel` — LSR ↔ SSB velocity frame
- [ ] `novas_add_vel` / `novas_add_beta` — relativistic velocity addition

---

## ITRF / EOP transforms

- [ ] `novas_itrf_transform` / `novas_itrf_transform_site` — transform between ITRF realisations
- [ ] `novas_itrf_transform_eop` — apply EOP corrections between ITRF frames
- [ ] `novas_geodetic_transform_site` — transform between reference ellipsoids

---

## Internal / low-level (not planned)

These are building blocks used internally by the frame-based pipeline. Direct use is rarely
needed; they are accessible via `supernovas::sys` for advanced callers.

- `aberration` — stellar aberration correction
- `bary2obs` — barycentric → observer position
- `d_light` — light-travel distance
- `e_tilt` — Earth's obliquity and nutation angles
- `ee_ct` — equation of the equinoxes complementary terms
- `era` — Earth Rotation Angle
- `fund_args` — Delaunay fundamental arguments
- `grav_def` / `grav_planets` / `grav_undef` / `grav_vec` / `grav_undo_planets` — gravitational deflection
- `iau2000a` / `iau2000b` / `nu2000k` / `nutation_angles` — nutation algorithms
- `ira_equinox` — RA of the true equinox
- `light_time` / `light_time2` — light-time iteration
- `limb_angle` — limb and nadir angles
- `mean_obliq` — mean obliquity of the ecliptic
- `nutation` — apply nutation rotation to a vector
- `novas_Rx` / `novas_Ry` / `novas_Rz` / `novas_tiny_rotate` — rotation matrix helpers
- `novas_vdot` / `novas_vlen` / `novas_vdist` / `novas_vdist2` — vector math
- `novas_cio_gcrs_ra` / `cio_location` / `cio_basis` / `cio_ra` / `cio_array` — CIO calculations
- `novas_clock_skew` / `novas_mean_clock_skew` — relativistic clock corrections
- `novas_diurnal_eop` / `novas_diurnal_libration` / `novas_diurnal_ocean_tides` — diurnal EOP corrections
- `novas_gast` / `novas_gmst` / `novas_gmst_prec` — Greenwich sidereal/mean time
- `obs_posvel` / `obs_planets` — observer position and planet bundle
- `planet_lon` / `accum_prec` — planetary longitude / accumulated precession
- `polar_dxdy_to_dpsideps` — EOP pole offset conversion
- `proper_motion` — apply proper motion to a position vector
- `rad_vel` / `rad_vel2` — radial velocity corrections
- `spin` — diurnal rotation
- `starvectors` — catalog entry → position + motion vectors
- `tdb2tt` / `tt2tdb_hp` / `tt2tdb_fp` — TDB ↔ TT variants
- `terra` — site position/velocity relative to geocenter

---

## Legacy NOVAS API (will not be wrapped)

These are the pre-frame NOVAS interfaces. The frame-based API (`novas_make_frame` + `novas_sky_pos`) supersedes them entirely.

- `app_star` / `virtual_star` / `astro_star` / `local_star` / `topo_star`
- `app_planet` / `virtual_planet` / `astro_planet` / `local_planet` / `topo_planet` / `radec_planet`
- `place` / `place_star` / `place_cirs` / `place_gcrs` / `place_icrs` / `place_j2000` / `place_mod` / `place_tod`
- `gcrs2equ` / `geo_posvel` / `radec_star` / `mean_star`
- `equ2hor` — old horizontal conversion
- `sidereal_time` / `cel2ter` / `ter2cel` — old sidereal time and frame rotation
- `cel_pole` / `set_cio_locator_file` — old CIO file and pole-offset interface
- `cio_array` / `cio_basis` / `cio_location` / `cio_ra` — old CIO interface
- `earth_sun_calc` / `earth_sun_calc_hp` / `planet_ephem_provider` / `planet_ephem_provider_hp` — old ephemeris hooks
- `enable_earth_sun_hp` — legacy Earth/Sun precision toggle
- `readeph` — old Fortran-era ephemeris reader
- `grav_redshift` — scalar gravitational redshift (superseded by `novas_clock_skew`)
