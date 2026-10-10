//! Closed fixture implementation: copies a resolved target, with no TD rules.
use core::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use clinkz_wot_core::*;
use clinkz_wot_foundation::{WorkBudget, WorkClass};

pub const COMPATIBILITY: BindingArtifactCompatibility =
    BindingArtifactCompatibility::new([0x61; 16]);
pub const CAPACITY: usize = 64;
pub static CALLBACKS: AtomicUsize = AtomicUsize::new(0);
pub static ABORTS: AtomicUsize = AtomicUsize::new(0);
pub static ARTIFACT_DROPS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scenario {
    Success,
    StartFailure,
    StepFailure,
}

pub struct InlineCompiler {
    pub(crate) capacity: usize,
    pub(crate) scenario: Scenario,
}

pub struct InlineCursor(pub(crate) u8);

pub struct InlineArtifact {
    bytes: [u8; CAPACITY],
    len: usize,
}

impl InlineArtifact {
    pub fn target(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).expect("copied from a valid str")
    }
}

impl Drop for InlineArtifact {
    fn drop(&mut self) {
        ARTIFACT_DROPS.fetch_add(1, SeqCst);
    }
}

pub(crate) fn error() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}

impl BindingCompilerExtension for InlineCompiler {
    type Cursor = InlineCursor;
    type Artifact = InlineArtifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        CALLBACKS.fetch_add(1, SeqCst);
        COMPATIBILITY
    }
    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        CALLBACKS.fetch_add(1, SeqCst);
        if input.role() != BindingArtifactRole::ConsumerCall
            || input.logical_plan().resolved_target().len() > self.capacity
        {
            return Err(error());
        }
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, core::mem::size_of::<InlineArtifact>() as u64),
            core::mem::size_of::<InlineCursor>() as u64,
            core::mem::size_of::<BindingCompilerStep<InlineCursor, InlineArtifact>>() as u64,
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 2),
        ))
    }
    fn start(&self, _: &BindingCompilerInput<'_>) -> CoreResult<InlineCursor> {
        CALLBACKS.fetch_add(1, SeqCst);
        if self.scenario == Scenario::StartFailure {
            return Err(error());
        }
        Ok(InlineCursor(0))
    }
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: InlineCursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<InlineCursor, InlineArtifact> {
        CALLBACKS.fetch_add(1, SeqCst);
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return BindingCompilerStep::Pending(cursor);
        }
        if cursor.0 == 0 {
            return BindingCompilerStep::Pending(InlineCursor(1));
        }
        let target = input.logical_plan().resolved_target().as_bytes();
        if self.scenario == Scenario::StepFailure || target.len() > self.capacity {
            return BindingCompilerStep::Failed(BindingCompilerFailure::new(error(), cursor));
        }
        let mut payload = InlineArtifact {
            bytes: [0; CAPACITY],
            len: target.len(),
        };
        payload.bytes[..target.len()].copy_from_slice(target);
        BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
            COMPATIBILITY,
            BindingArtifactFootprint::new(1, core::mem::size_of::<InlineArtifact>() as u64),
            payload,
        )))
    }
    fn abort(&self, _: InlineCursor) {
        CALLBACKS.fetch_add(1, SeqCst);
        ABORTS.fetch_add(1, SeqCst);
    }
}

/// Same compatibility and associated types do not make this implementation supported.
pub struct Impostor;
impl BindingCompilerExtension for Impostor {
    type Cursor = InlineCursor;
    type Artifact = InlineArtifact;
    fn compatibility(&self) -> BindingArtifactCompatibility {
        CALLBACKS.fetch_add(1, SeqCst);
        COMPATIBILITY
    }
    fn bounds(&self, _: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        panic!("unsupported bounds reached")
    }
    fn start(&self, _: &BindingCompilerInput<'_>) -> CoreResult<InlineCursor> {
        panic!("unsupported start reached")
    }
    fn step(
        &self,
        _: &BindingCompilerInput<'_>,
        _: InlineCursor,
        _: &mut WorkBudget,
    ) -> BindingCompilerStep<InlineCursor, InlineArtifact> {
        panic!("unsupported step reached")
    }
    fn abort(&self, _: InlineCursor) {
        panic!("unsupported abort reached")
    }
}
