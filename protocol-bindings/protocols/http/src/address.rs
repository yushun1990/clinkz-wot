//! HTTP placement/compatibility, separate from shared RFC expression meaning.
use super::*;
use clinkz_wot_core::uri_variables::{MAX_NAME_BYTES, MAX_VARIABLES, UriTemplate};

#[cfg(feature = "prepare")]
pub(super) fn qualify(text: &str, secure: bool) -> CoreResult<Target> {
    let template = UriTemplate::parse(text)?;
    let scheme = if secure { "https://" } else { "http://" };
    let rest = text
        .strip_prefix(scheme)
        .ok_or_else(|| error(601, "fixed HTTP scheme required"))?;
    // A dynamic authority is limited to one simple port expression. Host and
    // scheme expansion, userinfo and fragments are outside this native slice.
    // A query expression starts at its opening brace, including when the URI
    // has no explicit path. Its '?' is not literal authority syntax.
    let end = rest
        .char_indices()
        .find(|(i, c)| {
            matches!(c, '/' | '?' | '#')
                || rest[*i..].starts_with("{?")
                || rest[*i..].starts_with("{&")
        })
        .map_or(rest.len(), |(i, _)| i);
    let authority = &rest[..end];
    if let Some((host, expression)) = authority.split_once('{')
        && (!host.ends_with(':')
            || !expression.ends_with('}')
            || expression[..expression.len() - 1].contains(['{', '}', '?', '&', ',']))
    {
        return Err(error(601, "unsupported HTTP authority template"));
    }
    let probe = template.expand(|_| Some("1"), 256)?;
    Target::parse_for(&probe, secure)
}

pub(super) fn resolve(
    text: &str,
    variables: &[(&str, &str)],
    secure: bool,
    capacity: usize,
) -> CoreResult<Box<str>> {
    let template = UriTemplate::parse(text)?;
    if variables.len() > MAX_VARIABLES
        || variables.iter().enumerate().any(|(i, (name, _))| {
            name.len() > MAX_NAME_BYTES
                || !template.references().any(|n| n == *name)
                || variables[..i].iter().any(|(n, _)| n == name)
        })
    {
        return Err(error(601, "unsupported HTTP variables"));
    }
    let result = template.expand(
        |name| variables.iter().find(|(n, _)| *n == name).map(|(_, v)| *v),
        capacity,
    )?;
    // Recheck every actual native destination. No redirects, DNS, proxy,
    // retries or fallback can replace this already authorized operand.
    Target::parse_for(&result, secure)?;
    Ok(result)
}
