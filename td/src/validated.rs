//! Bounded, move-only validation and retained-source census for owned Things.

use alloc::{
    boxed::Box,
    collections::{BTreeMap, btree_map},
    string::String,
    vec::Vec,
};
use core::{mem::size_of, slice};

use clinkz_wot_foundation::{ResourceKind, ResourceLimits, WorkBudget, WorkClass};
use serde_json::Value;

use crate::{
    affordance::{ActionAffordance, EventAffordance, PropertyAffordance},
    context::{Context, ContextEntry},
    data_schema::{DataSchema, DataSchemaContext},
    data_type::{
        AbsoluteUri, AdditionalExpectedResponse, BaseUri, ExtensionMap, FormHref, Metadata,
        MultiLanguage, Operation, UriReference, VersionInfo,
    },
    form::Form,
    link::Link,
    security_scheme::{SecurityScheme, SecuritySchemeContext},
    td_defaults::{FormContext, effective_form_operations},
    thing::Thing,
    validate::{Validate, ValidateError, ValidationLevel},
};

/// A unique continuation for bounded validation of one exact owned [`Thing`].
///
/// The cursor intentionally has no `Clone`, `Copy`, or raw-Thing projection:
///
/// ```compile_fail
/// use clinkz_wot_td::ValidatedThingCursor;
/// fn require_clone<T: Clone>() {}
/// require_clone::<ValidatedThingCursor>();
/// ```
///
/// ```compile_fail
/// use clinkz_wot_td::ValidatedThingCursor;
/// fn require_copy<T: Copy>() {}
/// require_copy::<ValidatedThingCursor>();
/// ```
///
/// ```compile_fail
/// # use clinkz_wot_td::{ValidatedThingCursor, thing::Thing};
/// # use clinkz_wot_foundation::{GatewayDefaultV1, StaticResourceProfile};
/// # let thing = Thing::builder("thing").nosec().build().unwrap();
/// let cursor = ValidatedThingCursor::new(thing, GatewayDefaultV1::limits());
/// let _: &mut Thing = cursor.thing_mut();
/// ```
pub struct ValidatedThingCursor {
    // These self-referential tasks must be dropped before `thing`.
    tasks: Vec<Task>,
    thing: Box<Thing>,
    limits: ValidationLimits,
    phase: Phase,
    document_work_remaining: u64,
    document_work_used: u64,
    census: Census,
}

/// The sole result surface for one [`ValidatedThingCursor::step`] call.
pub enum ValidatedThingStep {
    /// The supplied per-step budget ended before the next unit could start.
    Pending(ValidatedThingCursor),
    /// Basic validation and the complete retained-source census succeeded.
    Complete(ValidatedThing),
    /// The existing Basic semantic authority rejected the Thing.
    Invalid { thing: Thing, error: ValidateError },
    /// A structural, retained-resource, or lifetime-work ceiling was exceeded.
    Limit {
        thing: Thing,
        kind: ResourceKind,
        configured: Option<u64>,
        observed: Option<u64>,
    },
    /// Cancellation won before the next unit of work.
    Cancelled(Thing),
}

/// One exact owned Thing proven Basic-valid and completely censused.
///
/// This value is deliberately move-only and exposes no mutable Thing access:
///
/// ```compile_fail
/// use clinkz_wot_td::ValidatedThing;
/// fn require_clone<T: Clone>() {}
/// require_clone::<ValidatedThing>();
/// ```
///
/// ```compile_fail
/// use clinkz_wot_td::ValidatedThing;
/// fn require_copy<T: Copy>() {}
/// require_copy::<ValidatedThing>();
/// ```
///
/// ```compile_fail
/// # use clinkz_wot_td::{ValidatedThing, thing::Thing};
/// fn mutate(validated: &mut ValidatedThing) -> &mut Thing {
///     validated.thing_mut()
/// }
/// ```
pub struct ValidatedThing {
    thing: Thing,
    retained_source_bytes: u64,
    property_count: u64,
    readable_property_form_count: u64,
}

impl ValidatedThing {
    /// Borrows the exact Thing consumed by the validation cursor.
    pub fn thing(&self) -> &Thing {
        &self.thing
    }

    /// Returns the conservative representation-aware retained footprint.
    pub const fn retained_source_bytes(&self) -> u64 {
        self.retained_source_bytes
    }

    /// Returns every declared Property affordance, readable or otherwise.
    pub const fn property_count(&self) -> u64 {
        self.property_count
    }

    /// Returns Property Forms whose effective operations include ReadProperty.
    pub const fn readable_property_form_count(&self) -> u64 {
        self.readable_property_form_count
    }
}

impl ValidatedThingCursor {
    /// Starts a cursor without traversing or semantically validating `thing`.
    pub fn new(thing: Thing, limits: &ResourceLimits) -> Self {
        let limits = ValidationLimits::capture(limits);
        Self {
            tasks: Vec::new(),
            // Moving the exact value into a fixed-size allocation makes every
            // borrowed task address stable while the public cursor itself moves.
            thing: Box::new(thing),
            phase: Phase::CensusNotStarted,
            document_work_remaining: limits.document_validation_work_units_max.unwrap_or(0),
            document_work_used: 0,
            census: Census::new(),
            limits,
        }
    }

    /// Advances census/validation until terminal or the next charge is unavailable.
    pub fn step(mut self, budget: &mut WorkBudget, cancel_requested: bool) -> ValidatedThingStep {
        loop {
            if cancel_requested {
                return ValidatedThingStep::Cancelled(self.into_thing());
            }

            match self.phase {
                Phase::CensusNotStarted => {
                    if let Some(kind) = self.limits.first_missing() {
                        return self.limit(kind, None);
                    }
                    let thing = static_ref(self.thing.as_ref());
                    self.tasks.push(Task::Thing(thing));
                    self.phase = Phase::Census;
                }
                Phase::Census => {
                    let Some(task) = self.tasks.pop() else {
                        self.phase = Phase::Basic;
                        continue;
                    };

                    let Some(charge) = task.charge() else {
                        continue;
                    };
                    if budget.remaining(charge.class) < charge.units {
                        self.tasks.push(task);
                        return ValidatedThingStep::Pending(self);
                    }
                    if charge.class == WorkClass::DocumentNodes
                        && self.document_work_remaining < charge.units
                    {
                        let observed = self.document_work_used.checked_add(charge.units);
                        return self.limit(ResourceKind::DocumentValidationWorkUnitsMax, observed);
                    }

                    // The checks above make these two charges infallible and keep
                    // the caller budget unchanged when the unit is rejected.
                    let consumed = budget.consume(charge.class, charge.units);
                    debug_assert!(consumed.is_ok());
                    if charge.class == WorkClass::DocumentNodes {
                        self.document_work_remaining -= charge.units;
                        self.document_work_used += charge.units;
                    }
                    if charge.semantic
                        && let Err(failure) = self.census.semantic.add(charge.class, charge.units)
                    {
                        return self.limit_failure(failure);
                    }

                    if let Err(failure) = self.process(task) {
                        return self.limit_failure(failure);
                    }
                }
                Phase::Basic => {
                    // `cancel_requested` is checked at the top of this loop,
                    // immediately before this bounded bulk phase.
                    let charges = self.census.semantic;
                    if !charges.fit(budget) {
                        return ValidatedThingStep::Pending(self);
                    }
                    if self.document_work_remaining < charges.document_nodes {
                        let observed = self.document_work_used.checked_add(charges.document_nodes);
                        return self.limit(ResourceKind::DocumentValidationWorkUnitsMax, observed);
                    }

                    charges.consume(budget);
                    self.document_work_remaining -= charges.document_nodes;
                    self.document_work_used += charges.document_nodes;

                    let validation = self.thing.validate_with_level(ValidationLevel::Basic);
                    let retained_source_bytes = self.census.retained_bytes;
                    let property_count = self.census.property_count;
                    let readable_property_form_count = self.census.readable_property_form_count;
                    let thing = self.into_thing();
                    return match validation {
                        Ok(()) => ValidatedThingStep::Complete(ValidatedThing {
                            thing,
                            retained_source_bytes,
                            property_count,
                            readable_property_form_count,
                        }),
                        Err(error) => ValidatedThingStep::Invalid { thing, error },
                    };
                }
            }
        }
    }

    fn process(&mut self, task: Task) -> Result<(), LimitFailure> {
        match task {
            Task::Thing(thing) => self.process_thing(thing),
            Task::Context(context) => self.process_context(context),
            Task::ContextEntries(mut entries) => {
                let entry = entries.next().expect("nonempty task");
                if !entries.as_slice().is_empty() {
                    self.tasks.push(Task::ContextEntries(entries));
                }
                match entry {
                    ContextEntry::Uri(uri) => self.census.add_uri(uri, false, &self.limits),
                    ContextEntry::Object(object) => {
                        self.census.observe_json_node(&self.limits)?;
                        self.census.observe_json_depth(1, &self.limits)?;
                        self.census.account_btree(object, false, &self.limits)?;
                        self.tasks
                            .push(Task::ContextObject(static_btree_iter(object), 1));
                        Ok(())
                    }
                }
            }
            Task::ContextObject(mut entries, depth) => {
                let (key, value) = entries.next().expect("nonempty task");
                if entries.len() != 0 {
                    self.tasks.push(Task::ContextObject(entries, depth));
                }
                self.census.add_string(key, false, &self.limits)?;
                self.tasks.push(Task::Json(value, depth, false));
                Ok(())
            }
            Task::Metadata(metadata) => self.process_metadata(metadata),
            Task::MultiLanguage(values) => {
                self.census
                    .account_btree(values.as_map(), false, &self.limits)?;
                self.tasks
                    .push(Task::MultiLanguageEntries(static_btree_iter(
                        values.as_map(),
                    )));
                Ok(())
            }
            Task::MultiLanguageEntries(mut entries) => {
                let (key, value) = entries.next().expect("nonempty task");
                if entries.len() != 0 {
                    self.tasks.push(Task::MultiLanguageEntries(entries));
                }
                self.census.add_string(key, false, &self.limits)?;
                self.census.add_string(value, false, &self.limits)
            }
            Task::Version(version) => {
                self.census
                    .add_string(&version.instance, false, &self.limits)?;
                if let Some(model) = &version.model {
                    self.census.add_string(model, false, &self.limits)?;
                }
                self.push_extension_map(&version._extra_fields, false)
            }
            Task::Properties(mut entries) => {
                let (key, property) = entries.next().expect("nonempty task");
                if entries.len() != 0 {
                    self.tasks.push(Task::Properties(entries));
                }
                self.census.add_string(key, false, &self.limits)?;
                self.tasks.push(Task::Property(property));
                Ok(())
            }
            Task::Actions(mut entries) => {
                let (key, action) = entries.next().expect("nonempty task");
                if entries.len() != 0 {
                    self.tasks.push(Task::Actions(entries));
                }
                self.census.add_string(key, false, &self.limits)?;
                self.tasks.push(Task::Action(action));
                Ok(())
            }
            Task::Events(mut entries) => {
                let (key, event) = entries.next().expect("nonempty task");
                if entries.len() != 0 {
                    self.tasks.push(Task::Events(entries));
                }
                self.census.add_string(key, false, &self.limits)?;
                self.tasks.push(Task::Event(event));
                Ok(())
            }
            Task::Property(property) => self.process_property(property),
            Task::Action(action) => self.process_action(action),
            Task::Event(event) => self.process_event(event),
            Task::Forms(mut forms, owner) => {
                let form = forms.next().expect("nonempty task");
                if !forms.as_slice().is_empty() {
                    self.tasks.push(Task::Forms(forms, owner));
                }
                self.tasks.push(Task::Form(form, owner));
                Ok(())
            }
            Task::Form(form, owner) => self.process_form(form, owner),
            Task::AdditionalResponses(mut responses) => {
                let response = responses.next().expect("nonempty task");
                if !responses.as_slice().is_empty() {
                    self.tasks.push(Task::AdditionalResponses(responses));
                }
                self.tasks.push(Task::AdditionalResponse(response));
                Ok(())
            }
            Task::AdditionalResponse(response) => {
                if let Some(content_type) = &response.content_type {
                    self.census.add_string(content_type, false, &self.limits)?;
                }
                if let Some(schema) = &response.schema {
                    self.census.add_string(schema, false, &self.limits)?;
                }
                self.push_extension_map(&response._extra_fields, false)
            }
            Task::Links(mut links) => {
                let link = links.next().expect("nonempty task");
                if !links.as_slice().is_empty() {
                    self.tasks.push(Task::Links(links));
                }
                self.tasks.push(Task::Link(link));
                Ok(())
            }
            Task::Link(link) => self.process_link(link),
            Task::Strings(mut strings, class) => {
                let value = strings.next().expect("nonempty task");
                if !strings.as_slice().is_empty() {
                    self.tasks.push(Task::Strings(strings, class));
                }
                self.census.add_string(value, false, &self.limits)
            }
            Task::Uris(mut uris) => {
                let uri = uris.next().expect("nonempty task");
                if !uris.as_slice().is_empty() {
                    self.tasks.push(Task::Uris(uris));
                }
                self.census.add_uri(uri, false, &self.limits)
            }
            Task::Operations(mut operations) => {
                let _ = operations.next().expect("nonempty task");
                if !operations.as_slice().is_empty() {
                    self.tasks.push(Task::Operations(operations));
                }
                Ok(())
            }
            Task::SchemaMap(mut entries, depth) => {
                let (key, schema) = entries.next().expect("nonempty task");
                if entries.len() != 0 {
                    self.tasks.push(Task::SchemaMap(entries, depth));
                }
                self.census.add_string(key, false, &self.limits)?;
                self.tasks.push(Task::Schema(schema, depth));
                Ok(())
            }
            Task::Schemas(mut schemas, depth) => {
                let schema = schemas.next().expect("nonempty task");
                if !schemas.as_slice().is_empty() {
                    self.tasks.push(Task::Schemas(schemas, depth));
                }
                self.tasks.push(Task::Schema(schema, depth));
                Ok(())
            }
            Task::Schema(schema, depth) => self.process_schema(schema, depth),
            Task::Json(value, parent_depth, extension) => {
                self.process_json(value, parent_depth, extension)
            }
            Task::JsonArray(mut values, depth, extension) => {
                let value = values.next().expect("nonempty task");
                if !values.as_slice().is_empty() {
                    self.tasks.push(Task::JsonArray(values, depth, extension));
                }
                self.tasks.push(Task::Json(value, depth, extension));
                Ok(())
            }
            Task::JsonObject(mut values, depth, extension) => {
                let (key, value) = values.next().expect("nonempty task");
                if values.len() != 0 {
                    self.tasks.push(Task::JsonObject(values, depth, extension));
                }
                self.census.add_string(key, extension, &self.limits)?;
                self.tasks.push(Task::Json(value, depth, extension));
                Ok(())
            }
            Task::ExtensionMap(mut values, inherited_extension) => {
                let (key, value) = values.next().expect("nonempty task");
                if values.len() != 0 {
                    self.tasks
                        .push(Task::ExtensionMap(values, inherited_extension));
                }
                self.census.add_string(key, true, &self.limits)?;
                self.tasks.push(Task::Json(value, 0, true));
                Ok(())
            }
            Task::SecurityDefinitions(mut entries) => {
                let (key, scheme) = entries.next().expect("nonempty task");
                if entries.len() != 0 {
                    self.tasks.push(Task::SecurityDefinitions(entries));
                }
                self.census.add_string(key, false, &self.limits)?;
                self.tasks.push(Task::SecurityScheme(scheme));
                Ok(())
            }
            Task::SecurityScheme(scheme) => self.process_security_scheme(scheme),
            Task::SecurityExpression {
                mut pending,
                branches,
                definitions,
            } => {
                let reference = pending.pop().expect("nonempty task");
                let next_branches = branches.checked_add(1).ok_or(LimitFailure {
                    kind: ResourceKind::SecurityBranchesPerPlanMax,
                    configured: self.limits.security_branches_per_plan_max,
                    observed: None,
                })?;
                self.census.check(
                    ResourceKind::SecurityBranchesPerPlanMax,
                    self.limits.security_branches_per_plan_max,
                    next_branches,
                )?;
                self.census.check(
                    ResourceKind::SecurityExpressionDepthMax,
                    self.limits.security_expression_depth_max,
                    reference.depth,
                )?;

                if let Some(SecurityScheme::Combo(combo)) = definitions.get(reference.name) {
                    let next_depth = reference.depth.checked_add(1).ok_or(LimitFailure {
                        kind: ResourceKind::SecurityExpressionDepthMax,
                        configured: self.limits.security_expression_depth_max,
                        observed: None,
                    })?;
                    let child_count = usize_to_u64(
                        combo.one_of.len().saturating_add(combo.all_of.len()),
                        ResourceKind::SecurityBranchesPerPlanMax,
                        self.limits.security_branches_per_plan_max,
                    )?;
                    let pending_count = usize_to_u64(
                        pending.len(),
                        ResourceKind::SecurityBranchesPerPlanMax,
                        self.limits.security_branches_per_plan_max,
                    )?;
                    let projected = next_branches
                        .checked_add(pending_count)
                        .and_then(|value| value.checked_add(child_count))
                        .ok_or(LimitFailure {
                            kind: ResourceKind::SecurityBranchesPerPlanMax,
                            configured: self.limits.security_branches_per_plan_max,
                            observed: None,
                        })?;
                    self.census.check(
                        ResourceKind::SecurityBranchesPerPlanMax,
                        self.limits.security_branches_per_plan_max,
                        projected,
                    )?;
                    for name in combo.one_of.iter().chain(&combo.all_of) {
                        pending.push(SecurityReference {
                            name: static_str(name.as_str()),
                            depth: next_depth,
                        });
                    }
                }

                if !pending.is_empty() {
                    self.tasks.push(Task::SecurityExpression {
                        pending,
                        branches: next_branches,
                        definitions,
                    });
                }
                Ok(())
            }
            Task::UriTemplate {
                bytes,
                offset,
                variables,
            } => {
                let next_variables = if bytes[offset] == b'{' {
                    variables.checked_add(1).ok_or(LimitFailure {
                        kind: ResourceKind::UriTemplateVariablesMax,
                        configured: self.limits.uri_template_variables_max,
                        observed: None,
                    })?
                } else {
                    variables
                };
                self.census.check(
                    ResourceKind::UriTemplateVariablesMax,
                    self.limits.uri_template_variables_max,
                    next_variables,
                )?;
                let next_offset = offset + 1;
                if next_offset < bytes.len() {
                    self.tasks.push(Task::UriTemplate {
                        bytes,
                        offset: next_offset,
                        variables: next_variables,
                    });
                }
                Ok(())
            }
        }
    }

    fn process_thing(&mut self, thing: &'static Thing) -> Result<(), LimitFailure> {
        self.census
            .add_retained(size_of::<Thing>() as u64, false, false, &self.limits)?;

        let property_count = map_len(thing.properties.as_ref());
        let action_count = map_len(thing.actions.as_ref());
        let event_count = map_len(thing.events.as_ref());
        let affordances = property_count
            .checked_add(action_count)
            .and_then(|value| value.checked_add(event_count))
            .ok_or(LimitFailure {
                kind: ResourceKind::AffordancesPerThingMax,
                configured: self.limits.affordances_per_thing_max,
                observed: None,
            })?;
        self.census.check(
            ResourceKind::AffordancesPerThingMax,
            self.limits.affordances_per_thing_max,
            affordances,
        )?;
        self.census.property_count = property_count;

        self.tasks.push(Task::Context(static_ref(&thing.context)));
        self.tasks
            .push(Task::Metadata(static_ref(&thing._metadata)));
        if let Some(id) = &thing.id {
            self.census.add_uri(id, false, &self.limits)?;
        }
        if let Some(version) = &thing.version {
            self.tasks.push(Task::Version(static_ref(version)));
        }
        if let Some(support) = &thing.support {
            self.census.add_uri(support, false, &self.limits)?;
        }
        if let Some(base) = &thing.base {
            self.add_base_uri(base)?;
        }

        if let Some(properties) = &thing.properties {
            self.census.account_btree(properties, false, &self.limits)?;
            self.tasks
                .push(Task::Properties(static_btree_iter(properties)));
        }
        if let Some(actions) = &thing.actions {
            self.census.account_btree(actions, false, &self.limits)?;
            self.tasks.push(Task::Actions(static_btree_iter(actions)));
        }
        if let Some(events) = &thing.events {
            self.census.account_btree(events, false, &self.limits)?;
            self.tasks.push(Task::Events(static_btree_iter(events)));
        }
        if let Some(links) = &thing.links {
            self.census.account_vec(links, false, &self.limits)?;
            self.tasks.push(Task::Links(static_slice_iter(links)));
        }
        if let Some(forms) = &thing.forms {
            self.push_forms(forms, FormOwner::Thing)?;
        }

        self.census
            .account_vec(&thing.security, false, &self.limits)?;
        self.tasks.push(Task::Strings(
            static_slice_iter(&thing.security),
            WorkClass::SecurityBranches,
        ));
        self.push_security_expressions(&thing.security, &thing.security_definitions)?;

        self.census
            .account_btree(&thing.security_definitions, false, &self.limits)?;
        self.tasks.push(Task::SecurityDefinitions(static_btree_iter(
            &thing.security_definitions,
        )));

        if let Some(profile) = &thing.profile {
            self.census.account_vec(profile, false, &self.limits)?;
            self.tasks.push(Task::Uris(static_slice_iter(profile)));
        }
        if let Some(schemas) = &thing.schema_definitions {
            self.push_schema_map(schemas, 1, false)?;
        }
        if let Some(uri_variables) = &thing.uri_variables {
            self.check_uri_variables(uri_variables)?;
            self.push_schema_map(uri_variables, 1, false)?;
        }
        self.push_extension_map(&thing._extra_fields, false)
    }

    fn process_context(&mut self, context: &'static Context) -> Result<(), LimitFailure> {
        let (entries, capacity) = context.entry_storage();
        self.census.account_vec_storage::<ContextEntry>(
            entries.len(),
            capacity,
            false,
            &self.limits,
        )?;
        self.tasks
            .push(Task::ContextEntries(static_slice_iter(entries)));
        Ok(())
    }

    fn process_metadata(&mut self, metadata: &'static Metadata) -> Result<(), LimitFailure> {
        if let Some(tags) = &metadata.tags {
            self.census.account_vec(tags, false, &self.limits)?;
            self.tasks.push(Task::Strings(
                static_slice_iter(tags),
                WorkClass::DocumentNodes,
            ));
        }
        if let Some(title) = &metadata.title {
            self.census.add_string(title, false, &self.limits)?;
        }
        if let Some(titles) = &metadata.titles {
            self.tasks.push(Task::MultiLanguage(static_ref(titles)));
        }
        if let Some(description) = &metadata.description {
            self.census.add_string(description, false, &self.limits)?;
        }
        if let Some(descriptions) = &metadata.descriptions {
            self.tasks
                .push(Task::MultiLanguage(static_ref(descriptions)));
        }
        Ok(())
    }

    fn process_property(
        &mut self,
        property: &'static PropertyAffordance,
    ) -> Result<(), LimitFailure> {
        self.tasks
            .push(Task::Schema(static_ref(&property._schema), 1));
        if let Some(uri_variables) = &property._interaction.uri_variables {
            self.check_uri_variables(uri_variables)?;
            self.push_schema_map(uri_variables, 1, false)?;
        }
        self.push_forms(&property._interaction.forms, FormOwner::Property(property))
    }

    fn process_action(&mut self, action: &'static ActionAffordance) -> Result<(), LimitFailure> {
        self.tasks
            .push(Task::Metadata(static_ref(&action._metadata)));
        if let Some(uri_variables) = &action._interaction.uri_variables {
            self.check_uri_variables(uri_variables)?;
            self.push_schema_map(uri_variables, 1, false)?;
        }
        if let Some(input) = &action.input {
            self.tasks.push(Task::Schema(static_ref(input), 1));
        }
        if let Some(output) = &action.output {
            self.tasks.push(Task::Schema(static_ref(output), 1));
        }
        self.push_extension_map(&action._extra_fields, false)?;
        self.push_forms(&action._interaction.forms, FormOwner::Action)
    }

    fn process_event(&mut self, event: &'static EventAffordance) -> Result<(), LimitFailure> {
        self.tasks
            .push(Task::Metadata(static_ref(&event._metadata)));
        if let Some(uri_variables) = &event._interaction.uri_variables {
            self.check_uri_variables(uri_variables)?;
            self.push_schema_map(uri_variables, 1, false)?;
        }
        for schema in [
            event.subscription.as_ref(),
            event.data.as_ref(),
            event.data_response.as_ref(),
            event.cancellation.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            self.tasks.push(Task::Schema(static_ref(schema), 1));
        }
        self.push_extension_map(&event._extra_fields, false)?;
        self.push_forms(&event._interaction.forms, FormOwner::Event)
    }

    fn process_form(&mut self, form: &'static Form, owner: FormOwner) -> Result<(), LimitFailure> {
        match &form.href {
            FormHref::Reference(reference) => {
                self.census
                    .add_uri_reference(reference, false, &self.limits)?;
            }
            FormHref::Template(template) => {
                self.census.add_string(template, false, &self.limits)?;
                let observed = usize_to_u64(
                    template.len(),
                    ResourceKind::UriTemplateSourceBytesMax,
                    self.limits.uri_template_source_bytes_max,
                )?;
                self.census.check(
                    ResourceKind::UriTemplateSourceBytesMax,
                    self.limits.uri_template_source_bytes_max,
                    observed,
                )?;
                if !template.is_empty() {
                    self.tasks.push(Task::UriTemplate {
                        bytes: static_bytes(template.as_bytes()),
                        offset: 0,
                        variables: 0,
                    });
                }
            }
        }
        self.census
            .add_string(&form.content_type, false, &self.limits)?;
        for value in [form.content_coding.as_ref(), form.subprotocol.as_ref()]
            .into_iter()
            .flatten()
        {
            self.census.add_string(value, false, &self.limits)?;
        }
        if let Some(security) = &form.security {
            self.census.account_vec(security, false, &self.limits)?;
            self.tasks.push(Task::Strings(
                static_slice_iter(security),
                WorkClass::SecurityBranches,
            ));
            self.push_security_expressions(security, static_ref(&self.thing.security_definitions))?;
        }
        if let Some(scopes) = &form.scopes {
            self.census.account_vec(scopes, false, &self.limits)?;
            self.tasks.push(Task::Strings(
                static_slice_iter(scopes),
                WorkClass::DocumentNodes,
            ));
        }
        if let Some(response) = &form.response {
            self.census
                .add_string(&response.content_type, false, &self.limits)?;
            self.push_extension_map(&response._extra_fields, false)?;
        }
        if let Some(responses) = &form.additional_responses {
            let observed = usize_to_u64(
                responses.len(),
                ResourceKind::AdditionalResponsesPerFormMax,
                self.limits.additional_responses_per_form_max,
            )?;
            self.census.check(
                ResourceKind::AdditionalResponsesPerFormMax,
                self.limits.additional_responses_per_form_max,
                observed,
            )?;
            self.census.account_vec(responses, false, &self.limits)?;
            self.tasks
                .push(Task::AdditionalResponses(static_slice_iter(responses)));
        }
        if let Some(operations) = &form.op {
            self.census.account_vec(operations, false, &self.limits)?;
            self.tasks
                .push(Task::Operations(static_slice_iter(operations)));
        }
        self.push_extension_map(&form._extra_fields, false)?;

        if let FormOwner::Property(property) = owner
            && effective_form_operations(FormContext::Property(property), form)
                .contains(&Operation::ReadProperty)
        {
            self.census.readable_property_form_count = self
                .census
                .readable_property_form_count
                .checked_add(1)
                .ok_or(LimitFailure {
                    kind: ResourceKind::FormsPerThingMax,
                    configured: self.limits.forms_per_thing_max,
                    observed: None,
                })?;
        }
        Ok(())
    }

    fn process_link(&mut self, link: &'static Link) -> Result<(), LimitFailure> {
        self.census
            .add_uri_reference(&link.href, false, &self.limits)?;
        for value in [
            link.content_type.as_ref(),
            link.rel.as_ref(),
            link.sizes.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            self.census.add_string(value, false, &self.limits)?;
        }
        if let Some(anchor) = &link.anchor {
            self.census.add_uri_reference(anchor, false, &self.limits)?;
        }
        if let Some(hreflang) = &link.hreflang {
            self.census.account_vec(hreflang, false, &self.limits)?;
            self.tasks.push(Task::Strings(
                static_slice_iter(hreflang),
                WorkClass::DocumentNodes,
            ));
        }
        self.push_extension_map(&link._extra_fields, false)
    }

    fn process_schema(
        &mut self,
        schema: &'static DataSchema,
        depth: u64,
    ) -> Result<(), LimitFailure> {
        self.census.schema_nodes = self
            .census
            .schema_nodes
            .checked_add(1)
            .ok_or(LimitFailure {
                kind: ResourceKind::SchemaNodesPerDocumentMax,
                configured: self.limits.schema_nodes_per_document_max,
                observed: None,
            })?;
        self.census.check(
            ResourceKind::SchemaNodesPerDocumentMax,
            self.limits.schema_nodes_per_document_max,
            self.census.schema_nodes,
        )?;
        self.census.check(
            ResourceKind::SchemaCompositionDepthMax,
            self.limits.schema_composition_depth_max,
            depth,
        )?;

        let context = schema.context();
        self.process_schema_context(context, depth)?;
        match schema {
            DataSchema::Array(array) => {
                if let Some(items) = &array.items {
                    self.add_schema_edges(items.len())?;
                    self.census.account_vec(items, false, &self.limits)?;
                    self.tasks.push(Task::Schemas(
                        static_slice_iter(items),
                        next_depth(depth, &self.limits)?,
                    ));
                }
            }
            DataSchema::Object(object) => {
                if let Some(properties) = &object.properties {
                    self.add_schema_edges(properties.len())?;
                    self.push_schema_map(properties, next_depth(depth, &self.limits)?, false)?;
                }
                if let Some(required) = &object.required {
                    self.census.account_vec(required, false, &self.limits)?;
                    self.tasks.push(Task::Strings(
                        static_slice_iter(required),
                        WorkClass::JsonSchemaNodes,
                    ));
                }
            }
            DataSchema::String(string) => {
                for value in [
                    string.pattern.as_ref(),
                    string.content_encoding.as_ref(),
                    string.content_media_type.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    self.census.add_string(value, false, &self.limits)?;
                }
            }
            DataSchema::Boolean(_)
            | DataSchema::Number(_)
            | DataSchema::Integer(_)
            | DataSchema::Null(_) => {}
        }
        Ok(())
    }

    fn process_schema_context(
        &mut self,
        context: &'static DataSchemaContext,
        depth: u64,
    ) -> Result<(), LimitFailure> {
        self.tasks
            .push(Task::Metadata(static_ref(&context._metadata)));
        if let Some(value) = &context.constant {
            self.tasks.push(Task::Json(static_ref(value), 0, false));
        }
        if let Some(value) = &context.default {
            self.tasks.push(Task::Json(static_ref(value), 0, false));
        }
        for value in [
            context.unit.as_ref(),
            context.format.as_ref(),
            context.data_type.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            self.census.add_string(value, false, &self.limits)?;
        }
        if let Some(one_of) = &context.one_of {
            self.add_schema_edges(one_of.len())?;
            self.census.account_vec(one_of, false, &self.limits)?;
            self.tasks.push(Task::Schemas(
                static_slice_iter(one_of),
                next_depth(depth, &self.limits)?,
            ));
        }
        if let Some(enumerate) = &context.enumerate {
            self.census.account_vec(enumerate, false, &self.limits)?;
            self.tasks
                .push(Task::JsonArray(static_slice_iter(enumerate), 0, false));
        }
        self.push_extension_map(&context._extra_fields, false)
    }

    fn process_json(
        &mut self,
        value: &'static Value,
        parent_depth: u64,
        extension: bool,
    ) -> Result<(), LimitFailure> {
        self.census.observe_json_node(&self.limits)?;
        match value {
            Value::Null | Value::Bool(_) | Value::Number(_) => Ok(()),
            Value::String(value) => self.census.add_string(value, extension, &self.limits),
            Value::Array(values) => {
                let depth = next_json_depth(parent_depth, &self.limits)?;
                self.census.account_vec(values, extension, &self.limits)?;
                self.tasks
                    .push(Task::JsonArray(static_slice_iter(values), depth, extension));
                Ok(())
            }
            Value::Object(values) => {
                let depth = next_json_depth(parent_depth, &self.limits)?;
                self.census
                    .account_json_map(values, extension, &self.limits)?;
                self.tasks
                    .push(Task::JsonObject(static_json_iter(values), depth, extension));
                Ok(())
            }
        }
    }

    fn process_security_scheme(
        &mut self,
        scheme: &'static SecurityScheme,
    ) -> Result<(), LimitFailure> {
        let context = match scheme {
            SecurityScheme::NoSec(value) => &value._context,
            SecurityScheme::Auto(value) => &value._context,
            SecurityScheme::Combo(value) => {
                self.push_security_strings(&value.one_of)?;
                self.push_security_strings(&value.all_of)?;
                &value._context
            }
            SecurityScheme::Basic(value) => {
                if let Some(name) = &value.name {
                    self.census.add_string(name, false, &self.limits)?;
                }
                &value._context
            }
            SecurityScheme::Digest(value) => {
                if let Some(name) = &value.name {
                    self.census.add_string(name, false, &self.limits)?;
                }
                &value._context
            }
            SecurityScheme::APIKey(value) => {
                if let Some(name) = &value.name {
                    self.census.add_string(name, false, &self.limits)?;
                }
                &value._context
            }
            SecurityScheme::Bearer(value) => {
                if let Some(authorization) = &value.authorization {
                    self.census.add_uri(authorization, false, &self.limits)?;
                }
                if let Some(name) = &value.name {
                    self.census.add_string(name, false, &self.limits)?;
                }
                self.census.add_string(&value.alg, false, &self.limits)?;
                self.census.add_string(&value.format, false, &self.limits)?;
                &value._context
            }
            SecurityScheme::PSK(value) => {
                if let Some(identity) = &value.identity {
                    self.census.add_string(identity, false, &self.limits)?;
                }
                &value._context
            }
            SecurityScheme::OAuth2(value) => {
                for uri in [
                    value.authorization.as_ref(),
                    value.token.as_ref(),
                    value.refresh.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    self.census.add_uri(uri, false, &self.limits)?;
                }
                if let Some(scopes) = &value.scopes {
                    self.census.account_vec(scopes, false, &self.limits)?;
                    self.tasks.push(Task::Strings(
                        static_slice_iter(scopes),
                        WorkClass::SecurityBranches,
                    ));
                }
                self.census.add_string(&value.flow, false, &self.limits)?;
                &value._context
            }
        };
        self.process_security_context(static_ref(context))
    }

    fn process_security_context(
        &mut self,
        context: &'static SecuritySchemeContext,
    ) -> Result<(), LimitFailure> {
        if let Some(tags) = &context.tags {
            self.census.account_vec(tags, false, &self.limits)?;
            self.tasks.push(Task::Strings(
                static_slice_iter(tags),
                WorkClass::SecurityBranches,
            ));
        }
        if let Some(description) = &context.description {
            self.census.add_string(description, false, &self.limits)?;
        }
        if let Some(descriptions) = &context.descriptions {
            self.tasks
                .push(Task::MultiLanguage(static_ref(descriptions)));
        }
        if let Some(proxy) = &context.proxy {
            self.census.add_uri(proxy, false, &self.limits)?;
        }
        self.census
            .add_string(&context.scheme, false, &self.limits)?;
        self.push_extension_map(&context._extra_fields, false)
    }

    fn push_forms(
        &mut self,
        forms: &'static Vec<Form>,
        owner: FormOwner,
    ) -> Result<(), LimitFailure> {
        let observed = usize_to_u64(
            forms.len(),
            ResourceKind::FormsPerContextMax,
            self.limits.forms_per_context_max,
        )?;
        self.census.check(
            ResourceKind::FormsPerContextMax,
            self.limits.forms_per_context_max,
            observed,
        )?;
        self.census.forms_per_thing =
            self.census
                .forms_per_thing
                .checked_add(observed)
                .ok_or(LimitFailure {
                    kind: ResourceKind::FormsPerThingMax,
                    configured: self.limits.forms_per_thing_max,
                    observed: None,
                })?;
        self.census.check(
            ResourceKind::FormsPerThingMax,
            self.limits.forms_per_thing_max,
            self.census.forms_per_thing,
        )?;
        self.census.account_vec(forms, false, &self.limits)?;
        self.tasks
            .push(Task::Forms(static_slice_iter(forms), owner));
        Ok(())
    }

    fn push_schema_map(
        &mut self,
        schemas: &'static BTreeMap<String, DataSchema>,
        depth: u64,
        extension: bool,
    ) -> Result<(), LimitFailure> {
        self.census
            .account_btree(schemas, extension, &self.limits)?;
        self.tasks
            .push(Task::SchemaMap(static_btree_iter(schemas), depth));
        Ok(())
    }

    fn push_extension_map(
        &mut self,
        values: &'static ExtensionMap,
        inherited_extension: bool,
    ) -> Result<(), LimitFailure> {
        self.census.account_btree(values, true, &self.limits)?;
        self.tasks.push(Task::ExtensionMap(
            static_btree_iter(values),
            inherited_extension,
        ));
        Ok(())
    }

    fn push_security_strings(&mut self, values: &'static Vec<String>) -> Result<(), LimitFailure> {
        let observed = usize_to_u64(
            values.len(),
            ResourceKind::SecurityBranchesPerPlanMax,
            self.limits.security_branches_per_plan_max,
        )?;
        self.census.check(
            ResourceKind::SecurityBranchesPerPlanMax,
            self.limits.security_branches_per_plan_max,
            observed,
        )?;
        self.census.account_vec(values, false, &self.limits)?;
        self.tasks.push(Task::Strings(
            static_slice_iter(values),
            WorkClass::SecurityBranches,
        ));
        Ok(())
    }

    fn push_security_expressions(
        &mut self,
        roots: &'static [String],
        definitions: &'static BTreeMap<String, SecurityScheme>,
    ) -> Result<(), LimitFailure> {
        let roots_len = usize_to_u64(
            roots.len(),
            ResourceKind::SecurityBranchesPerPlanMax,
            self.limits.security_branches_per_plan_max,
        )?;
        self.census.check(
            ResourceKind::SecurityBranchesPerPlanMax,
            self.limits.security_branches_per_plan_max,
            roots_len,
        )?;
        if !roots.is_empty() {
            let mut pending = Vec::with_capacity(roots.len());
            for root in roots.iter().rev() {
                pending.push(SecurityReference {
                    name: static_str(root.as_str()),
                    depth: 1,
                });
            }
            self.tasks.push(Task::SecurityExpression {
                pending,
                branches: 0,
                definitions,
            });
        }
        Ok(())
    }

    fn check_uri_variables(
        &self,
        values: &BTreeMap<String, DataSchema>,
    ) -> Result<(), LimitFailure> {
        let observed = usize_to_u64(
            values.len(),
            ResourceKind::UriVariablesPerFormMax,
            self.limits.uri_variables_per_form_max,
        )?;
        self.census.check(
            ResourceKind::UriVariablesPerFormMax,
            self.limits.uri_variables_per_form_max,
            observed,
        )
    }

    fn add_schema_edges(&mut self, edges: usize) -> Result<(), LimitFailure> {
        let edges = usize_to_u64(
            edges,
            ResourceKind::SchemaReferenceEdgesPerDocumentMax,
            self.limits.schema_reference_edges_per_document_max,
        )?;
        self.census.schema_edges =
            self.census
                .schema_edges
                .checked_add(edges)
                .ok_or(LimitFailure {
                    kind: ResourceKind::SchemaReferenceEdgesPerDocumentMax,
                    configured: self.limits.schema_reference_edges_per_document_max,
                    observed: None,
                })?;
        self.census.check(
            ResourceKind::SchemaReferenceEdgesPerDocumentMax,
            self.limits.schema_reference_edges_per_document_max,
            self.census.schema_edges,
        )
    }

    fn add_base_uri(&mut self, base: &BaseUri) -> Result<(), LimitFailure> {
        match base {
            BaseUri::Absolute(uri) => self.census.add_uri(uri, false, &self.limits),
            BaseUri::Template(template) => self.census.add_string(template, false, &self.limits),
        }
    }

    fn limit(self, kind: ResourceKind, observed: Option<u64>) -> ValidatedThingStep {
        let configured = self.limits.get(kind);
        ValidatedThingStep::Limit {
            thing: self.into_thing(),
            kind,
            configured,
            observed,
        }
    }

    fn limit_failure(self, failure: LimitFailure) -> ValidatedThingStep {
        ValidatedThingStep::Limit {
            thing: self.into_thing(),
            kind: failure.kind,
            configured: failure.configured,
            observed: failure.observed,
        }
    }

    fn into_thing(mut self) -> Thing {
        // Erased-lifetime tasks borrow only this boxed tree. Drop every borrow
        // before moving the exact value back out of its fixed-size box.
        self.tasks.clear();
        *self.thing
    }
}

#[derive(Clone, Copy)]
enum Phase {
    CensusNotStarted,
    Census,
    Basic,
}

#[derive(Clone, Copy)]
struct Charge {
    class: WorkClass,
    units: u64,
    semantic: bool,
}

#[derive(Clone, Copy)]
enum FormOwner {
    Thing,
    Property(&'static PropertyAffordance),
    Action,
    Event,
}

struct SecurityReference {
    name: &'static str,
    depth: u64,
}

enum Task {
    Thing(&'static Thing),
    Context(&'static Context),
    ContextEntries(slice::Iter<'static, ContextEntry>),
    ContextObject(btree_map::Iter<'static, String, Value>, u64),
    Metadata(&'static Metadata),
    MultiLanguage(&'static MultiLanguage),
    MultiLanguageEntries(btree_map::Iter<'static, String, String>),
    Version(&'static VersionInfo),
    Properties(btree_map::Iter<'static, String, PropertyAffordance>),
    Actions(btree_map::Iter<'static, String, ActionAffordance>),
    Events(btree_map::Iter<'static, String, EventAffordance>),
    Property(&'static PropertyAffordance),
    Action(&'static ActionAffordance),
    Event(&'static EventAffordance),
    Forms(slice::Iter<'static, Form>, FormOwner),
    Form(&'static Form, FormOwner),
    AdditionalResponses(slice::Iter<'static, AdditionalExpectedResponse>),
    AdditionalResponse(&'static AdditionalExpectedResponse),
    Links(slice::Iter<'static, Link>),
    Link(&'static Link),
    Strings(slice::Iter<'static, String>, WorkClass),
    Uris(slice::Iter<'static, AbsoluteUri>),
    Operations(slice::Iter<'static, Operation>),
    SchemaMap(btree_map::Iter<'static, String, DataSchema>, u64),
    Schemas(slice::Iter<'static, DataSchema>, u64),
    Schema(&'static DataSchema, u64),
    Json(&'static Value, u64, bool),
    JsonArray(slice::Iter<'static, Value>, u64, bool),
    JsonObject(serde_json::map::Iter<'static>, u64, bool),
    ExtensionMap(btree_map::Iter<'static, String, Value>, bool),
    SecurityDefinitions(btree_map::Iter<'static, String, SecurityScheme>),
    SecurityScheme(&'static SecurityScheme),
    SecurityExpression {
        pending: Vec<SecurityReference>,
        branches: u64,
        definitions: &'static BTreeMap<String, SecurityScheme>,
    },
    UriTemplate {
        bytes: &'static [u8],
        offset: usize,
        variables: u64,
    },
}

impl Task {
    fn charge(&self) -> Option<Charge> {
        let (class, semantic, has_work) = match self {
            Self::ContextEntries(values) => (
                WorkClass::DocumentNodes,
                true,
                !values.as_slice().is_empty(),
            ),
            Self::ContextObject(values, _) | Self::ExtensionMap(values, _) => {
                (WorkClass::JsonSchemaNodes, false, values.len() != 0)
            }
            Self::MultiLanguageEntries(values) => {
                (WorkClass::DocumentNodes, true, values.len() != 0)
            }
            Self::Properties(values) => (WorkClass::DocumentNodes, true, values.len() != 0),
            Self::Actions(values) => (WorkClass::DocumentNodes, true, values.len() != 0),
            Self::Events(values) => (WorkClass::DocumentNodes, true, values.len() != 0),
            Self::Forms(values, _) => (
                WorkClass::DocumentNodes,
                true,
                !values.as_slice().is_empty(),
            ),
            Self::AdditionalResponses(values) => (
                WorkClass::DocumentNodes,
                true,
                !values.as_slice().is_empty(),
            ),
            Self::Links(values) => (
                WorkClass::DocumentNodes,
                true,
                !values.as_slice().is_empty(),
            ),
            Self::Strings(values, class) => (*class, true, !values.as_slice().is_empty()),
            Self::Uris(values) => (
                WorkClass::DocumentNodes,
                true,
                !values.as_slice().is_empty(),
            ),
            Self::Operations(values) => (
                WorkClass::DocumentNodes,
                true,
                !values.as_slice().is_empty(),
            ),
            Self::SchemaMap(values, _) => (WorkClass::JsonSchemaNodes, true, values.len() != 0),
            Self::Schemas(values, _) => (
                WorkClass::JsonSchemaNodes,
                false,
                !values.as_slice().is_empty(),
            ),
            Self::Schema(_, _) => (WorkClass::JsonSchemaNodes, true, true),
            Self::Json(_, _, _) => (WorkClass::JsonSchemaNodes, false, true),
            Self::JsonArray(values, _, _) => (
                WorkClass::JsonSchemaNodes,
                false,
                !values.as_slice().is_empty(),
            ),
            Self::JsonObject(values, _, _) => {
                (WorkClass::JsonSchemaNodes, false, values.len() != 0)
            }
            Self::SecurityDefinitions(values) => {
                (WorkClass::SecurityBranches, true, values.len() != 0)
            }
            Self::SecurityScheme(_) => (WorkClass::SecurityBranches, true, true),
            Self::SecurityExpression { pending, .. } => {
                (WorkClass::SecurityBranches, false, !pending.is_empty())
            }
            Self::UriTemplate { bytes, offset, .. } => {
                (WorkClass::UriBytes, false, *offset < bytes.len())
            }
            Self::Thing(_)
            | Self::Context(_)
            | Self::Metadata(_)
            | Self::MultiLanguage(_)
            | Self::Version(_)
            | Self::Property(_)
            | Self::Action(_)
            | Self::Event(_)
            | Self::Form(_, _)
            | Self::AdditionalResponse(_)
            | Self::Link(_) => (WorkClass::DocumentNodes, true, true),
        };
        has_work.then_some(Charge {
            class,
            units: 1,
            semantic,
        })
    }
}

#[derive(Clone, Copy, Default)]
struct WorkCharges {
    document_nodes: u64,
    json_schema_nodes: u64,
    uri_bytes: u64,
    security_branches: u64,
}

impl WorkCharges {
    fn add(&mut self, class: WorkClass, units: u64) -> Result<(), LimitFailure> {
        let counter = match class {
            WorkClass::DocumentNodes => &mut self.document_nodes,
            WorkClass::JsonSchemaNodes => &mut self.json_schema_nodes,
            WorkClass::UriBytes => &mut self.uri_bytes,
            WorkClass::SecurityBranches => &mut self.security_branches,
            _ => return Ok(()),
        };
        *counter = counter.checked_add(units).ok_or(LimitFailure {
            kind: ResourceKind::DocumentValidationWorkUnitsMax,
            configured: None,
            observed: None,
        })?;
        Ok(())
    }

    fn fit(self, budget: &WorkBudget) -> bool {
        budget.remaining(WorkClass::DocumentNodes) >= self.document_nodes
            && budget.remaining(WorkClass::JsonSchemaNodes) >= self.json_schema_nodes
            && budget.remaining(WorkClass::UriBytes) >= self.uri_bytes
            && budget.remaining(WorkClass::SecurityBranches) >= self.security_branches
    }

    fn consume(self, budget: &mut WorkBudget) {
        for (class, units) in [
            (WorkClass::DocumentNodes, self.document_nodes),
            (WorkClass::JsonSchemaNodes, self.json_schema_nodes),
            (WorkClass::UriBytes, self.uri_bytes),
            (WorkClass::SecurityBranches, self.security_branches),
        ] {
            let consumed = budget.consume(class, units);
            debug_assert!(consumed.is_ok());
        }
    }
}

struct Census {
    retained_bytes: u64,
    string_bytes: u64,
    extension_bytes: u64,
    json_nodes: u64,
    schema_nodes: u64,
    schema_edges: u64,
    forms_per_thing: u64,
    property_count: u64,
    readable_property_form_count: u64,
    semantic: WorkCharges,
}

impl Census {
    const fn new() -> Self {
        Self {
            retained_bytes: 0,
            string_bytes: 0,
            extension_bytes: 0,
            json_nodes: 0,
            schema_nodes: 0,
            schema_edges: 0,
            forms_per_thing: 0,
            property_count: 0,
            readable_property_form_count: 0,
            semantic: WorkCharges {
                document_nodes: 0,
                json_schema_nodes: 0,
                uri_bytes: 0,
                security_branches: 0,
            },
        }
    }

    fn check(
        &self,
        kind: ResourceKind,
        configured: Option<u64>,
        observed: u64,
    ) -> Result<(), LimitFailure> {
        match configured {
            Some(limit) if observed <= limit => Ok(()),
            _ => Err(LimitFailure {
                kind,
                configured,
                observed: Some(observed),
            }),
        }
    }

    fn add_retained(
        &mut self,
        bytes: u64,
        extension: bool,
        allocation: bool,
        limits: &ValidationLimits,
    ) -> Result<(), LimitFailure> {
        self.retained_bytes = self.retained_bytes.checked_add(bytes).ok_or(LimitFailure {
            kind: ResourceKind::RetainedSourceBytesPerOwnerMax,
            configured: limits.retained_source_bytes_per_owner_max,
            observed: None,
        })?;
        self.check(
            ResourceKind::DocumentBytesMax,
            limits.document_bytes_max,
            self.retained_bytes,
        )?;
        self.check(
            ResourceKind::RetainedSourceBytesPerOwnerMax,
            limits.retained_source_bytes_per_owner_max,
            self.retained_bytes,
        )?;
        if extension {
            self.extension_bytes = self
                .extension_bytes
                .checked_add(bytes)
                .ok_or(LimitFailure {
                    kind: ResourceKind::ExtensionBytesMax,
                    configured: limits.extension_bytes_max,
                    observed: None,
                })?;
            self.check(
                ResourceKind::ExtensionBytesMax,
                limits.extension_bytes_max,
                self.extension_bytes,
            )?;
        }
        if allocation {
            self.check(
                ResourceKind::LargestContiguousAllocationBytesMax,
                limits.largest_contiguous_allocation_bytes_max,
                bytes,
            )?;
        }
        Ok(())
    }

    fn add_string(
        &mut self,
        value: &String,
        extension: bool,
        limits: &ValidationLimits,
    ) -> Result<(), LimitFailure> {
        let length = usize_to_u64(
            value.len(),
            ResourceKind::StringBytesMax,
            limits.string_bytes_max,
        )?;
        self.string_bytes = self.string_bytes.checked_add(length).ok_or(LimitFailure {
            kind: ResourceKind::StringBytesMax,
            configured: limits.string_bytes_max,
            observed: None,
        })?;
        self.check(
            ResourceKind::StringBytesMax,
            limits.string_bytes_max,
            self.string_bytes,
        )?;
        let capacity = usize_to_u64(
            value.capacity(),
            ResourceKind::RetainedSourceBytesPerOwnerMax,
            limits.retained_source_bytes_per_owner_max,
        )?;
        self.add_retained(capacity, extension, true, limits)
    }

    fn add_uri(
        &mut self,
        value: &AbsoluteUri,
        extension: bool,
        limits: &ValidationLimits,
    ) -> Result<(), LimitFailure> {
        self.add_uri_text(value.as_str(), extension, limits)
    }

    fn add_uri_reference(
        &mut self,
        value: &UriReference,
        extension: bool,
        limits: &ValidationLimits,
    ) -> Result<(), LimitFailure> {
        self.add_uri_text(value.as_str(), extension, limits)
    }

    fn add_uri_text(
        &mut self,
        value: &str,
        extension: bool,
        limits: &ValidationLimits,
    ) -> Result<(), LimitFailure> {
        let length = usize_to_u64(
            value.len(),
            ResourceKind::StringBytesMax,
            limits.string_bytes_max,
        )?;
        self.string_bytes = self.string_bytes.checked_add(length).ok_or(LimitFailure {
            kind: ResourceKind::StringBytesMax,
            configured: limits.string_bytes_max,
            observed: None,
        })?;
        self.check(
            ResourceKind::StringBytesMax,
            limits.string_bytes_max,
            self.string_bytes,
        )?;
        // URI wrappers are constructed only by parsing and owning this exact
        // text; unlike public String fields, callers cannot inject spare String
        // capacity into the wrapper representation.
        self.add_retained(length, extension, true, limits)
    }

    fn account_vec<T>(
        &mut self,
        values: &Vec<T>,
        extension: bool,
        limits: &ValidationLimits,
    ) -> Result<(), LimitFailure> {
        self.account_vec_storage::<T>(values.len(), values.capacity(), extension, limits)
    }

    fn account_vec_storage<T>(
        &mut self,
        length: usize,
        capacity: usize,
        extension: bool,
        limits: &ValidationLimits,
    ) -> Result<(), LimitFailure> {
        let observed = usize_to_u64(
            length,
            ResourceKind::JsonArrayItemsMax,
            limits.json_array_items_max,
        )?;
        self.check(
            ResourceKind::JsonArrayItemsMax,
            limits.json_array_items_max,
            observed,
        )?;
        let bytes = allocation_bytes::<T>(capacity, limits)?;
        self.add_retained(bytes, extension, true, limits)
    }

    fn account_btree<K, V>(
        &mut self,
        values: &BTreeMap<K, V>,
        extension: bool,
        limits: &ValidationLimits,
    ) -> Result<(), LimitFailure> {
        let observed = usize_to_u64(
            values.len(),
            ResourceKind::JsonMembersPerObjectMax,
            limits.json_members_per_object_max,
        )?;
        self.check(
            ResourceKind::JsonMembersPerObjectMax,
            limits.json_members_per_object_max,
            observed,
        )?;
        let (bytes, largest_node) = btree_allocation_bytes::<K, V>(values.len(), limits)?;
        self.add_retained(bytes, extension, false, limits)?;
        if largest_node != 0 {
            self.check(
                ResourceKind::LargestContiguousAllocationBytesMax,
                limits.largest_contiguous_allocation_bytes_max,
                largest_node,
            )?;
        }
        Ok(())
    }

    fn account_json_map(
        &mut self,
        values: &serde_json::Map<String, Value>,
        extension: bool,
        limits: &ValidationLimits,
    ) -> Result<(), LimitFailure> {
        let observed = usize_to_u64(
            values.len(),
            ResourceKind::JsonMembersPerObjectMax,
            limits.json_members_per_object_max,
        )?;
        self.check(
            ResourceKind::JsonMembersPerObjectMax,
            limits.json_members_per_object_max,
            observed,
        )?;
        let (bytes, largest_node) = btree_allocation_bytes::<String, Value>(values.len(), limits)?;
        self.add_retained(bytes, extension, false, limits)?;
        if largest_node != 0 {
            self.check(
                ResourceKind::LargestContiguousAllocationBytesMax,
                limits.largest_contiguous_allocation_bytes_max,
                largest_node,
            )?;
        }
        Ok(())
    }

    fn observe_json_node(&mut self, limits: &ValidationLimits) -> Result<(), LimitFailure> {
        self.json_nodes = self.json_nodes.checked_add(1).ok_or(LimitFailure {
            kind: ResourceKind::JsonValueNodesPerDocumentMax,
            configured: limits.json_value_nodes_per_document_max,
            observed: None,
        })?;
        self.check(
            ResourceKind::JsonValueNodesPerDocumentMax,
            limits.json_value_nodes_per_document_max,
            self.json_nodes,
        )
    }

    fn observe_json_depth(
        &self,
        depth: u64,
        limits: &ValidationLimits,
    ) -> Result<(), LimitFailure> {
        self.check(
            ResourceKind::JsonNestingDepthMax,
            limits.json_nesting_depth_max,
            depth,
        )
    }
}

#[derive(Clone, Copy)]
struct LimitFailure {
    kind: ResourceKind,
    configured: Option<u64>,
    observed: Option<u64>,
}

#[derive(Clone, Copy)]
struct ValidationLimits {
    document_bytes_max: Option<u64>,
    string_bytes_max: Option<u64>,
    extension_bytes_max: Option<u64>,
    retained_source_bytes_per_owner_max: Option<u64>,
    largest_contiguous_allocation_bytes_max: Option<u64>,
    json_nesting_depth_max: Option<u64>,
    json_members_per_object_max: Option<u64>,
    json_array_items_max: Option<u64>,
    json_value_nodes_per_document_max: Option<u64>,
    affordances_per_thing_max: Option<u64>,
    forms_per_context_max: Option<u64>,
    forms_per_thing_max: Option<u64>,
    additional_responses_per_form_max: Option<u64>,
    uri_variables_per_form_max: Option<u64>,
    schema_nodes_per_document_max: Option<u64>,
    schema_composition_depth_max: Option<u64>,
    schema_reference_edges_per_document_max: Option<u64>,
    document_validation_work_units_max: Option<u64>,
    security_expression_depth_max: Option<u64>,
    security_branches_per_plan_max: Option<u64>,
    uri_template_source_bytes_max: Option<u64>,
    uri_template_variables_max: Option<u64>,
}

impl ValidationLimits {
    fn capture(limits: &ResourceLimits) -> Self {
        Self {
            document_bytes_max: limits.get(ResourceKind::DocumentBytesMax),
            string_bytes_max: limits.get(ResourceKind::StringBytesMax),
            extension_bytes_max: limits.get(ResourceKind::ExtensionBytesMax),
            retained_source_bytes_per_owner_max: limits
                .get(ResourceKind::RetainedSourceBytesPerOwnerMax),
            largest_contiguous_allocation_bytes_max: limits
                .get(ResourceKind::LargestContiguousAllocationBytesMax),
            json_nesting_depth_max: limits.get(ResourceKind::JsonNestingDepthMax),
            json_members_per_object_max: limits.get(ResourceKind::JsonMembersPerObjectMax),
            json_array_items_max: limits.get(ResourceKind::JsonArrayItemsMax),
            json_value_nodes_per_document_max: limits
                .get(ResourceKind::JsonValueNodesPerDocumentMax),
            affordances_per_thing_max: limits.get(ResourceKind::AffordancesPerThingMax),
            forms_per_context_max: limits.get(ResourceKind::FormsPerContextMax),
            forms_per_thing_max: limits.get(ResourceKind::FormsPerThingMax),
            additional_responses_per_form_max: limits
                .get(ResourceKind::AdditionalResponsesPerFormMax),
            uri_variables_per_form_max: limits.get(ResourceKind::UriVariablesPerFormMax),
            schema_nodes_per_document_max: limits.get(ResourceKind::SchemaNodesPerDocumentMax),
            schema_composition_depth_max: limits.get(ResourceKind::SchemaCompositionDepthMax),
            schema_reference_edges_per_document_max: limits
                .get(ResourceKind::SchemaReferenceEdgesPerDocumentMax),
            document_validation_work_units_max: limits
                .get(ResourceKind::DocumentValidationWorkUnitsMax),
            security_expression_depth_max: limits.get(ResourceKind::SecurityExpressionDepthMax),
            security_branches_per_plan_max: limits.get(ResourceKind::SecurityBranchesPerPlanMax),
            uri_template_source_bytes_max: limits.get(ResourceKind::UriTemplateSourceBytesMax),
            uri_template_variables_max: limits.get(ResourceKind::UriTemplateVariablesMax),
        }
    }

    fn first_missing(self) -> Option<ResourceKind> {
        [
            (ResourceKind::DocumentBytesMax, self.document_bytes_max),
            (ResourceKind::StringBytesMax, self.string_bytes_max),
            (ResourceKind::ExtensionBytesMax, self.extension_bytes_max),
            (
                ResourceKind::RetainedSourceBytesPerOwnerMax,
                self.retained_source_bytes_per_owner_max,
            ),
            (
                ResourceKind::LargestContiguousAllocationBytesMax,
                self.largest_contiguous_allocation_bytes_max,
            ),
            (
                ResourceKind::JsonNestingDepthMax,
                self.json_nesting_depth_max,
            ),
            (
                ResourceKind::JsonMembersPerObjectMax,
                self.json_members_per_object_max,
            ),
            (ResourceKind::JsonArrayItemsMax, self.json_array_items_max),
            (
                ResourceKind::JsonValueNodesPerDocumentMax,
                self.json_value_nodes_per_document_max,
            ),
            (
                ResourceKind::AffordancesPerThingMax,
                self.affordances_per_thing_max,
            ),
            (ResourceKind::FormsPerContextMax, self.forms_per_context_max),
            (ResourceKind::FormsPerThingMax, self.forms_per_thing_max),
            (
                ResourceKind::AdditionalResponsesPerFormMax,
                self.additional_responses_per_form_max,
            ),
            (
                ResourceKind::UriVariablesPerFormMax,
                self.uri_variables_per_form_max,
            ),
            (
                ResourceKind::SchemaNodesPerDocumentMax,
                self.schema_nodes_per_document_max,
            ),
            (
                ResourceKind::SchemaCompositionDepthMax,
                self.schema_composition_depth_max,
            ),
            (
                ResourceKind::SchemaReferenceEdgesPerDocumentMax,
                self.schema_reference_edges_per_document_max,
            ),
            (
                ResourceKind::DocumentValidationWorkUnitsMax,
                self.document_validation_work_units_max,
            ),
            (
                ResourceKind::SecurityExpressionDepthMax,
                self.security_expression_depth_max,
            ),
            (
                ResourceKind::SecurityBranchesPerPlanMax,
                self.security_branches_per_plan_max,
            ),
            (
                ResourceKind::UriTemplateSourceBytesMax,
                self.uri_template_source_bytes_max,
            ),
            (
                ResourceKind::UriTemplateVariablesMax,
                self.uri_template_variables_max,
            ),
        ]
        .into_iter()
        .find_map(|(kind, value)| value.is_none().then_some(kind))
    }

    fn get(self, kind: ResourceKind) -> Option<u64> {
        match kind {
            ResourceKind::DocumentBytesMax => self.document_bytes_max,
            ResourceKind::StringBytesMax => self.string_bytes_max,
            ResourceKind::ExtensionBytesMax => self.extension_bytes_max,
            ResourceKind::RetainedSourceBytesPerOwnerMax => {
                self.retained_source_bytes_per_owner_max
            }
            ResourceKind::LargestContiguousAllocationBytesMax => {
                self.largest_contiguous_allocation_bytes_max
            }
            ResourceKind::JsonNestingDepthMax => self.json_nesting_depth_max,
            ResourceKind::JsonMembersPerObjectMax => self.json_members_per_object_max,
            ResourceKind::JsonArrayItemsMax => self.json_array_items_max,
            ResourceKind::JsonValueNodesPerDocumentMax => self.json_value_nodes_per_document_max,
            ResourceKind::AffordancesPerThingMax => self.affordances_per_thing_max,
            ResourceKind::FormsPerContextMax => self.forms_per_context_max,
            ResourceKind::FormsPerThingMax => self.forms_per_thing_max,
            ResourceKind::AdditionalResponsesPerFormMax => self.additional_responses_per_form_max,
            ResourceKind::UriVariablesPerFormMax => self.uri_variables_per_form_max,
            ResourceKind::SchemaNodesPerDocumentMax => self.schema_nodes_per_document_max,
            ResourceKind::SchemaCompositionDepthMax => self.schema_composition_depth_max,
            ResourceKind::SchemaReferenceEdgesPerDocumentMax => {
                self.schema_reference_edges_per_document_max
            }
            ResourceKind::DocumentValidationWorkUnitsMax => self.document_validation_work_units_max,
            ResourceKind::SecurityExpressionDepthMax => self.security_expression_depth_max,
            ResourceKind::SecurityBranchesPerPlanMax => self.security_branches_per_plan_max,
            ResourceKind::UriTemplateSourceBytesMax => self.uri_template_source_bytes_max,
            ResourceKind::UriTemplateVariablesMax => self.uri_template_variables_max,
            _ => None,
        }
    }
}

fn map_len<T>(values: Option<&BTreeMap<String, T>>) -> u64 {
    match values {
        Some(values) => u64::try_from(values.len()).unwrap_or(u64::MAX),
        None => 0,
    }
}

fn next_depth(depth: u64, limits: &ValidationLimits) -> Result<u64, LimitFailure> {
    depth.checked_add(1).ok_or(LimitFailure {
        kind: ResourceKind::SchemaCompositionDepthMax,
        configured: limits.schema_composition_depth_max,
        observed: None,
    })
}

fn next_json_depth(parent_depth: u64, limits: &ValidationLimits) -> Result<u64, LimitFailure> {
    let depth = parent_depth.checked_add(1).ok_or(LimitFailure {
        kind: ResourceKind::JsonNestingDepthMax,
        configured: limits.json_nesting_depth_max,
        observed: None,
    })?;
    match limits.json_nesting_depth_max {
        Some(limit) if depth <= limit => Ok(depth),
        configured => Err(LimitFailure {
            kind: ResourceKind::JsonNestingDepthMax,
            configured,
            observed: Some(depth),
        }),
    }
}

fn allocation_bytes<T>(capacity: usize, limits: &ValidationLimits) -> Result<u64, LimitFailure> {
    let capacity = u64::try_from(capacity).map_err(|_| LimitFailure {
        kind: ResourceKind::LargestContiguousAllocationBytesMax,
        configured: limits.largest_contiguous_allocation_bytes_max,
        observed: None,
    })?;
    let item_size = u64::try_from(size_of::<T>()).map_err(|_| LimitFailure {
        kind: ResourceKind::LargestContiguousAllocationBytesMax,
        configured: limits.largest_contiguous_allocation_bytes_max,
        observed: None,
    })?;
    capacity.checked_mul(item_size).ok_or(LimitFailure {
        kind: ResourceKind::LargestContiguousAllocationBytesMax,
        configured: limits.largest_contiguous_allocation_bytes_max,
        observed: None,
    })
}

fn btree_allocation_bytes<K, V>(
    length: usize,
    limits: &ValidationLimits,
) -> Result<(u64, u64), LimitFailure> {
    if length == 0 {
        return Ok((0, 0));
    }
    // alloc::collections::BTreeMap stores eleven key/value slots per node.
    // Non-root nodes retain at least five keys, so this node-count envelope is
    // conservative even after deletions. The internal-node edge envelope also
    // covers parent links, length/index metadata, and allocator alignment.
    const NODE_CAPACITY: u64 = 11;
    const MIN_KEYS: u64 = 5;
    let length = u64::try_from(length).map_err(|_| LimitFailure {
        kind: ResourceKind::RetainedSourceBytesPerOwnerMax,
        configured: limits.retained_source_bytes_per_owner_max,
        observed: None,
    })?;
    let nodes = 1_u64
        .checked_add(length.saturating_sub(1).div_ceil(MIN_KEYS))
        .ok_or(LimitFailure {
            kind: ResourceKind::RetainedSourceBytesPerOwnerMax,
            configured: limits.retained_source_bytes_per_owner_max,
            observed: None,
        })?;
    let pair = u64::try_from(size_of::<K>())
        .ok()
        .and_then(|key| {
            u64::try_from(size_of::<V>())
                .ok()
                .and_then(|value| key.checked_add(value))
        })
        .ok_or(LimitFailure {
            kind: ResourceKind::RetainedSourceBytesPerOwnerMax,
            configured: limits.retained_source_bytes_per_owner_max,
            observed: None,
        })?;
    let pointer_overhead = u64::try_from(size_of::<usize>())
        .ok()
        .and_then(|pointer| pointer.checked_mul(16))
        .and_then(|bytes| bytes.checked_add(16))
        .ok_or(LimitFailure {
            kind: ResourceKind::RetainedSourceBytesPerOwnerMax,
            configured: limits.retained_source_bytes_per_owner_max,
            observed: None,
        })?;
    let node_bytes = pair
        .checked_mul(NODE_CAPACITY)
        .and_then(|bytes| bytes.checked_add(pointer_overhead))
        .ok_or(LimitFailure {
            kind: ResourceKind::RetainedSourceBytesPerOwnerMax,
            configured: limits.retained_source_bytes_per_owner_max,
            observed: None,
        })?;
    let total = nodes.checked_mul(node_bytes).ok_or(LimitFailure {
        kind: ResourceKind::RetainedSourceBytesPerOwnerMax,
        configured: limits.retained_source_bytes_per_owner_max,
        observed: None,
    })?;
    Ok((total, node_bytes))
}

fn usize_to_u64(
    value: usize,
    kind: ResourceKind,
    configured: Option<u64>,
) -> Result<u64, LimitFailure> {
    u64::try_from(value).map_err(|_| LimitFailure {
        kind,
        configured,
        observed: None,
    })
}

// The cursor owns an immutably boxed Thing for the entire lifetime of every
// erased reference below. No public or private path mutates the tree while a
// task exists, tasks are declared before the box so they drop first, and every
// terminal path explicitly clears them before moving the Thing out.
fn static_ref<T: 'static + ?Sized>(value: &T) -> &'static T {
    // SAFETY: upheld by `ValidatedThingCursor`'s ownership/drop invariant above.
    unsafe { &*(value as *const T) }
}

fn static_str(value: &str) -> &'static str {
    // SAFETY: the str is retained by the cursor-owned Thing.
    unsafe { &*(value as *const str) }
}

fn static_bytes(value: &[u8]) -> &'static [u8] {
    // SAFETY: the bytes are retained by the cursor-owned Thing.
    unsafe { &*(value as *const [u8]) }
}

fn static_slice_iter<T: 'static>(values: &[T]) -> slice::Iter<'static, T> {
    static_ref(values).iter()
}

fn static_btree_iter<K: 'static, V: 'static>(
    values: &BTreeMap<K, V>,
) -> btree_map::Iter<'static, K, V> {
    static_ref(values).iter()
}

fn static_json_iter(values: &serde_json::Map<String, Value>) -> serde_json::map::Iter<'static> {
    static_ref(values).iter()
}

#[cfg(test)]
mod tests {
    use clinkz_wot_foundation::{GatewayDefaultV1, StaticResourceProfile};
    use serde_json::json;

    use super::*;

    fn unit_budget() -> WorkBudget {
        WorkBudget::new()
            .with_remaining(WorkClass::DocumentNodes, 1)
            .with_remaining(WorkClass::JsonSchemaNodes, 1)
            .with_remaining(WorkClass::UriBytes, 1)
            .with_remaining(WorkClass::SecurityBranches, 1)
    }

    fn cursor_at_basic(thing: Thing) -> ValidatedThingCursor {
        let mut cursor = ValidatedThingCursor::new(thing, GatewayDefaultV1::limits());
        for _ in 0..4096 {
            let mut budget = unit_budget();
            cursor = match cursor.step(&mut budget, false) {
                ValidatedThingStep::Pending(cursor) => cursor,
                _ => panic!("census must pause before the Basic bulk phase"),
            };
            if matches!(cursor.phase, Phase::Basic) {
                return cursor;
            }
        }
        panic!("census did not reach the bounded Basic phase");
    }

    #[test]
    fn basic_bulk_charge_is_atomic_and_cancellable_at_its_boundary() {
        let thing = Thing::builder("bulk cancellation").nosec().build().unwrap();
        let title_pointer = thing._metadata.title.as_ref().unwrap().as_ptr();
        let cursor = cursor_at_basic(thing);
        let charges = cursor.census.semantic;
        assert!(charges.document_nodes > 1);

        let mut insufficient = WorkBudget::new()
            .with_remaining(WorkClass::DocumentNodes, charges.document_nodes - 1)
            .with_remaining(WorkClass::JsonSchemaNodes, charges.json_schema_nodes)
            .with_remaining(WorkClass::UriBytes, charges.uri_bytes)
            .with_remaining(WorkClass::SecurityBranches, charges.security_branches);
        let before = [
            insufficient.remaining(WorkClass::DocumentNodes),
            insufficient.remaining(WorkClass::JsonSchemaNodes),
            insufficient.remaining(WorkClass::UriBytes),
            insufficient.remaining(WorkClass::SecurityBranches),
        ];
        let cursor = match cursor.step(&mut insufficient, false) {
            ValidatedThingStep::Pending(cursor) => cursor,
            _ => panic!("incomplete whole-pass charge must remain pending"),
        };
        assert_eq!(
            before,
            [
                insufficient.remaining(WorkClass::DocumentNodes),
                insufficient.remaining(WorkClass::JsonSchemaNodes),
                insufficient.remaining(WorkClass::UriBytes),
                insufficient.remaining(WorkClass::SecurityBranches),
            ]
        );

        let mut zero = WorkBudget::new();
        match cursor.step(&mut zero, true) {
            ValidatedThingStep::Cancelled(thing) => assert_eq!(
                thing._metadata.title.as_ref().unwrap().as_ptr(),
                title_pointer
            ),
            _ => panic!("cancellation must win immediately before Basic validation"),
        }
    }

    #[test]
    fn exact_complete_basic_charge_runs_the_single_semantic_pass() {
        let cursor = cursor_at_basic(Thing::builder("exact bulk charge").nosec().build().unwrap());
        let charges = cursor.census.semantic;
        let mut exact = WorkBudget::new()
            .with_remaining(WorkClass::DocumentNodes, charges.document_nodes)
            .with_remaining(WorkClass::JsonSchemaNodes, charges.json_schema_nodes)
            .with_remaining(WorkClass::UriBytes, charges.uri_bytes)
            .with_remaining(WorkClass::SecurityBranches, charges.security_branches);
        assert!(matches!(
            cursor.step(&mut exact, false),
            ValidatedThingStep::Complete(_)
        ));
        assert_eq!(exact.remaining(WorkClass::DocumentNodes), 0);
        assert_eq!(exact.remaining(WorkClass::JsonSchemaNodes), 0);
        assert_eq!(exact.remaining(WorkClass::UriBytes), 0);
        assert_eq!(exact.remaining(WorkClass::SecurityBranches), 0);
    }

    #[test]
    fn cumulative_resource_and_lifetime_exact_boundaries_complete() {
        let thing = Thing::builder("cumulative boundaries")
            .nosec()
            .schema_definition(
                "schema",
                DataSchema::object().property("child", DataSchema::string()),
            )
            .extra_field("nested", json!([{"member": [1, 2]}]))
            .build()
            .unwrap();
        let measured = cursor_at_basic(thing);
        let document_work = measured
            .document_work_used
            .checked_add(measured.census.semantic.document_nodes)
            .unwrap();
        let limits = GatewayDefaultV1::limits()
            .clone()
            .with_limit(
                ResourceKind::DocumentBytesMax,
                Some(measured.census.retained_bytes),
            )
            .with_limit(
                ResourceKind::RetainedSourceBytesPerOwnerMax,
                Some(measured.census.retained_bytes),
            )
            .with_limit(
                ResourceKind::StringBytesMax,
                Some(measured.census.string_bytes),
            )
            .with_limit(
                ResourceKind::ExtensionBytesMax,
                Some(measured.census.extension_bytes),
            )
            .with_limit(
                ResourceKind::JsonValueNodesPerDocumentMax,
                Some(measured.census.json_nodes),
            )
            .with_limit(
                ResourceKind::SchemaNodesPerDocumentMax,
                Some(measured.census.schema_nodes),
            )
            .with_limit(
                ResourceKind::SchemaReferenceEdgesPerDocumentMax,
                Some(measured.census.schema_edges),
            )
            .with_limit(
                ResourceKind::DocumentValidationWorkUnitsMax,
                Some(document_work),
            );
        let mut zero = WorkBudget::new();
        let thing = match measured.step(&mut zero, true) {
            ValidatedThingStep::Cancelled(thing) => thing,
            _ => unreachable!(),
        };

        let mut cursor = ValidatedThingCursor::new(thing, &limits);
        loop {
            let mut budget = WorkBudget::new()
                .with_remaining(WorkClass::DocumentNodes, u64::MAX)
                .with_remaining(WorkClass::JsonSchemaNodes, u64::MAX)
                .with_remaining(WorkClass::UriBytes, u64::MAX)
                .with_remaining(WorkClass::SecurityBranches, u64::MAX);
            match cursor.step(&mut budget, false) {
                ValidatedThingStep::Pending(next) => cursor = next,
                ValidatedThingStep::Complete(_) => break,
                ValidatedThingStep::Limit {
                    kind,
                    configured,
                    observed,
                    ..
                } => panic!(
                    "exact cumulative boundary failed for {kind:?}: {observed:?} > {configured:?}"
                ),
                ValidatedThingStep::Invalid { error, .. } => {
                    panic!("exact cumulative fixture must remain Basic-valid: {error:?}")
                }
                ValidatedThingStep::Cancelled(_) => unreachable!(),
            }
        }
    }
}
