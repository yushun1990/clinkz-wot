#![no_std]

use core::task::Context;

use clinkz_wot_core::{
    CoreResult, Deadline, HandlerFootprint, HostBindingRegistration, PollServerBinding,
    ReadPropertyHandler, StaticBindingRegistration, StaticHandlerRegistration, StepStatus,
    ThingSlotId,
};
use clinkz_wot_foundation::{ResourceLimits, WorkBudget};
use clinkz_wot_servient::{StaticServient, StaticServientBuilder};
use clinkz_wot_td::thing::Thing;

/// Type-checks the complete application-static authoring boundary. The
/// builder, not this fixture, derives plan-set, route, activation, and cleanup
/// records from the production roots supplied here.
pub fn build_static_property_read<'h, B, H>(
    td: Thing,
    thing_slot: ThingSlotId,
    limits: ResourceLimits,
    deadline: Deadline,
    binding: StaticBindingRegistration<B>,
    handler: StaticHandlerRegistration<'h, H>,
) -> CoreResult<impl StaticServient + 'h>
where
    B: PollServerBinding + 'h,
    H: ReadPropertyHandler + 'h,
{
    StaticServientBuilder::new(td, thing_slot, limits, deadline)
        .binding_registration(binding)
        .read_property_handler("level", handler)
        .build()
}

/// Drives the real manually progressed product boundary with caller-owned
/// wake and budget policy only.
pub fn step_static_property_read(
    servient: &mut impl StaticServient,
    cx: &mut Context<'_>,
    budget: &mut WorkBudget,
) -> StepStatus<()> {
    servient.step(cx, budget)
}

#[cfg(feature = "std")]
use clinkz_wot_servient::{ExposedThingHandle, Servient, ServientBuilder};

/// Type-checks complete host binding installation. Bare server/compiler
/// values have no legal entry here.
#[cfg(feature = "std")]
pub fn build_host_property_read(
    limits: ResourceLimits,
    binding: HostBindingRegistration,
) -> CoreResult<Servient> {
    ServientBuilder::new(limits)
        .binding_registration(binding)
        .build()
}

/// Installs one real synchronous handler and begins the product-owned expose
/// transaction. The runner never constructs a plan, route, or prepare input.
#[cfg(feature = "std")]
pub fn begin_host_property_read<H>(
    servient: &Servient,
    td: Thing,
    handler: H,
    footprint: HandlerFootprint,
) -> CoreResult<ExposedThingHandle>
where
    H: ReadPropertyHandler + Send + Sync + 'static,
{
    let exposed = servient.produce_td(td)?;
    exposed.set_read_property_handler("level", handler, footprint)?;
    exposed.begin_expose()?;
    Ok(exposed)
}

/// Drives the host adapter through the same bounded transition kernel.
#[cfg(feature = "std")]
pub fn step_host_property_read(
    servient: &Servient,
    cx: &mut Context<'_>,
    budget: &mut WorkBudget,
) -> StepStatus<()> {
    servient.step(cx, budget)
}
