//! Non-production construction witness. Only production Core capture can mint
//! the real checked owners used by the separate aggregate composition probe.
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

mod compiler;
mod registration;
#[cfg(feature = "std")]
mod reserved;

pub use compiler::{
    ABORTS, ARTIFACT_DROPS, CALLBACKS, CAPACITY, COMPATIBILITY, InlineArtifact, Scenario,
};
#[cfg(feature = "std")]
pub use registration::legacy_host_registration;
#[cfg(feature = "std")]
pub use registration::production_host_registration;
pub use registration::production_static_registration;
pub use registration::{
    CaptureError, CheckedRegistration, CompleteRegistration, Representation, StaticCursor,
    StaticStep, capture, mismatched_configuration_registration, producer_only_registration,
    registration, unsupported_registration,
};
#[cfg(feature = "std")]
pub use reserved::{HostOutput, HostStep, ReservedCursor, reserve};
