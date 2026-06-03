use alloc::{borrow::Cow, format, string::ToString};

use clinkz_wot_td::{
    data_type::{Operation, ResolvedFormHref, resolve_form_href},
    form::Form,
    td_defaults::{FormContext, effective_form_operations},
    thing::Thing,
};

use crate::{BindingCoreError, BindingCoreResult};

/// A TD form selected for an interaction operation.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectedForm<'a> {
    /// Index of the selected form in the candidate form slice.
    pub index: usize,
    /// Selected TD form.
    pub form: &'a Form,
    /// Effective operations for the selected form after TD defaults are applied.
    pub operations: Cow<'a, [Operation]>,
}

/// Resolved binding target for a selected form.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFormTarget {
    /// Form href resolved against the Thing-level base when possible.
    pub href: ResolvedFormHref,
}

/// Selects the first form whose effective operations include the requested operation.
pub fn select_form<'a>(
    context: FormContext<'a>,
    forms: &'a [Form],
    operation: Operation,
) -> BindingCoreResult<SelectedForm<'a>> {
    for (index, form) in forms.iter().enumerate() {
        let operations = effective_form_operations(context, form);
        if operations.iter().any(|candidate| *candidate == operation) {
            return Ok(SelectedForm {
                index,
                form,
                operations,
            });
        }
    }

    Err(BindingCoreError::UnsupportedOperation(format!(
        "No form supports {:?}",
        operation
    )))
}

/// Resolves a selected form target using the Thing-level `base` value.
pub fn resolve_form_target(thing: &Thing, form: &Form) -> BindingCoreResult<ResolvedFormTarget> {
    resolve_form_href(thing.base.as_ref(), &form.href)
        .map(|href| ResolvedFormTarget { href })
        .map_err(|err| BindingCoreError::TargetResolution(err.to_string()))
}
