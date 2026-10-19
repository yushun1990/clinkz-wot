//! Downstream public Planning regressions. No engine hooks or runtime I/O.
use clinkz_wot_consumer_property_read_binding_fixture as fixture;
use clinkz_wot_core::*;
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, ResourceAccount, ResourceKind as K,
    ResourceLimits, SlotIndex, StaticResourceProfile, WorkBudget, WorkClass as W,
};
use clinkz_wot_planning::*;
use clinkz_wot_td::{
    ValidatedThing, ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress,
    thing::Thing,
};
use std::{
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

const COMPATIBILITY: BindingArtifactCompatibility =
    BindingArtifactCompatibility::new(*b"open-regression1");
const CONFIGURATION: BindingConfigurationDigest = BindingConfigurationDigest::new([79; 32]);
const WORK: [u64; 6] = [1, 1, 1, 7, 1, 1];
std::thread_local! { static DROPS: Cell<usize> = const { Cell::new(0) }; }

#[derive(Default)]
struct Stats {
    bounds: AtomicUsize,
    starts: AtomicUsize,
    steps: AtomicUsize,
    aborts: AtomicUsize,
    fail: AtomicBool,
}
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Good,
    LaterBounds,
    EarlierStepAndLaterBounds,
    WrongOutput,
    NoDebit,
}
struct Compiler {
    stats: Arc<Stats>,
    work: [u64; 6],
    mode: Mode,
}
#[repr(align(128))]
struct Cursor {
    bytes: [u8; 129],
}
#[repr(align(128))]
struct Artifact {
    _bytes: [u8; 257],
}
impl Drop for Artifact {
    fn drop(&mut self) {
        DROPS.with(|n| n.set(n.get() + 1));
    }
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}
impl BindingCompilerExtension for Compiler {
    type Cursor = Cursor;
    type Artifact = Artifact;
    fn consumer_contract(&self) -> Option<ConsumerCompilerContract> {
        Some(ConsumerCompilerContract::try_new(CONFIGURATION, COMPATIBILITY, self.work, 2).unwrap())
    }
    fn compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
    }
    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.stats.bounds.fetch_add(1, Ordering::SeqCst);
        assert_eq!(
            input.source().unwrap().form().href.as_str(),
            input.logical_plan().resolved_target()
        );
        if self.stats.fail.load(Ordering::SeqCst)
            && matches!(
                self.mode,
                Mode::LaterBounds | Mode::EarlierStepAndLaterBounds
            )
            && input.logical_plan().property_name() == "z"
        {
            return Err(unsupported());
        }
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, core::mem::size_of::<Artifact>() as u64),
            core::mem::size_of::<Cursor>() as u64,
            0,
            WorkBudget::new().with_remaining(W::BindingPolls, 2 * self.work[3]),
        ))
    }
    fn start(&self, _: &BindingCompilerInput<'_>) -> CoreResult<Cursor> {
        self.stats.starts.fetch_add(1, Ordering::SeqCst);
        Ok(Cursor { bytes: [0; 129] })
    }
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        mut cursor: Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Cursor, Artifact> {
        if budget.remaining(W::BindingPolls) < self.work[3] {
            return BindingCompilerStep::Pending(cursor);
        }
        let fail = self.stats.fail.load(Ordering::SeqCst);
        if !(fail && self.mode == Mode::NoDebit) {
            budget.consume(W::BindingPolls, self.work[3]).unwrap();
        }
        self.stats.steps.fetch_add(1, Ordering::SeqCst);
        if cursor.bytes[0] == 0 {
            cursor.bytes[0] = 1;
            return BindingCompilerStep::Pending(cursor);
        }
        if fail
            && self.mode == Mode::EarlierStepAndLaterBounds
            && input.logical_plan().property_name() == "b"
        {
            return BindingCompilerStep::Failed(BindingCompilerFailure::new(
                CoreError::Validation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)),
                cursor,
            ));
        }
        let compatibility = if fail && self.mode == Mode::WrongOutput {
            BindingArtifactCompatibility::new([0; 16])
        } else {
            COMPATIBILITY
        };
        BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
            compatibility,
            BindingArtifactFootprint::new(1, core::mem::size_of::<Artifact>() as u64),
            Artifact { _bytes: [0; 257] },
        )))
    }
    fn abort(&self, cursor: Cursor) {
        assert_eq!(
            cursor.bytes[0], 1,
            "abort retains the original progressed cursor"
        );
        self.stats.aborts.fetch_add(1, Ordering::SeqCst);
    }
}
struct Client;
impl PollClientBinding for Client {
    type Compiler = Compiler;
    type RequestState = ();
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
    }
    fn request_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<()>(BindingLifetimeFootprint::new(1, 1))
    }
    fn start_request(
        &mut self,
        _: OutboundRequest,
        _: &BindingArtifactEnvelope<Artifact>,
        _: &mut ClientRequestSlot<()>,
        _: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        panic!("no client invocation before publication")
    }
    fn poll_request(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ClientRequestSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        unreachable!()
    }
    fn start_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        _: CleanupPhaseContext,
        _: &mut ClientRequestSlot<()>,
        _: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        unreachable!()
    }
    fn poll_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ClientRequestSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        unreachable!()
    }
    fn acknowledge_request(&mut self, _: &mut ClientRequestSlot<()>) -> CoreResult<()> {
        unreachable!()
    }
}
#[cfg(feature = "std")]
struct HostClient;
#[cfg(feature = "std")]
impl binding::ClientBinding for HostClient {
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
    }
    fn invoke(
        &self,
        _: OutboundRequest,
        _: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    > {
        panic!("no client invocation before publication")
    }
}
fn identity() -> BindingRegistrationIdentity {
    BindingRegistrationIdentity::new(
        BindingId::new(79),
        BindingGeneration::INITIAL,
        CONFIGURATION,
        COMPATIBILITY,
        3,
    )
}
fn each_registration(work: [u64; 6], mode: Mode, mut run: impl Run) {
    let stats = Arc::new(Stats::default());
    stats.fail.store(true, Ordering::SeqCst);
    let registration =
        StaticConsumerBindingRegistration::new(StaticConsumerBindingRegistrationInput::new(
            identity(),
            BindingExecutionSupport::application_static(),
            StaticBindingCompilerRegistration::new(Compiler {
                stats: stats.clone(),
                work,
                mode,
            }),
            Client,
            fixture::resources(),
            BindingStatusPolicy::new(2, 17),
        ))
        .unwrap_or_else(|_| panic!("static registration"))
        .try_into_consumer_compiler()
        .unwrap_or_else(|_| panic!("static capture"));
    run.check(registration, &stats, false);
    #[cfg(feature = "std")]
    {
        let stats = Arc::new(Stats::default());
        stats.fail.store(true, Ordering::SeqCst);
        let registration =
            HostConsumerBindingRegistration::new(HostConsumerBindingRegistrationInput::new(
                identity(),
                BindingExecutionSupport::host_erased(),
                HostBindingCompilerRegistration::try_new_consumer(Compiler {
                    stats: stats.clone(),
                    work,
                    mode,
                })
                .unwrap_or_else(|_| panic!("Host compiler")),
                Box::new(HostClient),
                fixture::resources(),
                BindingStatusPolicy::new(2, 17),
            ))
            .unwrap_or_else(|_| panic!("Host registration"))
            .try_into_consumer_compiler()
            .unwrap_or_else(|_| panic!("Host capture"));
        run.check(registration, &stats, true);
    }
}
trait Run {
    fn check<R: ConsumerPropertyReadRegistration>(&mut self, r: R, stats: &Stats, host: bool);
}
fn budget(n: u64) -> WorkBudget {
    W::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
fn ledger() -> AdmissionLedger {
    AdmissionLedger::new(
        SlotIndex::new(79),
        Generation::INITIAL,
        0,
        8 << 20,
        0,
        8 << 20,
        0,
        8 << 20,
    )
}
fn source() -> Thing {
    serde_json::from_value(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "id":"urn:open:regression", "title":"open regression",
        "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
        "properties":{ "a":{"forms":[{"href":"wire://a"}]}, "b":{"forms":[{"href":"wire://b"}]}, "z":{"forms":[{"href":"wire://z"}]} }
    })).unwrap()
}
fn proof(source: &Thing) -> ValidatedThing<'_> {
    let cfg = ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
    let mut cursor = ValidatedThingCursor::from_thing(source, &cfg, ledger());
    for _ in 0..100_000 {
        match cursor.step(&mut budget(4096), false) {
            ValidatedThingProgress::Pending(next) => cursor = next,
            ValidatedThingProgress::Complete(proof) => return proof,
            ValidatedThingProgress::Failed(error) => panic!("TD {error:?}"),
        }
    }
    panic!("TD stalled")
}
fn slot<A: 'static>(
    cfg: &ConsumerPropertyReadBuildConfig,
) -> (ConsumerPropertyReadCleanupSlot<A>, ResourceAccount) {
    let bytes = ConsumerPropertyReadCleanupSlot::<A>::required_capacity();
    let mut account = ResourceAccount::new(
        SlotIndex::new(80),
        Generation::INITIAL,
        K::CleanupBytesMax,
        bytes,
    );
    let slot = ConsumerPropertyReadCleanupSlot::try_new(cfg, account.try_reserve(bytes).unwrap())
        .unwrap_or_else(|_| panic!("slot"));
    (slot, account)
}
fn finish<R: ConsumerPropertyReadRegistration>(
    mut build: ConsumerPropertyReadBuild<'_, '_, R>,
    stats: &Stats,
    limits: &ResourceLimits,
) -> Result<
    ConsumerPropertyReadDraft<R::Artifact>,
    (CoreError, ConsumerPropertyReadRollback<R::Artifact>),
> {
    let compile_cap = limits.get(K::PlanCompileWorkUnitsPerStepMax).unwrap();
    let cleanup_cap = limits.get(K::CleanupWorkItemsPerStepMax).unwrap();
    for _ in 0..100_000 {
        let before = (
            stats.bounds.load(Ordering::SeqCst),
            stats.starts.load(Ordering::SeqCst),
            stats.steps.load(Ordering::SeqCst),
        );
        build = match build.step(&mut budget(0), false) {
            ConsumerPropertyReadBuildStep::Pending(next) => next,
            _ => panic!("unpaid build advanced"),
        };
        assert_eq!(
            before,
            (
                stats.bounds.load(Ordering::SeqCst),
                stats.starts.load(Ordering::SeqCst),
                stats.steps.load(Ordering::SeqCst)
            )
        );
        let mut paid = budget(4096)
            .with_remaining(W::BindingPolls, compile_cap)
            .with_remaining(W::CleanupItems, cleanup_cap);
        match build.step(&mut paid, false) {
            ConsumerPropertyReadBuildStep::Pending(next) => build = next,
            ConsumerPropertyReadBuildStep::Preflight { build: next, .. } => {
                build = next
                    .admit_runtime(ledger())
                    .unwrap_or_else(|_| panic!("runtime"))
            }
            ConsumerPropertyReadBuildStep::Complete(draft) => return Ok(draft),
            ConsumerPropertyReadBuildStep::Failed { cause, cleanup } => {
                return Err((cause, cleanup.unwrap()));
            }
        }
    }
    panic!("build stalled at the configured step caps")
}
fn reclaim<A: 'static>(mut owner: ConsumerPropertyReadRollback<A>, cause: &CoreError) {
    for _ in 0..100_000 {
        assert_eq!(owner.cause(), cause);
        let drops = DROPS.with(Cell::get);
        owner = match owner.step(&mut budget(0)) {
            ConsumerPropertyReadRollbackStep::Pending(next) => next,
            _ => panic!("unpaid reclaim advanced"),
        };
        assert_eq!(DROPS.with(Cell::get), drops);
        match owner.step(&mut budget(31)) {
            ConsumerPropertyReadRollbackStep::Pending(next) => owner = next,
            ConsumerPropertyReadRollbackStep::Complete(actual) => {
                assert_eq!(&actual, cause);
                return;
            }
        }
    }
    panic!("reclaim stalled")
}

struct RejectCost(K);
impl Run for RejectCost {
    fn check<R: ConsumerPropertyReadRegistration>(&mut self, r: R, stats: &Stats, _: bool) {
        let cfg =
            ConsumerPropertyReadBuildConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
        let (mut slot, mut account) = slot::<R::Artifact>(&cfg);
        let reservation = slot.reservation();
        let source = source();
        let result = build_consumer_property_read_aggregate(
            proof(&source),
            &r,
            &cfg,
            PlanSetGeneration::INITIAL,
            ledger(),
            &mut slot,
        );
        assert!(
            matches!(result, Err(CoreError::LimitExceeded { resource, .. }) if resource == self.0)
        );
        drop(result);
        assert_eq!(
            (
                stats.bounds.load(Ordering::SeqCst),
                stats.starts.load(Ordering::SeqCst),
                stats.steps.load(Ordering::SeqCst),
                stats.aborts.load(Ordering::SeqCst)
            ),
            (0, 0, 0, 0)
        );
        assert!(slot.is_vacant());
        assert_eq!(slot.reservation(), reservation);
        drop(slot);
        assert!(account.release_committed(reservation.1));
        assert_eq!(account.used(), 0);
    }
}
#[test]
fn oversized_atomic_compiler_costs_reject_before_callbacks_in_both_representations() {
    for (index, resource) in [
        (0, K::PlanCompileWorkUnitsPerStepMax),
        (1, K::PlanCompileWorkUnitsPerStepMax),
        (2, K::PlanCompileWorkUnitsPerStepMax),
        (3, K::PlanCompileWorkUnitsPerStepMax),
        (4, K::CleanupWorkItemsPerStepMax),
        (5, K::CleanupWorkItemsPerStepMax),
    ] {
        let mut work = WORK;
        work[index] = GatewayDefaultV1::LIMITS.get(resource).unwrap() + 1;
        each_registration(work, Mode::Good, RejectCost(resource));
    }
    // Neither cleanup callback alone exceeds the limit; their indivisible
    // combined prepayment still has to fit (including Host allocation/release).
    let mut work = WORK;
    work[4] = GatewayDefaultV1::LIMITS
        .get(K::CleanupWorkItemsPerStepMax)
        .unwrap();
    each_registration(work, Mode::Good, RejectCost(K::CleanupWorkItemsPerStepMax));
}

struct Rollback(Mode);
impl Run for Rollback {
    fn check<R: ConsumerPropertyReadRegistration>(&mut self, r: R, stats: &Stats, _: bool) {
        DROPS.with(|n| n.set(0));
        assert_eq!(r.support().cursor_layout().align(), 128);
        assert_eq!(r.support().artifact_layout().align(), 128);
        let cfg =
            ConsumerPropertyReadBuildConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
        let (mut slot, mut account) = slot::<R::Artifact>(&cfg);
        let reservation = slot.reservation();
        let thing = source();
        let build = build_consumer_property_read_aggregate(
            proof(&thing),
            &r,
            &cfg,
            PlanSetGeneration::INITIAL,
            ledger(),
            &mut slot,
        )
        .unwrap();
        let (owner, expected_drops) = if self.0 == Mode::Good {
            let draft = finish(build, stats, GatewayDefaultV1::LIMITS)
                .unwrap_or_else(|(e, _)| panic!("{e:?}"));
            assert_eq!(
                (
                    stats.bounds.load(Ordering::SeqCst),
                    stats.starts.load(Ordering::SeqCst),
                    stats.steps.load(Ordering::SeqCst),
                    stats.aborts.load(Ordering::SeqCst)
                ),
                (3, 3, 6, 0)
            );
            (draft.into_rollback(), 3)
        } else {
            let (cause, owner) = finish(build, stats, GatewayDefaultV1::LIMITS)
                .err()
                .expect("rejected compiler");
            let (bounds, starts, steps, aborts, drops) = match self.0 {
                Mode::LaterBounds => {
                    assert!(matches!(cause, CoreError::UnsupportedOperation(_)));
                    (3, 2, 4, 0, 2)
                }
                Mode::EarlierStepAndLaterBounds => {
                    assert!(matches!(cause, CoreError::Validation(_)));
                    (2, 2, 4, 1, 1)
                }
                Mode::WrongOutput => (1, 1, 2, 0, 1),
                Mode::NoDebit => (1, 1, 1, 1, 0),
                Mode::Good => unreachable!(),
            };
            assert_eq!(
                (
                    stats.bounds.load(Ordering::SeqCst),
                    stats.starts.load(Ordering::SeqCst),
                    stats.steps.load(Ordering::SeqCst),
                    stats.aborts.load(Ordering::SeqCst)
                ),
                (bounds, starts, steps, aborts)
            );
            assert_eq!(owner.cause(), &cause);
            (owner, drops)
        };
        drop(thing);
        assert_eq!(
            DROPS.with(Cell::get),
            0,
            "unpublished artifacts stay in the funded root"
        );
        assert!(owner.live_bytes() > 0);
        let retry = source();
        assert!(matches!(
            build_consumer_property_read_aggregate(
                proof(&retry),
                &r,
                &cfg,
                PlanSetGeneration::INITIAL,
                ledger(),
                &mut slot
            ),
            Err(CoreError::Backpressure(_))
        ));
        let cause = owner.cause().clone();
        reclaim(owner, &cause);
        assert_eq!(DROPS.with(Cell::get), expected_drops);
        assert!(slot.take().is_none());
        assert_eq!(slot.reservation(), reservation);
        assert_eq!(account.used(), reservation.1);
        stats.fail.store(false, Ordering::SeqCst);
        let build = build_consumer_property_read_aggregate(
            proof(&retry),
            &r,
            &cfg,
            PlanSetGeneration::INITIAL,
            ledger(),
            &mut slot,
        )
        .unwrap();
        let draft = finish(build, stats, GatewayDefaultV1::LIMITS)
            .unwrap_or_else(|(e, _)| panic!("retry {e:?}"));
        drop(retry);
        drop(r);
        let owner = draft.into_rollback();
        let cause = owner.cause().clone();
        reclaim(owner, &cause);
        assert_eq!(DROPS.with(Cell::get), expected_drops + 3);
        assert!(slot.take().is_none());
        drop(slot);
        assert!(account.release_committed(reservation.1));
        assert_eq!(account.used(), 0);
    }
}
#[test]
fn external_later_bounds_first_cause_and_rejected_outputs_keep_funded_rollback() {
    for mode in [
        Mode::Good,
        Mode::LaterBounds,
        Mode::EarlierStepAndLaterBounds,
        Mode::WrongOutput,
        Mode::NoDebit,
    ] {
        each_registration(WORK, mode, Rollback(mode));
    }
}

struct ExactCost(usize);
impl Run for ExactCost {
    fn check<R: ConsumerPropertyReadRegistration>(&mut self, r: R, stats: &Stats, host: bool) {
        let mut limits = GatewayDefaultV1::LIMITS.clone();
        let support = r.support();
        // Set the selected cap to exactly the captured operation/combined
        // cleanup cost, so the same compiler exercises static and Host edges.
        let (resource, cost) = if self.0 < 4 {
            (
                K::PlanCompileWorkUnitsPerStepMax,
                [
                    support.compatibility_work(),
                    support.bounds_work(),
                    support.start_work(),
                    support.progress_work(),
                ][self.0],
            )
        } else {
            (
                K::CleanupWorkItemsPerStepMax,
                support.allocation_work()
                    + support.abort_work()
                    + support.destruction_work()
                    + support.release_work(),
            )
        };
        assert!(cost >= if host { 4 } else { 2 });
        assert!(limits.set(resource, Some(cost)));
        let cfg = ConsumerPropertyReadBuildConfig::try_from_limits(&limits).unwrap();
        let (mut slot, mut account) = slot::<R::Artifact>(&cfg);
        let bytes = account.used();
        let source = source();
        let short = limits.clone().with_limit(resource, Some(cost - 1));
        let short = ConsumerPropertyReadBuildConfig::try_from_limits(&short).unwrap();
        let rejected = build_consumer_property_read_aggregate(
            proof(&source),
            &r,
            &short,
            PlanSetGeneration::INITIAL,
            ledger(),
            &mut slot,
        );
        assert!(
            matches!(rejected, Err(CoreError::LimitExceeded { resource: actual, .. }) if actual == resource)
        );
        drop(rejected);
        assert_eq!(stats.bounds.load(Ordering::SeqCst), 0);
        assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
        assert!(slot.is_vacant());
        let build = build_consumer_property_read_aggregate(
            proof(&source),
            &r,
            &cfg,
            PlanSetGeneration::INITIAL,
            ledger(),
            &mut slot,
        )
        .unwrap();
        let draft =
            finish(build, stats, &limits).unwrap_or_else(|(e, _)| panic!("exact cost {e:?}"));
        let owner = draft.into_rollback();
        let cause = owner.cause().clone();
        reclaim(owner, &cause);
        assert!(slot.take().is_none());
        drop(slot);
        assert!(account.release_committed(bytes));
    }
}
#[test]
fn exact_atomic_compiler_costs_complete_at_the_selected_caps() {
    let cap = GatewayDefaultV1::LIMITS
        .get(K::PlanCompileWorkUnitsPerStepMax)
        .unwrap();
    let cleanup = GatewayDefaultV1::LIMITS
        .get(K::CleanupWorkItemsPerStepMax)
        .unwrap();
    for index in 0..6 {
        let mut work = WORK;
        work[index] = if index < 3 {
            cap
        } else if index == 3 {
            cap / 2 - 1
        } else {
            cleanup - 3
        };
        each_registration(work, Mode::Good, ExactCost(index));
    }
}
