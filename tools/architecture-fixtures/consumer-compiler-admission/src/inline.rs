//! A fixed-capacity compiler counterexample to mandatory allocating adapters.
//! Copies an already resolved target; implements no WoT/Planning rule.
use std::{mem, sync::atomic::AtomicUsize};

use clinkz_wot_core::{
    BindingArtifact, BindingArtifactCompatibility, BindingArtifactEnvelope,
    BindingArtifactFootprint, BindingArtifactIdentity, BindingArtifactRole, BindingCandidate,
    BindingCompilerBounds, BindingCompilerExtension, BindingCompilerFailure, BindingCompilerInput,
    BindingCompilerOutput, BindingCompilerStep, CoreError, CoreResult, ErrorContext, ErrorPhase,
    HostBindingCompilerRegistration, LogicalInteractionPlan, PlanSetGeneration, RetryClass,
    StaticBindingCompilerRegistration,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass};

use super::{Requests, SeqCst, layout, observe};

const TARGET_CAPACITY: usize = 64;
static ABORTS: AtomicUsize = AtomicUsize::new(0);

struct InlineTarget {
    bytes: [u8; TARGET_CAPACITY],
    len: usize,
}

impl InlineTarget {
    fn target(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap()
    }
}

struct InlineCompiler(BindingArtifactCompatibility);

fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}

impl BindingCompilerExtension for InlineCompiler {
    type Cursor = u8;
    type Artifact = InlineTarget;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.0
    }

    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        if input.role() != BindingArtifactRole::ConsumerCall
            || input.logical_plan().resolved_target().len() > TARGET_CAPACITY
        {
            return Err(unsupported());
        }
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, mem::size_of::<InlineTarget>() as u64),
            mem::size_of::<u8>() as u64,
            0,
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1),
        ))
    }

    fn start(&self, _input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        Ok(7)
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return BindingCompilerStep::Pending(cursor);
        }
        let target = input.logical_plan().resolved_target().as_bytes();
        if input.role() != BindingArtifactRole::ConsumerCall || target.len() > TARGET_CAPACITY {
            return BindingCompilerStep::Failed(BindingCompilerFailure::new(unsupported(), cursor));
        }
        let mut payload = InlineTarget {
            bytes: [0; TARGET_CAPACITY],
            len: target.len(),
        };
        payload.bytes[..target.len()].copy_from_slice(target);
        BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
            self.0,
            BindingArtifactFootprint::new(1, mem::size_of::<InlineTarget>() as u64),
            payload,
        )))
    }

    fn abort(&self, cursor: Self::Cursor) {
        assert_eq!(cursor, 7);
        ABORTS.fetch_add(1, SeqCst);
    }
}

fn no_requests(requests: Requests) {
    assert!(requests.allocations.is_empty() && requests.releases.is_empty());
}

pub(super) fn compare(plan: LogicalInteractionPlan, candidate: BindingCandidate) {
    let compatibility = candidate.compatibility();
    let portable = StaticBindingCompilerRegistration::new(InlineCompiler(compatibility));
    let host = HostBindingCompilerRegistration::new(InlineCompiler(compatibility));
    let input = BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);
    let (bounds, requests) = observe(|| portable.compiler().bounds(&input).unwrap());
    no_requests(requests);
    let (host_bounds, requests) = observe(|| host.bounds(&input).unwrap());
    no_requests(requests);
    assert_eq!(bounds, host_bounds);

    let (cursor, requests) = observe(|| portable.compiler().start(&input).unwrap());
    no_requests(requests);
    let mut zero = WorkBudget::new();
    let (pending, requests) = observe(|| portable.compiler().step(&input, cursor, &mut zero));
    no_requests(requests);
    let BindingCompilerStep::Pending(cursor) = pending else {
        panic!("zero compiler credit must preserve the inline cursor")
    };
    assert_eq!(cursor, 7);
    let (host_cursor, requests) = observe(|| host.start(&input).unwrap());
    assert_eq!(requests.allocations, [layout::<u8>()]);
    assert!(requests.releases.is_empty());

    let mut credit = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
    let (result, requests) = observe(|| portable.compiler().step(&input, cursor, &mut credit));
    no_requests(requests);
    assert_eq!(credit.remaining(WorkClass::BindingPolls), 0);
    let BindingCompilerStep::Complete(output) = result else {
        panic!("inline compiler must complete")
    };
    let mut host_credit = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
    let (result, host_complete) = observe(|| host.step(&input, host_cursor, &mut host_credit));
    assert_eq!(host_complete.allocations, [layout::<InlineTarget>()]);
    assert_eq!(host_complete.releases, [layout::<u8>()]);
    assert_eq!(host_credit.remaining(WorkClass::BindingPolls), 0);
    let BindingCompilerStep::Complete(host_output) = result else {
        panic!("same inline compiler must complete through production Host erasure")
    };
    let identity = BindingArtifactIdentity::new(
        PlanSetGeneration::INITIAL,
        plan.plan_id(),
        candidate.binding_id(),
        candidate.binding_generation(),
        candidate.configuration(),
        compatibility,
        BindingArtifactRole::ConsumerCall,
    );
    let output =
        BindingArtifactEnvelope::try_new(identity, bounds.artifact(), output.into_artifact())
            .unwrap_or_else(|_| panic!("inline envelope must admit its declared payload"));
    let host_output = BindingArtifactEnvelope::try_new(
        identity,
        host_bounds.artifact(),
        host_output.into_artifact(),
    )
    .unwrap_or_else(|_| panic!("Host envelope must preserve the same binding footprint"));

    // Exercise fixed abort separately from the completed cursors.
    let (cursor, requests) = observe(|| portable.compiler().start(&input).unwrap());
    no_requests(requests);
    let before = ABORTS.load(SeqCst);
    let (_, requests) = observe(|| portable.compiler().abort(cursor));
    no_requests(requests);
    let cursor = host.start(&input).unwrap();
    let (aborted, requests) = observe(|| host.abort(cursor));
    assert!(aborted.is_ok() && requests.allocations.is_empty());
    assert_eq!(requests.releases, [layout::<u8>()]);
    assert_eq!(ABORTS.load(SeqCst) - before, 2);

    // Both outputs must survive the actual source and compiler destruction.
    drop(portable);
    drop(host);
    drop(plan);
    assert_eq!(output.identity(), host_output.identity());
    assert_eq!(
        output.artifact().payload().target(),
        "mock://sensor/temperature"
    );
    assert_eq!(
        host_output
            .artifact()
            .try_payload::<InlineTarget>(compatibility)
            .unwrap()
            .target(),
        "mock://sensor/temperature"
    );
    let (_, requests) = observe(|| drop(output));
    no_requests(requests);
    let (_, host_drop) = observe(|| drop(host_output));
    assert!(host_drop.allocations.is_empty());
    assert_eq!(host_drop.releases, [layout::<InlineTarget>()]);
    println!(
        "inline static: no allocator requests in bounds/start/Pending/complete/abort/output drop; payload Layout={:?}",
        layout::<InlineTarget>()
    );
    println!(
        "inline Host complete: {host_complete:?}; output release: {host_drop:?}; same target/identity after source and compiler destruction"
    );
}
