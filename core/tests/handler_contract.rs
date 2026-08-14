use core::fmt;
use std::cell::Cell;
use std::rc::Rc;

use clinkz_wot_core::{
    AffordanceTarget, BindingGeneration, BindingId, CancellationView, CoreError, CoreResult,
    ErrorPhase, HandlerContext, HandlerFootprint, HandlerSlotId, HandlerStep, InteractionInput,
    InteractionOutput, InteractionStatus, PlanId, ReadPropertyHandler, RetryClass,
    StaticHandlerRegistration, SubscriptionAcceptance, ThingId, ThingSlotId,
};
use clinkz_wot_foundation::{Generation, SlotIndex};
use clinkz_wot_td::data_type::Operation;

fn thing_slot() -> ThingSlotId {
    ThingSlotId::new(SlotIndex::new(3), Generation::INITIAL)
}

fn handler_slot() -> HandlerSlotId {
    HandlerSlotId::new(SlotIndex::new(5), Generation::INITIAL)
}

fn plan_id() -> PlanId {
    PlanId::new(SlotIndex::new(7), Generation::INITIAL)
}

#[test]
fn passive_handler_values_preserve_linear_and_bounded_semantics() {
    assert_eq!(CancellationView::default(), CancellationView::Active);
    assert_eq!(CancellationView::Active as u8, 0);
    assert_eq!(CancellationView::Requested as u8, 1);
    assert!(!CancellationView::Active.is_requested());
    assert!(CancellationView::Requested.is_requested());

    let response = InteractionOutput::empty().with_status(InteractionStatus::Created);
    let acceptance = SubscriptionAcceptance::new(response.clone());
    assert_eq!(acceptance.response(), &response);
    assert_eq!(acceptance.into_response(), response);

    let maximum = HandlerFootprint::new(u64::MAX, u64::MAX, u64::MAX);
    assert_eq!(maximum.retained_bytes(), u64::MAX);
    assert_eq!(maximum.pending_call_bytes(), u64::MAX);
    assert_eq!(maximum.subscription_bytes(), u64::MAX);

    assert!(matches!(HandlerStep::<u32>::Pending, HandlerStep::Pending));
    assert_eq!(
        match HandlerStep::Ready(Ok(37_u32)) {
            HandlerStep::Pending => None,
            HandlerStep::Ready(result) => Some(result),
        },
        Some(Ok(37)),
    );
}

struct SecretHandler;

impl fmt::Debug for SecretHandler {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("handler-secret-must-not-appear")
    }
}

#[test]
fn static_registration_borrows_without_exposing_the_handler_in_debug() {
    let handler = SecretHandler;
    let footprint = HandlerFootprint::new(43, 47, 53);
    let registration = StaticHandlerRegistration::new(handler_slot(), &handler, footprint);

    assert_eq!(registration.slot_id(), handler_slot());
    assert!(core::ptr::eq(registration.handler(), &handler));
    assert_eq!(registration.footprint(), footprint);
    let rendered = format!("{registration:?}");
    assert!(rendered.contains("slot_id"));
    assert!(rendered.contains("footprint"));
    assert!(!rendered.contains("handler-secret-must-not-appear"));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TargetKind {
    Thing,
    Property,
    Action,
    Event,
}

const OPERATIONS: [Operation; 18] = [
    Operation::ReadProperty,
    Operation::WriteProperty,
    Operation::ObserveProperty,
    Operation::UnobserveProperty,
    Operation::InvokeAction,
    Operation::QueryAction,
    Operation::CancelAction,
    Operation::SubscribeEvent,
    Operation::UnsubscribeEvent,
    Operation::ReadAllProperties,
    Operation::WriteAllProperties,
    Operation::ReadMultipleProperties,
    Operation::WriteMultipleProperties,
    Operation::ObserveAllProperties,
    Operation::UnobserveAllProperties,
    Operation::QueryAllActions,
    Operation::SubscribeAllEvents,
    Operation::UnsubscribeAllEvents,
];

fn expected_target(operation: Operation) -> TargetKind {
    match operation {
        Operation::ReadProperty
        | Operation::WriteProperty
        | Operation::ObserveProperty
        | Operation::UnobserveProperty => TargetKind::Property,
        Operation::InvokeAction | Operation::QueryAction | Operation::CancelAction => {
            TargetKind::Action
        }
        Operation::SubscribeEvent | Operation::UnsubscribeEvent => TargetKind::Event,
        _ => TargetKind::Thing,
    }
}

fn target(kind: TargetKind) -> AffordanceTarget {
    match kind {
        TargetKind::Thing => AffordanceTarget::Thing,
        TargetKind::Property => AffordanceTarget::Property("p".into()),
        TargetKind::Action => AffordanceTarget::Action("a".into()),
        TargetKind::Event => AffordanceTarget::Event("e".into()),
    }
}

#[test]
fn handler_context_accepts_exactly_the_operation_target_matrix() {
    let thing_id = ThingId::from("urn:thing:handler-context");
    let binding = Some((BindingId::new(7), BindingGeneration::INITIAL));
    let kinds = [
        TargetKind::Thing,
        TargetKind::Property,
        TargetKind::Action,
        TargetKind::Event,
    ];
    let mut accepted = 0;
    let mut rejected = 0;

    for operation in OPERATIONS {
        for kind in kinds {
            let target = target(kind);
            let result = HandlerContext::try_new(
                &thing_id,
                thing_slot(),
                &target,
                operation,
                plan_id(),
                binding,
            );
            if kind == expected_target(operation) {
                accepted += 1;
                let context = result.expect("compatible operation and target");
                assert_eq!(context.thing_id(), &thing_id);
                assert_eq!(context.thing_slot(), thing_slot());
                assert_eq!(context.target(), &target);
                assert_eq!(context.operation(), operation);
                assert_eq!(context.plan_id(), plan_id());
                assert_eq!(context.binding(), binding);
            } else {
                rejected += 1;
                let error = result.expect_err("incompatible operation and target");
                assert!(matches!(error, CoreError::Validation(_)));
                assert_eq!(error.context().phase(), ErrorPhase::Validate);
                assert_eq!(error.retry_class(), RetryClass::Never);
                assert_eq!(error.context().thing(), Some(thing_slot()));
                assert_eq!(error.context().operation(), Some(operation));
                assert_eq!(error.context().plan(), Some(plan_id()));
                assert_eq!(error.context().binding(), binding);
            }
        }
    }

    assert_eq!(accepted, 18);
    assert_eq!(rejected, 54);
}

struct NonThreadSafeHandler {
    calls: Rc<Cell<u32>>,
}

impl ReadPropertyHandler for NonThreadSafeHandler {
    fn handle(
        &self,
        context: HandlerContext<'_>,
        _input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        assert_eq!(context.operation(), Operation::ReadProperty);
        self.calls.set(self.calls.get() + 1);
        Ok(InteractionOutput::empty())
    }
}

#[test]
fn synchronous_property_read_is_object_safe_and_has_no_thread_supertraits() {
    let calls = Rc::new(Cell::new(0));
    let handler = NonThreadSafeHandler {
        calls: Rc::clone(&calls),
    };
    let object: &dyn ReadPropertyHandler = &handler;
    let thing_id = ThingId::from("urn:clinkz:property-read");
    let target = AffordanceTarget::Property("temperature".into());
    let context = HandlerContext::try_new(
        &thing_id,
        thing_slot(),
        &target,
        Operation::ReadProperty,
        plan_id(),
        None,
    )
    .expect("property-read context");

    assert_eq!(
        object.handle(context, &InteractionInput::empty()),
        Ok(InteractionOutput::empty()),
    );
    assert_eq!(calls.get(), 1);
}
