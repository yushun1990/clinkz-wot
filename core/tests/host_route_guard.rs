#![cfg(feature = "std")]

use core::{cell::Cell, marker::PhantomPinned, pin::Pin};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingArtifactIdentity, BindingArtifactRef, BindingArtifactRole,
    BindingConfigurationDigest, BindingGeneration, BindingId, BindingLifetimeFootprint,
    CollisionDomainId, EndpointReservationKey, HostActiveRouteGuard, HostCommittedRouteGuard,
    HostPreparedRouteGuard, PlanId, PlanSetGeneration, PrepareInput, RouteReservationIdentity,
};
use clinkz_wot_foundation::{Generation, SlotIndex};

struct PinnedRouteState {
    stage: Cell<u8>,
    drops: Arc<AtomicU32>,
    _pin: PhantomPinned,
}

impl Drop for PinnedRouteState {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

fn prepare_input() -> PrepareInput {
    let binding_id = BindingId::new(0x5801);
    let binding_generation = BindingGeneration::INITIAL;
    let plan_set_generation = PlanSetGeneration::new(Generation::INITIAL);
    let plan_id = PlanId::new(SlotIndex::new(0), Generation::INITIAL);
    let configuration = BindingConfigurationDigest::new([0x58; 32]);
    let compatibility = BindingArtifactCompatibility::new([0x35; 16]);
    let reservation = RouteReservationIdentity::new(
        CollisionDomainId::new([0x21; 16]),
        EndpointReservationKey::new([0x13; 32]),
    );
    let artifact = BindingArtifactIdentity::new(
        plan_set_generation,
        plan_id,
        binding_id,
        binding_generation,
        configuration,
        compatibility,
        BindingArtifactRole::ProducerRoute,
    );
    let route = clinkz_wot_core::binding::BindingRouteKey::new(
        binding_id,
        binding_generation,
        Generation::INITIAL,
        plan_set_generation,
        plan_id,
        reservation,
    );
    PrepareInput::new(
        route,
        BindingArtifactRef::new(artifact, SlotIndex::new(0)),
        BindingLifetimeFootprint::new(3, 512),
    )
}

#[test]
fn host_stage_guards_keep_one_pinned_state_allocation_until_terminal_drop() {
    let input = prepare_input();
    let route = *input.route();
    let footprint = input.admitted_footprint();
    let drops = Arc::new(AtomicU32::new(0));
    let mut prepared = HostPreparedRouteGuard::new(
        input,
        footprint,
        PinnedRouteState {
            stage: Cell::new(0),
            drops: Arc::clone(&drops),
            _pin: PhantomPinned,
        },
    );

    assert_eq!(prepared.route(), &route);
    assert_eq!(prepared.lifetime_footprint(), footprint);
    assert!(
        Pin::new(&mut prepared).try_state_pin_mut::<u8>().is_none(),
        "a failed concrete-type check must not expose or replace state"
    );
    let prepared_address = {
        let state = Pin::new(&mut prepared)
            .try_state_pin_mut::<PinnedRouteState>()
            .expect("prepared state type");
        state.as_ref().get_ref().stage.set(1);
        state.as_ref().get_ref() as *const PinnedRouteState
    };
    assert_eq!(drops.load(Ordering::SeqCst), 0);

    let mut active = HostActiveRouteGuard::new(prepared);
    assert_eq!(active.route(), &route);
    assert_eq!(active.lifetime_footprint(), footprint);
    let active_address = {
        let state = Pin::new(&mut active)
            .try_state_pin_mut::<PinnedRouteState>()
            .expect("active state type");
        assert_eq!(state.as_ref().get_ref().stage.get(), 1);
        state.as_ref().get_ref().stage.set(2);
        state.as_ref().get_ref() as *const PinnedRouteState
    };
    assert_eq!(active_address, prepared_address);
    assert_eq!(drops.load(Ordering::SeqCst), 0);

    let mut committed = HostCommittedRouteGuard::new(active);
    assert_eq!(committed.route(), &route);
    assert_eq!(committed.lifetime_footprint(), footprint);
    let committed_address = {
        let state = Pin::new(&mut committed)
            .try_state_pin_mut::<PinnedRouteState>()
            .expect("committed state type");
        assert_eq!(state.as_ref().get_ref().stage.get(), 2);
        state.as_ref().get_ref().stage.set(3);
        state.as_ref().get_ref() as *const PinnedRouteState
    };
    assert_eq!(committed_address, prepared_address);
    assert_eq!(drops.load(Ordering::SeqCst), 0);

    drop(committed);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
