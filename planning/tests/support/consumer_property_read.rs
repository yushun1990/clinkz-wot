//! Test-only observation of the checked production program, never eligibility.
use super::*;
use alloc::rc::Rc;
use clinkz_wot_foundation::StaticResourceProfile;
pub fn layouts<A: 'static>() -> [[usize; 2]; 7] {
    [
        Layout::new::<Node<A>>(),
        Layout::new::<Data<A>>(),
        Layout::new::<PropertyRow>(),
        Layout::new::<Coordinate<A>>(),
        Layout::new::<Reclaim>(),
        Layout::new::<ConsumerPropertyReadCleanupSlot<A>>(),
        Layout::new::<ConsumerPropertyReadDraft<A>>(),
    ]
    .map(|l| [l.size(), l.align()])
}
#[derive(Default, Debug, Clone, Copy)]
pub struct Counts {
    pub bounds: usize,
    pub starts: usize,
    pub steps: usize,
    pub aborts: usize,
    pub failed: bool,
}
struct Traced<R> {
    registration: R,
    counts: Rc<Cell<Counts>>,
    bad_bound: Option<usize>,
}
impl<R: ConsumerPropertyReadRegistration> private::Sealed for Traced<R> {}
impl<R: ConsumerPropertyReadRegistration> ConsumerPropertyReadRegistration for Traced<R> {
    type Artifact = R::Artifact;
    type Cursor = R::Cursor;
    fn identity(&self) -> BindingRegistrationIdentity {
        self.registration.identity()
    }
    fn support(&self) -> &ConsumerCompilerSupport {
        self.registration.support()
    }
    fn bounds(&self, i: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        let mut c = self.counts.get();
        c.bounds += 1;
        self.counts.set(c);
        let b = self.registration.bounds(i)?;
        if self.bad_bound == Some(c.bounds) {
            return Ok(BindingCompilerBounds::new(
                b.artifact(),
                b.cursor_bytes(),
                b.temporary_bytes(),
                WorkBudget::new(),
            ));
        }
        Ok(b)
    }
    fn start(&self, i: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        let mut c = self.counts.get();
        c.starts += 1;
        assert_eq!(
            c.bounds, c.starts,
            "each coordinate is bounded before its start"
        );
        assert_eq!(c.aborts, 0);
        self.counts.set(c);
        self.registration.start(i)
    }
    fn progress(
        &self,
        i: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        b: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        let mut c = self.counts.get();
        c.steps += 1;
        self.counts.set(c);
        self.registration.progress(i, cursor, b)
    }
    fn abort(&self, cursor: Self::Cursor) {
        let mut c = self.counts.get();
        c.aborts += 1;
        self.counts.set(c);
        self.registration.abort(cursor)
    }
}
pub fn traced<R: ConsumerPropertyReadRegistration>(
    proof: ValidatedThing<'_>,
    registration: R,
    bad_bound: Option<usize>,
    cancel_pending: bool,
) -> Counts {
    traced_after_admit(proof, registration, bad_bound, cancel_pending, || {})
}
pub fn traced_after_admit<R: ConsumerPropertyReadRegistration>(
    proof: ValidatedThing<'_>,
    registration: R,
    bad_bound: Option<usize>,
    cancel_pending: bool,
    on_admit: impl FnOnce(),
) -> Counts {
    let counts = Rc::new(Cell::new(Counts::default()));
    let r = Traced {
        registration,
        counts: counts.clone(),
        bad_bound,
    };
    let cfg = ConsumerPropertyReadBuildConfig::try_from_limits(
        clinkz_wot_foundation::GatewayDefaultV1::LIMITS,
    )
    .unwrap();
    let mut account = clinkz_wot_foundation::ResourceAccount::new(
        SlotIndex::new(81),
        Generation::INITIAL,
        K::CleanupBytesMax,
        1 << 20,
    );
    let mut root = ConsumerPropertyReadCleanupSlot::try_new(
        &cfg,
        account
            .try_reserve(ConsumerPropertyReadCleanupSlot::<R::Artifact>::required_capacity())
            .unwrap(),
    )
    .unwrap_or_else(|_| panic!("slot"));
    let ledger = || {
        AdmissionLedger::new(
            SlotIndex::new(82),
            Generation::INITIAL,
            0,
            8 << 20,
            0,
            8 << 20,
            0,
            8 << 20,
        )
    };
    let work = || {
        W::ALL
            .into_iter()
            .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 4096))
    };
    let mut build = build_consumer_property_read_aggregate(
        proof,
        &r,
        &cfg,
        PlanSetGeneration::INITIAL,
        ledger(),
        &mut root,
    )
    .unwrap();
    let mut on_admit = Some(on_admit);
    let cleanup = loop {
        if counts.get().starts > 0 {
            let c = counts.get();
            let mut short =
                WorkBudget::new().with_remaining(W::BindingPolls, r.support().step_work() - 1);
            build = match build.step(&mut short, false) {
                ConsumerPropertyReadBuildStep::Pending(next) => next,
                _ => panic!("short credit advanced"),
            };
            assert_eq!(counts.get().steps, c.steps);
        }
        match build.step(&mut work(), cancel_pending && counts.get().steps == 1) {
            ConsumerPropertyReadBuildStep::Pending(next) => build = next,
            ConsumerPropertyReadBuildStep::Preflight { build: next, .. } => {
                on_admit.take().unwrap()();
                build = next
                    .admit_runtime(ledger())
                    .unwrap_or_else(|_| panic!("runtime"))
            }
            ConsumerPropertyReadBuildStep::Complete(draft) => break draft.into_rollback(),
            ConsumerPropertyReadBuildStep::Failed { cleanup, .. } => {
                let mut c = counts.get();
                c.failed = true;
                counts.set(c);
                break cleanup.unwrap();
            }
        }
    };
    let mut cleanup = cleanup;
    loop {
        match cleanup.step(&mut work()) {
            ConsumerPropertyReadRollbackStep::Pending(next) => cleanup = next,
            ConsumerPropertyReadRollbackStep::Complete(_) => break,
        }
    }
    assert!(root.take().is_none());
    drop(root);
    let bytes = account.used();
    assert!(account.release_committed(bytes));
    counts.get()
}

pub fn ready_retries<R: ConsumerPropertyReadRegistration>(
    proof: ValidatedThing<'_>,
    r: R,
    spare: u64,
) {
    let cfg = ConsumerPropertyReadBuildConfig::try_from_limits(
        clinkz_wot_foundation::GatewayDefaultV1::LIMITS,
    )
    .unwrap();
    let mut account = clinkz_wot_foundation::ResourceAccount::new(
        SlotIndex::new(81),
        Generation::INITIAL,
        K::CleanupBytesMax,
        1 << 20,
    );
    let mut root = ConsumerPropertyReadCleanupSlot::try_new(
        &cfg,
        account
            .try_reserve(ConsumerPropertyReadCleanupSlot::<R::Artifact>::required_capacity())
            .unwrap(),
    )
    .unwrap_or_else(|_| panic!("slot"));
    let ledger = || {
        AdmissionLedger::new(
            SlotIndex::new(82),
            Generation::INITIAL,
            0,
            8 << 20,
            0,
            8 << 20,
            0,
            8 << 20,
        )
    };
    let work = || {
        W::ALL
            .into_iter()
            .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 4096))
    };
    let mut build = build_consumer_property_read_aggregate(
        proof,
        &r,
        &cfg,
        PlanSetGeneration::INITIAL,
        ledger(),
        &mut root,
    )
    .unwrap();
    loop {
        match build.step(&mut work(), false) {
            ConsumerPropertyReadBuildStep::Pending(next) => build = next,
            ConsumerPropertyReadBuildStep::Preflight { build: next, .. } => {
                build = next
                    .admit_runtime(ledger())
                    .unwrap_or_else(|_| panic!("runtime"))
            }
            _ => panic!("early terminal"),
        }
        if build.phase == Phase::Materialize && !build.root.as_ref().unwrap().data().rows.is_empty()
        {
            break;
        }
    }
    let bytes = loop {
        match build.td.as_mut().unwrap().step(&mut work(), false).unwrap() {
            TdStep::Ready(Event::Form(f)) => break form_size(&f).unwrap().0,
            TdStep::Pending => {}
            _ => panic!("expected first Form"),
        }
    };
    let remaining = build.root.as_ref().unwrap().data().material;
    for _ in 0..12 {
        let mut short = work();
        short.set_remaining(W::CodecOutputBytes, bytes - 1);
        build = match build.step(&mut short, false) {
            ConsumerPropertyReadBuildStep::Pending(next) => next,
            _ => panic!("partial Ready accepted"),
        };
        assert_eq!(short.remaining(W::PlanningItems), 4096);
        assert_eq!(short.remaining(W::CleanupItems), 4096);
        for w in [
            W::DocumentNodes,
            W::UriBytes,
            W::SecurityBranches,
            W::CodecInputBytes,
        ] {
            assert_eq!(short.remaining(w), 4096, "semantic prefix replay");
        }
        let d = build.root.as_ref().unwrap().data();
        assert!(d.coordinates.is_empty());
        assert_eq!(d.material.bytes, remaining.bytes);
        assert_eq!(d.material.items, remaining.items);
    }
    let mut exact = work();
    exact.set_remaining(W::CodecOutputBytes, bytes + spare);
    build = match build.step(&mut exact, false) {
        ConsumerPropertyReadBuildStep::Pending(next) => next,
        _ => panic!("expected materialization"),
    };
    assert_eq!(exact.remaining(W::CodecOutputBytes), spare);
    assert_eq!(build.root.as_ref().unwrap().data().coordinates.len(), 1);
    drop(build);
    let mut cleanup = root.take().unwrap();
    loop {
        match cleanup.step(&mut work()) {
            ConsumerPropertyReadRollbackStep::Pending(next) => cleanup = next,
            ConsumerPropertyReadRollbackStep::Complete(_) => break,
        }
    }
    assert!(root.take().is_none());
    drop(root);
    let bytes = account.used();
    assert!(account.release_committed(bytes));
}
