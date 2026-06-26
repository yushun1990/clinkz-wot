use alloc::{borrow::Cow, collections::BTreeMap, format, string::String};

use clinkz_wot_td::{
    data_type::{Operation, ResolvedFormHref, resolve_form_href},
    form::Form,
    td_defaults::{FormContext, effective_form_operations, effective_form_security},
    thing::Thing,
};

use crate::{BindingError, BindingResult};

const NO_SCOPES: &[String] = &[];

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
    pub fn new(operation: Operation) -> Self {
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
        operations.contains(&self.operation)
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
pub struct SelectedAffordanceForm<'a, Target = ResolvedFormTarget> {
    /// Location used to find the selected form.
    pub affordance: AffordanceRef<'a>,
    /// Selected TD form and effective operation metadata.
    pub selection: SelectedForm<'a>,
    /// Resolved binding target for the selected form, or unit while unresolved.
    pub target: Target,
}

/// A selected affordance form without its resolved target.
pub type SelectedAffordanceSelection<'a> = SelectedAffordanceForm<'a, ()>;

/// Effective protocol-neutral security metadata for a selected TD form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveFormSecurity<'a> {
    /// Security definition names after TD form-level inheritance is resolved.
    pub security: &'a [String],
    /// Scope names declared on the form.
    pub scopes: &'a [String],
}

/// Selects the first form whose effective operations include the requested operation.
pub fn select_form<'a>(
    context: FormContext<'a>,
    forms: &'a [Form],
    operation: Operation,
) -> BindingResult<SelectedForm<'a>> {
    select_form_with_criteria(context, forms, FormSelectionCriteria::<'a>::new(operation))
}

/// Selects the first form matching the requested operation and metadata criteria.
pub fn select_form_with_criteria<'a>(
    context: FormContext<'a>,
    forms: &'a [Form],
    criteria: FormSelectionCriteria<'_>,
) -> BindingResult<SelectedForm<'a>> {
    select_form_with_filter(context, forms, criteria, |_| true)
}

/// Selects the first form matching the requested criteria and caller filter.
pub fn select_form_with_filter<'a, F>(
    context: FormContext<'a>,
    forms: &'a [Form],
    criteria: FormSelectionCriteria<'_>,
    mut filter: F,
) -> BindingResult<SelectedForm<'a>>
where
    F: FnMut(&Form) -> bool,
{
    select_form_with_result_filter(context, forms, criteria, |form| Ok(filter(form)))
}

/// Selects the first form matching the requested criteria and a fallible caller filter.
pub fn select_form_with_result_filter<'a, F, E>(
    context: FormContext<'a>,
    forms: &'a [Form],
    criteria: FormSelectionCriteria<'_>,
    mut filter: F,
) -> Result<SelectedForm<'a>, E>
where
    F: FnMut(&Form) -> Result<bool, E>,
    E: From<BindingError>,
{
    let mut operation_supported = false;
    let mut metadata_supported = false;

    for (index, form) in forms.iter().enumerate() {
        let operations = effective_form_operations(context, form);
        let matches_operation = criteria.matches_operation(operations.as_ref());
        let matches_metadata = criteria.matches_metadata(form);
        operation_supported |= matches_operation;
        metadata_supported |= matches_operation && matches_metadata;
        if matches_operation && matches_metadata && filter(form)? {
            return Ok(SelectedForm {
                index,
                form,
                operations,
            });
        }
    }

    if !operation_supported {
        return Err(E::from(BindingError::UnsupportedOperation(format!(
            "No form supports {:?}",
            criteria.operation
        ))));
    }

    if !metadata_supported {
        return Err(E::from(BindingError::MetadataMismatch(format!(
            "No form matches {:?}",
            criteria
        ))));
    }

    Err(E::from(BindingError::CallerFilterMismatch(format!(
        "No form matches {:?} after applying caller filter",
        criteria
    ))))
}

/// Resolves a selected form target using the Thing-level `base` value.
pub fn resolve_form_target(thing: &Thing, form: &Form) -> BindingResult<ResolvedFormTarget> {
    resolve_form_href(thing.base.as_ref(), &form.href)
        .map(|href| ResolvedFormTarget { href })
        .map_err(BindingError::TargetResolution)
}

/// Resolves protocol-neutral security metadata for a form.
///
/// Form-level `security` overrides Thing-level `security` according to TD
/// inheritance rules. `scopes` are returned from the selected form without
/// interpreting concrete authentication mechanisms.
pub fn resolve_form_security<'a>(thing: &'a Thing, form: &'a Form) -> EffectiveFormSecurity<'a> {
    EffectiveFormSecurity {
        security: effective_form_security(thing, form),
        scopes: form.scopes.as_deref().unwrap_or(NO_SCOPES),
    }
}

/// Resolves protocol-neutral security metadata for a selected affordance form.
pub fn resolve_selected_affordance_form_security<'a, TTarget>(
    thing: &'a Thing,
    selected: &SelectedAffordanceForm<'a, TTarget>,
) -> EffectiveFormSecurity<'a> {
    resolve_form_security(thing, selected.selection.form)
}

/// Selects and resolves a form from a Thing affordance for the requested operation.
pub fn select_affordance_form<'a>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    operation: Operation,
) -> BindingResult<SelectedAffordanceForm<'a>> {
    select_affordance_form_with_criteria(thing, affordance, FormSelectionCriteria::new(operation))
}

/// Selects and resolves a form from a Thing affordance using metadata criteria.
pub fn select_affordance_form_with_criteria<'a>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    criteria: FormSelectionCriteria<'_>,
) -> BindingResult<SelectedAffordanceForm<'a>> {
    select_affordance_form_with_filter(thing, affordance, criteria, |_| true)
}

/// Selects and resolves a form from a Thing affordance using criteria and a caller filter.
pub fn select_affordance_form_with_filter<'a, F>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    criteria: FormSelectionCriteria<'_>,
    filter: F,
) -> BindingResult<SelectedAffordanceForm<'a>>
where
    F: FnMut(&Form) -> bool,
{
    let selected =
        select_affordance_form_selection_with_filter(thing, affordance, criteria, filter)?;
    let target = resolve_form_target(thing, selected.selection.form)?;

    Ok(SelectedAffordanceForm {
        affordance: selected.affordance,
        selection: selected.selection,
        target,
    })
}

/// Selects a form from a Thing affordance using a fallible caller filter.
///
/// This helper returns the selected form without resolving its target. It is
/// useful for bindings that need to inspect the form before deciding whether
/// target resolution is required.
pub fn select_affordance_form_selection_with_filter<'a, F>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    criteria: FormSelectionCriteria<'_>,
    mut filter: F,
) -> BindingResult<SelectedAffordanceSelection<'a>>
where
    F: FnMut(&Form) -> bool,
{
    select_affordance_form_selection_with_result_filter(thing, affordance, criteria, |form| {
        Ok(filter(form))
    })
}

/// Selects a form from a Thing affordance using a fallible caller filter.
///
/// This helper returns the selected form without resolving its target. It is
/// useful for bindings that need to inspect the form before deciding whether
/// target resolution is required.
pub fn select_affordance_form_selection_with_result_filter<'a, F, E>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    criteria: FormSelectionCriteria<'_>,
    filter: F,
) -> Result<SelectedAffordanceSelection<'a>, E>
where
    F: FnMut(&Form) -> Result<bool, E>,
    E: From<BindingError>,
{
    let form_set = forms_for_affordance(thing, affordance)?;
    let selection =
        select_form_with_result_filter(form_set.context, form_set.forms, criteria, filter)?;

    Ok(SelectedAffordanceForm {
        affordance,
        selection,
        target: (),
    })
}

/// Selects and resolves a form from a Thing affordance using a fallible caller filter.
pub fn select_affordance_form_with_result_filter<'a, F, E>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    criteria: FormSelectionCriteria<'_>,
    filter: F,
) -> Result<SelectedAffordanceForm<'a>, E>
where
    F: FnMut(&Form) -> Result<bool, E>,
    E: From<BindingError>,
{
    let selection =
        select_affordance_form_selection_with_result_filter(thing, affordance, criteria, filter)?;
    let target = resolve_form_target(thing, selection.selection.form).map_err(E::from)?;

    Ok(SelectedAffordanceForm {
        affordance: selection.affordance,
        selection: selection.selection,
        target,
    })
}

/// Validates that a selected form belongs to an affordance and matches the requested operation.
pub fn validate_affordance_form<'a>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    form: &Form,
    operation: Operation,
) -> BindingResult<SelectedForm<'a>> {
    validate_affordance_form_with_criteria(
        thing,
        affordance,
        form,
        FormSelectionCriteria::new(operation),
    )
}

/// Lightweight operation check that resolves the affordance context and
/// validates that `form` supports `operation`, without performing an O(n)
/// membership search.
///
/// Use this in binding hot paths where the form was already selected from the
/// affordance by form-selection code. Use [`validate_affordance_form`] when you
/// need to verify that an externally-supplied form belongs to the affordance.
pub fn validate_form_operation(
    thing: &Thing,
    affordance: AffordanceRef<'_>,
    form: &Form,
    operation: Operation,
) -> BindingResult<()> {
    let form_set = forms_for_affordance(thing, affordance)?;
    let operations = effective_form_operations(form_set.context, form);
    if operations.contains(&operation) {
        Ok(())
    } else {
        Err(BindingError::UnsupportedOperation(format!(
            "Selected form does not support {:?}",
            operation
        )))
    }
}

/// Validates that a selected form belongs to an affordance and matches the requested criteria.
pub fn validate_affordance_form_with_criteria<'a>(
    thing: &'a Thing,
    affordance: AffordanceRef<'a>,
    form: &Form,
    criteria: FormSelectionCriteria<'_>,
) -> BindingResult<SelectedForm<'a>> {
    let form_set = forms_for_affordance(thing, affordance)?;

    for (index, candidate) in form_set.forms.iter().enumerate() {
        if candidate != form {
            continue;
        }

        let operations = effective_form_operations(form_set.context, candidate);
        if !criteria.matches_operation(operations.as_ref()) {
            return Err(BindingError::UnsupportedOperation(format!(
                "Selected form does not support {:?}",
                criteria.operation
            )));
        }

        if !criteria.matches_metadata(candidate) {
            return Err(BindingError::MetadataMismatch(format!(
                "Selected form does not match {:?}",
                criteria
            )));
        }

        return Ok(SelectedForm {
            index,
            form: candidate,
            operations,
        });
    }

    Err(BindingError::FormNotInAffordance)
}

struct FormSet<'a> {
    context: FormContext<'a>,
    forms: &'a [Form],
}

fn forms_for_affordance<'a>(
    thing: &'a Thing,
    affordance: AffordanceRef<'_>,
) -> BindingResult<FormSet<'a>> {
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

fn find_affordance<'a, T>(
    kind: &'static str,
    name: &str,
    affordances: &'a Option<BTreeMap<String, T>>,
) -> BindingResult<&'a T> {
    affordances
        .as_ref()
        .and_then(|affordances| affordances.get(name))
        .ok_or_else(|| BindingError::UnknownAffordance {
            kind,
            name: name.into(),
        })
}
