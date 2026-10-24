//! Optional HTTP preparation; all executable types are defined by the runtime.
use super::*;
use clinkz_wot_planning::*;
use clinkz_wot_td::semantic::{SemanticNode, SemanticScope, SemanticValue};
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
        &["http"]
    }
    fn compile(&self, input: ResolvedBindingInput<'_>) -> CoreResult<HttpArtifact> {
        use clinkz_wot_td::data_type::Operation;
        if input.operation != Operation::ReadProperty
            || input.content_type != Some("application/json")
            || input.subprotocol.is_some()
        {
            return Err(unsupported());
        }
        Target::parse(input.target).map_err(|_| unsupported())?;
        let facts = input
            .document
            .form_facts(input.property, input.form_index)
            .ok_or_else(unsupported)?;
        let [security_name] = facts.effective_security else {
            return Err(unsupported());
        };
        let security = input
            .document
            .security(security_name)
            .map_err(|_| unsupported())?;
        // This transport executes endpoint NoSec only. A proxy target must not
        // silently become endpoint security or an invented direct route.
        if security.proxy.is_some() {
            return Err(unsupported());
        }
        if facts.content_coding.is_some()
            || facts.scopes.is_some()
            || facts.additional_responses
            || facts
                .response_content_type
                .is_some_and(|s| s != "application/json")
        {
            return Err(unsupported());
        }
        let form = input
            .document
            .scope(SemanticScope::Form {
                property: input.property,
                index: input.form_index,
            })
            .ok_or_else(unsupported)?;
        let mut output = HttpArtifact {
            target_len: input.target.len() as u8,
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
            if http_name(node)
                && !node.name().is("http://www.w3.org/2011/http#methodName")
                && !node.name().is("http://www.w3.org/2011/http#headers")
            {
                return Err(unsupported());
            }
        }
        if let Some(response) = input.document.scope(SemanticScope::Response {
            property: input.property,
            index: input.form_index,
        }) {
            if let Some(status) = unique(response, "http://www.w3.org/2011/http#statusCodeValue")?
                && !matches!(status.value(), SemanticValue::Number(n) if n.as_u64() == Some(200))
            {
                return Err(unsupported());
            }
            if response.children().any(|n| {
                http_name(n) && !n.name().is("http://www.w3.org/2011/http#statusCodeValue")
            }) {
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
            .with_redacted_cause(610, "unsupported HTTP read mapping"),
    )
}
