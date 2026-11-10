//! v6 authority: current Property Read carriers and route-stage/plan-set APIs
//! are restricted implementations, not the general capability SPI. See
//! docs/spec/authority.toml and ADR-0025; no compatibility facade is promised.
//!
//! Protocol-neutral vocabulary and execution contracts.
//!
//! Semantic values, borrowed output and the finite execution kernel are available
//! without std or pointer atomics. Host owners and historical execution carriers
//! retain their platform requirements; see `execution` for the shared boundary.
#![no_std]

#[cfg(any(feature = "std", test))]
extern crate std;

extern crate alloc;

#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
pub mod action;
#[cfg(target_has_atomic = "ptr")]
pub mod artifact;
#[cfg(target_has_atomic = "ptr")]
pub mod binding;
pub mod collection;
#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
pub mod consumer;
#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
pub mod consumer_action;
pub mod data_schema;
pub mod deadline;
pub mod error;
pub mod execution;
#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
pub mod finite;
#[cfg(target_has_atomic = "ptr")]
pub mod handler;
#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
pub mod host_accounts;
#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
pub mod host_change;
pub mod identity;
pub mod interaction;
#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
pub mod notification;
pub mod operation;
#[cfg(target_has_atomic = "ptr")]
pub mod outbound;
pub mod payload;
#[cfg(target_has_atomic = "ptr")]
pub mod plan;
#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
pub mod producer;
#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
pub mod publication;
#[cfg(target_has_atomic = "ptr")]
mod response;
pub mod security;
pub mod status;
#[cfg(target_has_atomic = "ptr")]
pub mod sync;
pub mod thing;
pub mod uri_variables;

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
pub use deadline::Deadline;
pub use error::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, ResourceFailureDomain, RetryClass,
    SecurityFailureReason, SelectionFailureReason,
};
#[cfg(target_has_atomic = "ptr")]
pub use handler::{
    CancellationView, HandlerContext, HandlerFootprint, HandlerStep, ReadPropertyHandler,
    StaticHandlerRegistration,
};
pub use identity::{
    ActionInvocationRef, ActiveRouteId, AffordanceSlotId, BindingConfigurationDigest,
    BindingGeneration, BindingId, BindingSlotId, CleanupSlotId, CorrelationId, HandlerSlotId,
    PlanId, PlanSetGeneration, PlanSlotId, PreparedRouteId, PreparedRouteKey, SubscriptionId,
    SubscriptionSlotId, ThingId, ThingSlotId,
};
pub use interaction::{
    AcceptHint, BindingResponseMetadata, InteractionOptions, InteractionOutputMetadata,
    InteractionOutputView, InteractionStatus, MediaType, ResponsePayloadRole, ResponseSelection,
};
#[cfg(target_has_atomic = "ptr")]
pub use interaction::{InteractionInput, InteractionOutput, interaction_output_backing};
pub use operation::Operation;
#[cfg(target_has_atomic = "ptr")]
pub use outbound::OutboundRequest;
pub use payload::{CodecInput, PayloadView};
#[cfg(target_has_atomic = "ptr")]
pub use payload::{Payload, PayloadCodec};
#[cfg(target_has_atomic = "ptr")]
pub use plan::{BindingCandidate, LogicalInteractionPlan};
#[cfg(target_has_atomic = "ptr")]
pub use response::consumer_result_backing;
#[cfg(target_has_atomic = "ptr")]
pub use response::validate_untrusted_binding_output;
pub use security::{AuthMaterial, Principal, PrincipalId, check_scopes};
pub use status::{
    CleanupHandle, CleanupOperation, CleanupOutcome, CleanupRecord, PendingWork, PendingWorkClass,
    ProcessEvent, ProcessTerminal, StartStatus, StepStatus,
};
#[cfg(target_has_atomic = "ptr")]
pub use sync::WotLock;
#[cfg(target_has_atomic = "ptr")]
pub use thing::AffordanceTarget;
pub use thing::{AffordanceKind, AffordanceSite};
