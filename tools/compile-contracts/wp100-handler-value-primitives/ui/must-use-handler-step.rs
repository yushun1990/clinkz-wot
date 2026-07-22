#![deny(unused_must_use)]

use clinkz_wot_core::HandlerStep;

fn main() {
    HandlerStep::<()>::Pending;
}
