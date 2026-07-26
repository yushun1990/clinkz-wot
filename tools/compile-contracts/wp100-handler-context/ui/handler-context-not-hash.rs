use core::hash::Hash;

use clinkz_wot_core::HandlerContext;

fn require_hash<T: Hash>() {}

fn main() {
    require_hash::<HandlerContext<'static>>();
}
