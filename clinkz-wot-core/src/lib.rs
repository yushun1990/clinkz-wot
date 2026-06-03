#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod binding;
pub mod error;
pub mod payload;
pub mod security;
pub mod thing;
pub mod transport;

pub use binding::{BindingRequest, ProtocolBinding};
pub use error::{CoreError, CoreResult};
pub use payload::{CodecInput, Payload, PayloadCodec};
pub use security::{SecurityContext, SecurityProvider};
pub use thing::{
    AffordanceTarget, ConsumedThing, EventSink, ExposedThing, InteractionInput, InteractionOutput,
};
pub use transport::{TransportAdapter, TransportRequest, TransportResponse};
