//! Aggregate Property Read architecture proof.
//!
//! This runner supplies only legal roots: a Thing Description, generations,
//! complete external binding registrations, handler behavior, protocol-local
//! requests, and resource/work policy. Servient and its production
//! dependencies create every intermediate carrier exercised below.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(test, feature = "std"))]
mod tests {
    use core::sync::atomic::{AtomicU32, Ordering};
    use core::task::{Context, Poll, Waker};
    use std::sync::{Arc, Mutex};

    use clinkz_wot_core::{
        BindingGeneration, BindingId, CoreResult, HandlerContext, HandlerFootprint, HandlerSlotId,
        InteractionInput, InteractionOutput, PlanId, ReadPropertyHandler,
        StaticHandlerRegistration, StepStatus, ThingSlotId,
    };
    use clinkz_wot_foundation::{
        BenchmarkStaticReferenceV1, GatewayDefaultV1, Generation, ResourceKind, SlotIndex,
        StaticResourceProfile, WorkBudget, WorkClass,
    };
    use clinkz_wot_property_read_binding_fixture::{
        HostPropertyReadProbe, StaticPropertyReadProbe, host_property_read_fixture,
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

    fn assert_static_clean(probe: &StaticPropertyReadProbe, cx: &mut Context<'_>) {
        assert_eq!(probe.outstanding_counts(), (0, 0, 0, 0));
        assert_eq!(probe.poll_after_close(cx), Poll::Ready(false));
        assert_eq!(probe.artifact_drops(), 1);
    }

    fn assert_host_clean(probe: &HostPropertyReadProbe, cx: &mut Context<'_>) {
        assert_eq!(probe.outstanding_counts(), (0, 0, 0, 0));
        assert_eq!(probe.poll_after_close(cx), Poll::Ready(false));
        assert_eq!(probe.artifact_drops(), 1);
        assert_eq!(probe.route_state_drops(), 1);
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

        drive_static_until_idle(&mut servient, &mut cx);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(probe.delivered_responses(), 1);
        assert_handler_evidence(&evidence);

        servient.begin_destroy().expect("static destroy accepted");
        drive_static_until_idle(&mut servient, &mut cx);
        assert_static_clean(&probe, &mut cx);
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

        drive_host_until_idle(&servient, &mut cx);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(probe.delivered_responses(), 1);
        assert_handler_evidence(&evidence);

        exposed.begin_destroy().expect("Host destroy accepted");
        drive_host_until_idle(&servient, &mut cx);
        assert_host_clean(&probe, &mut cx);
    }

    #[test]
    fn incomplete_first_entry_policy_causes_no_preparation_side_effect() {
        let (binding, static_probe) = static_property_read_fixture();
        let mut static_limits = BenchmarkStaticReferenceV1::LIMITS.clone();
        assert!(static_limits.set(ResourceKind::CleanupItemsMax, Some(0)));
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
        assert!(host_limits.set(ResourceKind::CleanupItemsMax, Some(0)));
        let servient = ServientBuilder::new()
            .resource_limits(host_limits)
            .binding_registration(binding)
            .build()
            .expect("complete Host registration remains a legal root");
        assert!(servient.produce_td(thing()).is_err());
        assert_eq!(host_probe.carrier_checks(), 0);
        assert_eq!(host_probe.preparation_side_effects(), 0);
        assert_eq!(host_probe.route_state_drops(), 0);
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
