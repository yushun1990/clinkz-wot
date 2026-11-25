//! Engine execution. Shared semantic kernels live in Core; Host and borrowed
//! owners choose their own storage, authority, native facets and driving.
#![no_std]
extern crate alloc;

pub mod borrowed_action;
pub mod borrowed_consumer;
pub mod borrowed_producer;

#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
mod host;
#[cfg(all(feature = "std", target_has_atomic = "ptr"))]
pub use host::*;

use clinkz_wot_core::{CoreError, CoreResult, ErrorContext, ErrorPhase, RetryClass};

/// All linked checks precede this local decision. Each storage owner supplies
/// just its infallible registry change. Opening the checked gates is passive.
fn commit_producer_gates<'a>(
    gates: impl Iterator<Item = &'a clinkz_wot_core::execution::producer::ProducerIngressGate> + Clone,
    commit: impl FnOnce(),
) -> CoreResult<()> {
    if gates
        .clone()
        .enumerate()
        .any(|(i, g)| !g.is_staged() || gates.clone().take(i).any(|p| core::ptr::eq(p, g)))
    {
        return Err(CoreError::Validation(ErrorContext::new(
            ErrorPhase::Admission,
            RetryClass::Never,
        )));
    }
    commit();
    for gate in gates {
        gate.open();
    }
    Ok(())
}
