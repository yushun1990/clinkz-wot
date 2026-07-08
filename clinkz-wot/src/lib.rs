//! `clinkz-wot` — Rust Web of Things engine.
//!
//! Umbrella crate that re-exports the application-facing API surface from
//! the per-responsibility crates. Application code can write
//! `use clinkz_wot::*;` once and reach the full Scripting-API-aligned
//! surface without naming each sub-crate.
//!
//! # Layered re-exports
//!
//! | Module | Re-exports from | What's there |
//! |---|---|---|
//! | [`core`], [`td`], [`discovery`], [`protocol_bindings`] | the engine-internal crates | Handler traits, TD builders, Discovery sessions, form-selection utilities. |
//! | [`servient`] | `clinkz_wot_servient` | `Servient`, `ServientBuilder`, `ProtocolBinding`, `ConsumedThingHandle`, `ExposedThingHandle`, `ServientError`. |
//! | [`zenoh`], [`cbor`] | optional crates, behind features | Concrete protocol bindings and codecs. |
//!
//! # Quick start
//!
//! ```toml
//! # Cargo.toml
//! [dependencies]
//! clinkz-wot = { version = "0.1", features = ["zenoh"] }
//! ```
//!
//! Application code then writes `use clinkz_wot::prelude::*;` once and
//! reaches the full Scripting-API-aligned surface. See
//! `clinkz-wot/examples/minimal_local.rs` for a complete runnable example.
//!
//! # Features
//!
//! - `default = ["std"]` — std runtime + tokio.
//! - `async` — native-async handles + streaming subscriptions.
//! - `zenoh` — concrete zenoh binding (`clinkz_wot::zenoh`).
//! - `cbor` — CBOR codec (`clinkz_wot::cbor`).
//! - `td2-preview` — experimental TD 2.0 surface.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

/// Engine core: handler traits, interaction I/O, errors, locks, payloads,
/// security. Re-exported from `clinkz_wot_core`.
pub mod core {
    pub use clinkz_wot_core::*;

    /// Security material and provider traits.
    pub mod security {
        pub use clinkz_wot_core::{
            AuthMaterial, CredentialStore, Credentials, InMemoryCredentialStore, Principal,
            PrincipalId, SecurityContext, SecurityError, SecurityProvider, check_scopes,
        };
    }
}

/// Thing Description data model + builders. Re-exported from `clinkz_wot_td`.
pub mod td {
    pub use clinkz_wot_td::*;
}

/// Discovery directory + session traits. Re-exported from
/// `clinkz_wot_discovery`.
pub mod discovery {
    pub use clinkz_wot_discovery::*;
}

/// Protocol-neutral binding utilities (form selection, error mapping).
/// Re-exported from `clinkz_wot_protocol_bindings`.
pub mod protocol_bindings {
    pub use clinkz_wot_protocol_bindings::*;
}

/// Servient composition root + handles. Re-exported from
/// `clinkz_wot_servient`.
pub mod servient {
    pub use clinkz_wot_servient::*;
}

/// Concrete zenoh protocol binding. Available behind the `zenoh` feature.
#[cfg(feature = "zenoh")]
pub mod zenoh {
    pub use clinkz_wot_protocol_bindings_zenoh::*;
}

/// CBOR payload codec. Available behind the `cbor` feature.
#[cfg(feature = "cbor")]
pub mod cbor {
    pub use clinkz_wot_codec_cbor::*;
}

/// Prelude for application code: the most commonly used types.
///
/// ```no_run
/// use clinkz_wot::prelude::*;
/// ```
pub mod prelude {
    pub use crate::core::{
        ActionCancelHandler, ActionHandler, ActionQueryHandler, EventSubscribeHandler,
        EventUnsubscribeHandler, PropertyObserveHandler, PropertyReadHandler,
        PropertyUnobserveHandler, PropertyWriteHandler,
    };
    #[cfg(feature = "async")]
    pub use crate::core::{
        AsyncActionCancelHandler, AsyncActionHandler, AsyncActionQueryHandler,
        AsyncEventSubscribeHandler, AsyncEventUnsubscribeHandler, AsyncPropertyObserveHandler,
        AsyncPropertyReadHandler, AsyncPropertyUnobserveHandler, AsyncPropertyWriteHandler,
    };
    pub use crate::core::{
        CoreError, CoreResult, InteractionInput, InteractionOptions, InteractionOutput, Payload,
        PushFn,
    };
    #[cfg(feature = "std")]
    pub use crate::servient::ServientBuilder;
    #[cfg(feature = "async")]
    pub use crate::servient::{ConsumedThingHandle, ExposedThingHandle, Servient};
    pub use crate::servient::{ServientError, ServientResult};
}
