use alloc::{alloc::alloc, boxed::Box};
use clinkz_wot_core::{
    BindingArtifactEnvelope, BindingArtifactIdentity, BindingArtifactRole, BindingCandidate,
    BindingCompilerBounds, BindingCompilerExtension, BindingCompilerInput, BindingCompilerStep,
    LogicalInteractionPlan, PlanId, PlanSetGeneration, ThingId,
};
use clinkz_wot_foundation::{
    AdmissionLedger, Generation, ResourceKind, SlotIndex, WorkBudget, WorkClass as W,
};
use clinkz_wot_property_read_binding_fixture::{MockArtifact, MockCompiler, MockCompilerCursor};
use clinkz_wot_td::{
    ValidatedPropertyReadCursor, ValidatedPropertyReadEvent as Event, ValidatedPropertyReadForm,
    ValidatedPropertyReadStep as ReadStep, ValidatedThingCause,
};
use core::{alloc::Layout, cell::Cell, mem};

use crate::registration::Registration;

// Closed fixture envelope, deliberately unrelated to named product maxima.
pub const SLOTS: usize = 8;
pub const SCOPES: usize = 4;
#[derive(Clone, Copy)]
pub struct Limits {
    pub output_bytes: u64,
    pub copy_request_bytes: u64,
    pub artifact_bytes: u64,
    pub planning_work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            output_bytes: 4096,
            copy_request_bytes: 256,
            artifact_bytes: 128,
            planning_work: 100_000,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cause {
    Td(ValidatedThingCause),
    MissingId,
    IneligibleSecurity,
    Capacity,
    Allocation,
    Bounds,
    Compiler,
    Work,
    Cancelled,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Preflight,
    Materialize,
    Bounds,
    Compile,
}
#[derive(Default)]
pub struct Calls {
    pub bounds: Cell<usize>,
    pub starts: Cell<usize>,
    pub steps: Cell<usize>,
    pub aborts: Cell<usize>,
}

struct Memory {
    ledger: AdmissionLedger,
    request_max: u64,
}
impl Memory {
    fn copy(&mut self, text: &str) -> Result<Box<str>, Cause> {
        if text.is_empty() {
            return Ok(Box::from(""));
        }
        let size = text.len() as u64;
        if size > self.request_max {
            return Err(Cause::Capacity);
        }
        let layout = Layout::array::<u8>(text.len()).map_err(|_| Cause::Capacity)?;
        let permit = self
            .ledger
            .try_reserve_persistent_runtime(ResourceKind::CompiledRuntimeBytesPerThingMax, size)
            .ok_or(Cause::Capacity)?;
        // SAFETY: nonzero checked Layout, null handled, every byte initialized
        // from valid UTF-8; Box assumes the exact allocation and alignment.
        let pointer = unsafe { alloc(layout) };
        if pointer.is_null() {
            return Err(Cause::Allocation);
        }
        unsafe { pointer.copy_from_nonoverlapping(text.as_ptr(), text.len()) };
        permit.commit();
        Ok(unsafe {
            Box::from_raw(core::ptr::slice_from_raw_parts_mut(pointer, text.len()) as *mut str)
        })
    }
}
struct Lookup {
    name: Box<str>,
    start: usize,
    end: usize,
}
pub struct Row {
    pub property_ordinal: u32,
    pub plan: LogicalInteractionPlan,
    pub artifact: Option<BindingArtifactEnvelope<MockArtifact>>,
    pub raw: Box<str>,
    pub coding: Option<Box<str>>,
    pub scopes: [Option<Box<str>>; SCOPES],
}
/// Concrete owned output: no generic payload/callback or source lifetime.
pub struct Draft {
    id: Option<Box<str>>,
    rows: [Option<Row>; SLOTS],
    lookup: [Option<Lookup>; SLOTS],
    properties: usize,
    plans: usize,
    memory: Memory,
}
impl Draft {
    fn new(limits: Limits) -> Self {
        Self {
            id: None,
            rows: core::array::from_fn(|_| None),
            lookup: core::array::from_fn(|_| None),
            properties: 0,
            plans: 0,
            memory: Memory {
                ledger: AdmissionLedger::new(
                    SlotIndex::new(0),
                    Generation::INITIAL,
                    0,
                    0,
                    0,
                    limits.output_bytes,
                    0,
                    0,
                ),
                request_max: limits.copy_request_bytes,
            },
        }
    }
    pub fn counts(&self) -> (usize, usize) {
        (self.properties, self.plans)
    }
    pub fn heap_bytes(&self) -> u64 {
        self.memory.ledger.live_bytes()
    }
    pub fn lookup_range(&self, name: &str) -> Option<(usize, usize)> {
        self.lookup[..self.properties]
            .iter()
            .flatten()
            .find(|r| r.name.as_ref() == name)
            .map(|r| (r.start, r.end))
    }
    pub fn select(&self, name: &str, original: Option<u32>) -> Option<&Row> {
        let (start, end) = self.lookup_range(name)?;
        self.rows[start..end]
            .iter()
            .flatten()
            .find(|r| original.is_none_or(|i| i == r.plan.form_index()))
    }
}
impl Drop for Draft {
    fn drop(&mut self) {
        // Each child release was prepaid before construction. This finite
        // catalog is not the product's variable-output rollback/reclaimer.
        for row in &mut self.rows {
            drop(row.take());
        }
        for row in &mut self.lookup {
            drop(row.take());
        }
        drop(self.id.take());
        let bytes = self.memory.ledger.live_bytes();
        assert!(self.memory.ledger.release_persistent_runtime(bytes));
    }
}

pub struct Build<'td, 'registration> {
    read: Option<ValidatedPropertyReadCursor<'td>>,
    compiler: &'registration MockCompiler,
    calls: &'registration Calls,
    candidate: BindingCandidate,
    draft: Option<Draft>,
    phase: Phase,
    limits: Limits,
    remaining: u64,
    properties: usize,
    plans: usize,
    estimated: u64,
    bounds: [Option<BindingCompilerBounds>; SLOTS],
    coordinate: usize,
    cursor: Option<MockCompilerCursor>,
    copy_wait: bool,
}
#[allow(clippy::large_enum_variant)]
pub enum Step<'td, 'registration> {
    Pending(Build<'td, 'registration>),
    Complete(Draft),
    Failed(Cause),
}
impl<'td, 'registration> Build<'td, 'registration> {
    pub fn new(
        read: ValidatedPropertyReadCursor<'td>,
        registration: &'registration Registration,
        calls: &'registration Calls,
        limits: Limits,
    ) -> Result<Self, Cause> {
        let id_bytes = read.id().ok_or(Cause::MissingId)?.len() as u64;
        let identity = registration.identity();
        Ok(Self {
            read: Some(read),
            compiler: registration.compiler().compiler(),
            calls,
            candidate: BindingCandidate::new(
                identity.binding_id(),
                identity.binding_generation(),
                identity.configuration(),
                identity.artifact_compatibility(),
                0,
                0,
            ),
            draft: Some(Draft::new(limits)),
            phase: Phase::Preflight,
            limits,
            remaining: limits.planning_work,
            properties: 0,
            plans: 0,
            estimated: id_bytes,
            bounds: core::array::from_fn(|_| None),
            coordinate: 0,
            cursor: None,
            copy_wait: false,
        })
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn counts(&self) -> (usize, usize) {
        self.draft.as_ref().unwrap().counts()
    }
    pub fn copy_wait(&self) -> bool {
        self.copy_wait
    }
    pub fn compiler_pending(&self) -> bool {
        self.cursor.is_some()
    }
    pub fn remaining(&self) -> u64 {
        self.remaining
    }
    pub fn step(mut self, budget: &mut WorkBudget, cancel: bool) -> Step<'td, 'registration> {
        if cancel {
            return Step::Failed(Cause::Cancelled);
        }
        match self.tick(budget) {
            Ok(Some(draft)) => Step::Complete(draft),
            Ok(None) => Step::Pending(self),
            Err(cause) => Step::Failed(cause),
        }
    }
    fn tick(&mut self, budget: &mut WorkBudget) -> Result<Option<Draft>, Cause> {
        match self.phase {
            Phase::Preflight | Phase::Materialize => {
                let read = self.read.as_mut().unwrap();
                let draft = self.draft.as_mut().unwrap();
                if self.phase == Phase::Materialize && draft.id.is_none() {
                    let id = read.id().unwrap();
                    self.copy_wait = !debit(budget, &mut self.remaining, id.len() as u64, 1, 1, 0)?;
                    if self.copy_wait {
                        return Ok(None);
                    }
                    draft.id = Some(draft.memory.copy(id)?);
                }
                match read.step(budget, false).map_err(Cause::Td)? {
                    ReadStep::Pending => return Ok(None),
                    ReadStep::Ready(Event::Property { ordinal, name }) => {
                        let i = ordinal as usize;
                        if i >= SLOTS {
                            return Err(Cause::Capacity);
                        }
                        let materialize = self.phase == Phase::Materialize;
                        let bytes = if materialize { name.len() as u64 } else { 0 };
                        self.copy_wait = !debit(
                            budget,
                            &mut self.remaining,
                            bytes,
                            1,
                            u64::from(materialize),
                            0,
                        )?;
                        if self.copy_wait {
                            return Ok(None);
                        }
                        if materialize {
                            if i != draft.properties {
                                return Err(Cause::Capacity);
                            }
                            if i > 0 {
                                draft.lookup[i - 1].as_mut().unwrap().end = draft.plans;
                            }
                            draft.lookup[i] = Some(Lookup {
                                name: draft.memory.copy(name)?,
                                start: draft.plans,
                                end: draft.plans,
                            });
                            draft.properties += 1;
                        } else {
                            self.properties += 1;
                            self.estimated = self
                                .estimated
                                .checked_add(name.len() as u64)
                                .ok_or(Cause::Capacity)?;
                        }
                        read.acknowledge();
                    }
                    ReadStep::Ready(Event::Form(form)) => {
                        if form.security_count() != 1 || form.security_scheme() != Some("nosec") {
                            return Err(Cause::IneligibleSecurity);
                        }
                        if form.scopes().len() > SCOPES {
                            return Err(Cause::Capacity);
                        }
                        let materialize = self.phase == Phase::Materialize;
                        let bytes = if materialize { form.copy_bytes() } else { 0 };
                        let scopes = form.scopes().len() as u64;
                        // ID/name/target/content/raw, optional coding/subprotocol, scopes.
                        let cleanup = if materialize {
                            5 + u64::from(form.content_coding().is_some())
                                + u64::from(form.subprotocol().is_some())
                                + scopes
                        } else {
                            0
                        };
                        self.copy_wait = !debit(
                            budget,
                            &mut self.remaining,
                            bytes,
                            1 + if materialize { scopes } else { 0 },
                            cleanup,
                            0,
                        )?;
                        if self.copy_wait {
                            return Ok(None);
                        }
                        if materialize {
                            if draft.plans == SLOTS {
                                return Err(Cause::Capacity);
                            }
                            draft.rows[draft.plans] = Some(materialize_form(
                                &mut draft.memory,
                                draft.id.as_deref().unwrap(),
                                draft.plans,
                                &form,
                            )?);
                            draft.plans += 1;
                        } else {
                            self.plans += 1;
                            if self.plans > SLOTS {
                                return Err(Cause::Capacity);
                            }
                            // The closed compiler copies the resolved target once.
                            self.estimated = self
                                .estimated
                                .checked_add(form.copy_bytes())
                                .and_then(|n| n.checked_add(form.resolved_href().len() as u64))
                                .ok_or(Cause::Capacity)?;
                        }
                        read.acknowledge();
                    }
                    ReadStep::Done => {
                        if self.phase == Phase::Preflight {
                            if self.estimated > self.limits.output_bytes {
                                return Err(Cause::Capacity);
                            }
                            self.read = Some(self.read.take().unwrap().rewind());
                            self.phase = Phase::Materialize;
                        } else {
                            if draft.properties > 0 {
                                draft.lookup[draft.properties - 1].as_mut().unwrap().end =
                                    draft.plans;
                            }
                            if draft.counts() != (self.properties, self.plans) {
                                return Err(Cause::Capacity);
                            }
                            // All short loans and the entire TD child end before compiler work.
                            self.read = None;
                            self.phase = Phase::Bounds;
                        }
                    }
                }
            }
            Phase::Bounds => {
                if self.coordinate == self.plans {
                    self.coordinate = 0;
                    self.phase = Phase::Compile;
                    return Ok(None);
                }
                if !debit(budget, &mut self.remaining, 0, 1, 0, 0)? {
                    return Ok(None);
                }
                let row = self.draft.as_ref().unwrap().rows[self.coordinate]
                    .as_ref()
                    .unwrap();
                let input = BindingCompilerInput::new(
                    &row.plan,
                    self.candidate,
                    BindingArtifactRole::ConsumerCall,
                );
                self.calls.bounds.set(self.calls.bounds.get() + 1);
                let bound = self.compiler.bounds(&input).map_err(|_| Cause::Compiler)?;
                // Validate this actual compiler's closed support contract, not a
                // synthetic failure switch or generic compiler admission claim.
                if bound.artifact().retained_bytes() > self.limits.artifact_bytes
                    || bound.artifact().retained_bytes() != row.plan.resolved_target().len() as u64
                    || bound.artifact().retained_items() != 1
                    || bound.cursor_bytes() != mem::size_of::<MockCompilerCursor>() as u64
                    || bound.temporary_bytes() != 0
                    || W::ALL
                        .iter()
                        .any(|&w| bound.work().remaining(w) != u64::from(w == W::BindingPolls))
                {
                    return Err(Cause::Bounds);
                }
                self.bounds[self.coordinate] = Some(bound);
                self.coordinate += 1;
            }
            Phase::Compile => {
                if self.coordinate == self.plans {
                    return Ok(self.draft.take());
                }
                let draft = self.draft.as_mut().unwrap();
                let row = draft.rows[self.coordinate].as_mut().unwrap();
                let input = BindingCompilerInput::new(
                    &row.plan,
                    self.candidate,
                    BindingArtifactRole::ConsumerCall,
                );
                if self.cursor.is_none() {
                    if !debit(budget, &mut self.remaining, 0, 1, 1, 0)? {
                        return Ok(None);
                    }
                    self.calls.starts.set(self.calls.starts.get() + 1);
                    self.cursor = Some(self.compiler.start(&input).map_err(|_| Cause::Compiler)?);
                    return Ok(None);
                }
                let n = row.plan.resolved_target().len() as u64;
                if !debit(budget, &mut self.remaining, n, 1, 1, 1)? {
                    return Ok(None);
                }
                // Pre-admit the one inspected MockCompiler allocation. Its
                // infallible Box adapter is not a generic fallible compiler.
                draft
                    .memory
                    .ledger
                    .try_reserve_persistent_runtime(
                        ResourceKind::CompiledRuntimeBytesPerThingMax,
                        n,
                    )
                    .ok_or(Cause::Capacity)?
                    .commit();
                let bound = self.bounds[self.coordinate].take().unwrap();
                let admitted = bound.artifact();
                let mut coordinate_work = bound.into_work();
                self.calls.steps.set(self.calls.steps.get() + 1);
                let artifact = match self.compiler.step(
                    &input,
                    self.cursor.take().unwrap(),
                    &mut coordinate_work,
                ) {
                    BindingCompilerStep::Complete(out) => out.into_artifact(),
                    BindingCompilerStep::Pending(cursor) => {
                        self.cursor = Some(cursor);
                        return Err(Cause::Compiler);
                    }
                    BindingCompilerStep::Failed(failure) => {
                        self.cursor = Some(failure.into_parts().1);
                        return Err(Cause::Compiler);
                    }
                };
                let identity = BindingArtifactIdentity::new(
                    PlanSetGeneration::INITIAL,
                    row.plan.plan_id(),
                    self.candidate.binding_id(),
                    self.candidate.binding_generation(),
                    self.candidate.configuration(),
                    self.candidate.compatibility(),
                    BindingArtifactRole::ConsumerCall,
                );
                row.artifact = Some(
                    BindingArtifactEnvelope::try_new(identity, admitted, artifact)
                        .map_err(|_| Cause::Compiler)?,
                );
                self.coordinate += 1;
            }
        }
        Ok(None)
    }
}
impl Drop for Build<'_, '_> {
    fn drop(&mut self) {
        if let Some(cursor) = self.cursor.take() {
            self.calls.aborts.set(self.calls.aborts.get() + 1);
            self.compiler.abort(cursor);
        }
        self.read = None;
        drop(self.draft.take());
    }
}
fn debit(
    budget: &mut WorkBudget,
    remaining: &mut u64,
    bytes: u64,
    items: u64,
    cleanup: u64,
    polls: u64,
) -> Result<bool, Cause> {
    let charges = [
        (W::CodecOutputBytes, bytes),
        (W::PlanningItems, items),
        (W::CleanupItems, cleanup),
        (W::BindingPolls, polls),
    ];
    if charges.iter().any(|&(w, n)| budget.remaining(w) < n) {
        return Ok(false);
    }
    let total = charges
        .iter()
        .try_fold(0u64, |sum, &(_, n)| sum.checked_add(n))
        .ok_or(Cause::Work)?;
    *remaining = remaining.checked_sub(total).ok_or(Cause::Work)?;
    for (w, n) in charges {
        budget.consume(w, n).unwrap();
    }
    Ok(true)
}
fn materialize_form(
    memory: &mut Memory,
    id: &str,
    slot: usize,
    f: &ValidatedPropertyReadForm<'_>,
) -> Result<Row, Cause> {
    let plan = LogicalInteractionPlan::try_property_read(
        PlanId::new(SlotIndex::new(slot as u32), Generation::INITIAL),
        ThingId::new(memory.copy(id)?.into_string()),
        memory.copy(f.property_name())?,
        f.original_index(),
        memory.copy(f.resolved_href())?,
        Some(memory.copy(f.content_type())?),
        f.subprotocol().map(|s| memory.copy(s)).transpose()?,
    )
    .map_err(|_| Cause::Compiler)?;
    let raw = memory.copy(f.href())?;
    let coding = f.content_coding().map(|s| memory.copy(s)).transpose()?;
    let mut scopes = core::array::from_fn(|_| None);
    for (slot, value) in scopes.iter_mut().zip(f.scopes().iter()) {
        *slot = Some(memory.copy(value)?);
    }
    Ok(Row {
        property_ordinal: f.property_ordinal(),
        plan,
        artifact: None,
        raw,
        coding,
        scopes,
    })
}
