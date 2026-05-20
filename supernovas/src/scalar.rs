//! Scalar quantities: dimensioned newtypes around `f64`.
//!
//! These are the foundational "leaf" types of the wrapper. Each is a
//! finite, validated value with type-safe constructors and unit-aware
//! accessors. Heavier types (`Vector`, `Position`, `Frame`, ...) build on
//! these.

mod angle;
mod coordinate;
mod interval;
mod pressure;
mod scalar_velocity;
mod temperature;
mod time_angle;

pub use angle::Angle;
pub use coordinate::Coordinate;
pub use interval::Interval;
pub use pressure::Pressure;
pub use scalar_velocity::ScalarVelocity;
pub use temperature::Temperature;
pub use time_angle::TimeAngle;
