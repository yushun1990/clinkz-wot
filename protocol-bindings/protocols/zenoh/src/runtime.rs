mod selector;
#[cfg(feature = "zenoh")]
pub mod zenoh;
#[cfg(feature = "zenoh")]
pub mod pool;
#[cfg(feature = "zenoh-pico")]
pub mod zenoh_pico;

#[cfg(feature = "zenoh")]
pub use zenoh::{SharedZenohTransport, ZenohSessionTransport, ZenohSubscription};
#[cfg(feature = "zenoh")]
pub use pool::{DefaultSessionPolicy, ZenohSessionPool, ZenohSessionPolicy};
#[cfg(feature = "zenoh-pico")]
pub use zenoh_pico::{
    ZenohPicoError, ZenohPicoErrorKind, ZenohPicoPlatform, ZenohPicoRequest, ZenohPicoTransport,
};
#[cfg(feature = "zenoh")]
pub type ZenohRuntimeTransport = ZenohSessionTransport;
#[cfg(feature = "zenoh-pico")]
pub type ZenohRuntimeTransport = ZenohPicoTransport;
