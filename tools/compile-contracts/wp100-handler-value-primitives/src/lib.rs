#![no_std]
#![allow(dead_code)]

use core::{fmt::Debug, hash::Hash};

use clinkz_wot_core::{
    CancellationView, CoreResult, HandlerFootprint, HandlerSlotId, HandlerStep, InteractionOutput,
    StaticHandlerRegistration, SubscriptionAcceptance,
};
use clinkz_wot_foundation::{Generation, SlotIndex};

// All imports above deliberately use the Core crate root. A module-only export
// cannot satisfy this consumer contract.

fn require_cancellation_traits<T>()
where
    T: Clone + Copy + Debug + Default + Eq + Hash + Ord + PartialEq + PartialOrd,
{
}

fn require_acceptance_traits<T>()
where
    T: Debug + Eq + PartialEq,
{
}

fn require_footprint_traits<T>()
where
    T: Clone + Copy + Debug + Eq + Hash + Ord + PartialEq + PartialOrd,
{
}

fn require_step_traits<T>()
where
    T: Debug + Eq + PartialEq,
{
}

fn require_registration_traits<T>()
where
    T: Clone + Copy + Debug,
{
}

struct HandlerWithoutCopyCloneDebug;

fn positive_trait_contract() {
    require_cancellation_traits::<CancellationView>();
    require_acceptance_traits::<SubscriptionAcceptance>();
    require_footprint_traits::<HandlerFootprint>();
    require_step_traits::<HandlerStep<()>>();

    // These bounds must hold without imposing Copy, Clone, or Debug on H.
    require_registration_traits::<StaticHandlerRegistration<'static, HandlerWithoutCopyCloneDebug>>(
    );
}

trait AmbiguousIfCopy<Marker> {
    fn marker() {}
}

impl<T: ?Sized> AmbiguousIfCopy<()> for T {}
impl<T: ?Sized + Copy> AmbiguousIfCopy<u8> for T {}

trait AmbiguousIfClone<Marker> {
    fn marker() {}
}

impl<T: ?Sized> AmbiguousIfClone<()> for T {}
impl<T: ?Sized + Clone> AmbiguousIfClone<u8> for T {}

trait AmbiguousIfDefault<Marker> {
    fn marker() {}
}

impl<T: ?Sized> AmbiguousIfDefault<()> for T {}
impl<T: ?Sized + Default> AmbiguousIfDefault<u8> for T {}

fn negative_trait_contract() {
    // Each inference has exactly one candidate only while the public type does
    // not implement the prohibited trait. Adding that trait makes this
    // contract fail with an ambiguity error.
    let _ = <SubscriptionAcceptance as AmbiguousIfCopy<_>>::marker;
    let _ = <SubscriptionAcceptance as AmbiguousIfClone<_>>::marker;
    let _ = <SubscriptionAcceptance as AmbiguousIfDefault<_>>::marker;

    let _ = <HandlerStep<()> as AmbiguousIfCopy<_>>::marker;
    let _ = <HandlerStep<()> as AmbiguousIfClone<_>>::marker;
    let _ = <HandlerStep<()> as AmbiguousIfDefault<_>>::marker;

    let _ = <HandlerFootprint as AmbiguousIfDefault<_>>::marker;
}

const _: () = assert!(CancellationView::Active as u8 == 0);
const _: () = assert!(CancellationView::Requested as u8 == 1);
const _: () = assert!(core::mem::size_of::<CancellationView>() == 1);
const _: () = assert!(!CancellationView::Active.is_requested());
const _: () = assert!(CancellationView::Requested.is_requested());

const FOOTPRINT: HandlerFootprint = HandlerFootprint::new(11, 13, 17);
const _: () = assert!(FOOTPRINT.retained_bytes() == 11);
const _: () = assert!(FOOTPRINT.pending_call_bytes() == 13);
const _: () = assert!(FOOTPRINT.subscription_bytes() == 17);

static STATIC_HANDLER: HandlerWithoutCopyCloneDebug = HandlerWithoutCopyCloneDebug;
const SLOT_ID: HandlerSlotId = HandlerSlotId::new(SlotIndex::new(19), Generation::INITIAL);
const REGISTRATION: StaticHandlerRegistration<'static, HandlerWithoutCopyCloneDebug> =
    StaticHandlerRegistration::new(SLOT_ID, &STATIC_HANDLER, FOOTPRINT);
const _: HandlerSlotId = REGISTRATION.slot_id();
const _: &HandlerWithoutCopyCloneDebug = REGISTRATION.handler();
const _: HandlerFootprint = REGISTRATION.footprint();

pub const fn acceptance_from(response: InteractionOutput) -> SubscriptionAcceptance {
    SubscriptionAcceptance::new(response)
}

pub const fn acceptance_response(acceptance: &SubscriptionAcceptance) -> &InteractionOutput {
    acceptance.response()
}

pub const fn cancellation_discriminant(view: CancellationView) -> u8 {
    match view {
        CancellationView::Active => 0,
        CancellationView::Requested => 1,
    }
}

pub fn split_step<R>(step: HandlerStep<R>) -> Option<CoreResult<R>> {
    match step {
        HandlerStep::Pending => None,
        HandlerStep::Ready(result) => Some(result),
    }
}
