//! TD-aware entry to the managed finite Consumer owner. Execution and lookup
//! remain source-free in `clinkz-wot-servient-finite`; no second registry lives here.
use clinkz_wot_core::{CoreResult, consumer::invalid};
use clinkz_wot_foundation::WorkBudget;
use clinkz_wot_servient_finite::consumer::{ConsumerInstanceHandle, ConsumerOwner};
use clinkz_wot_td::thing::Thing;

/// Register the TD's public ID in this owner's scope. Anonymous TDs remain valid
/// documents, but this ID-addressed entry rejects a missing ID without inventing
/// one. This installs logical identity only; prepare and install named finite
/// configurations separately before operations. No TD or parser is retained.
pub fn register_thing(
    owner: &ConsumerOwner,
    thing: &Thing,
    budget: &mut WorkBudget,
) -> CoreResult<ConsumerInstanceHandle> {
    owner.register_thing(thing.id.as_ref().ok_or_else(invalid)?.as_str(), budget)
}
