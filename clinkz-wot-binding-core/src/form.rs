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

/// Selects and resolves a form from a Thing affordance for the requested operation.
pub fn select_affordance_form<'a>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    operation: Operation,
) -> BindingCoreResult<SelectedAffordanceForm<'a>> {
    let form_set = forms_for_affordance(thing, affordance)?;
    let selection = select_form(form_set.context, form_set.forms, operation)?;
    let target = resolve_form_target(thing, selection.form)?;

    Ok(SelectedAffordanceForm {
        affordance,
        selection,
        target,
    })
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
