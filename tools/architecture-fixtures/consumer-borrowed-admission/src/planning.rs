//! External bounded witness of the semantic loan -> real Core plan/artifact join.
//! Fixed sixteen-slot containers keep this a discriminator, not an aggregate API.
use crate::td::{self, Event, Fact, Read, SecurityFact, Validated};
use alloc::{
    alloc::{Layout, alloc},
    boxed::Box,
};
use clinkz_wot_core::{
    BindingArtifactEnvelope, BindingArtifactIdentity, BindingArtifactRole, BindingCandidate,
    BindingCompilerBounds, BindingCompilerExtension, BindingCompilerInput, BindingCompilerStep,
    LogicalInteractionPlan, PlanId, PlanSetGeneration, ThingId,
};
use clinkz_wot_foundation::{
    AdmissionLedger, Generation, ResourceKind, SlotIndex, WorkBudget, WorkClass as W,
};
use clinkz_wot_property_read_binding_fixture::{MockArtifact, MockCompiler, MockCompilerCursor};
use core::{cell::Cell, mem};
const SLOTS: usize = 16;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cause {
    Td(td::Cause),
    Capacity,
    Allocation,
    LaterBound,
    Compiler,
    Cancelled,
    Identity,
    Slot,
}
#[derive(Default)]
pub struct Calls {
    pub bounds: Cell<usize>,
    pub starts: Cell<usize>,
    pub aborts: Cell<usize>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Preflight,
    Materialize,
    Bounds,
    Compile,
    Complete,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Footprint {
    pub requested: u64,
    pub peak: u64,
    pub largest: u64,
    pub allocations: u64,
    pub inline: u64,
}

struct Memory {
    ledger: AdmissionLedger,
    largest_limit: u64,
    inline: u64,
    allocations: u64,
}
impl Memory {
    fn new(limit: u64, request: u64) -> Result<Self, Cause> {
        let inline = mem::size_of::<Draft>() as u64;
        let dynamic = limit.checked_sub(inline).ok_or(Cause::Capacity)?;
        Ok(Self {
            ledger: AdmissionLedger::new(
                SlotIndex::new(0),
                Generation::INITIAL,
                0,
                0,
                0,
                dynamic,
                0,
                0,
            ),
            largest_limit: request,
            inline,
            allocations: 0,
        })
    }
    // Call only after the complete copy + release work debit. This requests the
    // exact Layout before allocation, avoiding Vec/Box growth assumptions.
    fn copy(&mut self, text: &str) -> Result<Box<str>, Cause> {
        if text.is_empty() {
            return Ok(Box::from(""));
        }
        let n = text.len() as u64;
        if n > self.largest_limit {
            return Err(Cause::Capacity);
        }
        let layout = Layout::array::<u8>(text.len()).map_err(|_| Cause::Capacity)?;
        let permit = self
            .ledger
            .try_reserve_persistent_runtime(ResourceKind::CompiledRuntimeBytesPerThingMax, n)
            .ok_or(Cause::Capacity)?;
        // SAFETY: a pre-admitted nonzero checked Layout; null is handled, all
        // bytes initialized from valid UTF-8, and Box takes the same allocation.
        let pointer = unsafe { alloc(layout) };
        if pointer.is_null() {
            return Err(Cause::Allocation);
        }
        unsafe {
            pointer.copy_from_nonoverlapping(text.as_ptr(), text.len());
        }
        permit.commit();
        self.allocations += 1;
        let bytes = core::ptr::slice_from_raw_parts_mut(pointer, text.len());
        Ok(unsafe { Box::from_raw(bytes as *mut str) })
    }
    fn artifact_request(&mut self, n: u64) -> Result<(), Cause> {
        if n > self.largest_limit {
            return Err(Cause::Capacity);
        }
        if n != 0 {
            self.ledger
                .try_reserve_persistent_runtime(ResourceKind::CompiledRuntimeBytesPerThingMax, n)
                .ok_or(Cause::Capacity)?
                .commit();
            self.allocations += 1;
        }
        Ok(())
    }
    fn footprint(&self) -> Footprint {
        Footprint {
            requested: self.ledger.live_bytes(),
            peak: self.ledger.peak_live_bytes(),
            largest: self.ledger.largest_contiguous_allocation(),
            allocations: self.allocations,
            inline: self.inline,
        }
    }
}
struct Row {
    plan: LogicalInteractionPlan,
    artifact: Option<BindingArtifactEnvelope<MockArtifact>>,
    raw: Box<str>,
    coding: Option<Box<str>>,
    scopes: [Option<Box<str>>; SLOTS],
}
struct Lookup {
    name: Box<str>,
    start: usize,
    end: usize,
}
/// Closed concrete type: no generic artifact, source pointer, registration
/// callback or input lifetime can be smuggled into this handoff.
pub struct Draft {
    rows: [Option<Row>; SLOTS],
    lookup: [Option<Lookup>; SLOTS],
    plans: usize,
    properties: usize,
    memory: Memory,
}
impl Draft {
    fn new(limit: u64, request: u64) -> Result<Self, Cause> {
        Ok(Self {
            rows: core::array::from_fn(|_| None),
            lookup: core::array::from_fn(|_| None),
            plans: 0,
            properties: 0,
            memory: Memory::new(limit, request)?,
        })
    }
    pub fn footprint(&self) -> Footprint {
        self.memory.footprint()
    }
    pub fn counts(&self) -> (usize, usize) {
        (self.properties, self.plans)
    }
    pub fn select(
        &self,
        name: &str,
        original: Option<u32>,
    ) -> Option<(
        &LogicalInteractionPlan,
        &BindingArtifactEnvelope<MockArtifact>,
    )> {
        let row = self.lookup[..self.properties]
            .iter()
            .flatten()
            .find(|r| r.name.as_ref() == name)?;
        for i in row.start..row.end {
            let r = self.rows[i].as_ref().unwrap();
            if original.is_none_or(|n| n == r.plan.form_index()) {
                return Some((&r.plan, r.artifact.as_ref().unwrap()));
            }
        }
        None
    }
    pub fn raw_and_metadata(&self, slot: usize) -> (&str, Option<&str>, usize) {
        let r = self.rows[slot].as_ref().unwrap();
        (
            &r.raw,
            r.coding.as_deref(),
            r.scopes.iter().flatten().count(),
        )
    }
}
impl Drop for Draft {
    fn drop(&mut self) {
        // Fixed witness catalog: at most sixteen bounded rows. All individual
        // releases were prepaid; production variable output needs a reclaimer.
        for row in &mut self.rows {
            drop(row.take());
        }
        for row in &mut self.lookup {
            drop(row.take());
        }
        let n = self.memory.ledger.live_bytes();
        assert!(self.memory.ledger.release_persistent_runtime(n));
    }
}

pub struct Build<'td, 'registration> {
    read: Option<Read<'td>>,
    compiler: &'registration MockCompiler,
    calls: &'registration Calls,
    candidate: BindingCandidate,
    phase: Phase,
    draft: Option<Draft>,
    properties: usize,
    plans: usize,
    estimated: u64,
    limit: u64,
    request: u64,
    remaining: u64,
    bounds: [Option<BindingCompilerBounds>; SLOTS],
    coordinate: usize,
    cursor: Option<MockCompilerCursor>,
    fail_bound: Option<usize>,
    first_cause: Option<Cause>,
}
// Inline capacity is intentional and reported. Boxing the result would add a
// new allocation solely to make this fixed architecture witness smaller.
#[allow(clippy::large_enum_variant)]
pub enum Step {
    Pending,
    Complete(Draft),
    Failed(Cause),
}
impl<'td, 'registration> Build<'td, 'registration> {
    pub fn new(
        proof: Validated<'td>,
        compiler: &'registration MockCompiler,
        calls: &'registration Calls,
        candidate: BindingCandidate,
        limit: u64,
        request: u64,
        fail_bound: Option<usize>,
    ) -> Result<Self, Cause> {
        if proof.id().is_none() {
            return Err(Cause::Td(td::Cause::MissingId));
        }
        Ok(Self {
            read: Some(Read::new(proof)),
            compiler,
            calls,
            candidate,
            phase: Phase::Preflight,
            draft: None,
            properties: 0,
            plans: 0,
            estimated: 0,
            limit,
            request,
            remaining: 10_000_000,
            bounds: core::array::from_fn(|_| None),
            coordinate: 0,
            cursor: None,
            fail_bound,
            first_cause: None,
        })
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn trace(&self) -> td::Trace {
        self.read.as_ref().unwrap().trace()
    }
    pub fn remaining(&self) -> u64 {
        self.remaining
    }
    fn debit(&mut self, b: &mut WorkBudget, units: u64, cleanup: u64) -> Result<bool, Cause> {
        if b.remaining(W::PlanningItems) < units || b.remaining(W::CleanupItems) < cleanup {
            return Ok(false);
        }
        let n = units + cleanup;
        if self.remaining < n {
            return Err(Cause::Capacity);
        }
        b.consume(W::PlanningItems, units).unwrap();
        b.consume(W::CleanupItems, cleanup).unwrap();
        self.remaining -= n;
        Ok(true)
    }
    pub fn step(&mut self, budget: &mut WorkBudget, cancel: bool) -> Step {
        if let Some(cause) = self.first_cause {
            return Step::Failed(cause);
        }
        if cancel {
            self.first_cause = Some(Cause::Cancelled);
            self.rollback();
            return Step::Failed(Cause::Cancelled);
        }
        match self.tick(budget) {
            Ok(Some(d)) => Step::Complete(d),
            Ok(None) => Step::Pending,
            Err(c) => {
                self.first_cause = Some(c);
                self.rollback();
                Step::Failed(c)
            }
        }
    }
    fn tick(&mut self, budget: &mut WorkBudget) -> Result<Option<Draft>, Cause> {
        if self.phase == Phase::Complete {
            return Err(Cause::Slot);
        }
        let ready = self.phase == Phase::Materialize && self.read.as_ref().is_some_and(Read::ready);
        if !ready && !self.debit(budget, 1, 0)? {
            return Ok(None);
        }
        match self.phase {
            Phase::Preflight | Phase::Materialize => {
                let id = self.read.as_ref().unwrap().id().unwrap();
                let event = self
                    .read
                    .as_mut()
                    .unwrap()
                    .step(budget, false)
                    .map_err(Cause::Td)?;
                match event {
                    Event::Pending => return Ok(None),
                    Event::Property { ordinal, name } => {
                        if ordinal >= SLOTS {
                            return Err(Cause::Capacity);
                        }
                        if self.phase == Phase::Preflight {
                            self.properties += 1;
                            self.estimated += name.len() as u64;
                        } else {
                            let units = name.len() as u64 + 1;
                            if budget.remaining(W::PlanningItems) < units
                                || budget.remaining(W::CleanupItems) < 1
                            {
                                return Ok(None);
                            }
                            self.remaining = self
                                .remaining
                                .checked_sub(units + 1)
                                .ok_or(Cause::Capacity)?;
                            budget.consume(W::PlanningItems, units).unwrap();
                            budget.consume(W::CleanupItems, 1).unwrap();
                            let draft = self.draft.as_mut().unwrap();
                            if ordinal > 0 {
                                draft.lookup[ordinal - 1].as_mut().unwrap().end = draft.plans;
                            }
                            let name = draft.memory.copy(name)?;
                            draft.lookup[ordinal] = Some(Lookup {
                                name,
                                start: draft.plans,
                                end: draft.plans,
                            });
                            draft.properties += 1;
                        }
                        self.read.as_mut().unwrap().acknowledge();
                    }
                    Event::Form(f) => {
                        if !matches!(
                            f.security,
                            SecurityFact::Single {
                                scheme: "nosec",
                                ..
                            }
                        ) {
                            return Err(Cause::Td(td::Cause::Security));
                        }
                        if f.template
                            || f.name.is_empty()
                            || f.resolved.is_empty()
                            || f.scopes.len() > SLOTS
                        {
                            return Err(Cause::Capacity);
                        }
                        let bytes = form_bytes(id, &f);
                        if self.phase == Phase::Preflight {
                            self.plans += 1;
                            if self.plans > SLOTS {
                                return Err(Cause::Capacity);
                            }
                            self.estimated += bytes + f.resolved.len() as u64;
                        } else {
                            if budget.remaining(W::PlanningItems) < bytes + 1
                                || budget.remaining(W::CleanupItems) < (7 + f.scopes.len()) as u64
                            {
                                return Ok(None);
                            }
                            self.remaining = self
                                .remaining
                                .checked_sub(bytes + 8 + f.scopes.len() as u64)
                                .ok_or(Cause::Capacity)?;
                            budget.consume(W::PlanningItems, bytes + 1).unwrap();
                            budget
                                .consume(W::CleanupItems, (7 + f.scopes.len()) as u64)
                                .unwrap();
                            materialize(self.draft.as_mut().unwrap(), id, &f)?;
                        }
                        self.read.as_mut().unwrap().acknowledge();
                    }
                    Event::Done => {
                        if self.phase == Phase::Preflight {
                            if self.estimated + mem::size_of::<Draft>() as u64 > self.limit {
                                return Err(Cause::Capacity);
                            }
                            let proof = self.read.take().unwrap().finish();
                            self.read = Some(Read::new(proof));
                            self.draft = Some(Draft::new(self.limit, self.request)?);
                            self.phase = Phase::Materialize;
                        } else {
                            let draft = self.draft.as_mut().unwrap();
                            if draft.properties > 0 {
                                draft.lookup[draft.properties - 1].as_mut().unwrap().end =
                                    draft.plans;
                            }
                            assert_eq!(
                                (draft.properties, draft.plans),
                                (self.properties, self.plans)
                            );
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
                self.calls.bounds.set(self.calls.bounds.get() + 1);
                if self.fail_bound == Some(self.coordinate) {
                    return Err(Cause::LaterBound);
                }
                let plan = &self.draft.as_ref().unwrap().rows[self.coordinate]
                    .as_ref()
                    .unwrap()
                    .plan;
                let input = BindingCompilerInput::new(
                    plan,
                    self.candidate,
                    BindingArtifactRole::ConsumerCall,
                );
                let bound = self.compiler.bounds(&input).map_err(|_| Cause::Compiler)?;
                if bound.work().remaining(W::BindingPolls) != 1
                    || W::ALL
                        .iter()
                        .any(|&c| c != W::BindingPolls && bound.work().remaining(c) != 0)
                {
                    return Err(Cause::Compiler);
                }
                self.bounds[self.coordinate] = Some(bound);
                self.coordinate += 1;
            }
            Phase::Compile => {
                if self.coordinate == self.plans {
                    self.read = None;
                    self.phase = Phase::Complete;
                    return Ok(self.draft.take());
                }
                if budget.remaining(W::BindingPolls) == 0 || budget.remaining(W::CleanupItems) == 0
                {
                    return Ok(None);
                }
                self.remaining = self.remaining.checked_sub(2).ok_or(Cause::Capacity)?;
                budget.consume(W::BindingPolls, 1).unwrap();
                budget.consume(W::CleanupItems, 1).unwrap();
                let draft = self.draft.as_mut().unwrap();
                let row = draft.rows[self.coordinate].as_mut().unwrap();
                let input = BindingCompilerInput::new(
                    &row.plan,
                    self.candidate,
                    BindingArtifactRole::ConsumerCall,
                );
                if self.cursor.is_none() {
                    self.calls.starts.set(self.calls.starts.get() + 1);
                    self.cursor = Some(self.compiler.start(&input).map_err(|_| Cause::Compiler)?);
                }
                let bound = self.bounds[self.coordinate].take().unwrap();
                let admitted = bound.artifact();
                let mut coordinate_budget = bound.into_work();
                let n = row.plan.resolved_target().len() as u64;
                // The inspected mock makes exactly this one target allocation.
                // It is admitted BEFORE its callback, never inferred from the
                // artifact's post-allocation report alone.
                draft.memory.artifact_request(n)?;
                let cursor = self.cursor.take().unwrap();
                let artifact = match self.compiler.step(&input, cursor, &mut coordinate_budget) {
                    BindingCompilerStep::Complete(out) => out.into_artifact(),
                    BindingCompilerStep::Failed(f) => {
                        self.cursor = Some(f.into_parts().1);
                        return Err(Cause::Compiler);
                    }
                    BindingCompilerStep::Pending(cursor) => {
                        self.cursor = Some(cursor);
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
            Phase::Complete => unreachable!(),
        }
        Ok(None)
    }
    fn rollback(&mut self) {
        if let Some(cursor) = self.cursor.take() {
            self.calls.aborts.set(self.calls.aborts.get() + 1);
            self.compiler.abort(cursor);
        }
        drop(self.draft.take());
        self.read = None;
        self.phase = Phase::Complete;
    }
}
impl Drop for Build<'_, '_> {
    fn drop(&mut self) {
        self.rollback();
    }
}
fn form_bytes(id: &str, f: &Fact<'_>) -> u64 {
    (id.len()
        + f.name.len()
        + f.resolved.len()
        + f.content_type.len()
        + f.raw.len()
        + f.subprotocol.map_or(0, str::len)
        + f.content_coding.map_or(0, str::len)
        + f.scopes.iter().map(str::len).sum::<usize>()) as u64
}
fn materialize(draft: &mut Draft, id: &str, f: &Fact<'_>) -> Result<(), Cause> {
    let memory = &mut draft.memory;
    let id = ThingId::new(memory.copy(id)?.into_string());
    let name = memory.copy(f.name)?;
    let target = memory.copy(f.resolved)?;
    let content = memory.copy(f.content_type)?;
    let subprotocol = f.subprotocol.map(|v| memory.copy(v)).transpose()?;
    let plan = LogicalInteractionPlan::try_property_read(
        PlanId::new(SlotIndex::new(draft.plans as u32), Generation::INITIAL),
        id,
        name,
        f.original as u32,
        target,
        Some(content),
        subprotocol,
    )
    .map_err(|_| Cause::Compiler)?;
    let raw = memory.copy(f.raw)?;
    let coding = f.content_coding.map(|v| memory.copy(v)).transpose()?;
    let mut scopes = core::array::from_fn(|_| None);
    for (slot, value) in scopes.iter_mut().zip(f.scopes.iter()) {
        *slot = Some(memory.copy(value)?);
    }
    draft.rows[draft.plans] = Some(Row {
        plan,
        artifact: None,
        raw,
        coding,
        scopes,
    });
    draft.plans += 1;
    Ok(())
}

/// Single-thread, one-slot publication model; real Servient concurrency,
/// parent/global allowances, leases/drain/reclaim remain subsequent evidence.
#[derive(Default)]
pub struct Registry {
    published: Option<Draft>,
}
struct Permit(Draft);
impl Registry {
    pub fn publish(
        &mut self,
        draft: Draft,
        cancel: bool,
        identity_matches: bool,
    ) -> Result<(), Cause> {
        if cancel {
            return Err(Cause::Cancelled);
        }
        if !identity_matches {
            return Err(Cause::Identity);
        }
        if self.published.is_some() {
            return Err(Cause::Slot);
        }
        let permit = Permit(draft);
        self.install(permit);
        Ok(())
    }
    fn install(&mut self, permit: Permit) {
        self.published = Some(permit.0);
    }
    pub fn draft(&self) -> Option<&Draft> {
        self.published.as_ref()
    }
}
