//! Bounded RFC 8288 Link parsing and Recommendation listing navigation.
use super::*;
use hyper::HeaderMap;
use std::collections::BTreeMap;

pub(super) fn resolve(base: &str, reference: &str) -> Result<String> {
    if reference.len() > 2048 {
        return Err(E::InvalidContinuation);
    }
    let base = fluent_uri::Uri::parse(base).map_err(|_| E::Configuration)?;
    let reference = fluent_uri::UriRef::parse(reference).map_err(|_| E::InvalidContinuation)?;
    let result = reference
        .resolve_against(&base)
        .map_err(|_| E::InvalidContinuation)?
        .into_string();
    parse_url(&result).map_err(|_| E::InvalidContinuation)?;
    Ok(result)
}
struct Link {
    target: String,
    params: BTreeMap<String, String>,
}
fn token(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}
fn spaces(bytes: &[u8], i: &mut usize) {
    while bytes.get(*i).is_some_and(|b| *b == b' ' || *b == b'\t') {
        *i += 1;
    }
}
fn links(value: &str) -> Result<Vec<Link>> {
    let bytes = value.as_bytes();
    let mut i = 0;
    let mut result = Vec::new();
    while i < bytes.len() {
        spaces(bytes, &mut i);
        if bytes.get(i) != Some(&b'<') {
            return Err(E::InvalidContinuation);
        }
        i += 1;
        let start = i;
        while bytes.get(i).is_some_and(|b| *b != b'>') {
            i += 1;
        }
        if bytes.get(i) != Some(&b'>') || i - start > 2048 {
            return Err(E::InvalidContinuation);
        }
        let target = value[start..i].into();
        i += 1;
        let mut params = BTreeMap::new();
        loop {
            spaces(bytes, &mut i);
            if bytes.get(i) != Some(&b';') {
                break;
            }
            i += 1;
            spaces(bytes, &mut i);
            let start = i;
            while bytes.get(i).is_some_and(|b| token(*b)) {
                i += 1;
            }
            if start == i {
                return Err(E::InvalidContinuation);
            }
            let name = value[start..i].to_ascii_lowercase();
            spaces(bytes, &mut i);
            let mut val = String::new();
            if bytes.get(i) == Some(&b'=') {
                i += 1;
                spaces(bytes, &mut i);
                if bytes.get(i) == Some(&b'"') {
                    i += 1;
                    loop {
                        match bytes.get(i) {
                            Some(b'"') => {
                                i += 1;
                                break;
                            }
                            Some(b'\\') => {
                                i += 1;
                                let b = *bytes.get(i).ok_or(E::InvalidContinuation)?;
                                if !(32..=126).contains(&b) {
                                    return Err(E::InvalidContinuation);
                                }
                                val.push(b as char);
                                i += 1;
                            }
                            Some(b) if (32..=126).contains(b) => {
                                val.push(*b as char);
                                i += 1;
                            }
                            _ => return Err(E::InvalidContinuation),
                        }
                    }
                } else {
                    let start = i;
                    while bytes.get(i).is_some_and(|b| token(*b)) {
                        i += 1;
                    }
                    if start == i {
                        return Err(E::InvalidContinuation);
                    }
                    val.push_str(&value[start..i]);
                }
            }
            // Ambiguity in rel/anchor/etag is refused, never first/last wins.
            if params.insert(name, val).is_some() {
                return Err(E::InvalidContinuation);
            }
        }
        result.push(Link { target, params });
        if result.len() > 32 {
            return Err(E::Limit);
        }
        spaces(bytes, &mut i);
        if i == bytes.len() {
            break;
        }
        if bytes.get(i) != Some(&b',') {
            return Err(E::InvalidContinuation);
        }
        i += 1;
        if i == bytes.len() {
            return Err(E::InvalidContinuation);
        }
    }
    Ok(result)
}
fn query(uri: &Uri) -> Result<(usize, usize)> {
    let mut names = BTreeMap::new();
    for pair in uri.query().ok_or(E::InvalidContinuation)?.split('&') {
        let (name, val) = pair.split_once('=').ok_or(E::InvalidContinuation)?;
        // Reject encoded parameter names: no alternate spelling can hide a
        // duplicate limit/offset, while opaque session values remain untouched.
        if name.is_empty()
            || name.bytes().any(|b| !token(b) || b == b'%' || b == b'+')
            || names.insert(name, val).is_some()
        {
            return Err(E::InvalidContinuation);
        }
    }
    if names.get("format").is_some_and(|v| *v != "array") {
        return Err(E::InvalidContinuation);
    }
    let number = |name| -> Result<usize> {
        let text = *names.get(name).ok_or(E::InvalidContinuation)?;
        if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
            return Err(E::InvalidContinuation);
        }
        text.parse().map_err(|_| E::InvalidContinuation)
    };
    Ok((number("offset")?, number("limit")?))
}
#[allow(clippy::too_many_arguments)]
pub(super) fn navigation(
    headers: &HeaderMap,
    base: &str,
    collection: &str,
    prior_etag: Option<&str>,
    offset: usize,
    count: usize,
    limit: usize,
) -> Result<(Option<String>, Option<String>)> {
    let collection_uri = parse_url(collection)?;
    let mut next = None;
    let mut canonical = None;
    let mut link_count = 0;
    for header in headers.get_all("link") {
        for link in links(header.to_str().map_err(|_| E::InvalidContinuation)?)? {
            link_count += 1;
            if link_count > 32 {
                return Err(E::Limit);
            }
            let relations = link.params.get("rel").map(String::as_str).unwrap_or("");
            let is_next = relations
                .split_ascii_whitespace()
                .any(|r| r.eq_ignore_ascii_case("next"));
            let is_canonical = relations
                .split_ascii_whitespace()
                .any(|r| r.eq_ignore_ascii_case("canonical"));
            if !is_next && !is_canonical {
                continue;
            }
            if link.params.contains_key("anchor") || (is_next && is_canonical) {
                return Err(E::InvalidContinuation);
            }
            let target = resolve(base, &link.target)?;
            let uri = parse_url(&target).map_err(|_| E::InvalidContinuation)?;
            if uri.scheme() != collection_uri.scheme()
                || uri.authority() != collection_uri.authority()
                || uri.path() != collection_uri.path()
            {
                return Err(E::DestinationDenied);
            }
            if is_next {
                let (next_offset, next_limit) = query(&uri)?;
                if next.is_some()
                    || count == 0
                    || next_limit != limit
                    || next_offset != offset.checked_add(count).ok_or(E::Limit)?
                {
                    return Err(E::InvalidContinuation);
                }
                next = Some(target);
            } else {
                let etag = link
                    .params
                    .get("etag")
                    .filter(|v| !v.is_empty())
                    .ok_or(E::InvalidContinuation)?
                    .clone();
                if canonical.is_some() || uri.query().is_some() {
                    return Err(E::InvalidContinuation);
                }
                canonical = Some(etag);
            }
        }
    }
    if (next.is_some() || prior_etag.is_some()) && canonical.is_none() {
        return Err(E::InvalidContinuation);
    }
    if prior_etag.is_some_and(|prior| canonical.as_deref() != Some(prior)) {
        return Err(E::CollectionChanged);
    }
    Ok((next, canonical))
}
