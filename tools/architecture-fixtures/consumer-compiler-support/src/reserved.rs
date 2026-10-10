//! Prospective Core transport only. This does not replace production Host erasure.
use alloc::boxed::Box;
use core::{alloc::Layout, mem};
use std::any::Any;

use clinkz_wot_core::*;
use clinkz_wot_foundation::{AdmissionLedger, ResourceKind, WorkBudget, WorkClass};

use crate::{
    CheckedRegistration, Representation,
    compiler::{self, InlineArtifact, InlineCursor},
};

enum Slot {
    Vacant,
    Cursor(InlineCursor),
    Complete(BindingCompilerOutput<InlineArtifact>),
}

struct Owned {
    storage: Option<Box<dyn Any + Send + Sync>>,
    ledger: Option<AdmissionLedger>,
    identity: BindingArtifactIdentity,
}

impl Owned {
    fn slot(&self) -> &Slot {
        self.storage.as_ref().unwrap().downcast_ref().unwrap()
    }
    fn slot_mut(&mut self) -> &mut Slot {
        self.storage.as_mut().unwrap().downcast_mut().unwrap()
    }
    fn release(&mut self) {
        if let Some(storage) = self.storage.take() {
            drop(storage);
            assert!(
                self.ledger
                    .as_mut()
                    .unwrap()
                    .release_persistent_runtime(Layout::new::<Slot>().size() as u64)
            );
        }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        self.release();
    }
}

/// Physically held storage and complete-registration loan; no separate compiler half.
pub struct ReservedCursor<'r> {
    checked: &'r CheckedRegistration,
    owned: Option<Owned>,
    remaining: u32,
}
pub enum HostStep<'r> {
    Pending(ReservedCursor<'r>),
    Complete(HostOutput),
    Failed(CoreError, ReservedCursor<'r>),
}

/// No source, compiler or registration lifetime survives completion.
pub struct HostOutput(Owned);

fn try_slot() -> Option<Box<Slot>> {
    let layout = Layout::new::<Slot>();
    assert!(layout.size() != 0);
    // SAFETY: exact nonzero Layout; null is handled before initialization. The
    // initialized pointer is transferred once to Box, with that same Layout.
    let pointer = unsafe { std::alloc::alloc(layout) }.cast::<Slot>();
    if pointer.is_null() {
        return None;
    }
    unsafe {
        pointer.write(Slot::Vacant);
        Some(Box::from_raw(pointer))
    }
}

pub fn reserve<'r>(
    checked: &'r CheckedRegistration,
    input: &BindingCompilerInput<'_>,
    mut ledger: AdmissionLedger,
    largest_request_max: u64,
    budget: &mut WorkBudget,
) -> Result<ReservedCursor<'r>, (AdmissionLedger, CoreError)> {
    let layout = Layout::new::<Slot>();
    if checked.representation() != Representation::ReservedHostPrototype
        || !checked.matches(input)
        || layout.size() as u64 > largest_request_max
        || budget.remaining(WorkClass::CleanupItems) < 1
    {
        return Err((ledger, compiler::error()));
    }
    let Some(reservation) = ledger.try_reserve_persistent_runtime(
        ResourceKind::CompiledRuntimeBytesPerThingMax,
        layout.size() as u64,
    ) else {
        return Err((ledger, compiler::error()));
    };
    budget.consume(WorkClass::CleanupItems, 1).unwrap();
    let Some(storage) = try_slot() else {
        drop(reservation);
        return Err((ledger, compiler::error()));
    };
    reservation.commit();
    let candidate = input.candidate();
    let identity = BindingArtifactIdentity::new(
        PlanSetGeneration::INITIAL,
        input.logical_plan().plan_id(),
        candidate.binding_id(),
        candidate.binding_generation(),
        candidate.configuration(),
        candidate.compatibility(),
        BindingArtifactRole::ConsumerCall,
    );
    Ok(ReservedCursor {
        checked,
        owned: Some(Owned {
            storage: Some(storage),
            ledger: Some(ledger),
            identity,
        }),
        remaining: 2,
    })
}

impl ReservedCursor<'_> {
    pub fn slab_layout() -> Layout {
        Layout::new::<Slot>()
    }
    pub fn owner_layout() -> Layout {
        Layout::new::<Self>()
    }
    pub fn allocation_address(&self) -> usize {
        self.owned.as_ref().unwrap().slot() as *const Slot as usize
    }
    pub fn live_bytes(&self) -> u64 {
        self.owned
            .as_ref()
            .unwrap()
            .ledger
            .as_ref()
            .unwrap()
            .live_bytes()
    }
    pub fn start(
        mut self,
        input: &BindingCompilerInput<'_>,
        budget: &mut WorkBudget,
    ) -> Result<Self, (Self, CoreError)> {
        let owned = self.owned.as_ref().unwrap();
        if !matches!(owned.slot(), Slot::Vacant)
            || !self.checked.matches(input)
            || input.logical_plan().plan_id() != owned.identity.plan_id()
            || budget.remaining(WorkClass::BindingPolls) < 1
            || budget.remaining(WorkClass::CleanupItems) < 1
        {
            return Err((self, compiler::error()));
        }
        budget.consume(WorkClass::BindingPolls, 1).unwrap();
        budget.consume(WorkClass::CleanupItems, 1).unwrap();
        match self.checked.compiler().start(input) {
            Ok(cursor) => {
                *self.owned.as_mut().unwrap().slot_mut() = Slot::Cursor(cursor);
                Ok(self)
            }
            Err(error) => Err((self, error)),
        }
    }
}
impl<'r> ReservedCursor<'r> {
    pub fn step(
        mut self,
        input: &BindingCompilerInput<'_>,
        budget: &mut WorkBudget,
    ) -> HostStep<'r> {
        if !matches!(self.owned.as_ref().unwrap().slot(), Slot::Cursor(_)) {
            return HostStep::Failed(compiler::error(), self);
        }
        // Do not even move the erased payload on an unpaid retry.
        if budget.remaining(WorkClass::BindingPolls) < 2 {
            return HostStep::Pending(self);
        }
        let owned = self.owned.as_mut().unwrap();
        let Slot::Cursor(cursor) = mem::replace(owned.slot_mut(), Slot::Vacant) else {
            unreachable!()
        };
        match self.checked.drive(
            input,
            owned.identity.plan_id(),
            cursor,
            &mut self.remaining,
            budget,
        ) {
            BindingCompilerStep::Pending(cursor) => {
                *owned.slot_mut() = Slot::Cursor(cursor);
                HostStep::Pending(self)
            }
            BindingCompilerStep::Failed(failure) => {
                let (error, cursor) = failure.into_parts();
                *owned.slot_mut() = Slot::Cursor(cursor);
                HostStep::Failed(error, self)
            }
            BindingCompilerStep::Complete(output) => {
                *owned.slot_mut() = Slot::Complete(output);
                HostStep::Complete(HostOutput(self.owned.take().unwrap()))
            }
        }
    }
}
impl Drop for ReservedCursor<'_> {
    fn drop(&mut self) {
        if let Some(owned) = self.owned.as_mut() {
            if let Slot::Cursor(cursor) = mem::replace(owned.slot_mut(), Slot::Vacant) {
                self.checked.compiler().abort(cursor);
            }
        }
    }
}
impl HostOutput {
    pub fn identity(&self) -> BindingArtifactIdentity {
        self.0.identity
    }
    pub fn allocation_address(&self) -> usize {
        self.0.slot() as *const Slot as usize
    }
    pub fn binding_footprint(&self) -> BindingArtifactFootprint {
        let Slot::Complete(output) = self.0.slot() else {
            unreachable!()
        };
        output.artifact().footprint()
    }
    pub fn try_payload<T: 'static>(&self, expected: BindingArtifactCompatibility) -> Option<&T> {
        if expected != self.0.identity.compatibility() {
            return None;
        }
        let Slot::Complete(output) = self.0.slot() else {
            unreachable!()
        };
        if output.artifact().compatibility() != expected {
            return None;
        }
        (output.artifact().payload() as &dyn Any).downcast_ref()
    }
    pub fn reclaim(mut self) -> AdmissionLedger {
        self.0.release();
        self.0.ledger.take().unwrap()
    }
}
