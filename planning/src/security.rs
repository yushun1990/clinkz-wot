//! Bounded effective WoT security composition. No credential or protocol logic.
use alloc::{string::String, vec::Vec};
use clinkz_wot_core::{security::SecurityRequirement as R, *};
use clinkz_wot_td::{security_scheme::SecurityLocation, semantic::*};

/// Complete projection/expansion envelope, paid by preparation.
pub const PREPARATION_WORK: u64 = 32_768;
pub const CLEANUP_WORK: u64 = 1024;
pub const TEMPORARY_BYTES: u64 = 131_072;
const MAX_TEXT: usize = 1024;
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}

/// Lists are simultaneous requirements. Unavailable leaves remain explicit so
/// Consumer selection can choose another branch, while Producer qualification
/// cannot discard them. Extension interpretation remains a supported-subset limit.
pub fn prepare_security(
    document: &ResolvedSemanticDocument<'_>,
    owner: FormOwner<'_>,
    form: u32,
) -> CoreResult<R> {
    fn visit(
        doc: &ResolvedSemanticDocument<'_>,
        name: &str,
        scopes: Option<&[String]>,
        path: &mut Vec<String>,
        nodes: &mut usize,
        text: &mut usize,
    ) -> CoreResult<R> {
        *nodes += 1;
        if *nodes > R::MAX_NODES || path.len() >= R::MAX_DEPTH || path.iter().any(|n| n == name) {
            return Err(unsupported());
        }
        let facts = doc.security(name).map_err(|_| unsupported())?;
        // Public typed TD fields can be mutated without a Serde round trip.
        // A mismatched discriminator must not turn a declared authenticated
        // scheme into NoSec or a conjunction of anonymous children.
        let discriminator = match facts.details {
            SecurityDetails::NoSec => "nosec",
            SecurityDetails::Auto => "auto",
            SecurityDetails::Combo { .. } => "combo",
            SecurityDetails::Basic { .. } => "basic",
            SecurityDetails::Digest { .. } => "digest",
            SecurityDetails::ApiKey { .. } => "apikey",
            SecurityDetails::Bearer { .. } => "bearer",
            SecurityDetails::Psk { .. } => "psk",
            SecurityDetails::OAuth2 { .. } => "oauth2",
            SecurityDetails::Extension => facts.scheme,
        };
        if facts.scheme != discriminator {
            return Err(unsupported());
        }
        *text = text
            .checked_add(name.len() + facts.scheme.len())
            .ok_or_else(unsupported)?;
        if *text > MAX_TEXT {
            return Err(unsupported());
        }
        if facts.semantics.children().next().is_some() {
            return Ok(R::Unavailable {
                scheme: facts.scheme.into(),
            });
        }
        let mut parameters = Vec::new();
        let mut add = |key: &str, value: &str| -> CoreResult<()> {
            *text = text
                .checked_add(32 + key.len() + value.len())
                .ok_or_else(unsupported)?;
            if *text > MAX_TEXT {
                return Err(unsupported());
            }
            parameters.push((String::from(key), String::from(value)));
            Ok(())
        };
        if let Some(proxy) = facts.proxy {
            add("proxy", proxy)?;
        }
        if let Some(scopes) = scopes {
            for scope in scopes {
                add("scope", scope)?;
            }
        }
        let location = |v: &SecurityLocation| match v {
            SecurityLocation::Header => "header",
            SecurityLocation::Query => "query",
            SecurityLocation::Body => "body",
            SecurityLocation::Cookie => "cookie",
            SecurityLocation::Auto => "auto",
            SecurityLocation::Uri => "uri",
        };
        match facts.details {
            SecurityDetails::Combo { one_of, all_of } => {
                if one_of.is_some() == all_of.is_some() {
                    return Err(unsupported());
                }
                if facts.proxy.is_some() {
                    return Ok(R::Unavailable {
                        scheme: facts.scheme.into(),
                    });
                }
                let members = one_of.or(all_of).ok_or_else(unsupported)?;
                if members.len() < 2 {
                    return Err(unsupported());
                }
                path.push(name.into());
                let children = members
                    .iter()
                    .map(|n| visit(doc, n, scopes, path, nodes, text))
                    .collect::<CoreResult<Vec<_>>>()?;
                path.pop();
                return Ok(if one_of.is_none() {
                    R::AllOf(children)
                } else {
                    R::OneOf(children)
                });
            }
            SecurityDetails::NoSec if facts.proxy.is_none() && scopes.is_none() => {
                return Ok(R::NOSEC);
            }
            SecurityDetails::Basic {
                name,
                location: place,
            } => {
                add("in", location(place))?;
                add("name", name.unwrap_or("Authorization"))?;
            }
            SecurityDetails::ApiKey {
                name,
                location: place,
            } => {
                add("in", location(place))?;
                if let Some(name) = name {
                    add("name", name)?;
                }
            }
            SecurityDetails::Digest {
                name,
                location: place,
                qop,
            } => {
                add("in", location(place))?;
                add("name", name.unwrap_or("Authorization"))?;
                add(
                    "qop",
                    match qop {
                        clinkz_wot_td::security_scheme::Qop::Auth => "auth",
                        clinkz_wot_td::security_scheme::Qop::AuthInt => "auth-int",
                    },
                )?;
            }
            SecurityDetails::Bearer {
                name,
                location: place,
                authorization,
                alg,
                format,
            } => {
                add("in", location(place))?;
                add("name", name.unwrap_or("Authorization"))?;
                if let Some(v) = authorization {
                    add("authorization", v)?;
                }
                add("alg", alg)?;
                add("format", format)?;
            }
            SecurityDetails::Psk { identity } => {
                if let Some(v) = identity {
                    add("identity", v)?;
                }
            }
            SecurityDetails::OAuth2 {
                flow,
                authorization,
                token,
                refresh,
                scopes,
            } => {
                add("flow", flow)?;
                for (k, v) in [
                    ("authorization", authorization),
                    ("token", token),
                    ("refresh", refresh),
                ] {
                    if let Some(v) = v {
                        add(k, v)?;
                    }
                }
                if let Some(scopes) = scopes {
                    for s in scopes {
                        add("definedScope", s)?;
                    }
                }
            }
            SecurityDetails::Extension | SecurityDetails::Auto => {
                return Ok(R::Unavailable {
                    scheme: facts.scheme.into(),
                });
            }
            SecurityDetails::NoSec => {}
        }
        Ok(R::Scheme {
            scheme: facts.scheme.into(),
            parameters,
        })
    }
    let facts = document.form(owner, form).ok_or_else(unsupported)?;
    if facts.effective_security.is_empty() {
        return Err(unsupported());
    }
    let mut nodes = 0;
    let mut text = 0;
    let mut requirements = facts
        .effective_security
        .iter()
        .map(|name| {
            visit(
                document,
                name,
                facts.scopes,
                &mut Vec::new(),
                &mut nodes,
                &mut text,
            )
        })
        .collect::<CoreResult<Vec<_>>>()?;
    let expression = if requirements.len() == 1 {
        requirements.pop().unwrap()
    } else {
        R::AllOf(requirements)
    };
    expression.alternatives()?;
    Ok(expression)
}

/// A compiler checks exactly the selected Consumer conjunction. Without a
/// selection it must qualify the complete effective expression (Producer use).
#[cfg(feature = "std")]
pub fn compilation_security(input: &crate::ResolvedBindingInput<'_>) -> CoreResult<R> {
    let expression = prepare_security(input.document, input.owner, input.form_index)?;
    if let Some(selected) = input.selected_security {
        if !expression.alternatives()?.contains(selected) {
            return Err(unsupported());
        }
        Ok(selected.clone())
    } else {
        Ok(expression)
    }
}
