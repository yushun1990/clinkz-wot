//! Public-entry counterexamples for the shared strict/typed decode boundary.
//! This does not implement a decoder or give private serde keys product status.

#[cfg(any(feature = "ap", feature = "validated-thing"))]
#[path = "../../validated-thing-feature-boundary/td-boundary/src/projection_step.rs"]
mod atomic;

mod unchanged_td {
    use clinkz_wot_td as td;
    #[cfg(any(feature = "ap", feature = "validated-thing"))]
    const AMENDED_BASIC: bool = false;
    include!("support/decode_boundary_cases.rs");
}

mod shared_basic_candidate {
    use validated_thing_schema_kernel_probe as td;
    #[cfg(any(feature = "ap", feature = "validated-thing"))]
    const AMENDED_BASIC: bool = true;
    include!("support/decode_boundary_cases.rs");
}
