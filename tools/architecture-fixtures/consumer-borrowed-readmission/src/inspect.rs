//! Per-occurrence typed-content oracle. This module receives an already paid
//! field action; it neither serializes nor creates native iterators or lookups.
use super::*;
#[derive(Clone, Copy, Default)]
pub(super) struct Scope {
    pub depth: u64,
    pub schema_depth: u64,
    pub extension: bool,
    pub variables: bool,
}
pub(super) fn container(task: Task<'_>) -> bool {
    !matches!(
        task,
        Task::Text(..)
            | Task::Scalar(..)
            | Task::Value(Value::Null | Value::Bool(_) | Value::String(_) | Value::Number(_))
            | Task::Entry(..)
    )
}
pub(super) fn array(task: Task<'_>) -> bool {
    matches!(
        task,
        Task::Context(_)
            | Task::Strings(_)
            | Task::Schemas(_)
            | Task::Values(_)
            | Task::Forms(_)
            | Task::Links(_)
            | Task::Additionals(_)
            | Task::Uris(_)
            | Task::Operations(_)
            | Task::Value(Value::Array(_))
    )
}
pub(super) fn map(task: Task<'_>) -> bool {
    matches!(task, Task::Map(_) | Task::Value(Value::Object(_)))
}
pub(super) fn child_scope(parent: Task<'_>, index: usize, mut scope: Scope) -> Scope {
    // One owning typed field program, including flattened schema metadata.
    let extra = match parent {
        Task::Thing(_) => index == 18,
        Task::SchemaContext(_) => matches!(index, 1 | 2 | 5 | 10),
        Task::Action(_) => index == 7,
        Task::Event(_) => index == 6,
        Task::Form(_) => index == 9,
        Task::Link(_) => index == 6,
        Task::Version(_) => index == 2,
        Task::Response(_) => index == 1,
        Task::Additional(_) => index == 3,
        Task::SecurityContext(_) => index == 5,
        _ => false,
    };
    scope.extension |= extra;
    scope.variables = match parent {
        Task::Thing(_) => index == 17,
        Task::Interaction(_) => index == 1,
        _ => scope.variables,
    };
    if matches!(parent, Task::Map(_)) {
        scope.variables = false;
    }
    scope
}
pub(super) fn limit(policy: Policy, kind: R, observed: u64) -> Result<(), Cause> {
    let configured = policy.get(kind);
    if observed > configured {
        Err(Cause::Resource {
            kind,
            configured,
            observed,
        })
    } else {
        Ok(())
    }
}
pub(super) fn add(value: &mut u64, n: u64) -> Result<(), Cause> {
    *value = value.checked_add(n).ok_or(Cause::Arithmetic)?;
    Ok(())
}
pub(super) fn visit(
    task: Task<'_>,
    mut scope: Scope,
    d: &t::Description<'_>,
    trace: &mut Trace,
    policy: Policy,
) -> Result<Scope, Cause> {
    trace.nodes = trace.nodes.checked_add(1).ok_or(Cause::Arithmetic)?;
    limit(policy, R::JsonValueNodesPerDocumentMax, trace.nodes as u64)?;
    trace.kinds[d.kind as usize] += 1;
    if container(task) {
        scope.depth += 1;
        trace.container_depth = trace.container_depth.max(scope.depth);
        limit(policy, R::JsonNestingDepthMax, scope.depth)?;
    }
    if array(task) {
        trace.array_items = trace.array_items.max(d.count as u64);
        limit(policy, R::JsonArrayItemsMax, d.count as u64)?;
    }
    if let Task::Map(m) = task {
        if matches!(m, Map::Properties(_) | Map::Actions(_) | Map::Events(_)) {
            add(&mut trace.affordances, d.count as u64)?;
            limit(policy, R::AffordancesPerThingMax, trace.affordances)?;
        }
        if scope.variables {
            trace.variables = trace.variables.max(d.count as u64);
            limit(policy, R::UriVariablesPerFormMax, d.count as u64)?;
        }
    }
    if let Task::Forms(v) = task {
        trace.forms_context = trace.forms_context.max(v.len() as u64);
        add(&mut trace.forms, v.len() as u64)?;
        limit(policy, R::FormsPerContextMax, v.len() as u64)?;
        limit(policy, R::FormsPerThingMax, trace.forms)?;
    }
    if let Task::Additionals(v) = task {
        trace.responses = trace.responses.max(v.len() as u64);
        limit(policy, R::AdditionalResponsesPerFormMax, v.len() as u64)?;
    }
    if let Task::Additional(v) = task {
        if v.schema.is_some() {
            add(&mut trace.schema_edges, 1)?;
            limit(
                policy,
                R::SchemaReferenceEdgesPerDocumentMax,
                trace.schema_edges,
            )?;
        }
    }
    if let Task::Schema(_) = task {
        add(&mut trace.supplied_schemas, 1)?;
        limit(policy, R::SchemaNodesPerDocumentMax, trace.supplied_schemas)?;
        if scope.schema_depth != 0 {
            add(&mut trace.schema_edges, 1)?;
            limit(
                policy,
                R::SchemaReferenceEdgesPerDocumentMax,
                trace.schema_edges,
            )?;
        }
        scope.schema_depth += 1;
        trace.schema_depth = trace.schema_depth.max(scope.schema_depth);
        limit(policy, R::SchemaCompositionDepthMax, scope.schema_depth)?;
    }
    if let Task::Form(v) = task {
        trace.uri_source = trace.uri_source.max(v.href.as_str().len() as u64);
        limit(
            policy,
            R::UriTemplateSourceBytesMax,
            v.href.as_str().len() as u64,
        )?;
    }
    let n = if let Some(text) = d.text {
        text.len() as u64
    } else if d.bits.is_some() {
        8
    } else {
        0
    };
    trace.content = trace
        .content
        .checked_add(n as usize)
        .ok_or(Cause::Arithmetic)?;
    limit(policy, R::DocumentBytesMax, trace.content as u64)?;
    limit(
        policy,
        R::GeneratedEffectiveDocumentBytesMax,
        trace.content as u64,
    )?;
    if d.kind == validated_thing_value_construction_probe::Kind::Number as u32 && d.text.is_some() {
        trace.number_bytes = trace.number_bytes.max(n);
        limit(policy, R::NumberLexemeBytesMax, n)?;
    } else if d.text.is_some() {
        add(&mut trace.text_bytes, n)?;
        limit(policy, R::StringBytesMax, trace.text_bytes)?;
    }
    if scope.extension {
        add(&mut trace.extension_bytes, n)?;
        limit(policy, R::ExtensionBytesMax, trace.extension_bytes)?;
    }
    Ok(scope)
}
