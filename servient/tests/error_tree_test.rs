//! Unified error tree: structured conversions, typed accessors, and source chains.
//!
//! Verifies:
//! - `From<BindingError> for CoreError` maps binding failures into the bounded
//!   core taxonomy without retaining arbitrary error strings.
//! - `From<BindingError> for ServientError` keeps the original `BindingError`
//!   typed payload intact (no lossy collapse).
//! - `ServientError` typed accessors preserve their wrapped values.
//! - `ServientError::source()` chains walk through to wrapped errors.

#![cfg(feature = "std")]

use clinkz_wot_core::{
    AffordanceKind, CoreError, ErrorContext, ErrorPhase, RetryClass, SelectionFailureReason,
};
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
            CoreError::Selection {
                reason: SelectionFailureReason::AffordanceMissing,
                ..
            }
        ),
        "unknown affordance should retain its selection category, got {core_err:?}"
    );
}

#[test]
fn binding_error_unsupported_operation_maps_to_selection_reason() {
    let binding_err = BindingError::UnsupportedOperation("readproperty".into());
    let core_err: CoreError = binding_err.into();
    assert!(
        matches!(
            core_err,
            CoreError::Selection {
                reason: SelectionFailureReason::NoFormSupportsOperation,
                ..
            }
        ),
        "unsupported binding operation should remain a selection failure, got {core_err:?}"
    );
}

#[test]
fn binding_error_metadata_mismatch_maps_to_selection() {
    let binding_err = BindingError::MetadataMismatch("cz-zenoh:qos missing".into());
    let core_err: CoreError = binding_err.into();
    assert!(
        matches!(core_err, CoreError::Selection { .. }),
        "metadata mismatch should remain a selection failure, got {core_err:?}"
    );
}

#[test]
fn binding_error_target_resolution_maps_structurally_to_selection() {
    let inner = ResolveFormHrefError::Resolve("rfc3986 failure".into());
    let binding_err = BindingError::TargetResolution(inner);
    let core_err: CoreError = binding_err.into();
    assert!(
        matches!(
            core_err,
            CoreError::Selection {
                reason: SelectionFailureReason::TargetResolutionFailed,
                ..
            }
        ),
        "target resolution should retain its structured reason, got {core_err:?}"
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
        matches!(
            servient_err.as_binding(),
            Some(BindingError::FormNotInAffordance)
        ),
        "ServientError should preserve the FormNotInAffordance payload"
    );
    assert!(servient_err.is_binding());
}

// --- ServientError typed accessors -----------------------------------------

#[test]
fn as_core_preserves_structured_handler_error() {
    let err = ServientError::Serve(CoreError::UnsupportedOperation(
        ErrorContext::new(ErrorPhase::Handler, RetryClass::Never)
            .with_operation(Operation::ReadProperty),
    ));
    let core = err.as_core().expect("serve error retains the core value");
    assert!(matches!(core, CoreError::UnsupportedOperation(_)));
    assert_eq!(core.context().phase(), ErrorPhase::Handler);
    assert_eq!(core.context().operation(), Some(Operation::ReadProperty));
    assert!(err.as_binding().is_none());
}

#[test]
fn is_discovery_and_as_discovery_round_trip() {
    let err = ServientError::Discovery(DiscoveryError::LeaseExpired);
    assert!(err.is_discovery());
    assert!(matches!(
        err.as_discovery(),
        Some(DiscoveryError::LeaseExpired)
    ));
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
        assert!(
            err.as_binding().is_none(),
            "{err:?} should have no BindingError"
        );
        assert!(
            err.as_discovery().is_none(),
            "{err:?} should have no DiscoveryError"
        );
    }
}

// --- source() chain ---------------------------------------------------------

#[test]
fn servient_error_source_walks_through_to_wrapped_core_error() {
    let core = CoreError::Payload(
        ErrorContext::new(ErrorPhase::Codec, RetryClass::Never)
            .with_redacted_cause(1, "decode failed"),
    );
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
