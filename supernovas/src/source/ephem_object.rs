use core::{ffi::CStr, mem::MaybeUninit};

use supernovas_ffi::{make_ephem_object, object};

use crate::error::{Error, Result};

/// An arbitrary solar-system body looked up by name and NAIF ID from the
/// installed ephemeris provider.
///
/// Use this for bodies not in the fixed [`crate::SolarBody`] list — comets,
/// asteroids, spacecraft, or any object your ephemeris knows by name or NAIF
/// ID. The installed provider must recognize the name/ID at observation time;
/// if it doesn't, `novas_sky_pos` will return an error.
///
/// Configure a provider before use:
/// - `CalcephEphemeris` (`calceph` feature)
/// - `AniseEphemeris` (`anise` feature)
#[derive(Clone, Copy)]
pub struct EphemObject {
    object: object,
}

impl core::fmt::Debug for EphemObject {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EphemObject")
            .field("number", &self.object.number)
            .finish_non_exhaustive()
    }
}

impl super::sealed::Sealed for EphemObject {}
impl super::Source for EphemObject {
    fn as_object(&self) -> &object {
        &self.object
    }
}

impl EphemObject {
    /// Construct an ephemeris source by name and NAIF ID.
    ///
    /// `name` must be ASCII (no interior NULs) and shorter than 50 bytes.
    /// `naif_id` is the SPICE/NAIF integer identifier (e.g. 499 for Mars,
    /// 1000012 for Halley's comet); pass 0 if unknown.
    pub fn new(name: &str, naif_id: i64) -> Result<Self> {
        let bytes = name.as_bytes();
        if bytes.contains(&0) || bytes.len() >= 50 {
            return Err(Error::Parse);
        }
        let mut name_buf = [0u8; 50];
        name_buf[..bytes.len()].copy_from_slice(bytes);
        let name_cs =
            CStr::from_bytes_with_nul(&name_buf[..=bytes.len()]).map_err(|_| Error::Parse)?;

        let mut obj = MaybeUninit::<object>::zeroed();
        // SAFETY: make_ephem_object fully initializes *obj on a zero return.
        let rc = unsafe { make_ephem_object(name_cs.as_ptr(), naif_id as _, obj.as_mut_ptr()) };
        if rc != 0 {
            return Err(Error::ffi(rc));
        }
        Ok(EphemObject {
            object: unsafe { obj.assume_init() },
        })
    }

    /// The NAIF ID this object was constructed with.
    #[allow(clippy::useless_conversion)] // c_long = i32 on 32-bit targets
    #[must_use]
    pub fn naif_id(&self) -> i64 {
        i64::from(self.object.number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_success_and_naif_id() {
        let obj = EphemObject::new("Voyager 1", -31).unwrap();
        assert_eq!(obj.naif_id(), -31);
    }

    #[test]
    fn rejects_interior_nul() {
        assert!(matches!(
            EphemObject::new("bad\0name", 0),
            Err(Error::Parse)
        ));
    }

    #[test]
    fn rejects_name_too_long() {
        let long = "x".repeat(50);
        assert!(matches!(EphemObject::new(&long, 0), Err(Error::Parse)));
    }

    #[test]
    fn naif_id_zero_is_valid() {
        EphemObject::new("Unknown", 0).unwrap();
    }

    #[test]
    fn debug_format_is_non_empty() {
        let obj = EphemObject::new("Halley", 1_000_012).unwrap();
        let s = format!("{obj:?}");
        assert!(!s.is_empty());
    }
}
