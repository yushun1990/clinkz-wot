#![no_std]

#[cfg(feature = "std")]
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
pub mod sync;
pub mod thing;
pub mod transport;

#[cfg(feature = "async")]
pub use binding::{BindingRequest, ClientBinding};
pub use binding::SubscriptionGuard;
pub use error::{CoreError, CoreResult};
pub use event::{
    BrokerDataSink, DEFAULT_SUBSCRIPTION_CAPACITY, EventBroker, EventName, PublisherSink,
    Subscription, SubscriptionSender,
};
pub use identity::{CorrelationId, ThingId};
pub use inbound::{InboundDispatcher, InboundRequest, InboundResponse, ServerBinding};
#[cfg(feature = "std")]
pub use inbound::FanInSender;
pub use interaction::{
    AcceptHint, InteractionInput, InteractionOptions, InteractionOutput, InteractionStatus,
    MediaType,
};
pub use payload::{CodecInput, Payload, PayloadCodec};
pub use security::{
    AuthMaterial, CredentialStore, Credentials, InMemoryCredentialStore, Principal, PrincipalId,
    SecurityContext, SecurityError, SecurityProvider, check_scopes,
};
pub use sync::WotLock;
pub use thing::{
    ActionCancelHandler, ActionHandler, ActionQueryHandler, AffordanceKind, AffordanceTarget,
    DataSink, EventSubscribeHandler, EventUnsubscribeHandler, LocalExposedThing, LocalThing,
    PropertyObserveHandler, PropertyReadHandler, PropertyUnobserveHandler, PropertyWriteHandler,
};
#[cfg(feature = "async")]
pub use thing::{
    AsyncActionCancelHandler, AsyncActionHandler, AsyncActionQueryHandler,
    AsyncEventSubscribeHandler, AsyncEventUnsubscribeHandler, AsyncPropertyObserveHandler,
    AsyncPropertyReadHandler, AsyncPropertyUnobserveHandler, AsyncPropertyWriteHandler,
    BoundConsumedThing,
};
pub use transport::{TransportAdapter, TransportRequest, TransportResponse};
