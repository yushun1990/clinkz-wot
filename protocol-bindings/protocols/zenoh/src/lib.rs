#![no_std]

#[cfg(all(feature = "zenoh", feature = "zenoh-pico"))]
compile_error!(
    "Only one concrete zenoh runtime backend can be enabled. Choose \
     `zenoh` for the std Rust zenoh backend or `zenoh-pico` \
     for the constrained zenoh-pico backend."
);

#[cfg(feature = "zenoh")]
extern crate std;

extern crate alloc;

mod error;
mod form;
#[cfg(any(feature = "zenoh", feature = "zenoh-pico"))]
mod runtime;

pub use error::{ZenohBindingError, ZenohBindingResult};
pub use form::{
    extract_zenoh_metadata, extract_zenoh_target, is_zenoh_form, is_zenoh_form_target,
    plan_zenoh_affordance_operation, plan_zenoh_affordance_operation_with_criteria,
    plan_zenoh_operation, zenoh_operation_kind, ZenohAffordanceOperationPlan,
    ZenohFormMetadata, ZenohFormTarget, ZenohOperationKind, ZenohOperationPlan,
    CZ_ZENOH_CONGESTION_CONTROL, CZ_ZENOH_ENCODING, CZ_ZENOH_KEY_EXPR, CZ_ZENOH_PRIORITY,
    CZ_ZENOH_QOS, ZENOH_SCHEME,
};
#[cfg(feature = "zenoh-pico")]
pub use runtime::{
    ZenohPicoError, ZenohPicoErrorKind, ZenohPicoPlatform, ZenohPicoRequest, ZenohPicoTransport,
};
#[cfg(feature = "zenoh")]
pub use runtime::{
    build_zenoh_transport_request, NoZenohTransport, SharedZenohTransport, ZenohBinding,
    ZenohSessionTransport, ZenohSubscription, ZenohTransport, ZenohTransportRequest,
};
