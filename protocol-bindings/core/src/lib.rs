#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod error;
pub mod form;

pub use error::{BindingCoreError, BindingCoreResult};
pub use form::{
    AffordanceRef, EffectiveFormSecurity, FormSelectionCriteria, ResolvedFormTarget,
    SelectedAffordanceForm, SelectedForm, resolve_form_security, resolve_form_target,
    resolve_selected_affordance_form_security, select_affordance_form,
    select_affordance_form_with_criteria, select_affordance_form_with_filter, select_form,
    select_form_with_criteria, select_form_with_filter, validate_affordance_form,
    validate_affordance_form_with_criteria,
};
