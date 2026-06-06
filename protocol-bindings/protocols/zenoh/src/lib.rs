#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod error;
mod form;
#[cfg(feature = "zenoh-runtime")]
mod runtime;

pub use error::{ZenohBindingError, ZenohBindingResult};
#[cfg(feature = "std")]
pub use form::SharedZenohTransport;
pub use form::{
    CZ_ZENOH_CONGESTION_CONTROL, CZ_ZENOH_ENCODING, CZ_ZENOH_KEY_EXPR, CZ_ZENOH_PRIORITY,
    CZ_ZENOH_QOS, NoZenohTransport, ZENOH_SCHEME, ZenohAffordanceOperationPlan, ZenohBinding,
    ZenohFormMetadata, ZenohFormTarget, ZenohOperationKind, ZenohOperationPlan, ZenohTransport,
    ZenohTransportRequest, build_zenoh_transport_request, extract_zenoh_metadata,
    extract_zenoh_target, is_zenoh_form, is_zenoh_form_target, plan_zenoh_affordance_operation,
    plan_zenoh_affordance_operation_with_criteria, plan_zenoh_operation, zenoh_operation_kind,
};
#[cfg(feature = "zenoh-runtime")]
pub use runtime::ZenohSessionTransport;
