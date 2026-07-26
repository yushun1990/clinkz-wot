use clinkz_wot_core::StepReadPropertyHandler;

fn main() {
    let _ = core::mem::size_of::<&dyn StepReadPropertyHandler>();
}
