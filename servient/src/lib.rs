//! Servient composition for Web of Things flows (baseline v4.1 §7).
//!
//! Wires protocol-neutral core dispatch, Discovery, and protocol bindings
//! into a single non-generic runtime: produce/consume/discover, binding-owned
//! driving, frozen-TD lifecycle.
//!
//! The legacy Servient facade requires the `async` feature (it holds a
//! `dyn Discoverer`, drives async handlers, and consumes via async
//! `ClientBinding`s). The independent [`consumer_property_read`] runtime and
//! static Property Read surface support manual progress in bare `no_std`.
//! The `std` Consumer runtime also uses explicit manual progress.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod consumer_property_read;
mod error;
mod property_read;
mod registry;

#[cfg(feature = "std")]
mod builder;
#[cfg(feature = "async")]
mod handle;
#[cfg(feature = "async")]
mod servient;

pub use error::{ServientError, ServientResult};
pub use property_read::{CompiledPlanSetState, ExposeState, StaticServient, StaticServientBuilder};

#[cfg(feature = "async")]
pub use handle::{ConsumedThingHandle, ExposedThingHandle};
#[cfg(feature = "async")]
pub use servient::{Servient, ShutdownHandle};

#[cfg(feature = "std")]
pub use builder::ServientBuilder;
