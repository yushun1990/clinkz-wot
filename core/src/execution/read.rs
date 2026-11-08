//! Source-free contracts for the caller-owned finite Read representation. Native
//! Bindings depend on Core, never on Servient or a concrete logical runtime.
use super::ValueExecution;
use crate::{
    BindingGeneration, BindingId, CoreError, CoreResult, Deadline, InteractionOutputView,
    PayloadView, PlanId, ThingSlotId, data_schema::PreparedSchema,
};
use clinkz_wot_foundation::WorkBudget;
use core::{
    cell::Cell,
    task::{Context, Poll},
};

/// Lifetime-bound local logical authority. Loans prove issuer identity without
/// a reference count; revocation is monotonic and cannot reactivate old work.
pub struct LocalConsumerInstance {
    slot: ThingSlotId,
    current: Cell<bool>,
    sequence: Cell<u64>,
}
impl LocalConsumerInstance {
    pub const fn new(slot: ThingSlotId) -> Self {
        Self {
            slot,
            current: Cell::new(true),
            sequence: Cell::new(0),
        }
    }
    pub fn slot(&self) -> ThingSlotId {
        self.slot
    }
    pub fn is_current(&self) -> bool {
        self.current.get()
    }
    pub fn revoke(&self) {
        self.current.set(false);
    }
    pub fn reference(&self) -> LocalConsumerRef<'_> {
        LocalConsumerRef(self)
    }
}

/// Identity/currentness loan, without revocation power. Borrowing retains the
/// original issuer's storage lifetime; it does not reactivate a revoked owner.
///
/// ```compile_fail,E0599
/// fn revoke(reference: clinkz_wot_core::execution::read::LocalConsumerRef<'_>) {
///     reference.revoke();
/// }
/// ```
#[derive(Clone, Copy)]
pub struct LocalConsumerRef<'a>(&'a LocalConsumerInstance);
impl LocalConsumerRef<'_> {
    pub fn slot(self) -> ThingSlotId {
        self.0.slot()
    }
    pub fn is_current(self) -> bool {
        self.0.is_current()
    }
    /// Reserve a non-reused obligation identity. Numeric issuance alone grants
    /// no dispatch or instance permission; the live runtime admits actual work.
    pub fn reserve_sequence(&self) -> CoreResult<u64> {
        let next = self.0.sequence.get().checked_add(1).ok_or_else(|| {
            CoreError::Backpressure(crate::ErrorContext::new(
                crate::ErrorPhase::Admission,
                crate::RetryClass::Never,
            ))
        })?;
        self.0.sequence.set(next);
        Ok(next)
    }
}
impl PartialEq for LocalConsumerRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.0, other.0)
    }
}
impl Eq for LocalConsumerRef<'_> {}

/// One actual native mapping. The endpoint is a policy operand, not an Engine
/// protocol instruction or authority. The native program retains its meaning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadMapping<'a> {
    pub binding: BindingId,
    pub generation: BindingGeneration,
    pub plan: PlanId,
    pub target: &'a str,
}

/// Original output owner lends bytes; it need not be Arc-backed or heap-backed.
/// Drop must perform finite local disposal without network waits.
pub trait ReadOutput {
    fn view(&self) -> InteractionOutputView<'_>;
}

/// Terminal native disposition. Cancelled certifies local native cleanup, not
/// remote cancellation or absence of remote work. A racing value stays owned.
pub enum ReadNativeSettlement<O> {
    Returned(CoreResult<O>),
    Cancelled,
}
/// Cooperative original native work. Drop performs bounded local disposal with
/// no network wait and makes no acknowledgement or remote-effect claim. Errors
/// during cancellation retain the complete work for another paid cleanup poll.
pub trait ReadNativeCall {
    type Output: ReadOutput;
    fn poll_result(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<Self::Output>>;
    fn start_cancel(
        &mut self,
        cause: &CoreError,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Poll<ReadNativeSettlement<Self::Output>>>;
    fn poll_cancel(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<ReadNativeSettlement<Self::Output>>>;
}

/// Concrete source-free Binding facet for this finite slice. Passive metadata
/// is bounded and stable; support checks are pure. No callback receives logical
/// authority. All partially constructed work is disposed on start refusal.
pub trait ReadBinding {
    type Program;
    type Configuration: Eq;
    type Call: ReadNativeCall;
    fn configuration(&self) -> &Self::Configuration;
    fn mapping<'a>(&'a self, program: &'a Self::Program) -> ReadMapping<'a>;
    fn supports(&self, program: &Self::Program, meaning: ValueExecution<'_>, media: &str) -> bool;
    /// Complete Engine-retained indirect backing beyond inline Work/Call
    /// layouts, including output/retirement opportunities. Binding-private
    /// socket/SDK resources keep their native owner's separate bounded policy.
    fn backing_bytes(&self) -> u64;
    fn start(&self, program: &Self::Program, deadline: Deadline) -> CoreResult<Self::Call>;
}

/// Actual bounded media/schema service. Constraint meaning is Core's shared
/// PreparedSchema; decoding stays with the actual codec. Support is complete,
/// not merely outer type recognition. No source TD or callback is retained.
/// Callback scratch is bounded caller/service storage, not an Engine allocation;
/// validation retains no payload and acquires no unreserved Engine backing.
pub trait ReadValidator {
    fn content_type(&self) -> &str;
    fn supports(&self, schema: &PreparedSchema) -> bool;
    fn body_bytes(&self) -> usize;
    fn work(&self) -> u64;
    fn available(&self) -> bool {
        true
    }
    fn validate(&self, schema: &PreparedSchema, payload: PayloadView<'_>) -> CoreResult<()>;
}

/// Fresh destination and original-instance permission, independent of NoSec.
/// Mutable dependencies advance revision without reuse, including in callbacks.
pub trait ReadAccess {
    fn revision(&self) -> u64 {
        0
    }
    fn work(&self) -> u64;
    fn authorize(&self, consumer: LocalConsumerRef<'_>, mapping: ReadMapping<'_>)
    -> CoreResult<()>;
}
