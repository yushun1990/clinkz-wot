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

#[test]
fn valid_operation_target_matrix_is_exact() {
    let thing_id = ThingId::from("urn:thing:handler-context");
    let cases = [
        (
            Operation::ReadProperty,
            AffordanceTarget::Property("p".into()),
        ),
        (
            Operation::WriteProperty,
            AffordanceTarget::Property("p".into()),
        ),
        (
            Operation::ObserveProperty,
            AffordanceTarget::Property("p".into()),
        ),
        (
            Operation::UnobserveProperty,
            AffordanceTarget::Property("p".into()),
        ),
        (
            Operation::InvokeAction,
            AffordanceTarget::Action("a".into()),
        ),
        (Operation::QueryAction, AffordanceTarget::Action("a".into())),
        (
            Operation::CancelAction,
            AffordanceTarget::Action("a".into()),
        ),
        (
            Operation::SubscribeEvent,
            AffordanceTarget::Event("e".into()),
        ),
        (
            Operation::UnsubscribeEvent,
            AffordanceTarget::Event("e".into()),
        ),
        (Operation::ReadAllProperties, AffordanceTarget::Thing),
        (Operation::WriteAllProperties, AffordanceTarget::Thing),
        (Operation::ReadMultipleProperties, AffordanceTarget::Thing),
        (Operation::WriteMultipleProperties, AffordanceTarget::Thing),
        (Operation::ObserveAllProperties, AffordanceTarget::Thing),
        (Operation::UnobserveAllProperties, AffordanceTarget::Thing),
        (Operation::QueryAllActions, AffordanceTarget::Thing),
        (Operation::SubscribeAllEvents, AffordanceTarget::Thing),
        (Operation::UnsubscribeAllEvents, AffordanceTarget::Thing),
    ];

    for (operation, target) in &cases {
        let context = HandlerContext::try_new(
            &thing_id,
            thing_slot(),
            target,
            *operation,
            plan_id(),
            Some(binding()),
        )
        .expect("the frozen operation/target pair must be valid");
        let copied = context;
        assert_eq!(copied, context);
        assert_eq!(context.thing_id(), &thing_id);
        assert_eq!(context.thing_slot(), thing_slot());
        assert_eq!(context.target(), target);
        assert_eq!(context.operation(), *operation);
        assert_eq!(context.plan_id(), plan_id());
        assert_eq!(context.binding(), Some(binding()));
    }
}

#[test]
fn incompatible_target_returns_exact_validation_context() {
    let thing_id = ThingId::from("urn:thing:handler-context");
    let invalid = [
        (Operation::ReadProperty, AffordanceTarget::Thing),
        (
            Operation::InvokeAction,
            AffordanceTarget::Property("p".into()),
        ),
        (
            Operation::SubscribeEvent,
            AffordanceTarget::Action("a".into()),
        ),
        (
            Operation::ReadAllProperties,
            AffordanceTarget::Event("e".into()),
        ),
    ];

    for (operation, target) in invalid {
        let error = HandlerContext::try_new(
            &thing_id,
            thing_slot(),
            &target,
            operation,
            plan_id(),
            Some(binding()),
        )
        .expect_err("a cross-kind target must be rejected");
        let CoreError::Validation(context) = error else {
            panic!("incompatible context must be a validation error");
        };
        assert_eq!(context.phase(), ErrorPhase::Validate);
        assert_eq!(context.retry_class(), RetryClass::Never);
        assert_eq!(context.thing(), Some(thing_slot()));
        assert_eq!(context.operation(), Some(operation));
        assert_eq!(context.plan(), Some(plan_id()));
        assert_eq!(context.binding(), Some(binding()));
        assert_eq!(context.target(), None);
        assert_eq!(context.correlation(), None);
    }
}

#[test]
fn absent_binding_remains_absent() {
    let thing_id = ThingId::from("urn:thing:handler-context");
    let context = HandlerContext::try_new(
        &thing_id,
        thing_slot(),
        &AffordanceTarget::Thing,
        Operation::ReadAllProperties,
        plan_id(),
        None,
    )
    .expect("Thing collection operation is valid");
    assert_eq!(context.binding(), None);
}
