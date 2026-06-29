use alloc::{format, sync::Arc, vec::Vec};

use clinkz_wot_core::{
    CoreError, InboundRequest, Principal, PrincipalId, SecurityError, check_scopes,
};
use clinkz_wot_td::security_scheme::SecurityScheme;

use crate::registry::ResolvedInboundSecurity;

use super::SecurityProviderRegistry;

pub(super) fn verify_inbound(
    security_providers: &SecurityProviderRegistry,
    request: &InboundRequest,
    resolved_security: &ResolvedInboundSecurity,
) -> Result<Principal, CoreError> {
    // Snapshot provider handles under a brief *read* lock, then verify
    // *outside* the registry lock so a slow provider (e.g. JWT key fetch,
    // network retrieval) does not serialize every inbound request across
    // every Thing. The snapshot is an `Arc`, so cloning it is a refcount
    // bump and never needs a write lock.
    let providers = security_providers.with_read_recover(Arc::clone);

    let mut resolved_principal: Option<Principal> = None;

    for (scheme_name, scheme) in &resolved_security.schemes {
        if matches!(scheme, SecurityScheme::NoSec(_)) {
            continue;
        }

        let provider = providers
            .iter()
            .find(|provider| provider.scheme_name() == scheme_name.as_str())
            .ok_or_else(|| {
                CoreError::Security(SecurityError::SchemeFailure(format!(
                    "No security provider registered for '{}'",
                    scheme_name
                )))
            })?;

        if !provider.supports_scopes(&resolved_security.scopes) {
            return Err(CoreError::Security(SecurityError::SchemeFailure(format!(
                "Security provider '{}' does not support scopes {:?}",
                scheme_name, resolved_security.scopes
            ))));
        }

        let principal = provider.verify(request, scheme).map_err(CoreError::from)?;
        check_scopes(&resolved_security.scopes, &principal.scopes).map_err(CoreError::from)?;
        resolved_principal = Some(principal);
    }

    Ok(resolved_principal.unwrap_or(Principal {
        id: PrincipalId::from("anonymous"),
        scopes: Vec::new(),
    }))
}
