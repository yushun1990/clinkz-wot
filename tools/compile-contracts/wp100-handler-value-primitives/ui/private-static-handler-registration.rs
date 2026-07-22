use clinkz_wot_core::{HandlerFootprint, HandlerSlotId, StaticHandlerRegistration};
use clinkz_wot_foundation::{Generation, SlotIndex};

fn main() {
    let handler = ();
    let _ = StaticHandlerRegistration {
        slot_id: HandlerSlotId::new(SlotIndex::new(1), Generation::INITIAL),
        handler: &handler,
        footprint: HandlerFootprint::new(2, 3, 5),
    };
}
