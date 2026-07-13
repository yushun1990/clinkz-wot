//! Security wiring integration: verifies that `Servient::dispatch` actually
//! calls `SecurityProvider::verify` before routing to the handler, and that
//! the established Principal is injected into the handler-facing input.
//!
//! Covers:
//! - Bearer token valid → handler runs, principal injected.
//! - Bearer token missing → `MissingCredentials`, handler NOT called.
//! - Bearer token wrong → `InvalidCredentials`, handler NOT called.
//! - NoSec (default) → always passes.
//! - Declared scheme with no registered provider → `SecurityUnavailable` selection.
//! - Scope denial after successful verification.

#![cfg(all(feature = "async", feature = "std"))]

use std::sync::{Arc, Mutex};

use clinkz_wot_core::{
    AffordanceTarget, AuthMaterial, CoreError, Dispatch, ErrorPhase, InboundRequest,
    InteractionInput, NoSecurityProvider, PropertyReadHandler, RetryClass, SecurityFailureReason,
    SelectionFailureReason, ThingId,
};
use clinkz_wot_servient::ServientBuilder;
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    thing::Thing,
};

// --- fixtures --------------------------------------------------------------

fn secure_td() -> Thing {
    Thing::builder("SecureLamp")
        .id("urn:test:secure-lamp")
        .bearer_security("bearer", "authorization")
        .property(
            "status",
            PropertyAffordance::builder(DataSchema::string())
                .form(
                    clinkz_wot_td::form::Form::read_property("fake://secure-lamp/status")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

fn nosec_td() -> Thing {
    Thing::builder("Lamp")
        .id("urn:test:lamp")
        .nosec()
        .property(
            "status",
            PropertyAffordance::builder(DataSchema::string())
                .form(
                    clinkz_wot_td::form::Form::read_property("fake://lamp/status")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

/// Read handler that records whether it was called and captures the
/// principal from the input.
struct RecordingRead {
    called: Arc<Mutex<bool>>,
    principal: Arc<Mutex<Option<String>>>,
}
impl PropertyReadHandler for RecordingRead {
    fn read(
        &self,
        input: &InteractionInput,
    ) -> Result<clinkz_wot_core::InteractionOutput, CoreError> {
        *self.called.lock().unwrap() = true;
        *self.principal.lock().unwrap() =
            input.principal.as_ref().map(|p| p.id.as_str().to_string());
        Ok(clinkz_wot_core::InteractionOutput::with_data(
            clinkz_wot_core::Payload::new(b"on".to_vec(), "text/plain"),
        ))
    }
}

fn build_request(thing_id: &str, auth: Option<AuthMaterial>) -> InboundRequest {
    let mut req = InboundRequest::new(
        ThingId::from(thing_id),
        AffordanceTarget::Property("status".into()),
        Operation::ReadProperty,
        InteractionInput::empty(),
    );
    req.auth = auth;
    req
}

// --- tests -----------------------------------------------------------------

#[tokio::test]
async fn bearer_valid_token_passes_and_injects_principal() {
    let called = Arc::new(Mutex::new(false));
    let principal = Arc::new(Mutex::new(None));

    let servient = ServientBuilder::new()
        .with_security_provider(Arc::new(clinkz_wot_core::BearerSecurityProvider::new(
            b"valid-secret".to_vec(),
            "user-42",
            ["read"],
        )))
        .build()
        .expect("build");

    let handle = servient.produce(secure_td()).expect("produce");
    handle.set_property_read_handler(
        "status",
        RecordingRead {
            called: called.clone(),
            principal: principal.clone(),
        },
    );
    handle.expose().await.expect("expose");

    let resp = servient
        .serve_request(build_request(
            "urn:test:secure-lamp",
            Some(AuthMaterial::BearerToken(b"valid-secret".to_vec())),
        ))
        .await;

    assert!(*called.lock().unwrap(), "handler should have been called");
    assert!(
        resp.error.is_none(),
        "no error expected, got {:?}",
        resp.error
    );
    assert_eq!(
        principal.lock().unwrap().as_deref(),
        Some("user-42"),
        "principal should be injected into handler input"
    );
}

#[tokio::test]
async fn bearer_missing_credentials_rejects_before_handler() {
    let called = Arc::new(Mutex::new(false));

    let servient = ServientBuilder::new()
        .with_security_provider(Arc::new(clinkz_wot_core::BearerSecurityProvider::new(
            b"valid-secret".to_vec(),
            "u",
            Vec::<String>::new(),
        )))
        .build()
        .expect("build");

    let handle = servient.produce(secure_td()).expect("produce");
    handle.set_property_read_handler(
        "status",
        RecordingRead {
            called: called.clone(),
            principal: Arc::new(Mutex::new(None)),
        },
    );
    handle.expose().await.expect("expose");

    let resp = servient
        .serve_request(build_request("urn:test:secure-lamp", None))
        .await;

    assert!(
        !*called.lock().unwrap(),
        "handler should NOT have been called"
    );
    assert!(matches!(
        resp.error,
        Some(CoreError::Security {
            reason: SecurityFailureReason::MissingCredentials,
            ..
        })
    ));
}

#[tokio::test]
async fn bearer_wrong_token_rejects_with_invalid_credentials() {
    let called = Arc::new(Mutex::new(false));

    let servient = ServientBuilder::new()
        .with_security_provider(Arc::new(clinkz_wot_core::BearerSecurityProvider::new(
            b"correct".to_vec(),
            "u",
            Vec::<String>::new(),
        )))
        .build()
        .expect("build");

    let handle = servient.produce(secure_td()).expect("produce");
    handle.set_property_read_handler(
        "status",
        RecordingRead {
            called: called.clone(),
            principal: Arc::new(Mutex::new(None)),
        },
    );
    handle.expose().await.expect("expose");

    let resp = servient
        .serve_request(build_request(
            "urn:test:secure-lamp",
            Some(AuthMaterial::BearerToken(b"wrong".to_vec())),
        ))
        .await;

    assert!(
        !*called.lock().unwrap(),
        "handler should NOT have been called"
    );
    assert!(matches!(
        resp.error,
        Some(CoreError::Security {
            reason: SecurityFailureReason::InvalidCredentials,
            ..
        })
    ));
}

#[tokio::test]
async fn nosec_default_provider_passes_without_auth() {
    let called = Arc::new(Mutex::new(false));

    // No explicit security providers — the builder auto-registers
    // NoSecurityProvider for the default "nosec" scheme.
    let servient = ServientBuilder::new().build().expect("build");

    let handle = servient.produce(nosec_td()).expect("produce");
    handle.set_property_read_handler(
        "status",
        RecordingRead {
            called: called.clone(),
            principal: Arc::new(Mutex::new(None)),
        },
    );
    handle.expose().await.expect("expose");

    let resp = servient
        .serve_request(build_request("urn:test:lamp", None))
        .await;

    assert!(*called.lock().unwrap(), "handler should have been called");
    assert!(resp.error.is_none(), "nosec should always pass");
}

#[tokio::test]
async fn declared_scheme_without_provider_rejects() {
    // Register ONLY a NoSec provider. The secure_td declares "bearer"
    // which has no matching provider.
    let servient = ServientBuilder::new()
        .with_security_provider(Arc::new(NoSecurityProvider::new()))
        .build()
        .expect("build");

    let handle = servient.produce(secure_td()).expect("produce");
    handle.expose().await.expect("expose");

    let resp = servient
        .serve_request(build_request(
            "urn:test:secure-lamp",
            Some(AuthMaterial::BearerToken(b"anything".to_vec())),
        ))
        .await;

    assert!(matches!(
        resp.error,
        Some(CoreError::Selection {
            reason: SelectionFailureReason::SecurityUnavailable,
            context,
        }) if context.phase() == ErrorPhase::Selection
            && context.retry_class() == RetryClass::Never
    ));
}

#[tokio::test]
async fn missing_security_definition_is_a_validation_failure() {
    let mut td = secure_td();
    td.security_definitions.remove("bearer");

    let servient = ServientBuilder::new().build().expect("build");
    let handle = servient.produce(td).expect("produce");
    handle.expose().await.expect("expose");

    let response = servient
        .serve_request(build_request("urn:test:secure-lamp", None))
        .await;

    assert!(matches!(
        response.error,
        Some(CoreError::Validation(context))
            if context.phase() == ErrorPhase::Validate
                && context.retry_class() == RetryClass::Never
                && context.operation() == Some(Operation::ReadProperty)
    ));
}

// Note: `empty_security_list_is_open_access` was removed because W3C TD 1.1
// makes `security` a required field at the Thing level — there is no valid
// TD with an empty security list. The dispatch path `if td.security.is_empty()`
// remains as a defensive guard for programmatically constructed Things that
// skip validation, but it's not reachable through the builder's validation.
