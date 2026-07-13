//! Inbound interaction request and response model, and the inbound binding
//! contract (baseline v4.1 §4.5).
//!
//! A [`ServerBinding`] produces [`InboundRequest`]s. On `std` the binding owns
//! its driving model: `serve()` declares routes and spawns a draining task
//! that calls `ctx.dispatch.serve_request(req).await`; on bare `no_std` the
//! super-loop polls [`ServerBinding::try_accept`]. The Servient's dispatch
//! resolves each request against the exposed Thing registry and returns an
//! [`InboundResponse`] that the binding matches back to its transport via the
//! echoed [`CorrelationId`].

#[cfg(feature = "async")]
use alloc::boxed::Box;

use clinkz_wot_td::{data_type::Operation, thing::Thing};

use crate::{
    AffordanceTarget, CoreError, CoreResult, EventBroker, ThingId,
    identity::CorrelationId,
    interaction::{InteractionInput, InteractionOutput},
    security::AuthMaterial,
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
    pub error: Option<CoreError>,
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
    pub fn error(correlation: CorrelationId, error: CoreError) -> Self {
        Self {
            output: InteractionOutput::empty(),
            correlation,
            error: Some(error),
        }
    }
}

/// All capabilities the Servient can inject into a binding at `serve()` time
/// (v4.1 AD56). Each binding picks what it needs.
#[derive(Clone)]
pub struct BindingContext {
    /// Event fan-out broker for event/observable property publish.
    pub event_broker: EventBroker,
    /// Direct-dispatch handle (async only). `None` on bare no_std or when the
    /// binding doesn't use direct dispatch. On std the binding's draining task
    /// calls `serve_request(req).await` to route requests through the Servient.
    #[cfg(feature = "async")]
    pub dispatch: Option<alloc::sync::Arc<dyn Dispatch>>,
}

/// Inbound protocol binding contract (baseline v4.1 §4.5, AD56).
///
/// The binding owns its lifecycle: [`serve`](Self::serve) declares routes for
/// one Thing AND starts the driving model (std: spawns a draining task; no_std:
/// configures poll state). [`shutdown`](Self::shutdown) is the teardown twin.
/// This replaces v4.0's `configure` + `register_thing` + `unregister_thing` +
/// `set_event_broker` + `set_request_sink`.
pub trait ServerBinding: Send + Sync {
    /// Starts serving inbound requests for `thing_id` based on `td`.
    ///
    /// On std this declares transport routes (queryables / listeners) AND may
    /// spawn a background draining task that recv()s from the binding's
    /// internal channel and calls `ctx.dispatch.serve_request(req).await`,
    /// then `self.send_response(resp)`. On no_std it declares routes and
    /// configures poll state; the super-loop drains via `try_accept`.
    ///
    /// Returns `Result<(), CoreError>` so multi-binding rollback (AD27) can
    /// detect a binding `k+1` failure, `shutdown` the succeeded `1..k`, and
    /// surface a fatal `Err`.
    fn serve(&self, thing_id: &ThingId, td: &Thing, ctx: &BindingContext) -> CoreResult<()>;

    /// Stops serving `thing_id`: undeclares routes, cancels background tasks,
    /// drops per-Thing state. Idempotent (AD27/E13) — best-effort across
    /// bindings.
    fn shutdown(&self, thing_id: &ThingId);

    /// Non-blocking drain of one currently-ready inbound request, or `None`.
    /// Default `None`: std bindings that self-drive via a background task
    /// never have `try_accept` called. no_std bindings override this so the
    /// super-loop can poll.
    fn try_accept(&self) -> Option<InboundRequest> {
        None
    }

    /// Sends a response back to the requester identified by the response's
    /// [`CorrelationId`]. Required — every binding that accepts requests
    /// must implement it.
    fn send_response(&self, response: InboundResponse);
}

/// Direct-dispatch handle for bindings that handle their own request lifecycle.
/// The binding's draining task calls
/// [`serve_request`](Self::serve_request) and gets the [`InboundResponse`]
/// directly.
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait Dispatch: Send + Sync {
    /// Dispatches an inbound request and returns the response directly.
    async fn serve_request(&self, request: InboundRequest) -> InboundResponse;
}

/// Dispatches an inbound request to the matching exposed Thing handler
/// (baseline v3.0 §2 / §11).
///
/// The dispatcher resolves the `Thing` from the exposed registry by
/// [`thing_id`](InboundRequest::thing_id), runs inbound security verification,
/// and routes the interaction to the handler. A concrete implementation lives
/// with the runtime that owns the exposed Thing registry (Servient); this trait
/// is the core contract.
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

    use crate::TransportRequest;
    use crate::security::SecurityProvider;
    use crate::security::{Principal, PrincipalId, SecurityContext, check_scopes};
    use crate::{ErrorContext, ErrorPhase, RetryClass, SecurityFailureReason};

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
        assert!(request.correlation.is_empty());
    }

    #[test]
    fn response_carries_correlation_for_binding_matching() {
        let correlation = CorrelationId::new(42);
        let response = InboundResponse::new(InteractionOutput::empty(), correlation);
        assert_eq!(response.correlation, correlation);
    }

    // --- Inbound security verification flow (SR-P0.5) -----------------------

    /// Bearer-token provider that authenticates a known token and grants the
    /// configured principal scopes.
    struct BearerProvider {
        valid_token: Vec<u8>,
        principal_scopes: Vec<String>,
    }

    impl SecurityProvider for BearerProvider {
        fn scheme_name(&self) -> &str {
            "bearer"
        }

        fn apply(&self, _: SecurityContext<'_>, _: &mut TransportRequest) -> CoreResult<()> {
            Ok(())
        }

        fn verify(
            &self,
            request: &InboundRequest,
            _scheme: &SecurityScheme,
        ) -> CoreResult<Principal> {
            match &request.auth {
                None => Err(security_failure(SecurityFailureReason::MissingCredentials)),
                Some(AuthMaterial::BearerToken(token)) if token == &self.valid_token => {
                    Ok(Principal {
                        id: PrincipalId::from("caller-1"),
                        scopes: self.principal_scopes.clone(),
                    })
                }
                Some(_) => Err(security_failure(SecurityFailureReason::InvalidCredentials)),
            }
        }
    }

    fn security_failure(reason: SecurityFailureReason) -> CoreError {
        CoreError::Security {
            reason,
            context: ErrorContext::new(ErrorPhase::Commit, RetryClass::Never),
        }
    }

    /// Minimal secure dispatcher: verify, then enforce form scopes, then route
    /// to a (stub) handler.
    struct SecureDispatcher<'a> {
        provider: &'a BearerProvider,
        scheme: SecurityScheme,
        required_scopes: Vec<String>,
    }

    impl InboundDispatcher for SecureDispatcher<'_> {
        fn dispatch(&self, request: InboundRequest) -> CoreResult<InboundResponse> {
            let principal = self.provider.verify(&request, &self.scheme)?;
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
            AffordanceTarget::Property(String::from("status").into()),
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
        request.correlation = CorrelationId::new(7);

        let response = dispatcher
            .dispatch(request)
            .expect("valid request dispatches");
        assert_eq!(response.correlation, CorrelationId::new(7));
        assert!(response.output.data().is_none());
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
            CoreError::Security {
                reason: SecurityFailureReason::MissingCredentials,
                ..
            }
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
            CoreError::Security {
                reason: SecurityFailureReason::InvalidCredentials,
                ..
            }
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
            CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                ..
            }
        ));
    }
}
