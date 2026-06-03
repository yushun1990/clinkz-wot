use alloc::string::String;

use clinkz_wot_td::{form::Form, security_scheme::SecurityScheme, thing::Thing};

use crate::{CoreResult, TransportRequest};

/// Security metadata available while preparing a transport request.
pub struct SecurityContext<'a> {
    /// Thing Description that owns the selected form.
    pub thing: &'a Thing,
    /// Selected form.
    pub form: &'a Form,
    /// Name of the security definition being applied.
    pub scheme_name: &'a str,
    /// Security scheme definition.
    pub scheme: &'a SecurityScheme,
}

/// Applies protocol-neutral security metadata to a transport request.
pub trait SecurityProvider {
    /// Returns the security definition name handled by this provider.
    fn scheme_name(&self) -> &str;

    /// Applies security material to a transport request.
    fn apply(
        &mut self,
        context: SecurityContext<'_>,
        request: &mut TransportRequest,
    ) -> CoreResult<()>;

    /// Optional hook for reporting unsupported scope names.
    fn supports_scopes(&self, _scopes: &[String]) -> bool {
        true
    }
}
