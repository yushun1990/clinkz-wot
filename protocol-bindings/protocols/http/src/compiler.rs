//! HTTP vocabulary/default interpretation belongs with the HTTP executor.
use super::*;
use clinkz_wot_td::semantic::{SemanticNode, SemanticScope, SemanticValue};

pub(super) const COMPATIBILITY: BindingArtifactCompatibility =
    BindingArtifactCompatibility::new(*b"clinkz-http-v001");
const HTTP: &str = "http://www.w3.org/2011/http#";

#[derive(Clone, Copy, Debug, Default)]
struct Span {
    name: (u16, u16),
    value: (u16, u16),
}
/// Source-free native request instructions. All retained backing is inline.
#[derive(Clone, Copy, Debug)]
pub(super) struct HttpArtifact {
    target: [u8; 64],
    target_len: u8,
    method: &'static str,
    headers: [Span; 8],
    count: u8,
    bytes: [u8; 1024],
    used: u16,
}
impl Default for HttpArtifact {
    fn default() -> Self {
        Self {
            target: [0; 64],
            target_len: 0,
            method: "GET",
            headers: [Span::default(); 8],
            count: 0,
            bytes: [0; 1024],
            used: 0,
        }
    }
}
impl HttpArtifact {
    pub(super) fn target(&self) -> &str {
        std::str::from_utf8(&self.target[..self.target_len as usize]).unwrap()
    }
    pub(super) fn method(&self) -> &'static str {
        self.method
    }
    pub(super) fn headers(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers[..self.count as usize].iter().map(|s| {
            let text = |(start, end)| {
                std::str::from_utf8(&self.bytes[start as usize..end as usize]).unwrap()
            };
            (text(s.name), text(s.value))
        })
    }
    fn header(&mut self, name: &str, value: &str) -> CoreResult<()> {
        if self.count == 8
            || self.used as usize + name.len() + value.len() > self.bytes.len()
            || hyper::header::HeaderName::from_bytes(name.as_bytes()).is_err()
            || hyper::header::HeaderValue::from_str(value).is_err()
            || [
                "host",
                "connection",
                "content-length",
                "transfer-encoding",
                "content-encoding",
                "trailer",
                "te",
                "upgrade",
                "authorization",
                "proxy-authorization",
                "cookie",
            ]
            .iter()
            .any(|h| name.eq_ignore_ascii_case(h))
            || (name.eq_ignore_ascii_case("accept")
                && (value != "application/json"
                    || self
                        .headers()
                        .any(|(n, _)| n.eq_ignore_ascii_case("accept"))))
        {
            return Err(unsupported());
        }
        let start = self.used as usize;
        self.bytes[start..start + name.len()].copy_from_slice(name.as_bytes());
        let middle = start + name.len();
        self.bytes[middle..middle + value.len()].copy_from_slice(value.as_bytes());
        self.used = (middle + value.len()) as u16;
        self.headers[self.count as usize] = Span {
            name: (start as u16, middle as u16),
            value: (middle as u16, self.used),
        };
        self.count += 1;
        Ok(())
    }
}
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
            if let Some(status) = unique(response, "http://www.w3.org/2011/http#statusCodeValue")? {
                if !matches!(status.value(), SemanticValue::Number(n) if n.as_u64() == Some(200)) {
                    return Err(unsupported());
                }
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
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
            .with_redacted_cause(610, "unsupported HTTP read mapping"),
    )
}
