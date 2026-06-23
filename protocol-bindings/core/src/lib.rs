#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod error;
pub mod error_status;
pub mod form;

pub use error::{BindingError, BindingResult};
pub use error_status::error_status;
pub use form::{
    AffordanceRef, EffectiveFormSecurity, FormSelectionCriteria, ResolvedFormTarget,
    SelectedAffordanceForm, SelectedAffordanceSelection, SelectedForm, resolve_form_security,
    resolve_form_target, resolve_selected_affordance_form_security, select_affordance_form,
    select_affordance_form_selection_with_filter,
    select_affordance_form_selection_with_result_filter, select_affordance_form_with_criteria,
    select_affordance_form_with_filter, select_affordance_form_with_result_filter, select_form,
    select_form_with_criteria, select_form_with_filter, select_form_with_result_filter,
    validate_affordance_form, validate_affordance_form_with_criteria,
};
