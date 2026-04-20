#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod thing;
pub mod validate;
pub mod core;
pub use core::data_type;

mod components;
pub use components:: {
    context, form, link, affordance, data_schema, security_scheme,
    util as components_util
};
