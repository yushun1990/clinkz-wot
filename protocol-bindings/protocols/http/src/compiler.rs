//! Optional HTTP preparation; all executable types are defined by the runtime.
use super::*;
use clinkz_wot_planning::*;
use clinkz_wot_td::semantic::{FormOwner, SemanticNode, SemanticScope, SemanticValue};
const HTTP: &str = "http://www.w3.org/2011/http#";
#[derive(Debug)]
pub(super) struct HttpCompiler(pub HttpConsumerConfig);
impl HostSemanticCompiler for HttpCompiler {
    type Artifact = HttpArtifact;
    // One atomic callback over at most 128 nodes and 1024 header bytes.
    const WORK: u64 = 1;
    fn compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
    }
    fn configuration(&self) -> ConsumerBindingConfiguration {
        self.0.configuration
    }
    fn schemes(&self) -> &'static [&'static str] {
        if self.0.secure { &["https"] } else { &["http"] }
    }
    fn compile(&self, input: ResolvedBindingInput<'_>) -> CoreResult<HttpArtifact> {
        use clinkz_wot_td::data_type::Operation;
        let collection = input.owner == FormOwner::Thing
            && matches!(
                input.operation,
                Operation::ReadAllProperties
                    | Operation::QueryAllActions
                    | Operation::ReadMultipleProperties
                    | Operation::WriteAllProperties
                    | Operation::WriteMultipleProperties
            );
        if !(collection
            || matches!(
                (input.owner, input.operation),
                (
                    FormOwner::Property(_),
                    Operation::ReadProperty | Operation::WriteProperty
                )
            ))
            || (input.operation != Operation::ReadProperty
                && input.content_type != Some("application/json"))
            || input.subprotocol.is_some()
        {
            return Err(unsupported());
        }
        let target =
            crate::address::qualify(input.target, self.0.secure).map_err(|_| unsupported())?;
        if input.target.contains('{')
            && !matches!(
                input.operation,
                Operation::ReadProperty | Operation::WriteProperty
            )
        {
            return Err(unsupported());
        }
        let requirement = clinkz_wot_planning::security::compilation_security(&input)?;
        let authentication = crate::security::HttpSecurity::from_requirement(&requirement)
            .map_err(|_| unsupported())?;
        if authentication.selected().is_none() {
            return Err(unsupported());
        }

        if self.0.secure {
            if !matches!(
                input.operation,
                Operation::ReadProperty | Operation::WriteProperty
            ) || !matches!(input.owner, FormOwner::Property(_))
                || target.uri.scheme_str() != Some("https")
            {
                return Err(unsupported());
            }
        } else if target.uri.scheme_str() != Some("http")
            || authentication != crate::security::HttpSecurity::default()
        {
            return Err(unsupported());
        }

        let facts = input
            .document
            .form(input.owner, input.form_index)
            .ok_or_else(unsupported)?;
        let write = matches!(
            input.operation,
            Operation::WriteProperty
                | Operation::WriteAllProperties
                | Operation::WriteMultipleProperties
        );
        let multiple = input.operation == Operation::ReadMultipleProperties;
        let actions = input.operation == Operation::QueryAllActions;
        if (multiple
            || actions
            || matches!(
                input.operation,
                Operation::WriteAllProperties | Operation::WriteMultipleProperties
            ))
            && input.document.scope(SemanticScope::Thing).is_none_or(|n| {
                n.children()
                    .any(|n| !crate::producer_compiler::class_assertion(n))
            })
        {
            return Err(unsupported());
        }
        if !facts.effective_operations.contains(&input.operation)
            || (!collection && {
                let flags = input.document.property().schema.facts();
                (write && flags.read_only) || (!write && flags.write_only)
            })
        {
            return Err(unsupported());
        }
        if facts.content_coding.is_some() {
            return Err(unsupported());
        }
        let responses = crate::responses::HttpResponses::compile(
            input.document,
            input.owner,
            input.form_index,
            input.operation,
        )?;
        let form = input
            .document
            .scope(if collection {
                SemanticScope::AffordanceForm {
                    owner: FormOwner::Thing,
                    index: input.form_index,
                }
            } else {
                SemanticScope::Form {
                    property: input.property,
                    index: input.form_index,
                }
            })
            .ok_or_else(unsupported)?;
        let mut output = HttpArtifact {
            operation: input.operation,
            responses,
            secure: self.0.secure,
            authentication,
            method: if write { "PUT" } else { "GET" },
            target_len: input.target.len() as u16,
            ..HttpArtifact::default()
        };
        output.target[..input.target.len()].copy_from_slice(input.target.as_bytes());
        // Only absence enables the protocol default. Malformed/unsupported
        // explicit instructions never silently turn into a default request.
        if let Some(method) = unique(form, "http://www.w3.org/2011/http#methodName")? {
            // TD 1.1 §8.3.1: a multi-operation Form uses operation defaults,
            // never one explicit method shared across the operation set.
            if facts.declared_operations.is_some_and(|ops| ops.len() > 1) {
                return Err(unsupported());
            }
            output.method = match method.value() {
                SemanticValue::Text("GET") => "GET",
                SemanticValue::Text("POST") => "POST",
                SemanticValue::Text("PUT") => "PUT",
                SemanticValue::Text("PATCH") => "PATCH",
                SemanticValue::Text("DELETE") => "DELETE",
                _ => return Err(unsupported()),
            };
            if write && output.method != "PUT" {
                return Err(unsupported());
            }
            if collection
                && output.method
                    != if write {
                        "PUT"
                    } else if multiple {
                        "POST"
                    } else {
                        "GET"
                    }
            {
                return Err(unsupported());
            }
        }
        if actions && unique(form, "http://www.w3.org/2011/http#methodName")?.is_none() {
            return Err(unsupported()); // TD defines no HTTP default for this operation.
        }
        if multiple && output.method != "POST" {
            return Err(unsupported());
        }
        if let Some(headers) = unique(form, "http://www.w3.org/2011/http#headers")? {
            if headers.value() != SemanticValue::Array {
                return Err(unsupported());
            }
            for header in headers.children() {
                if header.value() != SemanticValue::Object {
                    return Err(unsupported());
                }
                let response_class = |node: SemanticNode<'_>| matches!(node.value(), SemanticValue::Iri(i) if i.is("http://www.w3.org/2011/http#ResponseHeader"));
                if header
                    .children()
                    .filter(|n| n.name() == clinkz_wot_td::semantic::SemanticName::Keyword("@type"))
                    .any(|n| response_class(n) || n.children().any(response_class))
                {
                    return Err(unsupported());
                }
                let name = text(
                    unique(header, "http://www.w3.org/2011/http#fieldName")?
                        .ok_or_else(unsupported)?,
                )?;
                let value = text(
                    unique(header, "http://www.w3.org/2011/http#fieldValue")?
                        .ok_or_else(unsupported)?,
                )?;
                if header.children().any(|n| {
                    http_name(n)
                        && !n.name().is("http://www.w3.org/2011/http#fieldName")
                        && !n.name().is("http://www.w3.org/2011/http#fieldValue")
                }) {
                    return Err(unsupported());
                }
                output.header(name, value)?;
            }
        }
        // Other standardized HTTP instructions are outside this declared slice.
        for node in form.children() {
            if (multiple
                || actions
                || matches!(
                    input.operation,
                    Operation::WriteAllProperties | Operation::WriteMultipleProperties
                ))
                && !http_name(node)
                && !crate::producer_compiler::class_assertion(node)
            {
                return Err(unsupported());
            }
            if http_name(node)
                && !node.name().is("http://www.w3.org/2011/http#methodName")
                && !node.name().is("http://www.w3.org/2011/http#headers")
            {
                return Err(unsupported());
            }
        }
        Ok(output)
    }
}
fn http_name(node: SemanticNode<'_>) -> bool {
    match node.name() {
        clinkz_wot_td::semantic::SemanticName::Iri(iri) => {
            let (prefix, suffix) = iri.segments();
            prefix.starts_with(HTTP)
                || HTTP
                    .strip_prefix(prefix)
                    .is_some_and(|rest| suffix.starts_with(rest))
        }
        _ => false,
    }
}
fn unique<'a>(node: SemanticNode<'a>, iri: &str) -> CoreResult<Option<SemanticNode<'a>>> {
    let mut fields = node.children().filter(|n| n.name().is(iri));
    let first = fields.next();
    if fields.next().is_some() {
        return Err(unsupported());
    }
    Ok(first)
}
fn text(node: SemanticNode<'_>) -> CoreResult<&str> {
    match node.value() {
        SemanticValue::Text(text) => Ok(text),
        _ => Err(unsupported()),
    }
}
pub(super) fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
            .with_redacted_cause(610, "unsupported HTTP finite mapping"),
    )
}
