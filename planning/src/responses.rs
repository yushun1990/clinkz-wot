//! Common response alternatives. This contains no native status interpretation.
use alloc::vec::Vec;
use clinkz_wot_core::{execution::responses::*, *};
use clinkz_wot_td::semantic::{FormOwner, ResolvedSemanticDocument};

/// Complete extra construction/rollback work; inspect before allocating roles.
pub fn preparation(
    doc: &ResolvedSemanticDocument<'_>,
    owner: FormOwner<'_>,
    form: u32,
) -> CoreResult<(u64, u64)> {
    let mut work = crate::type_schema::SchemaPreparation::default();
    let facts = doc.form(owner, form).ok_or_else(refused)?;
    let primary_media = facts.response_content_type.unwrap_or(facts.content_type);
    if primary_media.is_empty() || primary_media.len() > 256 {
        return Err(refused());
    }
    let alternatives = doc.additional_responses(owner, form).ok_or_else(refused)?;
    for (i, response) in alternatives.enumerate() {
        if i >= FiniteResponses::MAX_ADDITIONAL
            || response.content_type.is_empty()
            || response.content_type.len() > 256
        {
            return Err(refused());
        }
        work.work += response.content_type.len() as u64 + 4;
        work.cleanup += 2;
        if let Some(name) = response.schema {
            work.include(doc.schema_definition(name).map_err(|_| refused())?)?;
        }
    }
    if work.nodes > clinkz_wot_core::data_schema::PreparedSchema::MAX_NODES
        || work.name_bytes > 4096
    {
        return Err(refused());
    }
    Ok((work.work + 260, work.cleanup + 2))
}
/// Construct after the caller has prepaid preparation and rollback and funded
/// temporary backing. This pure value is not a service-support certificate.
pub fn prepare(
    doc: &ResolvedSemanticDocument<'_>,
    owner: FormOwner<'_>,
    form: u32,
) -> CoreResult<FiniteResponses> {
    let facts = doc.form(owner, form).ok_or_else(refused)?;
    let primary = ResponseAlternative {
        content_type: facts
            .response_content_type
            .unwrap_or(facts.content_type)
            .into(),
        success: true,
        data: inherited_data(
            doc,
            owner,
            facts.content_type,
            facts.response_content_type.unwrap_or(facts.content_type),
        )?,
    };
    let mut additional = Vec::new();
    for (i, response) in doc
        .additional_responses(owner, form)
        .ok_or_else(refused)?
        .enumerate()
    {
        if i >= FiniteResponses::MAX_ADDITIONAL {
            return Err(refused());
        }
        additional.push(ResponseAlternative {
            content_type: response.content_type.into(),
            success: response.success,
            data: match response.schema {
                None => inherited_data(doc, owner, facts.content_type, response.content_type)?,
                Some(name) => ResponseData::Value(crate::type_schema::prepare_schema(
                    doc.schema_definition(name).map_err(|_| refused())?,
                )?),
            },
        });
    }
    FiniteResponses::try_new(primary, additional)
}
/// Exact borrowed comparison for checked prepared-content reuse/final TD.
pub fn matches(
    doc: &ResolvedSemanticDocument<'_>,
    owner: FormOwner<'_>,
    form: u32,
    expected: &FiniteResponses,
) -> CoreResult<bool> {
    let facts = doc.form(owner, form).ok_or_else(refused)?;
    let p = expected
        .get(ResponseSelection::Primary)
        .ok_or_else(refused)?;
    if p.content_type.as_ref() != facts.response_content_type.unwrap_or(facts.content_type)
        || !p.success
        || p.data
            != inherited_data(
                doc,
                owner,
                facts.content_type,
                facts.response_content_type.unwrap_or(facts.content_type),
            )?
    {
        return Ok(false);
    }
    let mut actual = doc.additional_responses(owner, form).ok_or_else(refused)?;
    for expected in expected.additional() {
        let Some(r) = actual.next() else {
            return Ok(false);
        };
        if r.content_type != expected.content_type.as_ref() || r.success != expected.success {
            return Ok(false);
        }
        match (r.schema, &expected.data) {
            (None, data)
                if *data == inherited_data(doc, owner, facts.content_type, r.content_type)? =>
            {
                ()
            }
            (Some(n), ResponseData::Value(s))
                if crate::type_schema::matches_schema(
                    doc.schema_definition(n).map_err(|_| refused())?,
                    s,
                )? =>
            {
                ()
            }
            _ => return Ok(false),
        }
    }
    Ok(actual.next().is_none())
}
// Repeating the Form's default media does not turn an absent Action output
// into a required value. TD also permits media-defined results without a
// schema (e.g. takePhoto), but different media alone cannot qualify their data
// role here. Decline that case until a concrete profile can establish it.
fn inherited_data(
    doc: &ResolvedSemanticDocument<'_>,
    owner: FormOwner<'_>,
    form_media: &str,
    response_media: &str,
) -> CoreResult<ResponseData> {
    if matches!(owner, FormOwner::Action(_))
        && doc.affordance_schema(owner, "output").is_none()
        && response_media != form_media
    {
        return Err(refused());
    }
    Ok(ResponseData::Operation)
}
fn refused() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}
