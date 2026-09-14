#![no_std]
// E0432 without TD capability, even under downstream serde/AP.
// A sibling enabling TD capability makes this import succeed.
pub use number_boundary_td_prototype::bounded::{Scan, decimal};
