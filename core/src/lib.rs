//! Protocol-neutral vocabulary and execution contracts.
//!
//! Operation vocabulary is available without pointer atomics. The current
//! execution representations require pointer atomics; their APIs are absent on
//! targets without that capability. TD tooling can use the vocabulary without
//! inheriting an Arc requirement. This is not a no-atomic execution claim.
#![no_std]

#[cfg(any(feature = "std", test))]
extern crate std;

extern crate alloc;

#[cfg(target_has_atomic = "ptr")]
pub mod artifact;
#[cfg(target_has_atomic = "ptr")]
pub mod binding;
#[cfg(target_has_atomic = "ptr")]
pub mod deadline;
#[cfg(target_has_atomic = "ptr")]
pub mod error;
#[cfg(target_has_atomic = "ptr")]
pub mod handler;
#[cfg(target_has_atomic = "ptr")]
pub mod identity;
#[cfg(target_has_atomic = "ptr")]
pub mod interaction;
pub mod operation;
#[cfg(target_has_atomic = "ptr")]
pub mod outbound;
#[cfg(target_has_atomic = "ptr")]
pub mod payload;
#[cfg(target_has_atomic = "ptr")]
pub mod plan;
#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
pub mod producer;
#[cfg(target_has_atomic = "ptr")]
mod response;
#[cfg(target_has_atomic = "ptr")]
pub mod security;
#[cfg(target_has_atomic = "ptr")]
pub mod status;
#[cfg(target_has_atomic = "ptr")]
pub mod sync;
#[cfg(target_has_atomic = "ptr")]
pub mod thing;

#[cfg(target_has_atomic = "ptr")]
pub use artifact::{
    BindingArtifact, BindingArtifactCompatibility, BindingArtifactEnvelope,
    BindingArtifactFootprint, BindingArtifactIdentity, BindingArtifactRef,
    BindingArtifactRejection, BindingArtifactRejectionReason, BindingArtifactRole,
    ConsumerBindingConfiguration,
};
#[cfg(feature = "std")]
#[cfg(target_has_atomic = "ptr")]
pub use artifact::{HostArtifactStorage, HostBindingArtifact};
#[cfg(target_has_atomic = "ptr")]
pub use binding::{
    BindingCallSettlement, BindingCancellationDisposition, BindingDeliveryOutcome,
    BindingExecutionSupport, BindingIngressLimits, BindingIngressPolicy, BindingInputRejection,
    BindingLifetimeFootprint, BindingOperationalError, BindingRegistrationCapabilities,
    BindingRegistrationIdentity, BindingResourceContract, BindingResourceDeclarations,
    BindingStateLayout, BindingStatusPolicy, BindingTransientFootprint, CleanupPhaseContext,
    CleanupReservation, CleanupTransferAcceptance, CleanupTransferEnvelope, CleanupTransferRequest,
    CleanupTransferTarget, ClientRequestSlot, CollisionDomainId, EndpointReservationKey,
    NoCleanupSuccessor, PollClientBinding, PollServerBinding, PrepareInput, RouteAcceptClaim,
    RouteAcceptClaimError, RouteAcceptEvent, RouteAcceptLease, RouteActivationOutcome,
    RouteActivationPermit, RouteCleanupOutcome, RouteCleanupSuccessor, RouteCommitOutcome,
    RouteInboundRequest, RouteInboundResponse, RoutePreparationVisibility, RoutePrepareOutcome,
    RouteReadinessOutcome, RouteReadinessSlot, RouteReservationIdentity, RouteResponseOpportunity,
    RouteTerminal, ServerResponseSlot, ServerRouteSlot, ServingActivationAuthority,
    StaticBindingComponents, StaticBindingRegistration, StaticBindingRegistrationInput,
    StaticConsumerBindingRegistration, StaticConsumerBindingRegistrationInput,
    StaticConsumerPropertyReadSlot, StaticServerProgress,
};
#[cfg(feature = "std")]
#[cfg(target_has_atomic = "ptr")]
pub use binding::{
    HostActiveRouteGuard, HostBindingCall, HostBindingCallBox, HostBindingRegistration,
    HostBindingRegistrationInput, HostCommittedRouteGuard, HostConsumerBindingRegistration,
    HostConsumerBindingRegistrationInput, HostPreparedRouteGuard, HostRouteCleanupSuccessor,
    HostShutdownRouteGuard, RouteAbortInput, RouteServerBinding, RouteShutdownInput,
};
#[cfg(target_has_atomic = "ptr")]
pub use deadline::Deadline;
#[cfg(target_has_atomic = "ptr")]
pub use error::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, ResourceFailureDomain, RetryClass,
    SecurityFailureReason, SelectionFailureReason,
};
#[cfg(target_has_atomic = "ptr")]
pub use handler::{
    CancellationView, HandlerContext, HandlerFootprint, HandlerStep, ReadPropertyHandler,
    StaticHandlerRegistration,
};
#[cfg(target_has_atomic = "ptr")]
pub use identity::{
    ActionInvocationRef, ActiveRouteId, AffordanceSlotId, BindingConfigurationDigest,
    BindingGeneration, BindingId, BindingSlotId, CleanupSlotId, CorrelationId, HandlerSlotId,
    PlanId, PlanSetGeneration, PlanSlotId, PreparedRouteId, PreparedRouteKey, SubscriptionId,
    SubscriptionSlotId, ThingId, ThingSlotId,
};
#[cfg(target_has_atomic = "ptr")]
pub use interaction::{
    AcceptHint, BindingResponseMetadata, InteractionInput, InteractionOptions, InteractionOutput,
    InteractionOutputMetadata, InteractionStatus, MediaType, ResponsePayloadRole,
    ResponseSelection,
};
pub use operation::Operation;
#[cfg(target_has_atomic = "ptr")]
pub use outbound::OutboundRequest;
#[cfg(target_has_atomic = "ptr")]
pub use payload::{CodecInput, Payload, PayloadCodec};
#[cfg(target_has_atomic = "ptr")]
pub use plan::{BindingCandidate, LogicalInteractionPlan};
#[cfg(target_has_atomic = "ptr")]
pub use response::consumer_result_backing;
#[cfg(target_has_atomic = "ptr")]
pub use response::validate_untrusted_binding_output;
#[cfg(target_has_atomic = "ptr")]
pub use security::{AuthMaterial, Principal, PrincipalId, check_scopes};
#[cfg(target_has_atomic = "ptr")]
pub use status::{
    CleanupHandle, CleanupOperation, CleanupOutcome, CleanupRecord, PendingWork, PendingWorkClass,
    ProcessEvent, ProcessTerminal, StartStatus, StepStatus,
};
#[cfg(target_has_atomic = "ptr")]
pub use sync::WotLock;
#[cfg(target_has_atomic = "ptr")]
pub use thing::{AffordanceKind, AffordanceTarget};
