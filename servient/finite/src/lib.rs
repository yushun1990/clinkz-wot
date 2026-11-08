//! Finite Engine execution. The shared kernel lives in Core; Host and borrowed
//! owners choose their own storage, authority, native facets and driving.
#![no_std]
extern crate alloc;

pub mod borrowed_consumer;

#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
mod host;
#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
pub use host::*;
