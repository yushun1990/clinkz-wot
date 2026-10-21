//! Native Zenoh transport values used by the deferred backend probes.
use crate::ZenohOperationPlan;
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc};
use clinkz_wot_core::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, InteractionInput, InteractionOutput, Payload,
    RetryClass, Subscription,
};
/// Native backend lease, independent of the removed WoT client/dispatch SPI.
/// This deferred SDK adapter is not the target long-lived lifecycle contract.
pub trait ZenohSubscriptionLease: Send + Sync {
    fn close(self: Box<Self>);
}
/// Request passed from the zenoh binding planner to a zenoh transport adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZenohTransportRequest {
    /// Concrete zenoh execution plan, shared by reference so repeated
    /// interactions against a cached plan avoid cloning the plan's strings on
    /// every request.
    pub plan: Arc<ZenohOperationPlan>,
    /// Optional encoded payload from the WoT interaction input.
    pub payload: Option<Payload>,
    /// Runtime parameters supplied by the caller.
    pub parameters: BTreeMap<String, String>,
}

/// Transport adapter contract for concrete zenoh runtime integrations.
///
/// This trait deliberately avoids depending on a concrete zenoh session type so
/// std, constrained, and test runtimes can provide their own integration layer.
/// The receiver is `&self` (baseline addendum §2.4 / §7): each concrete backend
/// owns its own interior mutability for I/O state.
pub trait ZenohTransport {
    /// Executes a planned zenoh operation.
    fn execute(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput>;

    /// Opens a long-lived zenoh subscription for streaming observe/subscribe
    /// operations.
    ///
    /// Returns a consumer-side [`Subscription`] for draining samples and a
    /// [`ZenohSubscriptionLease`] that owns the underlying zenoh subscriber.
    /// The default implementation returns `UnsupportedOperation`.
    fn open_subscription(
        &self,
        _request: ZenohTransportRequest,
    ) -> CoreResult<(Subscription, Box<dyn ZenohSubscriptionLease>)> {
        Err(CoreError::UnsupportedOperation(ErrorContext::new(
            ErrorPhase::Binding,
            RetryClass::Never,
        )))
    }
}

/// Builds a transport request from a zenoh execution plan and WoT interaction input.
pub fn build_zenoh_transport_request(
    plan: Arc<ZenohOperationPlan>,
    input: InteractionInput,
) -> ZenohTransportRequest {
    ZenohTransportRequest {
        plan,
        payload: input.data,
        parameters: input.uri_variables,
    }
}
