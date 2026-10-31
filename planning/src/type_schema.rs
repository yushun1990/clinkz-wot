//! Pure preparation over the existing Host-only resolved semantic loan.
//! This implements the explicitly type-only DataSchema subset.
use clinkz_wot_core::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, RetryClass,
    value_type::{TypeSchema, ValueType},
};
use clinkz_wot_td::semantic::{
    SchemaConstraints, SchemaView, SemanticName, SemanticNode, SemanticValue,
};

/// TD 1.1 §6.3.9.6: return every readable Property, using its own complete
/// supported schema. No individual Form, behavior or native read is consulted.
pub fn prepare_read_all_schema(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
) -> CoreResult<clinkz_wot_core::collection::PropertyCollectionSchema> {
    use clinkz_wot_core::collection::{PropertyCollectionSchema as S, PropertyMemberSchema};
    let mut members = alloc::vec::Vec::new();
    let mut bytes = 0;
    for p in doc.collection_properties().ok_or_else(refused)? {
        if p.schema.facts().write_only {
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
        members
            .try_reserve_exact(1)
            .map_err(|_| clinkz_wot_core::consumer::busy())?;
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
