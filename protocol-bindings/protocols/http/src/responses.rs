//! HTTP's bounded native discriminator. Core never interprets these codes.
use crate::*;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct HttpResponses {
    codes: [u16; 33],
    count: u8,
    successes: u64,
    empty: u64,
    utf8_media: u64,
}
/// Normalized native response facts; no source schema, credential or authority.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct HttpResponseInput {
    code: u16,
    success: bool,
    has_data: bool,
    utf8_media: bool,
}
impl Default for HttpResponses {
    fn default() -> Self {
        Self::primary(false)
    }
}
impl HttpResponses {
    pub(super) fn primary(write: bool) -> Self {
        let mut codes = [0; 33];
        codes[0] = if write { 204 } else { 200 };
        Self {
            codes,
            count: 1,
            successes: 1,
            empty: u64::from(write),
            utf8_media: 0,
        }
    }
    pub(super) fn classify(
        &self,
        code: u16,
    ) -> CoreResult<(ResponseSelection, InteractionStatus, bool)> {
        let i = self.codes[..usize::from(self.count)]
            .iter()
            .position(|c| *c == code)
            .ok_or_else(|| error(606, "undeclared HTTP response"))?;
        Ok((
            if i == 0 {
                ResponseSelection::Primary
            } else {
                ResponseSelection::Additional((i - 1) as u16)
            },
            if self.successes & (1u64 << i) != 0 {
                InteractionStatus::Ok
            } else {
                InteractionStatus::ApplicationError
            },
            self.empty & (1u64 << i) != 0,
        ))
    }
    pub(super) fn code(&self, selection: ResponseSelection) -> CoreResult<u16> {
        let i = match selection {
            ResponseSelection::Primary => 0,
            ResponseSelection::Additional(i) => usize::from(i) + 1,
            _ => return Err(error(606, "unsupported HTTP response branch")),
        };
        self.codes
            .get(i)
            .copied()
            .filter(|c| *c != 0)
            .ok_or_else(|| error(606, "unsupported HTTP response branch"))
    }
    pub(super) fn media(&self, selection: ResponseSelection) -> CoreResult<&'static str> {
        self.code(selection)?;
        let i = match selection {
            ResponseSelection::Primary => 0,
            ResponseSelection::Additional(i) => u32::from(i) + 1,
            _ => return Err(error(606, "unsupported HTTP response branch")),
        };
        Ok(if self.utf8_media & (1u64 << i) != 0 {
            "application/json; charset=utf-8"
        } else {
            "application/json"
        })
    }
    #[cfg(feature = "prepare")]
    pub(super) fn compile(
        doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
        owner: clinkz_wot_td::semantic::FormOwner<'_>,
        form: u32,
        operation: Operation,
    ) -> CoreResult<Self> {
        Ok(Self::lower(&Self::extract(doc, owner, form, operation)?))
    }
    #[cfg(feature = "prepare")]
    pub(super) fn lower(input: &[HttpResponseInput]) -> Self {
        let mut result = Self {
            codes: [0; 33],
            count: input.len() as u8,
            successes: 0,
            empty: 0,
            utf8_media: 0,
        };
        for (i, response) in input.iter().enumerate() {
            result.codes[i] = response.code;
            result.successes |= u64::from(response.success) << i;
            result.empty |= u64::from(!response.has_data) << i;
            result.utf8_media |= u64::from(response.utf8_media) << i;
        }
        result
    }
    #[cfg(feature = "prepare")]
    pub(super) fn extract(
        doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
        owner: clinkz_wot_td::semantic::FormOwner<'_>,
        form: u32,
        operation: Operation,
    ) -> CoreResult<Vec<HttpResponseInput>> {
        use clinkz_wot_td::semantic::{FormOwner, SemanticScope, SemanticValue};
        let facts = doc
            .form(owner, form)
            .ok_or_else(crate::compiler::unsupported)?;
        let write = matches!(
            operation,
            Operation::WriteProperty
                | Operation::WriteAllProperties
                | Operation::WriteMultipleProperties
        );
        let additional: Vec<_> = doc
            .additional_responses(owner, form)
            .ok_or_else(crate::compiler::unsupported)?
            .collect();
        if additional.len() > 32
            || !additional.is_empty() && !matches!(owner, FormOwner::Property(_))
        {
            return Err(crate::compiler::unsupported());
        }
        let mut result: Vec<HttpResponseInput> = Vec::new();
        result
            .try_reserve_exact(additional.len() + 1)
            .map_err(|_| crate::compiler::unsupported())?;
        for i in 0..=additional.len() {
            let (media, success, has_data, scope) = if i == 0 {
                (
                    facts.response_content_type.unwrap_or(facts.content_type),
                    true,
                    !write,
                    match owner {
                        FormOwner::Property(property) => SemanticScope::Response {
                            property,
                            index: form,
                        },
                        _ => SemanticScope::AffordanceResponse { owner, index: form },
                    },
                )
            } else {
                let r = &additional[i - 1];
                (
                    r.content_type,
                    r.success,
                    r.schema.is_some() || !write,
                    match owner {
                        FormOwner::Property(property) => SemanticScope::AdditionalResponse {
                            property,
                            form,
                            index: (i - 1) as u32,
                        },
                        _ => SemanticScope::AffordanceAdditionalResponse {
                            owner,
                            form,
                            index: (i - 1) as u32,
                        },
                    },
                )
            };
            if !json_media(media)
                || !matches!(owner, FormOwner::Property(_)) && media != "application/json"
            {
                return Err(crate::compiler::unsupported());
            }
            let mut code = if i == 0 {
                Some(if write { 204 } else { 200 })
            } else {
                None
            };
            if let Some(scope) = doc.scope(scope) {
                let mut seen = false;
                for node in scope.children() {
                    if node
                        .name()
                        .is("http://www.w3.org/2011/http#statusCodeValue")
                        && !seen
                    {
                        seen = true;
                        code = match node.value() {
                            SemanticValue::Number(n) => {
                                n.as_u64().and_then(|n| u16::try_from(n).ok())
                            }
                            _ => None,
                        };
                    } else if !crate::producer_compiler::class_assertion(node) {
                        return Err(crate::compiler::unsupported());
                    }
                }
            }
            let code = code.ok_or_else(crate::compiler::unsupported)?;
            // This concrete framing profile supports JSON bodies and strict
            // empty 204 through EOF. Success meaning comes only from the TD.
            if !(200..=599).contains(&code)
                || code == 202
                || has_data && matches!(code, 204 | 205 | 304)
                || !has_data && code != 204
                || result.iter().any(|r| r.code == code)
            {
                return Err(crate::compiler::unsupported());
            }
            if i == 0 && code != if write { 204 } else { 200 } {
                return Err(crate::compiler::unsupported());
            }
            result.push(HttpResponseInput {
                code,
                success,
                has_data,
                utf8_media: media == "application/json; charset=utf-8",
            });
        }
        Ok(result)
    }
}

/// The supported JSON representation parameters are explicit, not an arbitrary
/// structured-suffix or charset promise. Core only sees the prepared media.
pub(super) fn json_media(media: &str) -> bool {
    matches!(
        media,
        "application/json" | "application/json; charset=utf-8"
    )
}
pub(super) fn validate_payload(
    schema: &clinkz_wot_core::data_schema::PreparedSchema,
    payload: &Payload,
) -> CoreResult<()> {
    if !json_media(&payload.content_type) || payload.content_coding.is_some() {
        return Err(CoreError::Validation(ErrorContext::new(
            ErrorPhase::Validate,
            RetryClass::Never,
        )));
    }
    crate::producer_validation::validate_json(schema, &payload.body)
}
pub(super) struct HttpResponse {
    pub code: u16,
    pub selection: ResponseSelection,
    pub status: InteractionStatus,
    pub empty: bool,
    pub media: &'static str,
    pub body: Vec<u8>,
}
