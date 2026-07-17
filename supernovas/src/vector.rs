//! 3-D vector quantities.
//!
//! These types share a common storage shape (`[f64; 3]`) but are distinct
//! Rust types - `Position` cannot be implicitly used where a `Velocity` is
//! expected, and vice versa. Cross-type operations are explicit:
//!
//! - `Position / Interval -> Velocity`
//! - `Velocity * Interval -> Position` (and the symmetric `Interval * Velocity`)
//! - [`Velocity::travel`] is a named alias for `velocity * interval`.

mod position;
mod velocity;

pub use position::Position;
pub use velocity::Velocity;
