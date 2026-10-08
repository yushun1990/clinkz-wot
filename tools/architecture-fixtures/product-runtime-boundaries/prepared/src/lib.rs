#![no_std]
include!(concat!(env!("OUT_DIR"), "/plan.rs"));
#[unsafe(no_mangle)]
pub static PROBE_INLINE_SIZES: [usize; 3] = [
    core::mem::size_of::<runtime_contracts_probe::CoreError>(),
    core::mem::size_of::<runtime_contracts_probe::InteractionOutput<'static>>(),
    core::mem::size_of::<runtime_contracts_probe::PlanImage>(),
];
