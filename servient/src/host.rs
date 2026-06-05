//! Host Servient APIs.
//!
//! The current host surface reuses the embedded-ready composition layer. Future
//! host-only backends for sockets, async runtimes, filesystems, observability,
//! or concrete protocol sessions belong behind this `std` feature.

pub use crate::embedded::*;
