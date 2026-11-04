//! Pure preparation over the existing Host-only resolved semantic loan.
//! This implements the explicitly type-only DataSchema subset.
use clinkz_wot_core::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, RetryClass,
    value_type::{TypeSchema, ValueType},
};
use clinkz_wot_td::semantic::{
    SchemaConstraints, SchemaView, SemanticName, SemanticNode, SemanticValue,
};

// Borrowed census lets callers prepay construction and destruction of each
// role-specific schema, including partial-construction paths.
#[derive(Clone, Copy)]
enum Members {
    Readable,
    Writable,
}
impl Members {
    fn includes(self, schema: SchemaView<'_>) -> bool {
        let facts = schema.facts();
        match self {
            Self::Readable => !facts.write_only,
            Self::Writable => !facts.read_only,
        }
    }
}
fn member_count(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
    role: Members,
) -> CoreResult<usize> {
    let count = doc
        .collection_properties()
        .ok_or_else(refused)?
        .filter(|p| role.includes(p.schema))
        .count();
    if count > clinkz_wot_core::collection::PropertyCollectionSchema::MAX_MEMBERS {
        return Err(refused());
    }
    Ok(count)
}
pub(crate) fn readable_member_count(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<usize> {
    member_count(doc, Members::Readable)
}
pub(crate) fn writable_member_count(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<usize> {
    member_count(doc, Members::Writable)
}
/// TD 1.1: every non-writeOnly Property and its complete supported schema.
/// No individual Form, behavior or native read is consulted.
pub fn prepare_readable_property_schema(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<clinkz_wot_core::collection::PropertyCollectionSchema> {
    prepare_members(doc, Members::Readable)
}
/// TD 1.1: every non-readOnly Property, including writeOnly members. The
/// invocation's all/selected scope is separate from this immutable eligibility.
pub fn prepare_writable_property_schema(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<clinkz_wot_core::collection::PropertyCollectionSchema> {
    prepare_members(doc, Members::Writable)
}
fn prepare_members(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
    role: Members,
) -> CoreResult<clinkz_wot_core::collection::PropertyCollectionSchema> {
    use clinkz_wot_core::collection::{PropertyCollectionSchema as S, PropertyMemberSchema};
    let mut members = alloc::vec::Vec::new();
    members
        .try_reserve_exact(member_count(doc, role)?)
        .map_err(|_| clinkz_wot_core::consumer::busy())?;
    let mut bytes = 0;
    for p in doc.collection_properties().ok_or_else(refused)? {
        if !role.includes(p.schema) {
            continue;
        }
        bytes += p.name.len();
        if members.len() == S::MAX_MEMBERS
            || p.name.len() > S::MAX_NAME_BYTES
            || bytes > S::MAX_TOTAL_NAME_BYTES
        {
            return Err(refused());
        }
        let schema = prepare_type_schema(p.schema)?;
        members.push(PropertyMemberSchema {
            name: p.name.into(),
            schema,
        });
    }
    S::try_new(members)
}

/// Capture all executable assertions or refuse. Metadata, default, unit and
/// read/write annotations impose no value constraint. No schema tree, literal,
/// source loan or format implementation survives this fixed-size result.
pub fn prepare_type_schema(schema: SchemaView<'_>) -> CoreResult<TypeSchema> {
    let facts = schema.facts();
    let unsupported = facts.constant.is_some()
        || facts.enumeration.is_some()
        || facts.format.is_some()
        || schema.one_of().is_some()
        || schema.properties().is_some()
        || schema.items().is_some()
        || !schema.extension_semantics().all(annotation)
        || match facts.constraints {
            SchemaConstraints::Array {
                min_items,
                max_items,
            } => min_items.is_some() || max_items.is_some(),
            SchemaConstraints::Object { required } => required.is_some(),
            SchemaConstraints::Number {
                minimum,
                exclusive_minimum,
                maximum,
                exclusive_maximum,
                multiple_of,
            } => [
                minimum,
                exclusive_minimum,
                maximum,
                exclusive_maximum,
                multiple_of,
            ]
            .iter()
            .any(Option::is_some),
            SchemaConstraints::Integer {
                minimum,
                exclusive_minimum,
                maximum,
                exclusive_maximum,
                multiple_of,
            } => [
                minimum,
                exclusive_minimum,
                maximum,
                exclusive_maximum,
                multiple_of,
            ]
            .iter()
            .any(Option::is_some),
            SchemaConstraints::String {
                min_length,
                max_length,
                pattern,
                content_encoding,
                content_media_type,
            } => {
                min_length.is_some()
                    || max_length.is_some()
                    || pattern.is_some()
                    || content_encoding.is_some()
                    || content_media_type.is_some()
            }
            SchemaConstraints::Boolean | SchemaConstraints::Null => false,
        };
    if unsupported {
        return Err(refused());
    }
    Ok(match facts.declared_type {
        None => TypeSchema::Any,
        Some(name) => TypeSchema::Type(match name {
            "null" => ValueType::Null,
            "boolean" => ValueType::Boolean,
            "number" => ValueType::Number,
            "integer" => ValueType::Integer,
            "string" => ValueType::String,
            "object" => ValueType::Object,
            "array" => ValueType::Array,
            _ => return Err(refused()),
        }),
    })
}

fn annotation(node: SemanticNode<'_>) -> bool {
    node.name() == SemanticName::Keyword("@type")
        && (matches!(node.value(), SemanticValue::Iri(_))
            || node.value() == SemanticValue::Array
                && node
                    .children()
                    .all(|n| matches!(n.value(), SemanticValue::Iri(_))))
}

fn refused() -> CoreError {
    CoreError::UnsupportedOperation(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
            .with_redacted_cause(624, "unsupported type-only schema assertion"),
    )
}

/// Action definitions are not status records. Retain each output role and
/// reject duplicate expanded role assertions before selecting a status codec.
pub fn prepare_action_collection_schema(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<clinkz_wot_core::action::ActionCollectionSchema> {
    use clinkz_wot_core::action::{ActionCollectionSchema as S, ActionMemberSchema};
    use clinkz_wot_td::semantic::{FormOwner, SemanticScope};
    let mut members = alloc::vec::Vec::new();
    members
        .try_reserve_exact(action_member_count(doc)?)
        .map_err(|_| clinkz_wot_core::consumer::busy())?;
    let mut bytes = 0;
    for name in doc.collection_actions().ok_or_else(refused)? {
        bytes += name.len();
        if name.len() > 256 || bytes > S::MAX_TOTAL_NAME_BYTES {
            return Err(refused());
        }
        let output = doc.affordance_schema(FormOwner::Action(name), "output");
        let scope = doc.scope(SemanticScope::Action(name)).ok_or_else(refused)?;
        // Input has no data role in a query. Its actual schema need not be
        // executable by this Consumer, but cannot masquerade as another output.
        if scope
            .children()
            .filter(|n| n.name().is("https://www.w3.org/2019/wot/td#output"))
            .count()
            != usize::from(output.is_some())
            || scope.children().any(|n| {
                !n.name().is("https://www.w3.org/2019/wot/td#input")
                    && !n.name().is("https://www.w3.org/2019/wot/td#output")
                    && !annotation(n)
            })
        {
            return Err(refused());
        }
        members.push(ActionMemberSchema {
            name: name.into(),
            output: output.map(prepare_type_schema).transpose()?,
        });
    }
    S::try_new(members)
}
pub(crate) fn action_member_count(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<usize> {
    let count = doc.collection_actions().ok_or_else(refused)?.count();
    if count > clinkz_wot_core::action::ActionCollectionSchema::MAX_MEMBERS {
        return Err(refused());
    }
    Ok(count)
}
