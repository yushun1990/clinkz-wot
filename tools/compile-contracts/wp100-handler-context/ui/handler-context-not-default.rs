use clinkz_wot_core::HandlerContext;

fn require_default<T: Default>() {}

fn main() {
    require_default::<HandlerContext<'static>>();
}
