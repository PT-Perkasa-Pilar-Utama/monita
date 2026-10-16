//! SQL for the checks module, one file per table group.

pub mod heartbeats;
mod monitors;

pub use monitors::*;
