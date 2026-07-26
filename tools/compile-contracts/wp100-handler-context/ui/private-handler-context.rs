use clinkz_wot_core::{
    AffordanceTarget, BindingGeneration, BindingId, HandlerContext, PlanId, ThingId, ThingSlotId,
};
use clinkz_wot_foundation::{Generation, SlotIndex};
use clinkz_wot_td::data_type::Operation;

fn main() {
    let thing_id = ThingId::from("urn:thing:private");
    let target = AffordanceTarget::Thing;
    let _ = HandlerContext {
        thing_id: &thing_id,
        thing_slot: ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL),
        target: &target,
        operation: Operation::ReadAllProperties,
        plan_id: PlanId::new(SlotIndex::new(2), Generation::INITIAL),
        binding: Some((BindingId::new(3), BindingGeneration::INITIAL)),
    };
}
