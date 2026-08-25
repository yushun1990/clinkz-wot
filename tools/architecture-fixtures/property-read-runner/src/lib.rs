//! Aggregate Property Read architecture proof.
//!
//! This runner supplies only legal roots: a Thing Description, generations,
//! complete external binding registrations, handler behavior, protocol-local
//! requests, and resource/work policy. Servient and its production
//! dependencies create every intermediate carrier exercised below.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "async")]
use clinkz_wot_core::{
    CoreResult, Deadline, HandlerFootprint, HandlerSlotId, ReadPropertyHandler,
    StaticHandlerRegistration, ThingSlotId,
};
#[cfg(feature = "async")]
use clinkz_wot_foundation::{
    BenchmarkStaticReferenceV1, Generation, SlotIndex, StaticResourceProfile,
};
#[cfg(feature = "async")]
use clinkz_wot_property_read_binding_fixture::static_property_read_fixture;
#[cfg(feature = "async")]
use clinkz_wot_servient::{StaticServient, StaticServientBuilder};
#[cfg(feature = "async")]
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    form::Form,
    thing::Thing,
};

#[cfg(feature = "async")]
pub fn async_no_std_property_read_projection<'h, H>(
    handler: &'h H,
) -> CoreResult<impl StaticServient + 'h>
where
    H: ReadPropertyHandler + 'h,
{
    const PROPERTY: &str = "level";
    let td = Thing::builder("Tank")
        .id("urn:fixture:aggregate-property-read-async")
        .nosec()
        .property(
            PROPERTY,
            PropertyAffordance::builder(DataSchema::number())
                .form(
                    Form::read_property("mock://tank/level")
                        .build()
                        .expect("valid URI"),
                )
                .build()
                .expect("valid property"),
        )
        .build()
        .expect("valid TD");
    let handler = StaticHandlerRegistration::new(
        HandlerSlotId::new(SlotIndex::new(0), Generation::INITIAL),
        handler,
        HandlerFootprint::new(1, 0, 0),
    );
    let (binding, _probe) = static_property_read_fixture();
    StaticServientBuilder::new(
        td,
        ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL),
        BenchmarkStaticReferenceV1::LIMITS.clone(),
        Deadline::NONE,
    )
    .binding_registration(binding)
    .read_property_handler(PROPERTY, handler)
    .build()
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use core::sync::atomic::{AtomicU32, Ordering};
    use core::task::{Context, Poll, Waker};
    use std::sync::{Arc, Mutex};

    use clinkz_wot_core::{
        BindingGeneration, BindingId, CleanupOperation, CoreResult, CorrelationId, HandlerContext,
        HandlerFootprint, HandlerSlotId, InteractionInput, InteractionOutput, Payload, PlanId,
        ReadPropertyHandler, StaticHandlerRegistration, StepStatus, ThingSlotId,
    };
    use clinkz_wot_foundation::{
        BenchmarkStaticReferenceV1, GatewayDefaultV1, Generation, ResourceKind, SlotIndex,
        StaticResourceProfile, WorkBudget, WorkClass,
    };
    use clinkz_wot_property_read_binding_fixture::{
        DeliveredResponseEvidence, HostPropertyReadProbe, MockLifecyclePhase,
        StaticPropertyReadProbe, host_property_read_activation_transfer_fixture,
        host_property_read_cleanup_transfer_fixture,
        host_property_read_delivery_error_transfer_fixture, host_property_read_fixture,
        host_property_read_oversized_activation_fixture,
        host_property_read_oversized_shutdown_fixture,
        host_property_read_readiness_rejection_fixture, static_property_read_fixture,
        static_property_read_readiness_failure_fixture,
    };
    use clinkz_wot_servient::{Servient, ServientBuilder, StaticServient, StaticServientBuilder};
    use clinkz_wot_td::{
        affordance::{InteractionHelper, PropertyAffordance},
        data_schema::DataSchema,
        data_type::Operation,
        form::Form,
        thing::Thing,
    };

    const THING_ID: &str = "urn:fixture:aggregate-property-read";
    const PROPERTY: &str = "level";
    const TARGET: &str = "mock://tank/level";
    const RESPONSE_PAYLOAD: &[u8] = br#"{"level":42}"#;
    const RESPONSE_MEDIA_TYPE: &str = "application/json";

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct HandlerEvidence {
        thing_id: String,
        thing_slot: ThingSlotId,
        plan_id: PlanId,
        binding: Option<(BindingId, BindingGeneration)>,
        operation: Operation,
        target: String,
    }

    struct Handler {
        calls: Arc<AtomicU32>,
        evidence: Arc<Mutex<Option<HandlerEvidence>>>,
    }

    impl ReadPropertyHandler for Handler {
        fn handle(
            &self,
            context: HandlerContext<'_>,
            _input: &InteractionInput,
        ) -> CoreResult<InteractionOutput> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let observed = HandlerEvidence {
                thing_id: context.thing_id().as_str().to_owned(),
                thing_slot: context.thing_slot(),
                plan_id: context.plan_id(),
                binding: context.binding(),
                operation: context.operation(),
                target: context.target().name().unwrap_or_default().to_owned(),
            };
            let mut evidence = self
                .evidence
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            assert!(
                evidence.replace(observed).is_none(),
                "handler invoked twice"
            );
            Ok(InteractionOutput::with_data(Payload::new(
                RESPONSE_PAYLOAD.to_vec(),
                RESPONSE_MEDIA_TYPE,
            )))
        }
    }

    struct MissingPayloadHandler {
        calls: Arc<AtomicU32>,
    }

    impl ReadPropertyHandler for MissingPayloadHandler {
        fn handle(
            &self,
            _context: HandlerContext<'_>,
            _input: &InteractionInput,
        ) -> CoreResult<InteractionOutput> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(InteractionOutput::empty())
        }
    }

    fn thing() -> Thing {
        Thing::builder("Tank")
            .id(THING_ID)
            .nosec()
            .property(
                PROPERTY,
                PropertyAffordance::builder(DataSchema::number())
                    .form(Form::read_property(TARGET).build().expect("valid form"))
                    .build()
                    .expect("valid property"),
            )
            .build()
            .expect("valid Thing Description")
    }

    fn work_budget() -> WorkBudget {
        WorkBudget::new()
            .with_remaining(WorkClass::BindingPolls, 8)
            .with_remaining(WorkClass::HandlerSteps, 1)
            .with_remaining(WorkClass::CleanupItems, 8)
    }

    fn drive_static_until_idle(servient: &mut impl StaticServient, cx: &mut Context<'_>) {
        for _ in 0..32 {
            if matches!(servient.step(cx, &mut work_budget()), StepStatus::Idle) {
                return;
            }
        }
        panic!("bounded static aggregate cell did not become idle");
    }

    fn drive_host_until_idle(servient: &Servient, cx: &mut Context<'_>) {
        for _ in 0..32 {
            if matches!(servient.step(cx, &mut work_budget()), StepStatus::Idle) {
                return;
            }
        }
        panic!("bounded Host aggregate cell did not become idle");
    }

    fn assert_handler_evidence(evidence: &Arc<Mutex<Option<HandlerEvidence>>>) {
        let evidence = evidence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .expect("production dispatch supplied a handler context");
        assert_eq!(evidence.thing_id, THING_ID);
        assert_eq!(evidence.thing_slot.generation(), Generation::INITIAL);
        assert_eq!(evidence.plan_id.generation(), Generation::INITIAL);
        assert_eq!(
            evidence.binding,
            Some((BindingId::new(7), BindingGeneration::INITIAL))
        );
        assert_eq!(evidence.operation, Operation::ReadProperty);
        assert_eq!(evidence.target, PROPERTY);
    }

    fn assert_success_delivery(
        accepted_correlation: Option<CorrelationId>,
        delivered_responses: u32,
        response_settlements: u32,
        delivered_validation_errors: u32,
        delivered_response: Option<DeliveredResponseEvidence>,
    ) {
        assert_eq!(delivered_responses, 1);
        assert_eq!(response_settlements, 1);
        assert_eq!(delivered_validation_errors, 0);
        let delivered_response = delivered_response.expect("protocol edge observed one response");
        assert_eq!(accepted_correlation, Some(delivered_response.correlation()));
        assert_eq!(delivered_response.correlation(), CorrelationId::new(1));
        assert_eq!(delivered_response.payload(), Some(RESPONSE_PAYLOAD));
        assert_eq!(delivered_response.media_type(), Some(RESPONSE_MEDIA_TYPE));
        assert!(!delivered_response.is_validation_failure());
    }

    fn assert_validation_failure_delivery(
        accepted_correlation: Option<CorrelationId>,
        delivered_responses: u32,
        response_settlements: u32,
        delivered_validation_errors: u32,
        delivered_response: Option<DeliveredResponseEvidence>,
    ) {
        assert_eq!(delivered_responses, 1);
        assert_eq!(response_settlements, 1);
        assert_eq!(delivered_validation_errors, 1);
        let delivered_response = delivered_response.expect("protocol edge observed one response");
        assert_eq!(accepted_correlation, Some(delivered_response.correlation()));
        assert_eq!(delivered_response.correlation(), CorrelationId::new(1));
        assert_eq!(delivered_response.payload(), None);
        assert_eq!(delivered_response.media_type(), None);
        assert!(delivered_response.is_validation_failure());
    }

    fn assert_static_clean(probe: &StaticPropertyReadProbe, cx: &mut Context<'_>) {
        assert_eq!(probe.outstanding_counts(), (0, 0, 0, 0));
        assert_eq!(probe.ingress_usage(), [(0, 0); 3]);
        assert_eq!(probe.poll_after_close(cx), Poll::Ready(false));
        assert_eq!(probe.artifact_drops(), 1);
    }

    fn assert_host_clean(probe: &HostPropertyReadProbe, cx: &mut Context<'_>) {
        assert_eq!(probe.outstanding_counts(), (0, 0, 0, 0));
        assert_eq!(probe.ingress_usage(), [(0, 0); 3]);
        assert_eq!(probe.poll_after_close(cx), Poll::Ready(false));
        assert_eq!(probe.artifact_drops(), 1);
        assert_eq!(probe.route_state_drops(), 1);
    }

    fn assert_ingress_live(usage: [(u32, u64); 3]) {
        assert!(usage[0].1 != 0);
        assert_eq!(usage, [usage[0]; 3]);
        assert_eq!(usage[0].0, 1);
    }

    fn assert_cleanup_context_round_trip(
        probe: &HostPropertyReadProbe,
        phase: MockLifecyclePhase,
        operation: CleanupOperation,
    ) -> clinkz_wot_property_read_binding_fixture::CleanupContextEvidence {
        let (started, settled) = probe.cleanup_context_evidence(phase);
        let started = started.expect("pending Host call retained a cleanup context");
        let settled = settled.expect("pending Host call settled that cleanup context");
        assert_eq!(started, settled, "{phase:?} changed cleanup context");
        assert_eq!(started.operation(), operation);
        started
    }

    fn cancel_static_call(phase: MockLifecyclePhase) {
        let (binding, probe) = static_property_read_fixture();
        let calls = Arc::new(AtomicU32::new(0));
        let handler_value = Handler {
            calls: Arc::clone(&calls),
            evidence: Arc::new(Mutex::new(None)),
        };
        let handler = StaticHandlerRegistration::new(
            HandlerSlotId::new(SlotIndex::new(0), Generation::INITIAL),
            &handler_value,
            HandlerFootprint::new(1, 0, 0),
        );
        let mut servient = StaticServientBuilder::new(
            thing(),
            ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL),
            BenchmarkStaticReferenceV1::LIMITS.clone(),
            clinkz_wot_core::Deadline::NONE,
        )
        .binding_registration(binding)
        .read_property_handler(PROPERTY, handler)
        .build()
        .unwrap();
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        for _ in 0..32 {
            if probe.lifecycle_starts(phase) != 0 {
                break;
            }
            let _ = servient.step(&mut cx, &mut work_budget());
        }
        assert_eq!(probe.lifecycle_starts(phase), 1);
        servient.begin_destroy().unwrap();
        drive_static_until_idle(&mut servient, &mut cx);
        assert_eq!(probe.lifecycle_cancellations(phase), 1, "{phase:?}");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_static_clean(&probe, &mut cx);
    }

    fn cancel_host_call(phase: MockLifecyclePhase) {
        let (binding, probe) = host_property_read_fixture();
        let servient = ServientBuilder::new()
            .resource_limits(GatewayDefaultV1::LIMITS.clone())
            .binding_registration(binding)
            .build()
            .unwrap();
        let calls = Arc::new(AtomicU32::new(0));
        let exposed = servient.produce_td(thing()).unwrap();
        exposed
            .set_read_property_handler(
                PROPERTY,
                Handler {
                    calls: Arc::clone(&calls),
                    evidence: Arc::new(Mutex::new(None)),
                },
                HandlerFootprint::new(1, 0, 0),
            )
            .unwrap();
        exposed.begin_expose().unwrap();
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        for _ in 0..32 {
            if probe.lifecycle_starts(phase) != 0 {
                break;
            }
            let _ = servient.step(&mut cx, &mut work_budget());
        }
        assert_eq!(probe.lifecycle_starts(phase), 1);
        exposed.begin_destroy().unwrap();
        drive_host_until_idle(&servient, &mut cx);
        assert_eq!(probe.lifecycle_cancellations(phase), 1);
        let call_context = assert_cleanup_context_round_trip(
            &probe,
            phase,
            CleanupOperation::CancelRouteReadiness,
        );
        let route_context = match phase {
            MockLifecyclePhase::Readiness | MockLifecyclePhase::Activate => {
                Some(assert_cleanup_context_round_trip(
                    &probe,
                    MockLifecyclePhase::Abort,
                    CleanupOperation::AbortPreparedRoute,
                ))
            }
            MockLifecyclePhase::Commit => Some(assert_cleanup_context_round_trip(
                &probe,
                MockLifecyclePhase::Shutdown,
                CleanupOperation::ShutdownRoute,
            )),
            MockLifecyclePhase::Prepare => None,
            _ => unreachable!("helper receives a lifecycle phase"),
        };
        if let Some(route_context) = route_context {
            assert_ne!(
                call_context.subject(),
                route_context.subject(),
                "route rollback cannot mask the outstanding-call cleanup context"
            );
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(probe.outstanding_counts(), (0, 0, 0, 0));
        assert_eq!(probe.artifact_drops(), 1);
    }

    #[test]
    fn application_static_cell_uses_the_complete_production_path() {
        let (binding, probe) = static_property_read_fixture();
        let thing_slot = ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL);
        let calls = Arc::new(AtomicU32::new(0));
        let evidence = Arc::new(Mutex::new(None));
        let handler = Handler {
            calls: Arc::clone(&calls),
            evidence: Arc::clone(&evidence),
        };
        let handler = StaticHandlerRegistration::new(
            HandlerSlotId::new(SlotIndex::new(0), Generation::INITIAL),
            &handler,
            HandlerFootprint::new(1, 0, 0),
        );
        let mut servient = StaticServientBuilder::new(
            thing(),
            thing_slot,
            BenchmarkStaticReferenceV1::LIMITS.clone(),
            clinkz_wot_core::Deadline::NONE,
        )
        .binding_registration(binding)
        .read_property_handler(PROPERTY, handler)
        .build()
        .expect("complete application-static first-entry roots");

        assert_eq!(probe.preparation_side_effects(), 0);
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        drive_static_until_idle(&mut servient, &mut cx);
        assert_eq!(probe.carrier_checks(), 1);
        assert_eq!(probe.preparation_side_effects(), 1);
        assert_eq!(probe.prepared_target().as_deref(), Some(TARGET));
        assert_eq!(probe.artifact_drops(), 0);

        probe.enqueue_property_read(PROPERTY, InteractionInput::empty());
        let mut handler_exhausted = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 8);
        let _ = servient.step(&mut cx, &mut handler_exhausted);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(probe.outstanding_counts(), (1, 0, 1, 0));
        assert_ingress_live(probe.ingress_usage());

        drive_static_until_idle(&mut servient, &mut cx);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_success_delivery(
            probe.last_accepted_correlation(),
            probe.delivered_responses(),
            probe.response_settlements(),
            probe.delivered_validation_errors(),
            probe.delivered_response(),
        );
        assert_handler_evidence(&evidence);
        assert_eq!(probe.ingress_usage(), [(0, 0); 3]);

        servient.begin_destroy().expect("static destroy accepted");
        drive_static_until_idle(&mut servient, &mut cx);
        assert_static_clean(&probe, &mut cx);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(probe.response_settlements(), 1);
    }

    #[test]
    fn host_erased_cell_uses_the_same_production_path_and_one_route_carrier() {
        let (binding, probe) = host_property_read_fixture();
        let servient = ServientBuilder::new()
            .resource_limits(GatewayDefaultV1::LIMITS.clone())
            .binding_registration(binding)
            .build()
            .expect("complete Host binding installation");
        let calls = Arc::new(AtomicU32::new(0));
        let evidence = Arc::new(Mutex::new(None));
        let exposed = servient.produce_td(thing()).expect("produced Thing roots");
        exposed
            .set_read_property_handler(
                PROPERTY,
                Handler {
                    calls: Arc::clone(&calls),
                    evidence: Arc::clone(&evidence),
                },
                HandlerFootprint::new(1, 0, 0),
            )
            .expect("complete handler coverage");
        exposed
            .begin_expose()
            .expect("exposure transaction accepted");

        assert_eq!(probe.preparation_side_effects(), 0);
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        drive_host_until_idle(&servient, &mut cx);
        assert_eq!(probe.carrier_checks(), 1);
        assert_eq!(probe.preparation_side_effects(), 1);
        assert_eq!(probe.prepared_target().as_deref(), Some(TARGET));
        assert_eq!(probe.artifact_drops(), 0);
        let (prepared, active, committed, prepared_bytes, active_bytes, committed_bytes) =
            probe.carrier_evidence();
        assert_eq!(prepared, active);
        assert_eq!(prepared, committed);
        assert_eq!(prepared_bytes, active_bytes);
        assert_eq!(prepared_bytes, committed_bytes);
        assert_eq!(probe.route_state_drops(), 0);

        probe.enqueue_property_read(PROPERTY, InteractionInput::empty());
        let mut handler_exhausted = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 8);
        let _ = servient.step(&mut cx, &mut handler_exhausted);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(probe.outstanding_counts(), (1, 0, 1, 0));
        assert_ingress_live(probe.ingress_usage());

        drive_host_until_idle(&servient, &mut cx);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_success_delivery(
            probe.last_accepted_correlation(),
            probe.delivered_responses(),
            probe.response_settlements(),
            probe.delivered_validation_errors(),
            probe.delivered_response(),
        );
        assert_handler_evidence(&evidence);
        assert_eq!(probe.ingress_usage(), [(0, 0); 3]);

        exposed.begin_destroy().expect("Host destroy accepted");
        drive_host_until_idle(&servient, &mut cx);
        assert_host_clean(&probe, &mut cx);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(probe.response_settlements(), 1);
    }

    #[test]
    fn invalid_handler_success_is_sealed_once_on_the_original_opportunity_in_both_cells() {
        let (binding, static_probe) = static_property_read_fixture();
        let static_calls = Arc::new(AtomicU32::new(0));
        let static_handler_value = MissingPayloadHandler {
            calls: Arc::clone(&static_calls),
        };
        let static_handler = StaticHandlerRegistration::new(
            HandlerSlotId::new(SlotIndex::new(0), Generation::INITIAL),
            &static_handler_value,
            HandlerFootprint::new(1, 0, 0),
        );
        let mut static_servient = StaticServientBuilder::new(
            thing(),
            ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL),
            BenchmarkStaticReferenceV1::LIMITS.clone(),
            clinkz_wot_core::Deadline::NONE,
        )
        .binding_registration(binding)
        .read_property_handler(PROPERTY, static_handler)
        .build()
        .expect("complete application-static first-entry roots");
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        drive_static_until_idle(&mut static_servient, &mut cx);
        static_probe.enqueue_property_read(PROPERTY, InteractionInput::empty());
        drive_static_until_idle(&mut static_servient, &mut cx);

        assert_eq!(static_calls.load(Ordering::SeqCst), 1);
        assert_validation_failure_delivery(
            static_probe.last_accepted_correlation(),
            static_probe.delivered_responses(),
            static_probe.response_settlements(),
            static_probe.delivered_validation_errors(),
            static_probe.delivered_response(),
        );
        assert_eq!(static_probe.outstanding_counts(), (1, 0, 0, 0));
        static_servient
            .begin_destroy()
            .expect("static destroy accepted");
        drive_static_until_idle(&mut static_servient, &mut cx);
        assert_static_clean(&static_probe, &mut cx);
        assert_eq!(static_calls.load(Ordering::SeqCst), 1);
        assert_eq!(static_probe.response_settlements(), 1);

        let (binding, host_probe) = host_property_read_fixture();
        let host_servient = ServientBuilder::new()
            .resource_limits(GatewayDefaultV1::LIMITS.clone())
            .binding_registration(binding)
            .build()
            .expect("complete Host binding installation");
        let host_calls = Arc::new(AtomicU32::new(0));
        let exposed = host_servient
            .produce_td(thing())
            .expect("produced Thing roots");
        exposed
            .set_read_property_handler(
                PROPERTY,
                MissingPayloadHandler {
                    calls: Arc::clone(&host_calls),
                },
                HandlerFootprint::new(1, 0, 0),
            )
            .expect("complete handler coverage");
        exposed
            .begin_expose()
            .expect("exposure transaction accepted");
        drive_host_until_idle(&host_servient, &mut cx);
        host_probe.enqueue_property_read(PROPERTY, InteractionInput::empty());
        drive_host_until_idle(&host_servient, &mut cx);

        assert_eq!(host_calls.load(Ordering::SeqCst), 1);
        assert_validation_failure_delivery(
            host_probe.last_accepted_correlation(),
            host_probe.delivered_responses(),
            host_probe.response_settlements(),
            host_probe.delivered_validation_errors(),
            host_probe.delivered_response(),
        );
        assert_eq!(host_probe.outstanding_counts(), (1, 0, 0, 0));
        exposed.begin_destroy().expect("Host destroy accepted");
        drive_host_until_idle(&host_servient, &mut cx);
        assert_host_clean(&host_probe, &mut cx);
        assert_eq!(host_calls.load(Ordering::SeqCst), 1);
        assert_eq!(host_probe.response_settlements(), 1);
    }

    #[test]
    fn incomplete_first_entry_policy_causes_no_preparation_side_effect() {
        let (binding, static_probe) = static_property_read_fixture();
        let mut static_limits = BenchmarkStaticReferenceV1::LIMITS.clone();
        assert!(static_limits.set(ResourceKind::CleanupItemsMax, Some(1)));
        let calls = Arc::new(AtomicU32::new(0));
        let evidence = Arc::new(Mutex::new(None));
        let handler = Handler { calls, evidence };
        let handler = StaticHandlerRegistration::new(
            HandlerSlotId::new(SlotIndex::new(0), Generation::INITIAL),
            &handler,
            HandlerFootprint::new(1, 0, 0),
        );
        let result = StaticServientBuilder::new(
            thing(),
            ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL),
            static_limits,
            clinkz_wot_core::Deadline::NONE,
        )
        .binding_registration(binding)
        .read_property_handler(PROPERTY, handler)
        .build();
        assert!(result.is_err());
        assert_eq!(static_probe.carrier_checks(), 0);
        assert_eq!(static_probe.preparation_side_effects(), 0);

        let (binding, host_probe) = host_property_read_fixture();
        let mut host_limits = GatewayDefaultV1::LIMITS.clone();
        assert!(host_limits.set(ResourceKind::CleanupItemsMax, Some(1)));
        let servient = ServientBuilder::new()
            .resource_limits(host_limits)
            .binding_registration(binding)
            .build()
            .expect("complete Host registration remains a legal root");
        assert!(servient.produce_td(thing()).is_err());
        assert_eq!(host_probe.carrier_checks(), 0);
        assert_eq!(host_probe.preparation_side_effects(), 0);
        assert_eq!(host_probe.route_state_drops(), 0);

        let (binding, static_probe) = static_property_read_fixture();
        let mut limits = BenchmarkStaticReferenceV1::LIMITS.clone();
        assert!(limits.set(ResourceKind::BindingArtifactBytesPerItemMax, Some(0)));
        let handler_value = Handler {
            calls: Arc::new(AtomicU32::new(0)),
            evidence: Arc::new(Mutex::new(None)),
        };
        let handler = StaticHandlerRegistration::new(
            HandlerSlotId::new(SlotIndex::new(0), Generation::INITIAL),
            &handler_value,
            HandlerFootprint::new(1, 0, 0),
        );
        let mut static_servient = StaticServientBuilder::new(
            thing(),
            ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL),
            limits,
            clinkz_wot_core::Deadline::NONE,
        )
        .binding_registration(binding)
        .read_property_handler(PROPERTY, handler)
        .build()
        .unwrap();
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        drive_static_until_idle(&mut static_servient, &mut cx);
        assert_eq!(static_probe.carrier_checks(), 0);
        assert_eq!(static_probe.preparation_side_effects(), 0);

        let (binding, host_probe) = host_property_read_fixture();
        let mut limits = GatewayDefaultV1::LIMITS.clone();
        assert!(limits.set(ResourceKind::BindingArtifactBytesPerItemMax, Some(0)));
        let host_servient = ServientBuilder::new()
            .resource_limits(limits)
            .binding_registration(binding)
            .build()
            .unwrap();
        let exposed = host_servient.produce_td(thing()).unwrap();
        exposed
            .set_read_property_handler(
                PROPERTY,
                Handler {
                    calls: Arc::new(AtomicU32::new(0)),
                    evidence: Arc::new(Mutex::new(None)),
                },
                HandlerFootprint::new(1, 0, 0),
            )
            .unwrap();
        exposed.begin_expose().unwrap();
        drive_host_until_idle(&host_servient, &mut cx);
        assert_eq!(host_probe.carrier_checks(), 0);
        assert_eq!(host_probe.preparation_side_effects(), 0);
    }

    #[test]
    fn readiness_failure_returns_production_owned_routes_to_cleanup() {
        let (binding, static_probe) = static_property_read_readiness_failure_fixture();
        let calls = Arc::new(AtomicU32::new(0));
        let evidence = Arc::new(Mutex::new(None));
        let handler = Handler {
            calls: Arc::clone(&calls),
            evidence,
        };
        let handler = StaticHandlerRegistration::new(
            HandlerSlotId::new(SlotIndex::new(0), Generation::INITIAL),
            &handler,
            HandlerFootprint::new(1, 0, 0),
        );
        let mut static_servient = StaticServientBuilder::new(
            thing(),
            ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL),
            BenchmarkStaticReferenceV1::LIMITS.clone(),
            clinkz_wot_core::Deadline::NONE,
        )
        .binding_registration(binding)
        .read_property_handler(PROPERTY, handler)
        .build()
        .expect("complete static first-entry roots");
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        drive_static_until_idle(&mut static_servient, &mut cx);
        assert_eq!(static_probe.carrier_checks(), 1);
        assert_eq!(static_probe.preparation_side_effects(), 1);
        assert_eq!(static_probe.aborted_routes(), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_static_clean(&static_probe, &mut cx);

        let (binding, host_probe) = host_property_read_readiness_rejection_fixture();
        let host_servient = ServientBuilder::new()
            .resource_limits(GatewayDefaultV1::LIMITS.clone())
            .binding_registration(binding)
            .build()
            .expect("complete Host binding installation");
        let exposed = host_servient
            .produce_td(thing())
            .expect("produced Thing roots");
        exposed
            .set_read_property_handler(
                PROPERTY,
                Handler {
                    calls: Arc::new(AtomicU32::new(0)),
                    evidence: Arc::new(Mutex::new(None)),
                },
                HandlerFootprint::new(1, 0, 0),
            )
            .expect("complete handler coverage");
        exposed
            .begin_expose()
            .expect("exposure transaction accepted");
        drive_host_until_idle(&host_servient, &mut cx);
        assert_eq!(host_probe.carrier_checks(), 1);
        assert_eq!(host_probe.preparation_side_effects(), 1);
        assert_eq!(host_probe.input_rejections(), (1, 1, 0));
        assert_eq!(host_probe.cleanup_attempts(), (1, 0));
        assert_host_clean(&host_probe, &mut cx);
    }

    #[test]
    fn outstanding_static_lifecycle_calls_cancel_before_route_rollback() {
        for phase in [
            MockLifecyclePhase::Prepare,
            MockLifecyclePhase::Readiness,
            MockLifecyclePhase::Activate,
            MockLifecyclePhase::Commit,
        ] {
            cancel_static_call(phase);
        }
    }

    #[test]
    fn outstanding_host_lifecycle_calls_cancel_before_route_rollback() {
        for phase in [
            MockLifecyclePhase::Prepare,
            MockLifecyclePhase::Readiness,
            MockLifecyclePhase::Activate,
            MockLifecyclePhase::Commit,
        ] {
            cancel_host_call(phase);
        }
    }

    #[test]
    fn ingress_capacity_is_admitted_at_every_scope_before_buffering_or_side_effects() {
        for kind in [
            ResourceKind::BindingIngressItemsPerRouteMax,
            ResourceKind::BindingIngressItemsPerBindingMax,
            ResourceKind::BindingIngressItemsGlobalMax,
            ResourceKind::BindingIngressBytesPerRouteMax,
            ResourceKind::BindingIngressBytesPerBindingMax,
            ResourceKind::BindingIngressBytesGlobalMax,
        ] {
            let (binding, static_probe) = static_property_read_fixture();
            let mut limits = BenchmarkStaticReferenceV1::LIMITS.clone();
            assert!(limits.set(kind, Some(0)));
            let handler_value = Handler {
                calls: Arc::new(AtomicU32::new(0)),
                evidence: Arc::new(Mutex::new(None)),
            };
            let handler = StaticHandlerRegistration::new(
                HandlerSlotId::new(SlotIndex::new(0), Generation::INITIAL),
                &handler_value,
                HandlerFootprint::new(1, 0, 0),
            );
            let result = StaticServientBuilder::new(
                thing(),
                ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL),
                limits,
                clinkz_wot_core::Deadline::NONE,
            )
            .binding_registration(binding)
            .read_property_handler(PROPERTY, handler)
            .build();
            assert!(
                result.is_err(),
                "static ingress scope {kind:?} was not admitted"
            );
            assert_eq!(static_probe.outstanding_counts(), (0, 0, 0, 0));
            assert_eq!(static_probe.ingress_usage(), [(0, 0); 3]);
            assert_eq!(static_probe.carrier_checks(), 0);
            assert_eq!(static_probe.preparation_side_effects(), 0);

            let (binding, host_probe) = host_property_read_fixture();
            let mut limits = GatewayDefaultV1::LIMITS.clone();
            assert!(limits.set(kind, Some(0)));
            let servient = ServientBuilder::new()
                .resource_limits(limits)
                .binding_registration(binding)
                .build()
                .expect("registration remains a legal root");
            assert!(
                servient.produce_td(thing()).is_err(),
                "Host ingress scope {kind:?} was not admitted"
            );
            assert_eq!(host_probe.outstanding_counts(), (0, 0, 0, 0));
            assert_eq!(host_probe.ingress_usage(), [(0, 0); 3]);
            assert_eq!(host_probe.carrier_checks(), 0);
            assert_eq!(host_probe.preparation_side_effects(), 0);
            assert_eq!(host_probe.route_state_drops(), 0);
        }
    }

    #[test]
    fn oversized_activation_uses_separately_admitted_recovery_and_preserves_predecessor() {
        let (binding, probe) = host_property_read_oversized_activation_fixture();
        let servient = ServientBuilder::new()
            .resource_limits(GatewayDefaultV1::LIMITS.clone())
            .binding_registration(binding)
            .build()
            .expect("complete Host binding installation");
        let exposed = servient.produce_td(thing()).expect("produced Thing roots");
        exposed
            .set_read_property_handler(
                PROPERTY,
                Handler {
                    calls: Arc::new(AtomicU32::new(0)),
                    evidence: Arc::new(Mutex::new(None)),
                },
                HandlerFootprint::new(1, 0, 0),
            )
            .unwrap();
        exposed.begin_expose().unwrap();
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        drive_host_until_idle(&servient, &mut cx);

        assert_eq!(probe.lifecycle_starts(MockLifecyclePhase::Activate), 0);
        assert_eq!(
            probe.lifecycle_cancellations(MockLifecyclePhase::Activate),
            1
        );
        assert_eq!(probe.cancellation_polls(MockLifecyclePhase::Activate), 2);
        assert_eq!(probe.cleanup_attempts(), (1, 0));
        let call_context = assert_cleanup_context_round_trip(
            &probe,
            MockLifecyclePhase::Activate,
            CleanupOperation::CancelRouteReadiness,
        );
        let route_context = assert_cleanup_context_round_trip(
            &probe,
            MockLifecyclePhase::Abort,
            CleanupOperation::AbortPreparedRoute,
        );
        assert_ne!(call_context.subject(), route_context.subject());
        let (prepared, active, committed, _, _, _) = probe.carrier_evidence();
        assert!(prepared.is_some());
        assert!(active.is_none());
        assert!(committed.is_none());
        assert_host_clean(&probe, &mut cx);
    }

    #[test]
    fn insufficient_host_call_recovery_capacity_rejects_before_construction_or_polling() {
        for (kind, limit) in [
            (ResourceKind::CleanupItemsMax, 31),
            (ResourceKind::BindingCancelBufferBytesPerCallMax, 1_023),
        ] {
            let (binding, probe) = host_property_read_oversized_activation_fixture();
            let mut limits = GatewayDefaultV1::LIMITS.clone();
            assert!(limits.set(kind, Some(limit)));
            let servient = ServientBuilder::new()
                .resource_limits(limits)
                .binding_registration(binding)
                .build()
                .expect("binding registration remains a legal root");
            assert!(
                servient.produce_td(thing()).is_err(),
                "{kind:?} did not reject insufficient recovery admission"
            );
            assert_eq!(probe.carrier_checks(), 0);
            assert_eq!(probe.preparation_side_effects(), 0);
            assert_eq!(probe.lifecycle_starts(MockLifecyclePhase::Prepare), 0);
            assert_eq!(probe.lifecycle_starts(MockLifecyclePhase::Activate), 0);
            assert_eq!(
                probe.lifecycle_cancellations(MockLifecyclePhase::Activate),
                0
            );
            assert_eq!(probe.cancellation_polls(MockLifecyclePhase::Activate), 0);
            assert_eq!(probe.outstanding_counts(), (0, 0, 0, 0));
            assert_eq!(probe.route_state_drops(), 0);
        }
    }

    #[test]
    fn lifecycle_transfer_uses_production_owner_and_retains_context_to_terminal() {
        let (binding, probe) = host_property_read_activation_transfer_fixture();
        let servient = ServientBuilder::new()
            .resource_limits(GatewayDefaultV1::LIMITS.clone())
            .binding_registration(binding)
            .build()
            .unwrap();
        let exposed = servient.produce_td(thing()).unwrap();
        exposed
            .set_read_property_handler(
                PROPERTY,
                Handler {
                    calls: Arc::new(AtomicU32::new(0)),
                    evidence: Arc::new(Mutex::new(None)),
                },
                HandlerFootprint::new(1, 0, 0),
            )
            .unwrap();
        exposed.begin_expose().unwrap();
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        for _ in 0..32 {
            let _ = servient.step(&mut cx, &mut work_budget());
            if probe.lifecycle_starts(MockLifecyclePhase::Activate) == 1 {
                break;
            }
        }
        exposed.begin_destroy().unwrap();
        drive_host_until_idle(&servient, &mut cx);

        let context = assert_cleanup_context_round_trip(
            &probe,
            MockLifecyclePhase::Activate,
            CleanupOperation::CancelRouteReadiness,
        );
        let (requested, continuations, owner) =
            probe.transfer_evidence(MockLifecyclePhase::Activate);
        assert_eq!(requested, Some(context.clone()));
        assert_eq!(continuations, 1);
        assert_eq!(owner, context.transfer_owner());
        assert_ne!(owner, Some(context.subject()));
        assert_eq!(probe.cancellation_polls(MockLifecyclePhase::Activate), 3);
        assert_eq!(probe.cleanup_attempts(), (1, 0));
        assert_host_clean(&probe, &mut cx);
    }

    #[test]
    fn delivery_transfer_uses_production_owner_across_error_retry_and_terminal() {
        let (binding, probe) = host_property_read_delivery_error_transfer_fixture();
        let servient = ServientBuilder::new()
            .resource_limits(GatewayDefaultV1::LIMITS.clone())
            .binding_registration(binding)
            .build()
            .unwrap();
        let calls = Arc::new(AtomicU32::new(0));
        let exposed = servient.produce_td(thing()).unwrap();
        exposed
            .set_read_property_handler(
                PROPERTY,
                Handler {
                    calls: Arc::clone(&calls),
                    evidence: Arc::new(Mutex::new(None)),
                },
                HandlerFootprint::new(1, 0, 0),
            )
            .unwrap();
        exposed.begin_expose().unwrap();
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        drive_host_until_idle(&servient, &mut cx);
        probe.enqueue_property_read(PROPERTY, InteractionInput::empty());
        for _ in 0..32 {
            let _ = servient.step(&mut cx, &mut work_budget());
            if probe.lifecycle_starts(MockLifecyclePhase::ResponseDelivery) == 1 {
                break;
            }
        }
        exposed.begin_destroy().unwrap();
        drive_host_until_idle(&servient, &mut cx);

        let context = assert_cleanup_context_round_trip(
            &probe,
            MockLifecyclePhase::ResponseDelivery,
            CleanupOperation::CancelResponseDelivery,
        );
        let (requested, continuations, owner) =
            probe.transfer_evidence(MockLifecyclePhase::ResponseDelivery);
        assert_eq!(probe.delivery_cancel_errors(), 1);
        assert_eq!(
            probe.cancellation_polls(MockLifecyclePhase::ResponseDelivery),
            4
        );
        assert_eq!(requested, Some(context.clone()));
        assert_eq!(continuations, 1);
        assert_eq!(owner, context.transfer_owner());
        assert_ne!(owner, Some(context.subject()));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(probe.response_settlements(), 1);
        assert_eq!(probe.delivered_responses(), 0);
        assert_host_clean(&probe, &mut cx);
    }

    #[test]
    fn cleanup_call_transfer_uses_production_owner_and_retains_both_contexts() {
        let (binding, probe) = host_property_read_cleanup_transfer_fixture();
        let servient = ServientBuilder::new()
            .resource_limits(GatewayDefaultV1::LIMITS.clone())
            .binding_registration(binding)
            .build()
            .unwrap();
        let exposed = servient.produce_td(thing()).unwrap();
        exposed
            .set_read_property_handler(
                PROPERTY,
                Handler {
                    calls: Arc::new(AtomicU32::new(0)),
                    evidence: Arc::new(Mutex::new(None)),
                },
                HandlerFootprint::new(1, 0, 0),
            )
            .unwrap();
        exposed.begin_expose().unwrap();
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        drive_host_until_idle(&servient, &mut cx);
        exposed.begin_destroy().unwrap();
        drive_host_until_idle(&servient, &mut cx);

        let route_context = assert_cleanup_context_round_trip(
            &probe,
            MockLifecyclePhase::Shutdown,
            CleanupOperation::ShutdownRoute,
        );
        let call_context = assert_cleanup_context_round_trip(
            &probe,
            MockLifecyclePhase::CleanupCallCancellation,
            CleanupOperation::CancelProcess,
        );
        let (requested, continuations, owner) =
            probe.transfer_evidence(MockLifecyclePhase::CleanupCallCancellation);
        assert_ne!(route_context.subject(), call_context.subject());
        assert_eq!(requested, Some(call_context.clone()));
        assert_eq!(continuations, 1);
        assert_eq!(owner, call_context.transfer_owner());
        assert_ne!(owner, Some(call_context.subject()));
        assert_eq!(
            probe.cancellation_polls(MockLifecyclePhase::CleanupCallCancellation),
            3
        );
        assert_eq!(probe.cleanup_attempts(), (0, 1));
        assert_host_clean(&probe, &mut cx);
    }

    #[test]
    fn pending_response_delivery_cancellation_retains_exact_context_and_cleans_once() {
        let (binding, probe) = host_property_read_fixture();
        let servient = ServientBuilder::new()
            .resource_limits(GatewayDefaultV1::LIMITS.clone())
            .binding_registration(binding)
            .build()
            .unwrap();
        let calls = Arc::new(AtomicU32::new(0));
        let exposed = servient.produce_td(thing()).unwrap();
        exposed
            .set_read_property_handler(
                PROPERTY,
                Handler {
                    calls: Arc::clone(&calls),
                    evidence: Arc::new(Mutex::new(None)),
                },
                HandlerFootprint::new(1, 0, 0),
            )
            .unwrap();
        exposed.begin_expose().unwrap();
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        drive_host_until_idle(&servient, &mut cx);
        probe.enqueue_property_read(PROPERTY, InteractionInput::empty());
        for _ in 0..32 {
            let _ = servient.step(&mut cx, &mut work_budget());
            if probe.lifecycle_starts(MockLifecyclePhase::ResponseDelivery) == 1 {
                break;
            }
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            probe.lifecycle_starts(MockLifecyclePhase::ResponseDelivery),
            1
        );
        assert_eq!(probe.delivered_responses(), 0);
        assert_ingress_live(probe.ingress_usage());
        exposed.begin_destroy().unwrap();
        drive_host_until_idle(&servient, &mut cx);

        assert_eq!(
            probe.lifecycle_cancellations(MockLifecyclePhase::ResponseDelivery),
            1
        );
        assert_eq!(probe.response_settlements(), 1);
        assert_eq!(probe.delivered_responses(), 0);
        let delivery = assert_cleanup_context_round_trip(
            &probe,
            MockLifecyclePhase::ResponseDelivery,
            CleanupOperation::CancelResponseDelivery,
        );
        let shutdown = assert_cleanup_context_round_trip(
            &probe,
            MockLifecyclePhase::Shutdown,
            CleanupOperation::ShutdownRoute,
        );
        assert_ne!(delivery.subject(), shutdown.subject());
        assert_host_clean(&probe, &mut cx);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn pending_cleanup_call_cancellation_retains_route_and_call_contexts_to_terminal() {
        let (binding, probe) = host_property_read_oversized_shutdown_fixture();
        let servient = ServientBuilder::new()
            .resource_limits(GatewayDefaultV1::LIMITS.clone())
            .binding_registration(binding)
            .build()
            .unwrap();
        let exposed = servient.produce_td(thing()).unwrap();
        exposed
            .set_read_property_handler(
                PROPERTY,
                Handler {
                    calls: Arc::new(AtomicU32::new(0)),
                    evidence: Arc::new(Mutex::new(None)),
                },
                HandlerFootprint::new(1, 0, 0),
            )
            .unwrap();
        exposed.begin_expose().unwrap();
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        drive_host_until_idle(&servient, &mut cx);
        exposed.begin_destroy().unwrap();
        drive_host_until_idle(&servient, &mut cx);

        assert_eq!(
            probe.lifecycle_cancellations(MockLifecyclePhase::CleanupCallCancellation),
            1
        );
        let route_cleanup = assert_cleanup_context_round_trip(
            &probe,
            MockLifecyclePhase::Shutdown,
            CleanupOperation::ShutdownRoute,
        );
        let call_cleanup = assert_cleanup_context_round_trip(
            &probe,
            MockLifecyclePhase::CleanupCallCancellation,
            CleanupOperation::CancelProcess,
        );
        assert_ne!(route_cleanup.subject(), call_cleanup.subject());
        assert_eq!(probe.cleanup_attempts(), (0, 1));
        assert_host_clean(&probe, &mut cx);
    }

    #[test]
    fn runner_source_constructs_no_forbidden_intermediate_carrier() {
        let source = include_str!("lib.rs");
        for (carrier, constructor) in [
            ("LogicalInteractionPlan", "::"),
            ("BindingArtifact", "::"),
            ("BindingArtifactEnvelope", "::"),
            ("BindingArtifactRef", "::new"),
            ("RouteReservationIdentity", "::new"),
            ("BindingRouteKey", "::new"),
            ("PrepareInput", "::new"),
            ("HandlerContext", "::try_new"),
            ("RouteResponseOpportunity", "::new"),
            ("CleanupReservation", "::new"),
        ] {
            let forbidden = format!("{carrier}{constructor}");
            assert!(
                !source.contains(&forbidden),
                "aggregate runner constructs forbidden carrier with {forbidden}"
            );
        }
    }
}
