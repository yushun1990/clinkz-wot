//! Host Discovery APIs.
//!
//! The current host surface reuses the embedded-ready query model and
//! in-memory directory. Production storage backends that require networking,
//! filesystems, databases, or async runtimes belong behind this `std` feature.

pub use crate::embedded::*;
