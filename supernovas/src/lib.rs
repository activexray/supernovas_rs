//! Safe Rust bindings to [SuperNOVAS](https://github.com/sigmyne/supernovas).

#![no_std]

extern crate alloc;

pub use supernovas_ffi as sys;

pub mod error;
pub mod frame;
pub mod observer;
pub mod scalar;
pub mod source;
pub mod spherical;
pub mod time;
pub mod unit;
pub mod vector;

pub use error::{Error, Result};
pub use frame::{Accuracy, Frame};
pub use observer::{Observer, Site, Weather};
pub use scalar::{Angle, Coordinate, Interval, Pressure, ScalarVelocity, Temperature, TimeAngle};
pub use source::CatalogEntry;
pub use spherical::{Galactic, Horizontal, Spherical};
pub use time::Time;
pub use vector::{Position, Velocity};
