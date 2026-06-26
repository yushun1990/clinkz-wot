#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod binding;
pub mod error;
pub mod event;
pub mod identity;
pub mod inbound;
pub mod payload;
pub mod security;
pub mod sync;
pub mod thing;
pub mod transport;

#[cfg(feature = "async")]
pub use binding::AsyncClientBinding;
pub use binding::{BindingRequest, ClientBinding, SubscriptionGuard};
pub use error::{CoreError, CoreResult};
pub use event::{
    BrokerEventSink, DEFAULT_SUBSCRIPTION_CAPACITY, EventBroker, EventName, PublisherSink,
    Subscription, SubscriptionSender,
};
pub use identity::{CorrelationId, ThingId};
#[cfg(feature = "async")]
pub use inbound::AsyncServerBinding;
pub use inbound::{InboundDispatcher, InboundRequest, InboundResponse, ServerBinding};
pub use payload::{CodecInput, Payload, PayloadCodec};
pub use security::{
    AuthMaterial, CredentialStore, Credentials, InMemoryCredentialStore, Principal, PrincipalId,
    SecurityContext, SecurityError, SecurityProvider, check_scopes,
};
pub use sync::{MapLock, MapLockError};
pub use thing::{
    ActionHandler, AffordanceTarget, BoundConsumedThing, ConsumedThing, EventSink,
    EventSubscribeHandler, EventUnsubscribeHandler, ExposedThing, InteractionInput,
    InteractionOutput, LocalThing, PropertyObserveHandler, PropertyReadHandler,
    PropertyWriteHandler,
};
#[cfg(feature = "async")]
pub use thing::{AsyncActionHandler, AsyncPropertyReadHandler, AsyncPropertyWriteHandler};
pub use transport::{TransportAdapter, TransportRequest, TransportResponse};
