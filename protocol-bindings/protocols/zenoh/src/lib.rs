#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod error;
mod form;

pub use error::{ZenohBindingError, ZenohBindingResult};
pub use form::{
    CZ_ZENOH_KEY_EXPR, ZENOH_SCHEME, ZenohBinding, ZenohFormTarget, extract_zenoh_target,
    is_zenoh_form,
};
