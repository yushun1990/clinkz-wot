#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod error;
pub mod form;

pub use error::{BindingCoreError, BindingCoreResult};
pub use form::{
    AffordanceRef, ResolvedFormTarget, SelectedAffordanceForm, SelectedForm, resolve_form_target,
    select_affordance_form, select_form,
};
