#![no_std]

#[cfg(any(feature = "std", test))]
extern crate std;

extern crate alloc;

pub mod binding;
pub mod error;
pub mod event;
pub mod identity;
pub mod inbound;
pub mod interaction;
pub mod payload;
pub mod security;
pub mod status;
pub mod sync;
pub mod thing;
pub mod transport;

pub use binding::SubscriptionGuard;
#[cfg(feature = "async")]
pub use binding::{BindingRequest, ClientBinding};
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
pub use identity::{
    ActionInvocationRef, ActiveRouteId, AffordanceSlotId, BindingGeneration, BindingId,
    BindingSlotId, CleanupSlotId, CorrelationId, HandlerSlotId, PlanId, PlanSlotId,
    PreparedRouteId, PreparedRouteKey, SubscriptionId, SubscriptionSlotId, ThingId, ThingSlotId,
};
#[cfg(feature = "async")]
pub use inbound::Dispatch;
pub use inbound::{
    BindingContext, InboundDispatcher, InboundRequest, InboundResponse, ServerBinding,
};
pub use interaction::{
    AcceptHint, InteractionInput, InteractionOptions, InteractionOutput, InteractionStatus,
    MediaType,
};
pub use payload::{CodecInput, Payload, PayloadCodec};
pub use security::{
    AuthMaterial, BasicSecurityProvider, BearerSecurityProvider, CredentialStore, Credentials,
    InMemoryCredentialStore, NoSecurityProvider, Principal, PrincipalId, SecurityContext,
    SecurityError, SecurityProvider, check_scopes,
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
