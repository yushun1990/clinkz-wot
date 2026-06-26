use alloc::sync::Arc;
use alloc::{borrow::ToOwned, string::String};

use clinkz_wot_core::{AffordanceTarget, ClientBinding};
use clinkz_wot_protocol_bindings::{AffordanceRef, FormSelectionCriteria};
use clinkz_wot_td::{data_type::Operation, form::Form};

/// Converts a borrowed [`AffordanceRef`] into an owned [`AffordanceTarget`].
pub(crate) fn affordance_target_from_ref(affordance: AffordanceRef<'_>) -> AffordanceTarget {
    match affordance {
        AffordanceRef::Thing => AffordanceTarget::Thing,
        AffordanceRef::Property(name) => AffordanceTarget::Property(name.to_owned()),
        AffordanceRef::Action(name) => AffordanceTarget::Action(name.to_owned()),
        AffordanceRef::Event(name) => AffordanceTarget::Event(name.to_owned()),
    }
}

/// Cache key for a Servient-selected TD form.
///
/// This key is scoped to a single interned [`ConsumedThingEntry`]: the Thing
/// identity is implied by the entry that owns the cache, so it is not part of
/// the key. Keeping the Thing id out of the key removes a `String` allocation
/// from every cache lookup (the consumed-interaction hot path).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectedFormCacheKey {
    /// Affordance location used for form selection.
    pub affordance: AffordanceTarget,
    /// Required effective operation.
    pub operation: Operation,
    /// Optional required form content type.
    pub content_type: Option<String>,
    /// Optional required form subprotocol.
    pub subprotocol: Option<String>,
}

impl SelectedFormCacheKey {
    /// Creates a cache key from an affordance location and selection criteria.
    pub fn new(affordance: AffordanceTarget, criteria: FormSelectionCriteria<'_>) -> Self {
        Self {
            affordance,
            operation: criteria.operation,
            content_type: criteria.content_type.map(str::to_owned),
            subprotocol: criteria.subprotocol.map(str::to_owned),
        }
    }
}

/// Protocol-neutral cached binding plan for a criteria-selected remote request.
///
/// Holds the **live binding instance** (`Arc<dyn ClientBinding>`) alongside the
/// selected form, so a cache hit reuses the same binding (cheap `Arc` clone)
/// instead of reconstructing it via the factory on every consumed interaction.
/// `ClientBinding` is designed for shared-`&self` invocation (it owns its
/// interior mutability), so one live instance per `(entry, affordance,
/// operation)` is reused across requests — matching the baseline "live instance
/// reuse" goal and avoiding per-call session-handle/buffer construction.
#[derive(Clone)]
pub struct BindingPlan {
    /// Selected TD form for the remote interaction.
    pub form: Arc<Form>,
    /// Index of the protocol binding factory selected for this form.
    pub binding_factory_index: usize,
    /// The live binding instance, reused on cache hit.
    pub binding: Arc<dyn ClientBinding + Send + Sync>,
    /// Binding-factory registry generation observed when this plan was last
    /// validated. When the registry's current generation matches, the plan is
    /// still valid and the caller can skip revalidation.
    pub factory_generation: u64,
}
