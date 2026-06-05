//! Host runtime composition for Web of Things Servient flows.
//!
//! This crate wires protocol-neutral core dispatch, Discovery directory
//! storage, and protocol binding factories without making any concrete
//! protocol binding mandatory.

mod builder;
mod cache;
mod error;
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
