use alloc::{borrow::ToOwned, string::String};

use clinkz_wot_core::AffordanceTarget;
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectedFormCacheKey {
    /// Thing id used for the consumed interaction.
    pub thing_id: String,
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
    /// Creates a cache key from a Thing id, affordance location, and selection criteria.
    pub fn new(
        thing_id: impl Into<String>,
        affordance: AffordanceTarget,
        criteria: FormSelectionCriteria<'_>,
    ) -> Self {
        Self {
            thing_id: thing_id.into(),
            affordance,
            operation: criteria.operation,
            content_type: criteria.content_type.map(str::to_owned),
            subprotocol: criteria.subprotocol.map(str::to_owned),
        }
    }
}

/// Protocol-neutral cached binding plan for a criteria-selected remote request.
#[derive(Debug, Clone, PartialEq)]
pub struct BindingPlan {
    /// Selected TD form for the remote interaction.
    pub form: Form,
    /// Index of the protocol binding factory selected for this form.
    pub binding_factory_index: usize,
    /// Binding-factory registry generation observed when this plan was last
    /// validated. When the registry's current generation matches, the plan is
    /// still valid and the caller can skip revalidation.
    pub factory_generation: u64,
}
