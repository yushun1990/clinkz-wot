use alloc::sync::Arc;

use clinkz_wot_core::{AffordanceTarget, ClientBinding};
use clinkz_wot_protocol_bindings::FormSelectionCriteria;
use clinkz_wot_td::{data_type::Operation, form::Form};

/// Cache key for a Servient-selected TD form.
///
/// This key is scoped to a single interned [`ConsumedThingEntry`]: the Thing
/// identity is implied by the entry that owns the cache, so it is not part of
/// the key. Keeping the Thing id out of the key removes a `String` allocation
/// from every cache lookup (the consumed-interaction hot path).
///
/// `content_type` and `subprotocol` are stored as `Arc<str>` rather than
/// `String` so building a key from borrowed criteria is one refcount bump per
/// field instead of a heap allocation, and cloning the key (which happens on
/// every cache lookup) is also a refcount bump.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectedFormCacheKey {
    /// Affordance location used for form selection.
    pub affordance: AffordanceTarget,
    /// Required effective operation.
    pub operation: Operation,
    /// Optional required form content type.
    pub content_type: Option<Arc<str>>,
    /// Optional required form subprotocol.
    pub subprotocol: Option<Arc<str>>,
}

impl SelectedFormCacheKey {
    /// Creates a cache key from an affordance location and selection criteria.
    pub fn new(affordance: AffordanceTarget, criteria: FormSelectionCriteria<'_>) -> Self {
        Self {
            affordance,
            operation: criteria.operation,
            content_type: criteria.content_type.map(Arc::<str>::from),
            subprotocol: criteria.subprotocol.map(Arc::<str>::from),
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
