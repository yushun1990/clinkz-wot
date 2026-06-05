//! Embedded-ready Servient APIs.
//!
//! These items are usable with `no_std + alloc`. They compose local Thing
//! dispatch, consumed Thing dispatch, protocol-neutral bindings, in-memory
//! registries, and allocation-backed caches without owning concrete network or
//! operating-system resources.

pub use crate::builder::ServientBuilder;
pub use crate::cache::{
    BindingPlan, BindingPlanCache, ConsumedThingCache, InMemoryBindingPlanCache,
    InMemoryConsumedThingCache, InMemorySelectedFormCache, SelectedFormCache,
    SelectedFormCacheAffordance, SelectedFormCacheKey,
};
pub use crate::error::{ServientError, ServientResult};
pub use crate::registry::{ExposedThingRegistry, InMemoryExposedThingRegistry};
pub use crate::runtime::Servient;
