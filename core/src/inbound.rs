//! Inbound interaction request and response model (baseline v3.0 §11 /
//! addendum §2.3).
//!
//! The inbound path is the symmetric counterpart of the outbound path: a
//! [`ServerBinding`] produces an [`InboundRequest`], the Servient driving loop
//! dispatches it against the exposed Thing registry, and returns an
//! [`InboundResponse`] that the binding matches back to its transport via the
//! echoed [`CorrelationId`].

#[cfg(feature = "async")]
use alloc::boxed::Box;
use alloc::string::String;

use clinkz_wot_td::{data_type::Operation, thing::Thing};

use crate::{
    AffordanceTarget, CoreResult, EventBroker, InteractionInput, InteractionOutput, ThingId,
    identity::CorrelationId, security::AuthMaterial,
};

/// Request produced by a server binding for an inbound interaction.
///
/// The request does **not** carry the `Thing` or the matched `Form`: the
/// dispatcher resolves the `Thing` from the exposed registry by
/// [`thing_id`](Self::thing_id), resolves the matched `Form` internally (for
/// security scheme lookup), and never exposes the `Form` to handlers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboundRequest {
    /// Identity of the targeted exposed Thing.
    pub thing_id: ThingId,
    /// Affordance location of the interaction.
    pub target: AffordanceTarget,
    /// Effective operation being performed.
    pub operation: Operation,
    /// Caller input.
    pub input: InteractionInput,
    /// Transport-level credentials extracted by the binding, if any.
    pub auth: Option<AuthMaterial>,
    /// Opaque token echoed unchanged in the matching [`InboundResponse`].
    pub correlation: CorrelationId,
}

impl InboundRequest {
    /// Creates a new inbound request with no auth material and an empty
    /// correlation token.
    pub fn new(
        thing_id: ThingId,
        target: AffordanceTarget,
        operation: Operation,
        input: InteractionInput,
    ) -> Self {
        Self {
            thing_id,
            target,
            operation,
            input,
            auth: None,
            correlation: CorrelationId::empty(),
        }
    }
}

/// Response returned by the inbound dispatcher for a matching [`InboundRequest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboundResponse {
    /// Interaction output (empty when `error` is `Some`).
    pub output: InteractionOutput,
    /// Opaque token echoed from the request, for binding-side matching.
    pub correlation: CorrelationId,
    /// Dispatch error when the interaction failed (baseline addendum §4 / §5.1).
    ///
    /// When `Some`, `output` is empty and the binding maps this to a
    /// protocol-level error reply.
    pub error: Option<crate::CoreError>,
}

impl InboundResponse {
    /// Creates a successful response.
    pub fn new(output: InteractionOutput, correlation: CorrelationId) -> Self {
        Self {
            output,
            correlation,
            error: None,
        }
    }

    /// Creates an error response with empty output.
    pub fn error(correlation: CorrelationId, error: crate::CoreError) -> Self {
        Self {
            output: InteractionOutput::empty(),
            correlation,
            error: Some(error),
        }
    }
}

/// Inbound protocol binding contract: a source of [`InboundRequest`]s (baseline
/// v3.0 §2 / §4).
///
/// A Servient drives the server side by polling its registered server bindings.
/// The synchronous flavor uses [`poll_accept_sync`](Self::poll_accept_sync).
/// The native-async `poll_accept` surface is finalized when the async driving
/// layer lands (SR-P2.2); until then this trait is dyn-compatible so the
/// Servient can store `Box<dyn ServerBinding>`.
///
/// Responses are written back through [`send_response`](Self::send_response).
/// The binding matches the response to its transport via the echoed
/// [`CorrelationId`].
///
/// Route registration ([`register_thing`](Self::register_thing) /
/// [`unregister_thing`](Self::unregister_thing)) is called by the Servient
/// during `expose`/`destroy` coordination (baseline §10). A route-registration
/// failure is fatal — the Servient removes the entry and returns `Err`.
pub trait ServerBinding {
    /// Non-blocking immediate poll. Returns `None` when no inbound request is
    /// ready, never blocks.
    fn poll_accept_sync(&self) -> Option<InboundRequest>;

    /// Sends a response back to the requester identified by the response's
    /// [`CorrelationId`].
    fn send_response(&self, response: InboundResponse);

    /// Registers inbound routes for a newly exposed Thing (baseline §10 step 3).
    ///
    /// The binding derives protocol-specific routes (e.g. zenoh keys) from the
    /// Thing's affordance forms. Returns `Err(message)` when route registration
    /// fails; the Servient treats this as fatal and rolls back the `expose`.
    fn register_thing(&self, thing_id: &str, td: &Thing) -> Result<(), String>;

    /// Unregisters inbound routes for a Thing being destroyed (baseline §10
    /// destroy step 1).
    fn unregister_thing(&self, thing_id: &str);

    /// Provides the shared [`EventBroker`] so the binding can register
    /// [`PublisherSink`](crate::PublisherSink)s for event and observable
    /// property fan-out during [`register_thing`](Self::register_thing).
    ///
    /// The default implementation is a no-op, suitable for bindings that do
    /// not support inbound event publishing.
    fn set_event_broker(&self, _broker: EventBroker) {}
}

/// Native-async inbound protocol binding contract (baseline v3.0 §2 / §4 /
/// addendum §2.4).
///
/// This trait is the async counterpart of [`ServerBinding`]. It is gated behind
/// the `async` feature and uses `#[async_trait]` so it remains dyn-compatible
/// (`Arc<dyn AsyncServerBinding>`).
///
/// The async driving loop (`Servient::poll_serve`) races
/// [`poll_accept`](Self::poll_accept) across all registered async bindings
/// concurrently using `select_all`.
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncServerBinding: Send + Sync {
    /// Native-async accept; pending until a request arrives.
    ///
    /// Implementations should not busy-poll — they should await an efficient
    /// notification primitive (e.g. `tokio::sync::Notify`, a channel, or an
    /// embassy signal) and only drain the pending queue once woken.
    async fn poll_accept(&self) -> InboundRequest;

    /// Sends a response back to the requester identified by the response's
    /// [`CorrelationId`].
    fn send_response(&self, response: InboundResponse);

    /// Registers inbound routes for a newly exposed Thing (baseline §10 step 3).
    fn register_thing(&self, thing_id: &str, td: &Thing) -> Result<(), String>;

    /// Unregisters inbound routes for a Thing being destroyed (baseline §10
    /// destroy step 1).
    fn unregister_thing(&self, thing_id: &str);

    /// Provides the shared [`EventBroker`] (see [`ServerBinding::set_event_broker`]).
    fn set_event_broker(&self, _broker: EventBroker) {}
}

/// Dispatches an inbound request to the matching exposed Thing handler
/// (baseline v3.0 §2 / §11).
///
/// The dispatcher resolves the `Thing` from the exposed registry by
/// [`thing_id`](InboundRequest::thing_id), resolves the matched `Form`
/// internally (for security scheme lookup), runs inbound security verification,
/// and routes the interaction to the handler. It never exposes the matched
/// `Form` to handlers. A concrete implementation lives with the runtime that
/// owns the exposed Thing registry (Servient); this trait is the core contract.
pub trait InboundDispatcher {
    /// Resolves and routes an inbound request, returning the response that the
    /// server binding echoes back to its transport.
    fn dispatch(&self, request: InboundRequest) -> CoreResult<InboundResponse>;
}

#[cfg(test)]
mod tests {
    use super::*;

    use alloc::{string::String, vec, vec::Vec};
    use clinkz_wot_td::security_scheme::{NoSecurityScheme, SecurityScheme};

    use crate::CoreError;
    use crate::TransportRequest;
    use crate::security::SecurityProvider;
    use crate::security::{Principal, PrincipalId, SecurityContext, SecurityError, check_scopes};

    #[test]
    fn new_request_defaults_to_no_auth_and_empty_correlation() {
        let request = InboundRequest::new(
            ThingId::from("urn:thing:1"),
            AffordanceTarget::Property("status".into()),
            Operation::ReadProperty,
            InteractionInput::empty(),
        );
        assert_eq!(request.thing_id.as_str(), "urn:thing:1");
        assert!(request.auth.is_none());
        assert!(request.correlation.as_bytes().is_empty());
    }

    #[test]
    fn response_carries_correlation_for_binding_matching() {
        let correlation = CorrelationId::from(42u64);
        let response = InboundResponse::new(InteractionOutput::empty(), correlation.clone());
        assert_eq!(response.correlation, correlation);
    }

    // --- Inbound security verification flow (SR-P0.5) -----------------------

    /// Bearer-token provider that authenticates a known token and grants the
    /// configured principal scopes. Demonstrates the wiring a concrete
    /// dispatcher (Servient in SR-P1) will use.
    struct BearerProvider {
        valid_token: Vec<u8>,
        principal_scopes: Vec<String>,
    }

    impl SecurityProvider for BearerProvider {
        fn scheme_name(&self) -> &str {
            "bearer"
        }

        fn apply(&mut self, _: SecurityContext<'_>, _: &mut TransportRequest) -> CoreResult<()> {
            Ok(())
        }

        fn verify(
            &self,
            request: &InboundRequest,
            _scheme: &SecurityScheme,
        ) -> Result<Principal, SecurityError> {
            match &request.auth {
                None => Err(SecurityError::MissingCredentials),
                Some(AuthMaterial::BearerToken(token)) if token == &self.valid_token => {
                    Ok(Principal {
                        id: PrincipalId::from("caller-1"),
                        scopes: self.principal_scopes.clone(),
                    })
                }
                Some(_) => Err(SecurityError::InvalidCredentials),
            }
        }
    }

    /// Minimal secure dispatcher: verify, then enforce form scopes, then route
    /// to a (stub) handler. The resolved scheme and required scopes stand in for
    /// the form-to-scheme resolution the Servient performs.
    struct SecureDispatcher<'a> {
        provider: &'a BearerProvider,
        scheme: SecurityScheme,
        required_scopes: Vec<String>,
    }

    impl InboundDispatcher for SecureDispatcher<'_> {
        fn dispatch(&self, request: InboundRequest) -> CoreResult<InboundResponse> {
            let principal = self
                .provider
                .verify(&request, &self.scheme)
                .map_err(CoreError::from)?;
            check_scopes(&self.required_scopes, &principal.scopes)?;
            Ok(InboundResponse::new(
                InteractionOutput::empty(),
                request.correlation,
            ))
        }
    }

    fn nosec_scheme() -> SecurityScheme {
        SecurityScheme::NoSec(NoSecurityScheme::default())
    }

    fn request_with(auth: Option<AuthMaterial>) -> InboundRequest {
        let mut request = InboundRequest::new(
            ThingId::from("urn:thing:1"),
            AffordanceTarget::Property(String::from("status")),
            Operation::ReadProperty,
            InteractionInput::empty(),
        );
        request.auth = auth;
        request
    }

    #[test]
    fn dispatch_succeeds_when_credentials_and_scopes_are_valid() {
        let provider = BearerProvider {
            valid_token: vec![1, 2, 3],
            principal_scopes: vec![String::from("read")],
        };
        let dispatcher = SecureDispatcher {
            provider: &provider,
            scheme: nosec_scheme(),
            required_scopes: vec![String::from("read")],
        };
        let mut request = request_with(Some(AuthMaterial::BearerToken(vec![1, 2, 3])));
        request.correlation = CorrelationId::from(7u64);

        let response = dispatcher
            .dispatch(request)
            .expect("valid request dispatches");
        assert_eq!(response.correlation, CorrelationId::from(7u64));
        assert!(response.output.payload.is_none());
    }

    #[test]
    fn dispatch_denies_missing_credentials() {
        let provider = BearerProvider {
            valid_token: vec![1, 2, 3],
            principal_scopes: vec![],
        };
        let dispatcher = SecureDispatcher {
            provider: &provider,
            scheme: nosec_scheme(),
            required_scopes: vec![],
        };
        let error = dispatcher.dispatch(request_with(None)).unwrap_err();
        assert!(matches!(
            error,
            CoreError::Security(SecurityError::MissingCredentials)
        ));
    }

    #[test]
    fn dispatch_denies_invalid_credentials() {
        let provider = BearerProvider {
            valid_token: vec![1, 2, 3],
            principal_scopes: vec![],
        };
        let dispatcher = SecureDispatcher {
            provider: &provider,
            scheme: nosec_scheme(),
            required_scopes: vec![],
        };
        let error = dispatcher
            .dispatch(request_with(Some(AuthMaterial::BearerToken(vec![9]))))
            .unwrap_err();
        assert!(matches!(
            error,
            CoreError::Security(SecurityError::InvalidCredentials)
        ));
    }

    #[test]
    fn dispatch_denies_when_principal_lacks_required_scope() {
        let provider = BearerProvider {
            valid_token: vec![1, 2, 3],
            principal_scopes: vec![String::from("read")],
        };
        let dispatcher = SecureDispatcher {
            provider: &provider,
            scheme: nosec_scheme(),
            required_scopes: vec![String::from("write")],
        };
        let error = dispatcher
            .dispatch(request_with(Some(AuthMaterial::BearerToken(vec![1, 2, 3]))))
            .unwrap_err();
        assert!(matches!(
            error,
            CoreError::Security(SecurityError::ScopeDenied { .. })
        ));
    }
}
