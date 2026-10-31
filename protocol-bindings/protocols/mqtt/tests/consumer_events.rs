#![cfg(feature = "broker-tests")]
#[path = "support/broker.rs"]
mod broker;
mod support;
const KIND: clinkz_wot_core::notification::NotificationKind =
    clinkz_wot_core::notification::NotificationKind::Event;
const NAME: &str = "alarm";
include!("support/consumer_notifications.rs");
