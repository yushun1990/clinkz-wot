#![no_std]

#[cfg(test)]
extern crate std;

use core::task::Context;

use clinkz_wot_core::{
    CoreResult, Deadline, PollServerBinding, ReadPropertyHandler, StaticBindingRegistration,
    StaticHandlerRegistration, StepStatus, ThingSlotId,
};
#[cfg(feature = "std")]
use clinkz_wot_core::{HandlerFootprint, HostBindingRegistration};
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

/// Closes static route selection before the caller continues bounded cleanup.
pub fn begin_static_property_read_destroy(servient: &mut impl StaticServient) -> CoreResult<()> {
    servient.begin_destroy()
}

#[cfg(feature = "std")]
use clinkz_wot_servient::{ExposedThingHandle, Servient, ServientBuilder, ServientResult};

/// Type-checks complete host binding installation. Bare server/compiler
/// values have no legal entry here.
#[cfg(feature = "std")]
pub fn build_host_property_read(
    limits: ResourceLimits,
    binding: HostBindingRegistration,
) -> ServientResult<Servient> {
    ServientBuilder::new()
        .resource_limits(limits)
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
) -> ServientResult<ExposedThingHandle>
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

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicU32, Ordering};
    use core::task::{Context, Poll, Waker};
    use std::sync::Arc;

    use clinkz_wot_core::{
        CoreResult, HandlerContext, HandlerFootprint, HandlerSlotId, InteractionInput,
        InteractionOutput, ReadPropertyHandler, StaticHandlerRegistration, StepStatus, ThingSlotId,
    };
    use clinkz_wot_foundation::{
        BenchmarkStaticReferenceV1, Generation, SlotIndex, StaticResourceProfile, WorkBudget,
        WorkClass,
    };
    use clinkz_wot_td::{
        affordance::{InteractionHelper, PropertyAffordance},
        data_schema::DataSchema,
        form::Form,
        thing::Thing,
    };
    use clinkz_wot_wp300_property_read_binding_slice_contract::static_property_read_fixture;

    use super::{
        StaticServient, begin_static_property_read_destroy, build_static_property_read,
        step_static_property_read,
    };

    fn thing() -> Thing {
        Thing::builder("Tank")
            .id("urn:fixture:wp400-static-property-read")
            .nosec()
            .property(
                "level",
                PropertyAffordance::builder(DataSchema::number())
                    .form(
                        Form::read_property("mock://tank/level")
                            .build()
                            .expect("valid form"),
                    )
                    .build()
                    .expect("valid property"),
            )
            .build()
            .expect("valid Thing")
    }

    struct Handler {
        calls: Arc<AtomicU32>,
    }

    impl ReadPropertyHandler for Handler {
        fn handle(
            &self,
            _context: HandlerContext<'_>,
            _input: &InteractionInput,
        ) -> CoreResult<InteractionOutput> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(InteractionOutput::empty())
        }
    }

    fn drive_until_idle(servient: &mut impl StaticServient, cx: &mut Context<'_>) {
        for _ in 0..32 {
            let mut budget = WorkBudget::new()
                .with_remaining(WorkClass::BindingPolls, 8)
                .with_remaining(WorkClass::HandlerSteps, 1)
                .with_remaining(WorkClass::CleanupItems, 8);
            if matches!(
                step_static_property_read(servient, cx, &mut budget),
                StepStatus::Idle
            ) {
                return;
            }
        }
        panic!("bounded static fixture did not become idle");
    }

    #[test]
    fn static_runner_retains_request_and_completes_deactivation() {
        let (binding, probe) = static_property_read_fixture();
        let thing_slot = ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL);
        let calls = Arc::new(AtomicU32::new(0));
        let handler = Handler {
            calls: Arc::clone(&calls),
        };
        let registration = StaticHandlerRegistration::new(
            HandlerSlotId::new(SlotIndex::new(0), Generation::INITIAL),
            &handler,
            HandlerFootprint::new(1, 0, 0),
        );
        let mut servient = build_static_property_read(
            thing(),
            thing_slot,
            BenchmarkStaticReferenceV1::LIMITS.clone(),
            clinkz_wot_core::Deadline::NONE,
            binding,
            registration,
        )
        .expect("complete static Servient");

        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        drive_until_idle(&mut servient, &mut cx);
        assert_eq!(
            probe.prepared_target().as_deref(),
            Some("mock://tank/level")
        );
        assert_eq!(probe.artifact_drops(), 0);

        probe.enqueue_property_read("level", InteractionInput::empty());
        let mut exhausted = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 8);
        let _ = step_static_property_read(&mut servient, &mut cx, &mut exhausted);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(probe.outstanding_counts(), (1, 0, 1, 0));

        drive_until_idle(&mut servient, &mut cx);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(probe.delivered_responses(), 1);

        begin_static_property_read_destroy(&mut servient)
            .expect("accepted static destroy transaction");
        drive_until_idle(&mut servient, &mut cx);
        assert_eq!(probe.outstanding_counts(), (0, 0, 0, 0));
        assert_eq!(probe.poll_after_close(&mut cx), Poll::Ready(false));
        assert_eq!(probe.artifact_drops(), 1);
    }
}
