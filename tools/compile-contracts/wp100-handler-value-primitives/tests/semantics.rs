use core::fmt;

use clinkz_wot_core::{
    CancellationView, HandlerFootprint, HandlerSlotId, HandlerStep, InteractionOutput,
    InteractionStatus, StaticHandlerRegistration, SubscriptionAcceptance,
};
use clinkz_wot_foundation::{Generation, SlotIndex};

fn handler_slot(value: u32) -> HandlerSlotId {
    HandlerSlotId::new(SlotIndex::new(value), Generation::INITIAL)
}

#[test]
fn cancellation_view_has_exact_values_and_default() {
    assert_eq!(CancellationView::default(), CancellationView::Active);
    assert_eq!(CancellationView::Active as u8, 0);
    assert_eq!(CancellationView::Requested as u8, 1);
    assert!(!CancellationView::Active.is_requested());
    assert!(CancellationView::Requested.is_requested());

    for view in [CancellationView::Active, CancellationView::Requested] {
        let expected = match view {
            CancellationView::Active => false,
            CancellationView::Requested => true,
        };
        assert_eq!(view.is_requested(), expected);
    }
}

#[test]
fn subscription_acceptance_borrows_then_linearly_returns_its_response() {
    let response = InteractionOutput::empty().with_status(InteractionStatus::Created);
    let acceptance = SubscriptionAcceptance::new(response.clone());

    assert_eq!(acceptance.response(), &response);
    assert_eq!(acceptance.into_response(), response);
}

#[test]
fn handler_footprint_preserves_all_declared_bounds() {
    let zero = HandlerFootprint::new(0, 0, 0);
    assert_eq!(zero.retained_bytes(), 0);
    assert_eq!(zero.pending_call_bytes(), 0);
    assert_eq!(zero.subscription_bytes(), 0);

    let maximum = HandlerFootprint::new(u64::MAX, u64::MAX, u64::MAX);
    assert_eq!(maximum.retained_bytes(), u64::MAX);
    assert_eq!(maximum.pending_call_bytes(), u64::MAX);
    assert_eq!(maximum.subscription_bytes(), u64::MAX);
}

#[test]
fn handler_step_has_only_pending_and_ready_results() {
    let pending = HandlerStep::<u32>::Pending;
    assert_eq!(
        match pending {
            HandlerStep::Pending => None,
            HandlerStep::Ready(result) => Some(result),
        },
        None
    );

    let ready = HandlerStep::Ready(Ok(37_u32));
    assert_eq!(
        match ready {
            HandlerStep::Pending => None,
            HandlerStep::Ready(result) => Some(result),
        },
        Some(Ok(37))
    );
}

struct SecretHandler;

impl fmt::Debug for SecretHandler {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("handler-secret-must-not-appear")
    }
}

#[test]
fn static_registration_borrows_handler_and_redacts_it_from_debug() {
    let handler = SecretHandler;
    let slot_id = handler_slot(41);
    let footprint = HandlerFootprint::new(43, 47, 53);
    let registration = StaticHandlerRegistration::new(slot_id, &handler, footprint);

    assert_eq!(registration.slot_id(), slot_id);
    assert!(core::ptr::eq(registration.handler(), &handler));
    assert_eq!(registration.footprint(), footprint);

    let rendered = format!("{registration:?}");
    assert!(rendered.starts_with("StaticHandlerRegistration"));
    assert!(rendered.contains("slot_id"));
    assert!(rendered.contains("footprint"));
    assert!(rendered.contains(".."));
    assert!(!rendered.contains("handler:"));
    assert!(!rendered.contains("handler-secret-must-not-appear"));
}
