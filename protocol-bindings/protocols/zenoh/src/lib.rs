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
#[cfg(feature = "zenoh")]
pub use form::SharedZenohTransport;
pub use form::{
    build_zenoh_transport_request, extract_zenoh_metadata, extract_zenoh_target, is_zenoh_form,
    is_zenoh_form_target, plan_zenoh_affordance_operation,
    plan_zenoh_affordance_operation_with_criteria, plan_zenoh_operation, zenoh_operation_kind,
    NoZenohTransport, ZenohAffordanceOperationPlan, ZenohBinding, ZenohFormMetadata,
    ZenohFormTarget, ZenohOperationKind, ZenohOperationPlan, ZenohTransport, ZenohTransportRequest,
    CZ_ZENOH_CONGESTION_CONTROL, CZ_ZENOH_ENCODING, CZ_ZENOH_KEY_EXPR, CZ_ZENOH_PRIORITY,
    CZ_ZENOH_QOS, ZENOH_SCHEME,
};
#[cfg(feature = "zenoh-pico")]
pub use runtime::{
    ZenohPicoError, ZenohPicoErrorKind, ZenohPicoPlatform, ZenohPicoRequest, ZenohPicoTransport,
};
#[cfg(feature = "zenoh")]
pub use runtime::{ZenohSessionTransport, ZenohSubscription};
