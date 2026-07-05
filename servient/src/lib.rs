//! Servient composition for Web of Things flows (baseline v4.0 §7 / phase-p3).
//!
//! Wires protocol-neutral core dispatch, Discovery, and protocol binding
//! factories into a single non-generic runtime: produce/consume/discover,
//! async-only driving, frozen-TD lifecycle.
//!
//! The Servient fundamentally requires the `async` feature (it holds a
//! `dyn Discoverer`, drives async handlers, and consumes via async
//! `ClientBinding`s). On `no_std` that means `no_std + async` (embassy); bare
//! `no_std` without `async` compiles only the data-registry primitives.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod error;
mod registry;

#[cfg(feature = "std")]
mod builder;
#[cfg(feature = "async")]
mod handle;
#[cfg(feature = "async")]
mod servient;

pub use error::{ServientError, ServientResult};

#[cfg(feature = "async")]
pub use handle::{ConsumedThingHandle, ExposedThingHandle};
#[cfg(feature = "async")]
pub use servient::ClientBindingFactory;
#[cfg(feature = "async")]
pub use servient::{Servient, ShutdownHandle};

#[cfg(feature = "std")]
pub use builder::ServientBuilder;
