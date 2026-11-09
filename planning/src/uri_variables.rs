//! Shared TD scope/schema interpretation. Native address placement is not
//! interpreted here; complete effective declarations are qualified even unused.
use crate::type_schema::{self, SchemaPreparation};
use clinkz_wot_core::{CoreResult, uri_variables::*};
use clinkz_wot_td::semantic::ResolvedSemanticDocument;

pub(crate) fn preparation(
    doc: &ResolvedSemanticDocument<'_>,
    target: &str,
) -> CoreResult<SchemaPreparation> {
    let template = UriTemplate::parse(target)?;
    let mut result = SchemaPreparation::default();
    for (i, (name, schema)) in doc.effective_uri_variables().enumerate() {
        if i >= MAX_VARIABLES || !variable_name(name) {
            return Err(crate::open_consumer::unsupported());
        }
        // Scalar support is an explicit execution subset, not a TD validity
        // restriction. Arrays/objects/untyped/numeric facets are never ignored.
        if !matches!(
            schema.facts().declared_type,
            Some("string" | "integer" | "boolean")
        ) {
            return Err(crate::open_consumer::unsupported());
        }
        result.include(schema)?;
        result.cleanup += 1;
        result.backing_bytes += core::mem::size_of::<UriVariable>() + name.len();
        result.work += (target.len() + name.len() + MAX_VARIABLES * MAX_NAME_BYTES) as u64;
    }
    if template
        .references()
        .any(|name| !doc.effective_uri_variables().any(|(n, _)| n == name))
    {
        return Err(crate::open_consumer::unsupported());
    }
    result.work += (target.len() * (MAX_VARIABLES + 1) + 1) as u64;
    Ok(result)
}
pub(crate) fn prepare(
    doc: &ResolvedSemanticDocument<'_>,
    target: &str,
) -> CoreResult<PreparedUriVariables> {
    let template = UriTemplate::parse(target)?;
    let mut declarations = alloc::vec::Vec::new();
    let count = doc.effective_uri_variables().count();
    if count > MAX_VARIABLES {
        return Err(crate::open_consumer::unsupported());
    }
    declarations
        .try_reserve_exact(count)
        .map_err(|_| clinkz_wot_core::consumer::busy())?;
    for (name, schema) in doc.effective_uri_variables() {
        declarations.push(UriVariable {
            name: name.into(),
            schema: type_schema::prepare_schema(schema)?,
            referenced: template.references().any(|n| n == name),
        });
    }
    PreparedUriVariables::try_new(declarations)
}
pub(crate) fn matches(
    doc: &ResolvedSemanticDocument<'_>,
    target: &str,
    expected: &PreparedUriVariables,
) -> CoreResult<bool> {
    let template = UriTemplate::parse(target)?;
    if doc.effective_uri_variables().count() != expected.declarations().len() {
        return Ok(false);
    }
    for (name, schema) in doc.effective_uri_variables() {
        let Some(v) = expected
            .declarations()
            .iter()
            .find(|v| v.name.as_ref() == name)
        else {
            return Ok(false);
        };
        if v.referenced != template.references().any(|n| n == name)
            || !type_schema::matches_schema(schema, &v.schema)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}
