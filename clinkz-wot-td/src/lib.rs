#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod core;
pub mod td_defaults;
pub mod thing;
pub mod validate;
pub use core::data_type;

mod components;
pub use components::{
    affordance, context, data_schema, form, link, security_scheme, util as components_util,
};
