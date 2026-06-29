//! W3C Web of Things Thing Description and Thing Model data structures.
//!
//! This crate owns protocol-neutral TD/TM construction, serialization,
//! deserialization, validation, and round-trip preservation. Protocol-specific
//! behavior belongs in binding crates.
//!
//! Build a minimal Thing Description with no security:
//!
//! ```
//! use clinkz_wot_td::{
//!     affordance::{InteractionHelper, PropertyAffordance},
//!     data_schema::DataSchema,
//!     form::Form,
//!     thing::Thing,
//!     validate::Validate,
//! };
//!
//! # fn main() -> Result<(), String> {
//! let thing = Thing::builder("Lamp")
//!     .id("urn:dev:ops:lamp-1")
//!     .nosec()
//!     .property(
//!         "status",
//!         PropertyAffordance::builder(DataSchema::string())
//!             .form(
//!                 Form::read_property("/properties/status")
//!                     .build()
//!                     .map_err(|err| err.to_string())?,
//!             )
//!             .build()
//!             .map_err(|err| err.to_string())?,
//!     )
//!     .build()
//!     .map_err(|err| err.to_string())?;
//!
//! thing.validate().map_err(|err| err.to_string())?;
//! # Ok(())
//! # }
//! ```
//!
//! Build a Thing Description with a custom security definition name:
//!
//! ```
//! use clinkz_wot_td::{
//!     affordance::{ActionAffordance, InteractionHelper},
//!     data_schema::DataSchema,
//!     form::Form,
//!     thing::Thing,
//!     validate::Validate,
//! };
//!
//! # fn main() -> Result<(), String> {
//! let thing = Thing::builder("Door")
//!     .id("urn:dev:ops:door-1")
//!     .basic_security("doorAuth", "Authorization")
//!     .action(
//!         "unlock",
//!         ActionAffordance::builder()
//!             .input(DataSchema::object().property("reason", DataSchema::string()))
//!             .form(
//!                 Form::invoke_action("/actions/unlock")
//!                     .build()
//!                     .map_err(|err| err.to_string())?,
//!             )
//!             .build()
//!             .map_err(|err| err.to_string())?,
//!     )
//!     .build()
//!     .map_err(|err| err.to_string())?;
//!
//! thing.validate().map_err(|err| err.to_string())?;
//! # Ok(())
//! # }
//! ```
//!
//! Build a Thing Model with reusable affordance metadata:
//!
//! ```
//! use clinkz_wot_td::{
//!     affordance::{EventAffordance, InteractionHelper, PropertyAffordance},
//!     data_schema::DataSchema,
//!     form::Form,
//!     thing_model::ThingModel,
//!     validate::Validate,
//! };
//!
//! # fn main() -> Result<(), String> {
//! let model = ThingModel::builder("Temperature sensor model")
//!     .id("urn:dev:ops:tm:temperature-sensor")
//!     .nosec()
//!     .property(
//!         "temperature",
//!         PropertyAffordance::builder(DataSchema::number())
//!             .form(
//!                 Form::read_property("/properties/temperature")
//!                     .build()
//!                     .map_err(|err| err.to_string())?,
//!             )
//!             .build()
//!             .map_err(|err| err.to_string())?,
//!     )
//!     .event(
//!         "overheated",
//!         EventAffordance::builder()
//!             .data(DataSchema::object().property("temperature", DataSchema::number()))
//!             .form(
//!                 Form::subscribe_event("/events/overheated")
//!                     .build()
//!                     .map_err(|err| err.to_string())?,
//!             )
//!             .build()
//!             .map_err(|err| err.to_string())?,
//!     )
//!     .build()
//!     .map_err(|err| err.to_string())?;
//!
//! model.validate().map_err(|err| err.to_string())?;
//! # Ok(())
//! # }
//! ```
#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod core;
pub mod td_defaults;
pub mod thing;
pub mod thing_model;
pub mod validate;
pub use core::data_type;

mod components;
mod flat;
pub use components::{
    affordance, context, data_schema, form, link, security_scheme, util as components_util,
};
