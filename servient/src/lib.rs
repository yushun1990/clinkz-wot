#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod consumer_property_read;
mod error;
#[cfg(feature = "std")]
pub mod host_accounts;
#[cfg(feature = "std")]
pub mod producer_installation;
mod property_read;

#[cfg(feature = "std")]
mod builder;

pub use error::{ServientError, ServientResult};
pub use property_read::{CompiledPlanSetState, ExposeState, StaticServient, StaticServientBuilder};

#[cfg(feature = "std")]
pub use producer_installation::ProducerInstallationRuntime as Servient;

#[cfg(feature = "std")]
pub use builder::ServientBuilder;
