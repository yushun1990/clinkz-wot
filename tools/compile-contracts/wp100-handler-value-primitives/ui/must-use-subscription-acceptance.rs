#![deny(unused_must_use)]

use clinkz_wot_core::{InteractionOutput, SubscriptionAcceptance};

fn main() {
    SubscriptionAcceptance::new(InteractionOutput::empty());
}
