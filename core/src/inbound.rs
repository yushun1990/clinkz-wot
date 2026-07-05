//! Inbound interaction request and response model, and the inbound binding
//! contract (baseline v4.0 §4.5).
//!
//! A [`ServerBinding`] produces [`InboundRequest`]s. On `std` it self-pushes
//! into a bounded fan-in channel via [`ServerBinding::set_request_sink`]; on
//! `no_std` the driving loop polls [`ServerBinding::try_accept`]. The Servient
//! driving loop dispatches each request against the exposed Thing registry and
//! returns an [`InboundResponse`] that the binding matches back to its transport
//! via the echoed [`CorrelationId`].

use clinkz_wot_td::{data_type::Operation, thing::Thing};

use crate::{
    AffordanceTarget, CoreError, CoreResult, EventBroker, ThingId,
    identity::CorrelationId,
    interaction::{InteractionInput, InteractionOutput},
    security::AuthMaterial,
};

/// Bounded fan-in channel sender handed to each server binding on `std`
/// (baseline §4.5 / AD13 / D16). The Servient owns the matching `Receiver`;
/// each binding `try_send`s inbound requests from its **synchronous** transport
/// callbacks. Runtime-neutral (tokio/async-std/embassy-std).
#[cfg(feature = "std")]
pub type FanInSender<T> = async_channel::Sender<T>;

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

/// Inbound protocol binding contract (baseline v4.0 §4.5).
///
/// Routes are declared/undeclared wholesale per Thing during `expose()` /
/// `destroy()` (decision 2 — no per-affordance registration). On `std` the
/// binding self-pushes inbound requests into a bounded fan-in channel
/// ([`set_request_sink`](Self::set_request_sink)); on `no_std` the driving loop
/// polls [`try_accept`](Self::try_accept).
pub trait ServerBinding: Send + Sync {
    /// Non-blocking drain of one currently-ready inbound request, or `None`.
    ///
    /// Default `None`: a `std`-only binding that self-pushes via
    /// [`set_request_sink`](Self::set_request_sink) never has `try_accept`
    /// called and need not override it. On `no_std` this is the polled accept
    /// path (one request per tick, rotation cursor — see baseline §4.5/§7.2).
    fn try_accept(&self) -> Option<InboundRequest> {
        None
    }

    /// Sends a response back to the requester identified by the response's
    /// [`CorrelationId`]. Required by AD9's overload-error-reply semantics
    /// (`InboundRequest` carries no reply handle). No default — every binding
    /// that accepts requests must implement it.
    fn send_response(&self, response: InboundResponse);

    /// Provides the shared [`EventBroker`] so the binding can register
    /// [`PublisherSink`](crate::PublisherSink)s for event and observable
    /// property fan-out during [`register_thing`](Self::register_thing).
    ///
    /// Default no-op for bindings without event publish.
    fn set_event_broker(&self, _broker: EventBroker) {}

    /// Hands the binding a clone of the bounded fan-in sender at registration
    /// (`std` only — AD13). The binding `try_send`s inbound requests from its
    /// **synchronous** transport callbacks (zenoh callbacks cannot `.await`).
    /// On `no_std` there is no channel and the loop polls
    /// [`try_accept`](Self::try_accept) instead.
    #[cfg(feature = "std")]
    fn set_request_sink(&self, sender: FanInSender<InboundRequest>);

    /// Wholesale route registration for one Thing during `expose()`.
    ///
    /// Returns `Result<(), CoreError>` so the multi-binding rollback (E12/AD27)
    /// can detect a binding `k+1` failure, `unregister_thing` the succeeded
    /// `1..k`, and surface a fatal `Err` (AD38). A binding reports a structural
    /// failure via a structured `CoreError`, never a free-form `String`.
    fn register_thing(&self, thing_id: &ThingId, td: &Thing) -> Result<(), CoreError>;

    /// Wholesale route removal during `destroy()`. Returns `()` — `destroy()`
    /// is idempotent (AD27/E13) and best-effort across bindings.
    fn unregister_thing(&self, thing_id: &ThingId);
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
    /// to a (stub) handler.
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
        request.correlation = CorrelationId::from(7u64);

        let response = dispatcher
            .dispatch(request)
            .expect("valid request dispatches");
        assert_eq!(response.correlation, CorrelationId::from(7u64));
        assert!(response.output.data.is_none());
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
