//! Protocol-neutral primitives shared below the WoT data model and runtime.
//!
//! This crate owns resource policies, typed work budgets, monotonic time, source
//! timestamps, and generation-safe slot components. It supports `no_std + alloc`
//! and contains no TD vocabulary, protocol behavior, executor, or host runtime.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub mod budget;
pub mod generation;
pub mod resource;
pub mod time;

pub use budget::{BudgetExceeded, WorkBudget, WorkClass};
pub use generation::{Generation, SlotIndex};
pub use resource::{
    AdmissionLedger, BenchmarkStaticReferenceV1, DirectoryClientDefaultV1, GatewayDefaultV1,
    ResourceAccount, ResourceKind, ResourceLimits, ResourceProfileId, ResourceReservation,
    StaticResourceProfile,
};
pub use time::{ClockId, MonotonicInstant, RuntimeClock, SourceTimestamp};
