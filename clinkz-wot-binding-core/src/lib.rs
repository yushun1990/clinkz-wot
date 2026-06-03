#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod error;
pub mod form;

pub use error::{BindingCoreError, BindingCoreResult};
pub use form::{ResolvedFormTarget, SelectedForm, resolve_form_target, select_form};
