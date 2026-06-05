#![no_std]
//!
//! Runtime composition for Web of Things Servient flows.
//!
//! This crate wires protocol-neutral core dispatch, Discovery directory
//! storage, and protocol binding factories without making any concrete
//! protocol binding mandatory.

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod builder;
mod cache;
pub mod embedded;
mod error;
#[cfg(feature = "std")]
pub mod host;
mod interaction;
mod registry;
mod runtime;

pub use builder::ServientBuilder;
pub use cache::{
    BindingPlan, BindingPlanCache, ConsumedThingCache, InMemoryBindingPlanCache,
    InMemoryConsumedThingCache, InMemorySelectedFormCache, SelectedFormCache,
    SelectedFormCacheAffordance, SelectedFormCacheKey,
};
pub use error::{ServientError, ServientResult};
pub use registry::{ExposedThingRegistry, InMemoryExposedThingRegistry};
pub use runtime::Servient;
