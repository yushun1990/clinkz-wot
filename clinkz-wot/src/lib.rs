//! Web of Things preparation and explicitly composed execution capabilities.
#![no_std]
pub mod core {
    pub use clinkz_wot_core::*;
}
pub mod td {
    pub use clinkz_wot_td::*;
}
pub mod planning {
    pub use clinkz_wot_planning::*;
}
pub mod discovery {
    pub use clinkz_wot_discovery::*;
}
pub mod servient {
    pub use clinkz_wot_servient::*;
}
#[cfg(feature = "zenoh")]
pub mod zenoh {
    pub use clinkz_wot_protocol_bindings_zenoh::*;
}
#[cfg(feature = "cbor")]
pub mod cbor {
    pub use clinkz_wot_codec_cbor::*;
}
pub mod prelude {
    pub use crate::core::{
        CoreError, CoreResult, InteractionInput, InteractionOptions, InteractionOutput, Payload,
        ReadPropertyHandler,
    };
    #[cfg(feature = "std")]
    pub use crate::servient::{Servient, ServientBuilder};
    pub use crate::servient::{ServientError, ServientResult};
}
