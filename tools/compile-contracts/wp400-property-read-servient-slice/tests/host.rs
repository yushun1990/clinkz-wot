use core::sync::atomic::{AtomicU32, Ordering};
use core::task::{Context, Poll, Waker};
use std::sync::Arc;

use clinkz_wot_core::{
    CoreResult, HandlerContext, HandlerFootprint, InteractionInput, InteractionOutput,
    ReadPropertyHandler, StepStatus,
};
use clinkz_wot_foundation::{GatewayDefaultV1, StaticResourceProfile, WorkBudget, WorkClass};
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    form::Form,
    thing::Thing,
};
use clinkz_wot_wp300_property_read_binding_slice_contract::host_property_read_fixture;
use clinkz_wot_wp400_property_read_servient_slice_contract::{
    begin_host_property_read, build_host_property_read, step_host_property_read,
};

fn thing() -> Thing {
    Thing::builder("Tank")
        .id("urn:fixture:wp400-property-read")
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

fn drive_until_idle(servient: &clinkz_wot_servient::Servient, cx: &mut Context<'_>) {
    for _ in 0..32 {
        let mut budget = WorkBudget::new()
            .with_remaining(WorkClass::BindingPolls, 8)
            .with_remaining(WorkClass::HandlerSteps, 1)
            .with_remaining(WorkClass::CleanupItems, 8);
        if matches!(
            step_host_property_read(servient, cx, &mut budget),
            StepStatus::Idle
        ) {
            return;
        }
    }
    panic!("bounded host fixture did not become idle");
}

#[test]
fn host_runner_enters_servient_for_one_complete_property_read() {
    let (binding, probe) = host_property_read_fixture();
    let limits = GatewayDefaultV1::LIMITS.clone();
    let servient = build_host_property_read(limits, binding).expect("complete host Servient");
    let calls = Arc::new(AtomicU32::new(0));
    let exposed = begin_host_property_read(
        &servient,
        thing(),
        Handler {
            calls: Arc::clone(&calls),
        },
        HandlerFootprint::new(1, 0, 0),
    )
    .expect("accepted expose transaction");

    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    drive_until_idle(&servient, &mut cx);
    assert_eq!(
        probe.prepared_target().as_deref(),
        Some("mock://tank/level")
    );
    assert_eq!(probe.artifact_drops(), 0);

    probe.enqueue_property_read("level", InteractionInput::empty());
    drive_until_idle(&servient, &mut cx);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(probe.delivered_responses(), 1);

    exposed
        .begin_destroy()
        .expect("accepted destroy transaction");
    drive_until_idle(&servient, &mut cx);
    assert_eq!(probe.outstanding_counts(), (0, 0, 0, 0));
    assert_eq!(probe.poll_after_close(&mut cx), Poll::Ready(false));
    assert_eq!(probe.artifact_drops(), 1);
}

#[test]
fn accepted_request_survives_exhausted_handler_budget() {
    let (binding, probe) = host_property_read_fixture();
    let limits = GatewayDefaultV1::LIMITS.clone();
    let servient = build_host_property_read(limits, binding).expect("complete host Servient");
    let calls = Arc::new(AtomicU32::new(0));
    let exposed = begin_host_property_read(
        &servient,
        thing(),
        Handler {
            calls: Arc::clone(&calls),
        },
        HandlerFootprint::new(1, 0, 0),
    )
    .expect("accepted expose transaction");

    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    drive_until_idle(&servient, &mut cx);
    assert_eq!(probe.artifact_drops(), 0);

    probe.enqueue_property_read("level", InteractionInput::empty());
    let mut exhausted = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 8);
    let _ = step_host_property_read(&servient, &mut cx, &mut exhausted);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(probe.outstanding_counts(), (1, 0, 1, 0));

    drive_until_idle(&servient, &mut cx);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(probe.delivered_responses(), 1);

    exposed
        .begin_destroy()
        .expect("accepted destroy transaction");
    drive_until_idle(&servient, &mut cx);
    assert_eq!(probe.outstanding_counts(), (0, 0, 0, 0));
}
