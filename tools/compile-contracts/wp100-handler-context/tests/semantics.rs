use clinkz_wot_core::{
    AffordanceTarget, BindingGeneration, BindingId, CoreError, ErrorPhase, HandlerContext, PlanId,
    RetryClass, ThingId, ThingSlotId,
};
use clinkz_wot_foundation::{Generation, SlotIndex};
use clinkz_wot_td::data_type::Operation;

fn thing_slot() -> ThingSlotId {
    ThingSlotId::new(SlotIndex::new(3), Generation::INITIAL)
}

fn plan_id() -> PlanId {
    PlanId::new(SlotIndex::new(5), Generation::INITIAL)
}

fn binding() -> (BindingId, BindingGeneration) {
    (BindingId::new(7), BindingGeneration::INITIAL)
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

const TARGET_KINDS: [TargetKind; 4] = [
    TargetKind::Thing,
    TargetKind::Property,
    TargetKind::Action,
    TargetKind::Event,
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
        Operation::ReadAllProperties
        | Operation::WriteAllProperties
        | Operation::ReadMultipleProperties
        | Operation::WriteMultipleProperties
        | Operation::ObserveAllProperties
        | Operation::UnobserveAllProperties
        | Operation::QueryAllActions
        | Operation::SubscribeAllEvents
        | Operation::UnsubscribeAllEvents => TargetKind::Thing,
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

fn assert_validation_context(
    error: CoreError,
    operation: Operation,
    supplied_binding: Option<(BindingId, BindingGeneration)>,
) {
    let CoreError::Validation(context) = error else {
        panic!("incompatible context must be a validation error");
    };
    assert_eq!(context.phase(), ErrorPhase::Validate);
    assert_eq!(context.retry_class(), RetryClass::Never);
    assert_eq!(context.thing(), Some(thing_slot()));
    assert_eq!(context.operation(), Some(operation));
    assert_eq!(context.plan(), Some(plan_id()));
    assert_eq!(context.binding(), supplied_binding);
    assert_eq!(context.target(), None);
    assert_eq!(context.form_index(), None);
    assert_eq!(context.correlation(), None);
    assert_eq!(context.retry_after(), None);
    assert_eq!(context.cause_code(), None);
    assert_eq!(context.redacted_cause(), None);
    assert!(!context.cause_was_truncated());
}

#[test]
fn operation_target_matrix_is_exact() {
    let thing_id = ThingId::from("urn:thing:handler-context");
    let mut accepted = 0;
    let mut rejected = 0;
    let mut rejected_with_binding = 0;
    let mut rejected_without_binding = 0;

    for (operation_index, operation) in OPERATIONS.into_iter().enumerate() {
        for (target_index, kind) in TARGET_KINDS.into_iter().enumerate() {
            let target = target(kind);
            let supplied_binding = ((operation_index + target_index) % 2 == 0).then_some(binding());
            let result = HandlerContext::try_new(
                &thing_id,
                thing_slot(),
                &target,
                operation,
                plan_id(),
                supplied_binding,
            );

            if kind == expected_target(operation) {
                accepted += 1;
                let context = result.expect("the compatible operation/target pair must be valid");
                let copied = context;
                assert_eq!(copied, context);
                assert_eq!(context.thing_id(), &thing_id);
                assert_eq!(context.thing_slot(), thing_slot());
                assert_eq!(context.target(), &target);
                assert_eq!(context.operation(), operation);
                assert_eq!(context.plan_id(), plan_id());
                assert_eq!(context.binding(), supplied_binding);
            } else {
                rejected += 1;
                if supplied_binding.is_some() {
                    rejected_with_binding += 1;
                } else {
                    rejected_without_binding += 1;
                }
                assert_validation_context(
                    result.expect_err("every incompatible operation/target pair must be rejected"),
                    operation,
                    supplied_binding,
                );
            }
        }
    }

    assert_eq!(accepted, 18);
    assert_eq!(rejected, 54);
    assert!(rejected_with_binding > 0);
    assert!(rejected_without_binding > 0);
}
