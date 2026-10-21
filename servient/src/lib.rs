#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod consumer_property_read;
mod error;
mod property_read;

#[cfg(feature = "std")]
mod builder;
#[cfg(feature = "std")]
mod handle;
#[cfg(feature = "std")]
mod servient;

pub use error::{ServientError, ServientResult};
pub use property_read::{CompiledPlanSetState, ExposeState, StaticServient, StaticServientBuilder};

#[cfg(feature = "std")]
pub use handle::ExposedThingHandle;
#[cfg(feature = "std")]
pub use servient::Servient;

#[cfg(feature = "std")]
pub use builder::ServientBuilder;
