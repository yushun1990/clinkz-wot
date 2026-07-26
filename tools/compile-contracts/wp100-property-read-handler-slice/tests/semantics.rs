use std::cell::Cell;
use std::rc::Rc;

use clinkz_wot_core::{
    AffordanceTarget, CoreResult, HandlerContext, HandlerFootprint, HandlerSlotId,
    InteractionInput, InteractionOutput, PlanId, ReadPropertyHandler, StaticHandlerRegistration,
    ThingId, ThingSlotId,
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

struct NonThreadSafeHandler {
    calls: Rc<Cell<u32>>,
}

impl ReadPropertyHandler for NonThreadSafeHandler {
    fn handle(
        &self,
        context: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        assert_eq!(context.operation(), Operation::ReadProperty);
        assert_eq!(context.target().name(), Some("temperature"));
        let _ = input;
        self.calls.set(self.calls.get() + 1);
        Ok(InteractionOutput::empty())
    }
}

#[test]
fn synchronous_trait_is_object_safe_and_has_no_thread_supertraits() {
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
    .expect("property-read context must be valid");
    let input = InteractionInput::empty();

    assert_eq!(
        object.handle(context, &input),
        Ok(InteractionOutput::empty())
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn borrowed_static_registration_invokes_the_same_handler_once() {
    let calls = Rc::new(Cell::new(0));
    let handler = NonThreadSafeHandler {
        calls: Rc::clone(&calls),
    };
    let registration = StaticHandlerRegistration::new(
        handler_slot(),
        &handler,
        HandlerFootprint::new(0, 0, 0),
    );
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
    .expect("property-read context must be valid");
    let input = InteractionInput::empty();

    assert_eq!(
        registration.handler().handle(context, &input),
        Ok(InteractionOutput::empty())
    );
    assert_eq!(registration.slot_id(), handler_slot());
    assert_eq!(registration.footprint(), HandlerFootprint::new(0, 0, 0));
    assert_eq!(calls.get(), 1);
}
