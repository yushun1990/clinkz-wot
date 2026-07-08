#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod binding;
#[cfg(feature = "async")]
pub mod binding_facade;
pub mod error;
pub mod event;
pub mod identity;
pub mod inbound;
pub mod interaction;
pub mod payload;
pub mod security;
pub mod sync;
pub mod thing;
pub mod transport;

pub use binding::SubscriptionGuard;
#[cfg(feature = "async")]
pub use binding::{BindingRequest, ClientBinding, ClientBindingFactory};
#[cfg(feature = "async")]
pub use binding_facade::{
    ClientOnly, ProtocolBinding, ProtocolId, ServerOnly, client_only, server_only,
};
pub use error::{CoreError, CoreResult};
pub use event::{
    DEFAULT_SUBSCRIPTION_CAPACITY, EventBroker, EventName, PublisherSink, Subscription,
    SubscriptionSender,
};
#[cfg(feature = "async")]
pub use event::EventStream;
pub use identity::{CorrelationId, ThingId};
#[cfg(feature = "async")]
pub use inbound::Dispatch;
#[cfg(feature = "async")]
pub use inbound::FanInSender;
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
