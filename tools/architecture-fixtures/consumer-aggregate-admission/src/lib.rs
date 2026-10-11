//! Pre-code composition discriminator. This is not the aggregate implementation.
#![no_std]

use clinkz_wot_core::*;
use clinkz_wot_foundation::{ResourceKind, ResourceReservation, WorkBudget};
use clinkz_wot_td::{ValidatedPropertyReadCursor, ValidatedThing};
use core::alloc::Layout;

mod private {
    pub trait Sealed {}
}

/// Only a production Core checked complete owner can enter this fixture adapter.
/// An open compiler/component does not satisfy this bound:
/// ```compile_fail
/// use consumer_aggregate_admission_probe::Registration;
/// use clinkz_wot_core::ResolvedTargetCompiler;
/// fn eligible<R: Registration>(_: &R) {}
/// eligible(&ResolvedTargetCompiler::try_new(64).unwrap());
/// ```
pub trait Registration: private::Sealed {
    type Cursor;
    type Artifact: 'static;
    fn identity(&self) -> BindingRegistrationIdentity;
    fn support(&self) -> &ConsumerCompilerSupport;
    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds>;
    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor>;
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact>;
    fn abort(&self, cursor: Self::Cursor);
}

impl<B: PollServerBinding> private::Sealed
    for ConsumerCompilerRegistration<StaticBindingRegistration<B>>
where
    B::Compiler: 'static,
    <B::Compiler as BindingCompilerExtension>::Artifact: 'static,
{
}

impl<B: PollServerBinding> Registration
    for ConsumerCompilerRegistration<StaticBindingRegistration<B>>
where
    B::Compiler: 'static,
    <B::Compiler as BindingCompilerExtension>::Artifact: 'static,
{
    type Cursor = <B::Compiler as BindingCompilerExtension>::Cursor;
    type Artifact = <B::Compiler as BindingCompilerExtension>::Artifact;
    fn identity(&self) -> BindingRegistrationIdentity {
        self.registration().identity()
    }
    fn support(&self) -> &ConsumerCompilerSupport {
        self.support()
    }
    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.registration().compiler().compiler().bounds(input)
    }
    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        self.registration().compiler().compiler().start(input)
    }
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        self.registration()
            .compiler()
            .compiler()
            .step(input, cursor, budget)
    }
    fn abort(&self, cursor: Self::Cursor) {
        self.registration().compiler().compiler().abort(cursor)
    }
}

#[cfg(feature = "std")]
impl private::Sealed for ConsumerCompilerRegistration<HostBindingRegistration> {}
#[cfg(feature = "std")]
impl Registration for ConsumerCompilerRegistration<HostBindingRegistration> {
    type Cursor = HostBindingCompilerCursor;
    type Artifact = HostBindingArtifact;
    fn identity(&self) -> BindingRegistrationIdentity {
        self.registration().identity()
    }
    fn support(&self) -> &ConsumerCompilerSupport {
        self.support()
    }
    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.registration().compiler().bounds(input)
    }
    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        self.registration().compiler().start(input)
    }
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        self.registration().compiler().step(input, cursor, budget)
    }
    fn abort(&self, cursor: Self::Cursor) {
        assert!(self.registration().compiler().abort(cursor).is_ok());
    }
}

/// Fixed inline destination. The caller retains the committed account charge.
/// This tests reservation loan termination, not variable aggregate reclamation.
pub struct Slot<A: 'static> {
    pending: Option<A>,
    _charge: (
        (
            clinkz_wot_foundation::SlotIndex,
            clinkz_wot_foundation::Generation,
        ),
        u64,
    ),
}
impl<A: 'static> Slot<A> {
    pub fn try_new(capacity: ResourceReservation<'_>) -> Result<Self, ResourceReservation<'_>> {
        if capacity.kind() != ResourceKind::CleanupBytesMax
            || capacity.bytes() < Layout::new::<Self>().size() as u64
        {
            return Err(capacity);
        }
        let charge = (capacity.owner(), capacity.bytes());
        capacity.commit();
        Ok(Self {
            pending: None,
            _charge: charge,
        })
    }
    pub fn take(&mut self) -> Option<A> {
        self.pending.take()
    }
}

/// Real input/registration loans and a fixed, source-free abandonment destination.
/// Raw Thing input is rejected:
/// ```compile_fail
/// use consumer_aggregate_admission_probe::{InputOwner, Registration, Slot};
/// use clinkz_wot_td::thing::Thing;
/// fn invalid<R: Registration>(source: &Thing, r: &R, slot: &mut Slot<R::Artifact>) {
///     let _ = InputOwner::new(source, r, slot);
/// }
/// ```
/// Pending cannot outlive its source:
/// ```compile_fail
/// use consumer_aggregate_admission_probe::{InputOwner, Registration, Slot};
/// use clinkz_wot_td::{thing::Thing, ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress};
/// use clinkz_wot_foundation::{AdmissionLedger, WorkBudget};
/// fn invalid<R: Registration>(source: Thing, config: &ValidatedThingAdmissionConfig,
///     ledger: AdmissionLedger, r: &R, slot: &mut Slot<R::Artifact>) {
///     let proof = match ValidatedThingCursor::from_thing(&source, config, ledger).step(&mut WorkBudget::new(), false) {
///         ValidatedThingProgress::Complete(proof) => proof, _ => return,
///     };
///     let owner = InputOwner::new(proof, r, slot);
///     drop(source);
///     drop(owner);
/// }
/// ```
/// Pending retains the complete-registration loan:
/// ```compile_fail
/// use consumer_aggregate_admission_probe::{InputOwner, Registration, Slot};
/// use clinkz_wot_td::ValidatedThing;
/// fn invalid<R: Registration>(proof: ValidatedThing<'_>, r: R, slot: &mut Slot<R::Artifact>) {
///     let owner = InputOwner::new(proof, &r, slot);
///     drop(r);
///     drop(owner);
/// }
/// ```
pub struct InputOwner<'td, 'environment, R: Registration> {
    pub td: ValidatedPropertyReadCursor<'td>,
    pub registration: &'environment R,
    cleanup: &'environment mut Slot<R::Artifact>,
    output: Option<R::Artifact>,
}
/// The accepted consumed outcome shape: only Pending keeps input loans.
pub enum Outcome<'td, 'environment, R: Registration> {
    Pending(InputOwner<'td, 'environment, R>),
    Complete(R::Artifact),
    Failed { cleanup: Option<R::Artifact> },
}
impl<'td, 'environment, R: Registration> InputOwner<'td, 'environment, R> {
    pub fn new(
        td: ValidatedThing<'td>,
        registration: &'environment R,
        cleanup: &'environment mut Slot<R::Artifact>,
    ) -> Self {
        assert!(cleanup.pending.is_none());
        Self {
            td: td.into_property_read(),
            registration,
            cleanup,
            output: None,
        }
    }
    pub fn retain(&mut self, artifact: R::Artifact) {
        assert!(self.output.is_none());
        self.output = Some(artifact);
    }
    pub fn finish(mut self) -> Outcome<'td, 'environment, R> {
        let output = self.output.take().unwrap();
        drop(self); // Ends the actual TD scratch and complete-registration loans.
        Outcome::Complete(output)
    }
    pub fn fail(mut self) -> Outcome<'td, 'environment, R> {
        let cleanup = self.output.take();
        drop(self);
        Outcome::Failed { cleanup }
    }
}
impl<R: Registration> Drop for InputOwner<'_, '_, R> {
    fn drop(&mut self) {
        if let Some(output) = self.output.take() {
            assert!(self.cleanup.pending.is_none());
            self.cleanup.pending = Some(output);
        }
    }
}

/// The narrow eligibility rule; malformed bounds are fixture inputs, not a
/// claim that Core's closed primitive emits them. No compiler callback here.
pub fn compiler_envelope(
    bounds: &BindingCompilerBounds,
    coordinates: u64,
    per_step: u64,
) -> Option<u64> {
    let units = bounds
        .work()
        .remaining(clinkz_wot_foundation::WorkClass::BindingPolls);
    if units == 0
        || units > per_step
        || clinkz_wot_foundation::WorkClass::ALL
            .into_iter()
            .any(|class| {
                class != clinkz_wot_foundation::WorkClass::BindingPolls
                    && bounds.work().remaining(class) != 0
            })
    {
        return None;
    }
    coordinates.checked_mul(per_step)
}
