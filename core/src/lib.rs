#![no_std]

#[cfg(any(feature = "std", test))]
extern crate std;

extern crate alloc;

pub mod binding_compiler;
pub mod deadline;
pub mod error;
pub mod event;
pub mod handler;
pub mod identity;
pub mod inbound;
pub mod interaction;
pub mod outbound;
pub mod payload;
pub mod plan;
pub mod security;
pub mod status;
pub mod sync;
pub mod thing;
pub mod transport;

pub use binding_compiler::{
    BindingArtifact, BindingArtifactCompatibility, BindingArtifactEnvelope,
    BindingArtifactFootprint, BindingArtifactIdentity, BindingArtifactRef,
    BindingArtifactRejection, BindingArtifactRejectionReason, BindingArtifactRole,
    BindingCompilerBounds, BindingCompilerExtension, BindingCompilerFailure, BindingCompilerInput,
    BindingCompilerOutput, BindingCompilerStep, StaticBindingCompilerRegistration,
};
#[cfg(feature = "std")]
pub use binding_compiler::{
    HostBindingArtifact, HostBindingCompilerCursor, HostBindingCompilerRegistration,
};
pub use deadline::Deadline;
pub use error::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, RetryClass, SecurityFailureReason,
    SelectionFailureReason,
};
#[cfg(feature = "async")]
pub use event::EventStream;
pub use event::{
    DEFAULT_SUBSCRIPTION_CAPACITY, EventBroker, EventName, PublisherSink, Subscription,
    SubscriptionSender,
};
pub use handler::{
    CancellationView, HandlerContext, HandlerFootprint, HandlerStep, ReadPropertyHandler,
    StaticHandlerRegistration, SubscriptionAcceptance,
};
pub use identity::{
    ActionInvocationRef, ActiveRouteId, AffordanceSlotId, BindingConfigurationDigest,
    BindingGeneration, BindingId, BindingSlotId, CleanupSlotId, CorrelationId, HandlerSlotId,
    PlanId, PlanSetGeneration, PlanSlotId, PreparedRouteId, PreparedRouteKey, SubscriptionId,
    SubscriptionSlotId, ThingId, ThingSlotId,
};
#[cfg(feature = "async")]
pub use inbound::Dispatch;
pub use inbound::{
    BindingContext, InboundDispatcher, InboundRequest, InboundResponse, ServerBinding,
};
pub use interaction::{
    AcceptHint, BindingResponseMetadata, InteractionInput, InteractionOptions, InteractionOutput,
    InteractionOutputMetadata, InteractionStatus, MediaType, ResponsePayloadRole,
    ResponseSelection,
};
pub use outbound::SubscriptionGuard;
#[cfg(feature = "async")]
pub use outbound::{BindingRequest, ClientBinding};
pub use payload::{CodecInput, Payload, PayloadCodec};
pub use plan::{BindingCandidate, LogicalInteractionPlan};
pub use security::{
    AuthMaterial, BasicSecurityProvider, BearerSecurityProvider, CredentialStore, Credentials,
    InMemoryCredentialStore, NoSecurityProvider, Principal, PrincipalId, SecurityContext,
    SecurityProvider, check_scopes,
};
pub use status::{
    CleanupHandle, CleanupOperation, CleanupOutcome, CleanupRecord, PendingWork, PendingWorkClass,
    ProcessEvent, ProcessTerminal, StartStatus, StepStatus,
};
pub use sync::WotLock;
pub use thing::{
    ActionCancelHandler, ActionHandler, ActionQueryHandler, AffordanceKind, AffordanceTarget,
    CancelSlot, EventSubscribeHandler, EventUnsubscribeHandler, ExposedThing, InvokeSlot,
    LocalThing, ObserveSlot, PropertyObserveHandler, PropertyReadHandler, PropertyUnobserveHandler,
    PropertyWriteHandler, PushFn, QuerySlot, ReadSlot, SubscribeSlot, UnobserveSlot,
    UnsubscribeSlot, WriteSlot,
};
#[cfg(feature = "async")]
pub use thing::{
    AsyncActionCancelHandler, AsyncActionHandler, AsyncActionQueryHandler,
    AsyncEventSubscribeHandler, AsyncEventUnsubscribeHandler, AsyncPropertyObserveHandler,
    AsyncPropertyReadHandler, AsyncPropertyUnobserveHandler, AsyncPropertyWriteHandler,
    ConsumedThing,
};
pub use transport::{TransportAdapter, TransportRequest, TransportResponse};
