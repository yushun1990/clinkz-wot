#![cfg(feature = "validated-thing")]
use clinkz_wot_core::*;
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, ResourceAccount, ResourceKind, SlotIndex,
    StaticResourceProfile, WorkBudget, WorkClass,
};
use clinkz_wot_planning::*;
use clinkz_wot_td::{
    ValidatedThing, ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress,
    thing::Thing,
};
#[path = "support/compilation_registration.rs"]
mod registration;
use registration::production_static_registration;

fn budget(n: u64) -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
fn ledger(bytes: u64) -> AdmissionLedger {
    AdmissionLedger::new(
        SlotIndex::new(7),
        Generation::INITIAL,
        0,
        8 << 20,
        0,
        bytes,
        0,
        8 << 20,
    )
}
fn proof(source: &Thing) -> ValidatedThing<'_> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
    let mut cursor = ValidatedThingCursor::from_thing(source, &config, ledger(0));
    for _ in 0..100_000 {
        match cursor.step(&mut budget(4096), false) {
            ValidatedThingProgress::Pending(next) => cursor = next,
            ValidatedThingProgress::Complete(proof) => return proof,
            ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
        }
    }
    panic!("TD stalled")
}
fn source() -> Thing {
    serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"local root","id":"urn:local-root","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},"properties":{"value":{"forms":[{"href":"https://peer.test/value"}]}}}"#).unwrap()
}
fn identity(c: &ResolvedTargetCompiler) -> BindingRegistrationIdentity {
    BindingRegistrationIdentity::new(
        BindingId::new(217),
        BindingGeneration::INITIAL.checked_next().unwrap(),
        c.configuration(),
        c.compatibility(),
        29,
    )
}

// No publication claim. This local test owner preserves the production manual
// destination and its charged original account alongside the unpublished draft.
struct LocalOwner<A: 'static> {
    draft: ConsumerPropertyReadDraft<A>,
    slot: ConsumerPropertyReadCleanupSlot<A>,
    account: ResourceAccount,
}
fn complete<R: InlineConsumerPropertyReadRegistration>(
    source: &Thing,
    registration: &R,
) -> LocalOwner<R::Artifact> {
    let config =
        ConsumerPropertyReadBuildConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
    let bytes = ConsumerPropertyReadCleanupSlot::<R::Artifact>::required_capacity();
    let mut account = ResourceAccount::new(
        SlotIndex::new(8),
        Generation::INITIAL,
        ResourceKind::CleanupBytesMax,
        bytes,
    );
    let mut slot =
        ConsumerPropertyReadCleanupSlot::try_new(&config, account.try_reserve(bytes).unwrap())
            .unwrap_or_else(|_| panic!("slot"));
    let mut build = build_consumer_property_read_aggregate(
        proof(source),
        registration,
        &config,
        PlanSetGeneration::INITIAL,
        ledger(0),
        &mut slot,
    )
    .unwrap();
    let draft = 'drive: {
        for _ in 0..100_000 {
            match build.step(&mut budget(4096), false) {
                ConsumerPropertyReadBuildStep::Pending(next) => build = next,
                ConsumerPropertyReadBuildStep::Preflight {
                    build: next,
                    requirements,
                } => {
                    assert_eq!(
                        (requirements.property_count, requirements.coordinate_count),
                        (1, 1)
                    );
                    build = next
                        .admit_runtime(ledger(requirements.output_capacity_bytes))
                        .unwrap_or_else(|_| panic!("runtime"));
                }
                ConsumerPropertyReadBuildStep::Complete(draft) => break 'drive draft,
                ConsumerPropertyReadBuildStep::Failed { cause, .. } => panic!("{cause:?}"),
            }
        }
        panic!("build stalled")
    };
    LocalOwner {
        draft,
        slot,
        account,
    }
}
fn reclaim<A: 'static>(owner: LocalOwner<A>, abandon_draft: bool, abandon_cleanup: bool) {
    let LocalOwner {
        draft,
        mut slot,
        mut account,
    } = owner;
    assert!(slot.take().is_none(), "a live draft excludes cleanup");
    let bytes = account.used();
    assert!(bytes > 0);
    let mut cleanup = if abandon_draft {
        drop(draft);
        slot.take().unwrap()
    } else {
        draft.into_rollback()
    };
    if abandon_cleanup {
        drop(cleanup);
        cleanup = slot.take().unwrap();
    }
    // Moving all owners is legal locally; transferring any one to a Send task
    // is rejected by the compile-contract cases in the fixture library.
    cleanup = match cleanup.step(&mut budget(0)) {
        ConsumerPropertyReadRollbackStep::Pending(next) => next,
        ConsumerPropertyReadRollbackStep::Complete(_) => panic!("unpaid cleanup"),
    };
    for _ in 0..100_000 {
        assert_eq!(account.used(), bytes, "root charge stays live");
        match cleanup.step(&mut budget(4096)) {
            ConsumerPropertyReadRollbackStep::Pending(next) => cleanup = next,
            ConsumerPropertyReadRollbackStep::Complete(_) => {
                assert!(slot.take().is_none(), "terminal refresh releases backing");
                assert!(slot.is_vacant());
                drop(slot);
                assert!(account.release_committed(bytes));
                assert_eq!(account.used(), 0);
                return;
            }
        }
    }
    panic!("cleanup stalled")
}

#[test]
fn typed_local_owner_survives_source_and_registration_then_reclaims() {
    for (draft_drop, cleanup_drop) in [(false, false), (true, false), (true, true)] {
        let td = source();
        let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
        let id = identity(&compiler);
        let registration = production_static_registration(compiler, id);
        let owner = complete(&td, &registration);
        drop(td);
        drop(registration);
        let selected = select_consumer_property_read_aggregate(
            &owner.draft,
            "value",
            &InteractionOptions::default(),
        )
        .unwrap();
        assert_eq!(
            selected.artifact().artifact().payload().target(),
            "https://peer.test/value"
        );
        assert_eq!(owner.draft.footprint().retained_source_bytes, 0);
        reclaim(owner, draft_drop, cleanup_drop);
    }
}

#[cfg(feature = "std")]
#[test]
fn host_erasure_still_uses_a_local_manual_root() {
    fn thread_safe<T: Send + Sync>() {}
    thread_safe::<HostBindingArtifact>();
    thread_safe::<ResolvedTargetArtifact>();
    for (draft_drop, cleanup_drop) in [(false, false), (true, true)] {
        let td = source();
        let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
        let id = identity(&compiler);
        let registration = registration::production_host_registration(compiler, id);
        let owner = complete(&td, &registration);
        drop(td);
        drop(registration);
        let selected = select_consumer_property_read_aggregate(
            &owner.draft,
            "value",
            &InteractionOptions::default(),
        )
        .unwrap();
        assert_eq!(
            selected
                .artifact()
                .artifact()
                .try_payload::<ResolvedTargetArtifact>(id.artifact_compatibility())
                .unwrap()
                .target(),
            "https://peer.test/value"
        );
        assert_eq!(owner.draft.footprint().registration_delta_bytes, 0);
        reclaim(owner, draft_drop, cleanup_drop);
    }
}

#[test]
fn equal_child_identity_and_limits_do_not_authorize_parent_capacity() {
    let mut left = ledger(4096);
    let mut right = ledger(4096);
    // Even their public reservation owner tuples can be identical. These are
    // two independent child accounts, with no held parent/global authority.
    let l = left
        .try_reserve_persistent_runtime(ResourceKind::CompiledRuntimeBytesPerThingMax, 32)
        .unwrap();
    let r = right
        .try_reserve_persistent_runtime(ResourceKind::CompiledRuntimeBytesPerThingMax, 32)
        .unwrap();
    assert_eq!(l.owner(), r.owner());
    drop(l);
    drop(r);
    assert_eq!((left.live_bytes(), right.live_bytes()), (0, 0));
    assert_eq!((left.peak_live_bytes(), right.peak_live_bytes()), (32, 32));
}
