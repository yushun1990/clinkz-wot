//! Shared selected TD security meaning for the finite supported subset.
//! No credentials, TLS or protocol-specific transport code is present here.
use clinkz_wot_core::{security::SecurityRequirement, *};
use clinkz_wot_td::{security_scheme::SecurityLocation, semantic::*};

pub(crate) const BASIC_HEADER: SecurityRequirement = SecurityRequirement {
    scheme: "basic",
    parameters: &[("in", "header"), ("name", "Authorization")],
};

/// Refuse the complete unsupported expression. A list is simultaneous security,
/// not candidate preference; combo alternatives must never default to NoSec.
pub fn prepare_security(
    document: &ResolvedSemanticDocument<'_>,
    owner: FormOwner<'_>,
    form: u32,
) -> CoreResult<SecurityRequirement> {
    let unsupported = || {
        CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
    };
    let facts = document.form(owner, form).ok_or_else(unsupported)?;
    let [name] = facts.effective_security else {
        return Err(unsupported());
    };
    let security = document.security(name).map_err(|_| unsupported())?;
    if security.proxy.is_some()
        || security.semantics.children().next().is_some()
        || facts.scopes.is_some()
    {
        return Err(unsupported());
    }
    match security.details {
        SecurityDetails::NoSec => Ok(SecurityRequirement::NOSEC),
        SecurityDetails::Basic {
            name,
            location: SecurityLocation::Header,
        } if name.is_none_or(|n| n.eq_ignore_ascii_case("Authorization")) => Ok(BASIC_HEADER),
        _ => Err(unsupported()),
    }
}
