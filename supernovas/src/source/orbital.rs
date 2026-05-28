use core::{ffi::CStr, mem::MaybeUninit};

use supernovas_ffi::{
    make_orbital_object, novas_orbital, novas_orbital_system, novas_planet::NOVAS_SUN,
    novas_reference_plane::NOVAS_ECLIPTIC_PLANE, novas_reference_system::NOVAS_ICRS, object,
};

use crate::error::{Error, Result};

/// Keplerian orbital elements for a solar-system small body.
///
/// Angles are in degrees; distances in AU; time in Julian days (TDB).
/// The optional fields (`mean_daily_motion`, `apsis_precession_years`,
/// `node_precession_years`) default to zero, which tells SuperNOVAS to
/// either derive them from the semi-major axis or ignore the precession.
///
/// # Example — Halley's comet (approximate)
/// ```no_run
/// use supernovas::OrbitalElements;
///
/// let elements = OrbitalElements {
///     epoch_jd_tdb: 2_446_467.4,
///     semi_major_axis_au: 17.834,
///     eccentricity: 0.9673,
///     arg_of_perihelion_deg: 111.33,
///     ascending_node_deg: 58.42,
///     inclination_deg: 162.26,
///     mean_anomaly_at_epoch_deg: 38.38,
///     ..Default::default()
/// };
/// let halley = elements.into_source("Halley", 1000012)?;
/// # Ok::<(), supernovas::Error>(())
/// ```
#[derive(Debug, Clone, Copy)]
pub struct OrbitalElements {
    /// Epoch of the elements (TDB Julian date).
    pub epoch_jd_tdb: f64,
    /// Semi-major axis (AU).
    pub semi_major_axis_au: f64,
    /// Eccentricity (dimensionless; 0 = circular, <1 = elliptic, ≥1 = open).
    pub eccentricity: f64,
    /// Argument of perihelion ω (degrees, J2000 ecliptic).
    pub arg_of_perihelion_deg: f64,
    /// Longitude of ascending node Ω (degrees, J2000 ecliptic).
    pub ascending_node_deg: f64,
    /// Orbital inclination i (degrees, J2000 ecliptic).
    pub inclination_deg: f64,
    /// Mean anomaly at epoch M₀ (degrees).
    pub mean_anomaly_at_epoch_deg: f64,
    /// Mean daily motion n (deg/day). Set to `0.0` to derive from `a`.
    pub mean_daily_motion_deg_per_day: f64,
    /// Perihelion/aphelion precession period (years). Set to `0.0` if unknown.
    pub apsis_precession_years: f64,
    /// Nodal precession period (years). Set to `0.0` if unknown.
    pub node_precession_years: f64,
}

impl Default for OrbitalElements {
    fn default() -> Self {
        OrbitalElements {
            epoch_jd_tdb: 0.0,
            semi_major_axis_au: 0.0,
            eccentricity: 0.0,
            arg_of_perihelion_deg: 0.0,
            ascending_node_deg: 0.0,
            inclination_deg: 0.0,
            mean_anomaly_at_epoch_deg: 0.0,
            mean_daily_motion_deg_per_day: 0.0,
            apsis_precession_years: 0.0,
            node_precession_years: 0.0,
        }
    }
}

impl OrbitalElements {
    /// Build an [`OrbitalObject`] from these heliocentric ecliptic (ICRS)
    /// elements.
    ///
    /// `name` must be ASCII (no interior NULs) and shorter than 50 bytes.
    /// `num` is the object number (MPC packed designation, NAIF ID, or 0).
    pub fn into_source(self, name: &str, num: i64) -> Result<OrbitalObject> {
        OrbitalObject::new(name, num, self)
    }
}

/// A solar-system source defined by Keplerian orbital elements.
///
/// Apparent positions are propagated analytically from the elements — no
/// external ephemeris provider is needed. Accuracy degrades for highly
/// perturbed orbits (Jupiter-family comets, near-Earth asteroids) compared to
/// a full N-body ephemeris, but this is ideal for newly discovered objects or
/// when only MPC/MPeC orbital elements are available.
///
/// Construct via [`OrbitalElements::into_source`] or [`OrbitalObject::new`].
#[derive(Clone, Copy)]
pub struct OrbitalObject {
    object: object,
}

impl core::fmt::Debug for OrbitalObject {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OrbitalObject")
            .field("number", &self.object.number)
            .finish_non_exhaustive()
    }
}

impl super::sealed::Sealed for OrbitalObject {}
impl super::Source for OrbitalObject {
    fn as_object(&self) -> &object {
        &self.object
    }
}

impl OrbitalObject {
    /// Construct a heliocentric ecliptic (ICRS/J2000) orbital source.
    ///
    /// This is the standard form for MPC/JPL Horizons small-body elements.
    /// For geocentric or other reference-frame elements, build a
    /// [`novas_orbital`] manually and use the raw FFI.
    ///
    /// `name` must be ASCII (no interior NULs) and shorter than 50 bytes.
    /// `num` is the object number (MPC number, NAIF ID, or 0 if unknown).
    pub fn new(name: &str, num: i64, elements: OrbitalElements) -> Result<Self> {
        let bytes = name.as_bytes();
        if bytes.contains(&0) || bytes.len() >= 50 {
            return Err(Error::Parse);
        }
        let mut name_buf = [0u8; 50];
        name_buf[..bytes.len()].copy_from_slice(bytes);
        let name_cs =
            CStr::from_bytes_with_nul(&name_buf[..=bytes.len()]).map_err(|_| Error::Parse)?;

        if !elements.epoch_jd_tdb.is_finite()
            || !elements.semi_major_axis_au.is_finite()
            || !elements.eccentricity.is_finite()
            || !elements.arg_of_perihelion_deg.is_finite()
            || !elements.ascending_node_deg.is_finite()
            || !elements.inclination_deg.is_finite()
            || !elements.mean_anomaly_at_epoch_deg.is_finite()
            || !elements.mean_daily_motion_deg_per_day.is_finite()
            || !elements.apsis_precession_years.is_finite()
            || !elements.node_precession_years.is_finite()
        {
            return Err(Error::NotFinite);
        }

        let system = novas_orbital_system {
            center: NOVAS_SUN,
            plane: NOVAS_ECLIPTIC_PLANE,
            type_: NOVAS_ICRS,
            obl: 0.0,
            Omega: 0.0,
        };
        let orbit = novas_orbital {
            system,
            jd_tdb: elements.epoch_jd_tdb,
            a: elements.semi_major_axis_au,
            e: elements.eccentricity,
            omega: elements.arg_of_perihelion_deg,
            Omega: elements.ascending_node_deg,
            i: elements.inclination_deg,
            M0: elements.mean_anomaly_at_epoch_deg,
            n: elements.mean_daily_motion_deg_per_day,
            apsis_period: elements.apsis_precession_years,
            node_period: elements.node_precession_years,
        };

        let mut obj = MaybeUninit::<object>::zeroed();
        // SAFETY: make_orbital_object fully initializes *obj on a zero return.
        let rc =
            unsafe { make_orbital_object(name_cs.as_ptr(), num as _, &orbit, obj.as_mut_ptr()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(OrbitalObject {
            object: unsafe { obj.assume_init() },
        })
    }

    /// The object number this source was constructed with.
    #[allow(clippy::useless_conversion)] // c_long = i32 on 32-bit targets
    pub fn number(&self) -> i64 {
        i64::from(self.object.number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn halley() -> OrbitalElements {
        OrbitalElements {
            epoch_jd_tdb: 2_446_467.4,
            semi_major_axis_au: 17.834,
            eccentricity: 0.9673,
            arg_of_perihelion_deg: 111.33,
            ascending_node_deg: 58.42,
            inclination_deg: 162.26,
            mean_anomaly_at_epoch_deg: 38.38,
            ..Default::default()
        }
    }

    #[test]
    fn default_elements_are_all_zero() {
        let e = OrbitalElements::default();
        assert_eq!(e.epoch_jd_tdb, 0.0);
        assert_eq!(e.eccentricity, 0.0);
    }

    #[test]
    fn into_source_round_trips_number() {
        let obj = halley().into_source("Halley", 1_000_012).unwrap();
        assert_eq!(obj.number(), 1_000_012);
    }

    #[test]
    fn new_success() {
        OrbitalObject::new("Test", 0, halley()).unwrap();
    }

    #[test]
    fn rejects_interior_nul_in_name() {
        assert!(matches!(
            OrbitalObject::new("bad\0name", 0, halley()),
            Err(Error::Parse)
        ));
    }

    #[test]
    fn rejects_name_too_long() {
        let long = "x".repeat(50);
        assert!(matches!(
            OrbitalObject::new(&long, 0, halley()),
            Err(Error::Parse)
        ));
    }

    #[test]
    fn rejects_non_finite_element() {
        let bad = OrbitalElements {
            eccentricity: f64::NAN,
            ..halley()
        };
        assert!(matches!(
            OrbitalObject::new("Bad", 0, bad),
            Err(Error::NotFinite)
        ));
    }

    #[test]
    fn debug_format_is_non_empty() {
        let obj = OrbitalObject::new("Test", 42, halley()).unwrap();
        let s = format!("{obj:?}");
        assert!(!s.is_empty());
    }
}
