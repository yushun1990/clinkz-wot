//! Servient composition for Web of Things flows.
//!
//! This crate wires protocol-neutral core dispatch, Discovery directory
//! storage, and protocol binding factories without making any concrete
//! protocol binding mandatory.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod builder;
mod cache;
mod consumed;
mod error;
mod handle;
mod interaction;
mod lock;
mod registry;
mod servient;

pub use builder::ServientBuilder;
pub use cache::{BindingPlan, SelectedFormCacheKey};
pub use error::{ServientError, ServientResult};
pub use handle::{ConsumedThingHandle, ExposedThingHandle};
pub use servient::{Servient, ShutdownHandle};

pub(crate) use consumed::ConsumedThingRegistry;
pub(crate) use registry::ExposedThingRegistry;
