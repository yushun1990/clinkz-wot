//! Unified error tree: structured conversions, predicates, and source chains.
//!
//! Verifies:
//! - `From<BindingError> for CoreError` preserves structural variants
//!   (`UnknownAffordance`, `UnsupportedOperation`) and funnels the rest
//!   through `InvalidInteraction` with verbatim Display text.
//! - `From<BindingError> for ServientError` keeps the original `BindingError`
//!   typed payload intact (no lossy collapse).
//! - `ServientError` predicates (`is_missing_handler`, `is_security`,
//!   `is_timeout`, `is_discovery`, `is_binding`) and accessors
//!   (`as_core`, `as_binding`, `as_discovery`).
//! - `ServientError::source()` chains walk through to wrapped errors.

#![cfg(feature = "std")]

use clinkz_wot_core::{AffordanceKind, CoreError, SecurityError};
use clinkz_wot_discovery::DiscoveryError;
use clinkz_wot_protocol_bindings::BindingError;
use clinkz_wot_servient::ServientError;
use clinkz_wot_td::data_type::{Operation, ResolveFormHrefError};

// --- From<BindingError> for CoreError --------------------------------------

#[test]
fn binding_error_unknown_affordance_maps_structurally_to_core_error() {
    let binding_err = BindingError::UnknownAffordance {
        kind: AffordanceKind::Property,
        name: "temperature".into(),
    };
    let core_err: CoreError = binding_err.into();
    assert!(
        matches!(
            core_err,
            CoreError::UnknownAffordance { kind: AffordanceKind::Property, .. }
        ),
        "UnknownAffordance should map structurally, got {core_err:?}"
    );
}

#[test]
fn binding_error_unsupported_operation_maps_structurally_to_core_error() {
    let binding_err = BindingError::UnsupportedOperation("readproperty".into());
    let core_err: CoreError = binding_err.into();
    assert!(
        matches!(core_err, CoreError::UnsupportedOperation(_)),
        "UnsupportedOperation should map structurally, got {core_err:?}"
    );
}

#[test]
fn binding_error_metadata_mismatch_funnels_through_invalid_interaction() {
    let binding_err = BindingError::MetadataMismatch("cz-zenoh:qos missing".into());
    let core_err: CoreError = binding_err.into();
    let message = match core_err {
        CoreError::InvalidInteraction(msg) => msg,
        ref other => panic!("expected InvalidInteraction, got {other:?}"),
    };
    assert!(
        message.contains("cz-zenoh:qos missing"),
        "Display text should be preserved verbatim, got: {message}"
    );
}

#[test]
fn binding_error_target_resolution_funnels_through_invalid_interaction() {
    let inner = ResolveFormHrefError::Resolve("rfc3986 failure".into());
    let binding_err = BindingError::TargetResolution(inner);
    let core_err: CoreError = binding_err.into();
    assert!(
        matches!(core_err, CoreError::InvalidInteraction(_)),
        "TargetResolution should funnel through InvalidInteraction, got {core_err:?}"
    );
}

// --- From<BindingError> for ServientError ----------------------------------

#[test]
fn binding_error_into_servient_error_preserves_typed_payload() {
    // The Servient layer keeps the original BindingError variant intact
    // (unlike the lossy path through CoreError). This is the recommended
    // path for callers that need the structured binding taxonomy.
    let binding_err = BindingError::FormNotInAffordance;
    let servient_err: ServientError = binding_err.into();
    assert!(
        matches!(servient_err.as_binding(), Some(BindingError::FormNotInAffordance)),
        "ServientError should preserve the FormNotInAffordance payload"
    );
    assert!(servient_err.is_binding());
    assert!(!servient_err.is_missing_handler());
}

// --- ServientError predicates + accessors ----------------------------------

#[test]
fn is_missing_handler_detects_serve_missing_handler() {
    let err = ServientError::Serve(CoreError::MissingHandler {
        target: clinkz_wot_core::AffordanceTarget::Property("temperature".into()),
        operation: Operation::ReadProperty,
    });
    assert!(err.is_missing_handler());
    assert!(err.as_core().is_some());
    assert!(err.as_binding().is_none());
}

#[test]
fn is_security_detects_serve_security_variants() {
    let err = ServientError::Serve(CoreError::Security(
        SecurityError::MissingCredentials,
    ));
    assert!(err.is_security());
}

#[test]
fn is_timeout_detects_timeout_and_timeout_unsupported() {
    let timeout = ServientError::Serve(CoreError::Timeout);
    let unsupported = ServientError::Serve(CoreError::TimeoutUnsupported);
    assert!(timeout.is_timeout());
    assert!(unsupported.is_timeout());
}

#[test]
fn is_discovery_and_as_discovery_round_trip() {
    let err = ServientError::Discovery(DiscoveryError::LeaseExpired);
    assert!(err.is_discovery());
    assert!(matches!(err.as_discovery(), Some(DiscoveryError::LeaseExpired)));
    assert!(!err.is_binding());
    assert!(err.as_core().is_none());
}

#[test]
fn lifecycle_variants_have_no_core_binding_discovery_payload() {
    let id = clinkz_wot_core::ThingId::from("urn:test:lifecycle");
    let cases = [
        ServientError::AlreadyExposed(id.clone()),
        ServientError::ExposedThingNotFound(id),
        ServientError::MissingThingId,
    ];
    for err in cases {
        assert!(err.as_core().is_none(), "{err:?} should have no CoreError");
        assert!(err.as_binding().is_none(), "{err:?} should have no BindingError");
        assert!(err.as_discovery().is_none(), "{err:?} should have no DiscoveryError");
    }
}

// --- source() chain ---------------------------------------------------------

#[test]
fn servient_error_source_walks_through_to_wrapped_core_error() {
    let core = CoreError::Payload("decode failed".into());
    let servient_err: ServientError = core.into();
    let source = std::error::Error::source(&servient_err);
    assert!(
        source.is_some(),
        "ServientError::Serve should expose CoreError as source"
    );
}

#[test]
fn discovery_error_source_walks_through_to_validate_error() {
    use clinkz_wot_td::validate::ValidateError;
    let validate_err = ValidateError::MissingRequiredField("id".into());
    let discovery_err = DiscoveryError::InvalidThingDescription(validate_err);
    let servient_err: ServientError = discovery_err.into();
    // ServientError → DiscoveryError → ValidateError chain should be walkable.
    let source_l1 = std::error::Error::source(&servient_err);
    assert!(source_l1.is_some(), "top-level source missing");
    let discovery = source_l1.unwrap();
    let source_l2 = std::error::Error::source(discovery);
    assert!(
        source_l2.is_some(),
        "DiscoveryError::InvalidThingDescription should expose ValidateError as source"
    );
}

#[test]
fn binding_error_source_walks_through_to_resolve_form_href_error() {
    let inner = ResolveFormHrefError::TemplateBase("zenoh://{x}/".into());
    let binding_err = BindingError::TargetResolution(inner);
    let source = std::error::Error::source(&binding_err);
    assert!(
        source.is_some(),
        "BindingError::TargetResolution should expose ResolveFormHrefError as source"
    );
}
