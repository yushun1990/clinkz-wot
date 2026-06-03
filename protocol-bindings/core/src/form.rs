use alloc::{
    borrow::Cow,
    collections::BTreeMap,
    format,
    string::{String, ToString},
};

use clinkz_wot_td::{
    affordance::{ActionAffordance, EventAffordance, PropertyAffordance},
    data_type::{Operation, ResolvedFormHref, resolve_form_href},
    form::Form,
    td_defaults::{FormContext, effective_form_operations},
    thing::Thing,
};

use crate::{BindingCoreError, BindingCoreResult};

/// Location of an affordance within a Thing Description.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffordanceRef<'a> {
    /// A form declared at Thing level.
    Thing,
    /// A property affordance by name.
    Property(&'a str),
    /// An action affordance by name.
    Action(&'a str),
    /// An event affordance by name.
    Event(&'a str),
}

/// Protocol-neutral criteria used to choose a TD form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormSelectionCriteria<'a> {
    /// Required effective operation.
    pub operation: Operation,
    /// Optional required form content type.
    pub content_type: Option<&'a str>,
    /// Optional required form subprotocol.
    pub subprotocol: Option<&'a str>,
}

impl<'a> FormSelectionCriteria<'a> {
    /// Creates criteria for an operation without extra form metadata filters.
    pub fn operation(operation: Operation) -> Self {
        Self {
            operation,
            content_type: None,
            subprotocol: None,
        }
    }

    /// Requires a form content type.
    pub fn content_type(mut self, content_type: &'a str) -> Self {
        self.content_type = Some(content_type);
        self
    }

    /// Requires a form subprotocol.
    pub fn subprotocol(mut self, subprotocol: &'a str) -> Self {
        self.subprotocol = Some(subprotocol);
        self
    }

    fn matches_operation(&self, operations: &[Operation]) -> bool {
        operations
            .iter()
            .any(|candidate| *candidate == self.operation)
    }

    fn matches_metadata(&self, form: &Form) -> bool {
        let content_type_matches = match self.content_type {
            Some(content_type) => form.content_type == content_type,
            None => true,
        };
        let subprotocol_matches = match self.subprotocol {
            Some(subprotocol) => form.subprotocol.as_deref() == Some(subprotocol),
            None => true,
        };

        content_type_matches && subprotocol_matches
    }

    fn matches(&self, operations: &[Operation], form: &Form) -> bool {
        self.matches_operation(operations) && self.matches_metadata(form)
    }
}

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

/// A selected affordance form with its resolved target.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectedAffordanceForm<'a> {
    /// Location used to find the selected form.
    pub affordance: AffordanceRef<'a>,
    /// Selected TD form and effective operation metadata.
    pub selection: SelectedForm<'a>,
    /// Resolved binding target for the selected form.
    pub target: ResolvedFormTarget,
}

/// Selects the first form whose effective operations include the requested operation.
pub fn select_form<'a>(
    context: FormContext<'a>,
    forms: &'a [Form],
    operation: Operation,
) -> BindingCoreResult<SelectedForm<'a>> {
    select_form_with_criteria(context, forms, FormSelectionCriteria::operation(operation))
}

/// Selects the first form matching the requested operation and metadata criteria.
pub fn select_form_with_criteria<'a>(
    context: FormContext<'a>,
    forms: &'a [Form],
    criteria: FormSelectionCriteria<'_>,
) -> BindingCoreResult<SelectedForm<'a>> {
    select_form_with_filter(context, forms, criteria, |_| true)
}

/// Selects the first form matching the requested criteria and caller filter.
pub fn select_form_with_filter<'a, F>(
    context: FormContext<'a>,
    forms: &'a [Form],
    criteria: FormSelectionCriteria<'_>,
    filter: F,
) -> BindingCoreResult<SelectedForm<'a>>
where
    F: Fn(&Form) -> bool,
{
    let mut operation_supported = false;

    for (index, form) in forms.iter().enumerate() {
        let operations = effective_form_operations(context, form);
        operation_supported |= criteria.matches_operation(operations.as_ref());
        if criteria.matches(operations.as_ref(), form) && filter(form) {
            return Ok(SelectedForm {
                index,
                form,
                operations,
            });
        }
    }

    Err(BindingCoreError::UnsupportedOperation(
        unsupported_operation_message(criteria, operation_supported),
    ))
}

fn unsupported_operation_message(
    criteria: FormSelectionCriteria<'_>,
    operation_supported: bool,
) -> String {
    if !operation_supported {
        format!("No form supports {:?}", criteria.operation)
    } else if criteria.content_type.is_none() && criteria.subprotocol.is_none() {
        format!(
            "No form supports {:?} after applying caller filter",
            criteria.operation
        )
    } else {
        format!("No form matches {:?}", criteria)
    }
}

/// Resolves a selected form target using the Thing-level `base` value.
pub fn resolve_form_target(thing: &Thing, form: &Form) -> BindingCoreResult<ResolvedFormTarget> {
    resolve_form_href(thing.base.as_ref(), &form.href)
        .map(|href| ResolvedFormTarget { href })
        .map_err(|err| BindingCoreError::TargetResolution(err.to_string()))
}

/// Selects and resolves a form from a Thing affordance for the requested operation.
pub fn select_affordance_form<'a>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    operation: Operation,
) -> BindingCoreResult<SelectedAffordanceForm<'a>> {
    select_affordance_form_with_criteria(
        thing,
        affordance,
        FormSelectionCriteria::operation(operation),
    )
}

/// Selects and resolves a form from a Thing affordance using metadata criteria.
pub fn select_affordance_form_with_criteria<'a>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    criteria: FormSelectionCriteria<'_>,
) -> BindingCoreResult<SelectedAffordanceForm<'a>> {
    select_affordance_form_with_filter(thing, affordance, criteria, |_| true)
}

/// Selects and resolves a form from a Thing affordance using criteria and a caller filter.
pub fn select_affordance_form_with_filter<'a, F>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    criteria: FormSelectionCriteria<'_>,
    filter: F,
) -> BindingCoreResult<SelectedAffordanceForm<'a>>
where
    F: Fn(&Form) -> bool,
{
    let form_set = forms_for_affordance(thing, affordance)?;
    let selection = select_form_with_filter(form_set.context, form_set.forms, criteria, filter)?;
    let target = resolve_form_target(thing, selection.form)?;

    Ok(SelectedAffordanceForm {
        affordance,
        selection,
        target,
    })
}

/// Validates that a selected form belongs to an affordance and matches the requested operation.
pub fn validate_affordance_form<'a>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    form: &Form,
    operation: Operation,
) -> BindingCoreResult<SelectedForm<'a>> {
    validate_affordance_form_with_criteria(
        thing,
        affordance,
        form,
        FormSelectionCriteria::operation(operation),
    )
}

/// Validates that a selected form belongs to an affordance and matches the requested criteria.
pub fn validate_affordance_form_with_criteria<'a>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    form: &Form,
    criteria: FormSelectionCriteria<'_>,
) -> BindingCoreResult<SelectedForm<'a>> {
    let form_set = forms_for_affordance(thing, affordance)?;

    for (index, candidate) in form_set.forms.iter().enumerate() {
        if candidate != form {
            continue;
        }

        let operations = effective_form_operations(form_set.context, candidate);
        if criteria.matches(operations.as_ref(), candidate) {
            return Ok(SelectedForm {
                index,
                form: candidate,
                operations,
            });
        }

        return Err(BindingCoreError::UnsupportedOperation(format!(
            "Selected form does not support {:?}",
            criteria.operation
        )));
    }

    Err(BindingCoreError::FormNotInAffordance)
}

struct FormSet<'a> {
    context: FormContext<'a>,
    forms: &'a [Form],
}

fn forms_for_affordance<'a>(
    thing: &'a Thing,
    affordance: AffordanceRef<'_>,
) -> BindingCoreResult<FormSet<'a>> {
    match affordance {
        AffordanceRef::Thing => Ok(FormSet {
            context: FormContext::Thing,
            forms: thing.forms.as_deref().unwrap_or(&[]),
        }),
        AffordanceRef::Property(name) => {
            let property = find_affordance("property", name, &thing.properties)?;
            Ok(FormSet {
                context: FormContext::Property(property),
                forms: property._interaction.forms.as_slice(),
            })
        }
        AffordanceRef::Action(name) => {
            let action = find_affordance("action", name, &thing.actions)?;
            Ok(FormSet {
                context: FormContext::Action(action),
                forms: action._interaction.forms.as_slice(),
            })
        }
        AffordanceRef::Event(name) => {
            let event = find_affordance("event", name, &thing.events)?;
            Ok(FormSet {
                context: FormContext::Event(event),
                forms: event._interaction.forms.as_slice(),
            })
        }
    }
}

trait AffordanceMapValue {}

impl AffordanceMapValue for PropertyAffordance {}
impl AffordanceMapValue for ActionAffordance {}
impl AffordanceMapValue for EventAffordance {}

fn find_affordance<'a, T: AffordanceMapValue>(
    kind: &'static str,
    name: &str,
    affordances: &'a Option<BTreeMap<String, T>>,
) -> BindingCoreResult<&'a T> {
    affordances
        .as_ref()
        .and_then(|affordances| affordances.get(name))
        .ok_or_else(|| BindingCoreError::UnknownAffordance {
            kind,
            name: name.into(),
        })
}
