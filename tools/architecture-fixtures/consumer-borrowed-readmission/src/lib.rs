//! Non-production candidate for the ADR-0021 six-part readmission contract.
#![no_std]
extern crate alloc;
#[cfg(feature = "validated-thing")]
pub use td_candidate::readmission as td;
pub use td_candidate::{data_type, thing, validate};
#[cfg(feature = "validated-thing")]
pub mod planning;

#[cfg(feature = "validated-thing")]
pub use td::*;

/// Compile-time ABI observation. These values are recomputed by the selected
/// compiler/target; they are evidence, never product/profile constants.
#[cfg(feature = "validated-thing")]
#[used]
#[unsafe(no_mangle)]
pub static WP100_READMISSION_LAYOUT_WORDS: [usize; 18] = [
    core::mem::size_of::<ValidatedThingCursor<'static>>(),
    core::mem::align_of::<ValidatedThingCursor<'static>>(),
    core::mem::size_of::<ValidatedThing<'static>>(),
    core::mem::align_of::<ValidatedThing<'static>>(),
    core::mem::size_of::<ValidatedPropertyReadCursor<'static>>(),
    core::mem::align_of::<ValidatedPropertyReadCursor<'static>>(),
    core::mem::size_of::<ValidatedThingProgress<'static>>(),
    core::mem::align_of::<ValidatedThingProgress<'static>>(),
    core::mem::size_of::<planning::Build<'static, 'static>>(),
    core::mem::align_of::<planning::Build<'static, 'static>>(),
    core::mem::size_of::<planning::Step<'static, 'static>>(),
    core::mem::align_of::<planning::Step<'static, 'static>>(),
    core::mem::size_of::<planning::Draft>(),
    core::mem::align_of::<planning::Draft>(),
    core::mem::size_of::<ValidatedPropertyReadStep<'static>>(),
    core::mem::align_of::<ValidatedPropertyReadStep<'static>>(),
    td::layout_catalog()[4].size,
    td::layout_catalog()[4].alignment,
];
