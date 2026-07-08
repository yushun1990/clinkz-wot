#![no_std]

#[cfg(any(feature = "zenoh", feature = "zenoh-pico"))]
mod zenoh;

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
#[cfg(feature = "zenoh")]
mod protocol_binding;
#[cfg(any(feature = "zenoh", feature = "zenoh-pico"))]
mod runtime;
#[cfg(feature = "zenoh")]
mod server;

pub use error::{ZenohBindingError, ZenohBindingResult};
pub use form::{
    CZ_ZENOH_CONGESTION_CONTROL, CZ_ZENOH_PRIORITY, CZ_ZENOH_QOS, ZENOH_SCHEME,
    ZenohAffordanceOperationPlan, ZenohFormMetadata, ZenohFormTarget, ZenohOperationKind,
    ZenohOperationPlan, extract_zenoh_metadata, extract_zenoh_target, is_zenoh_form,
    is_zenoh_form_target, plan_zenoh_affordance_operation,
    plan_zenoh_affordance_operation_with_criteria, plan_zenoh_operation, try_extract_zenoh_target,
    zenoh_operation_kind,
};
#[cfg(feature = "zenoh")]
pub use protocol_binding::{client, server, shared};
#[cfg(any(feature = "zenoh", feature = "zenoh-pico"))]
pub use runtime::ZenohRuntimeTransport;
#[cfg(feature = "zenoh")]
pub use runtime::{SharedZenohTransport, ZenohSessionTransport, ZenohSubscription};
#[cfg(feature = "zenoh-pico")]
pub use runtime::{
    ZenohPicoError, ZenohPicoErrorKind, ZenohPicoPlatform, ZenohPicoRequest, ZenohPicoTransport,
};
#[cfg(feature = "zenoh")]
pub use server::ZenohServerBinding;
#[cfg(any(feature = "zenoh", feature = "zenoh-pico"))]
pub type ZenohBinding = zenoh::ZenohBindingTransport<ZenohRuntimeTransport>;
#[cfg(any(feature = "zenoh", feature = "zenoh-pico"))]
pub use zenoh::{
    ZenohBindingTransport, ZenohTransport, ZenohTransportRequest, build_zenoh_transport_request,
};
