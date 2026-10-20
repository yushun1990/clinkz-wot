//! Unpublished, paid, all-readable Consumer preparation.
//!
//! The slot is the external manual cleanup root. Its inline constructor does
//! not allocate. Before variable output, a checked fallible backing block is
//! installed under that root. Terminal handles share only this source-free
//! block; their Drop returns its exclusive cleanup lease to the root.

use alloc::{
    alloc::{alloc, dealloc},
    boxed::Box,
    string::String,
    vec::Vec,
};
use clinkz_wot_core::*;
use clinkz_wot_foundation::{
    AdmissionLedger, Generation, ResourceKind as K, ResourceLimits, ResourceReservation, SlotIndex,
    WorkBudget, WorkClass as W,
};
use clinkz_wot_td::data_type::Operation;
use clinkz_wot_td::{
    ValidatedPropertyReadCursor, ValidatedPropertyReadEvent as Event, ValidatedPropertyReadForm,
    ValidatedPropertyReadStep as TdStep, ValidatedThing,
};

fn td_error(cause: clinkz_wot_td::ValidatedThingCause) -> CoreError {
    use clinkz_wot_td::{ValidatedThingCause as C, ValidatedThingFailureKind as F};
    match cause {
        C::Limit(l) => limit(l.kind(), l.configured(), l.observed()),
        C::Cancelled { .. } => cancelled(),
        C::Failed(f) if f.kind() == F::AllocationFailed => CoreError::Backpressure(context()),
        _ => invalid(),
    }
}
use core::{
    alloc::Layout,
    cell::{Cell, UnsafeCell},
    marker::PhantomData,
    mem::{ManuallyDrop, size_of},
    ptr::NonNull,
};

mod private {
    pub trait Sealed {}
}

/// Aggregate eligibility is issued only by Core's checked complete owner.
/// ```compile_fail
/// use clinkz_wot_planning::ConsumerPropertyReadRegistration;
/// use clinkz_wot_core::ResolvedTargetCompiler;
/// fn eligible<R: ConsumerPropertyReadRegistration>(_: &R) {}
/// eligible(&ResolvedTargetCompiler::try_new(64).unwrap());
/// ```
pub trait ConsumerPropertyReadRegistration: private::Sealed {
    type Artifact: 'static;
    #[doc(hidden)]
    type Cursor;
    #[doc(hidden)]
    fn identity(&self) -> BindingRegistrationIdentity;
    #[doc(hidden)]
    fn support(&self) -> &ConsumerCompilerSupport;
    #[doc(hidden)]
    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds>;
    #[doc(hidden)]
    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor>;
    #[doc(hidden)]
    fn progress(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact>;
    #[doc(hidden)]
    fn abort(&self, cursor: Self::Cursor);
}

impl<B: PollServerBinding> private::Sealed
    for ConsumerCompilerRegistration<StaticBindingRegistration<B>>
where
    B::Compiler: 'static,
    <B::Compiler as BindingCompilerExtension>::Artifact: 'static,
{
}
impl<B: PollServerBinding> ConsumerPropertyReadRegistration
    for ConsumerCompilerRegistration<StaticBindingRegistration<B>>
where
    B::Compiler: 'static,
    <B::Compiler as BindingCompilerExtension>::Artifact: 'static,
{
    type Artifact = <B::Compiler as BindingCompilerExtension>::Artifact;
    type Cursor = <B::Compiler as BindingCompilerExtension>::Cursor;
    fn identity(&self) -> BindingRegistrationIdentity {
        self.registration().identity()
    }
    fn support(&self) -> &ConsumerCompilerSupport {
        self.support()
    }
    fn bounds(&self, i: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.registration().compiler().compiler().bounds(i)
    }
    fn start(&self, i: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        self.registration().compiler().compiler().start(i)
    }
    fn progress(
        &self,
        i: &BindingCompilerInput<'_>,
        c: Self::Cursor,
        b: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        self.registration().compiler().compiler().step(i, c, b)
    }
    fn abort(&self, c: Self::Cursor) {
        self.registration().compiler().compiler().abort(c)
    }
}
#[cfg(feature = "std")]
impl private::Sealed for ConsumerCompilerRegistration<HostBindingRegistration> {}
#[cfg(feature = "std")]
impl ConsumerPropertyReadRegistration for ConsumerCompilerRegistration<HostBindingRegistration> {
    type Artifact = HostBindingArtifact;
    type Cursor = HostBindingCompilerCursor;
    fn identity(&self) -> BindingRegistrationIdentity {
        self.registration().identity()
    }
    fn support(&self) -> &ConsumerCompilerSupport {
        self.support()
    }
    fn bounds(&self, i: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.registration().compiler().bounds(i)
    }
    fn start(&self, i: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        self.registration().compiler().start(i)
    }
    fn progress(
        &self,
        i: &BindingCompilerInput<'_>,
        c: Self::Cursor,
        b: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        self.registration().compiler().step(i, c, b)
    }
    fn abort(&self, c: Self::Cursor) {
        assert!(self.registration().compiler().abort(c).is_ok());
    }
}

impl<C: PollClientBinding> private::Sealed
    for ConsumerCompilerRegistration<StaticConsumerBindingRegistration<C>>
where
    C::Compiler: 'static,
    <C::Compiler as BindingCompilerExtension>::Artifact: 'static,
{
}
impl<C: PollClientBinding> ConsumerPropertyReadRegistration
    for ConsumerCompilerRegistration<StaticConsumerBindingRegistration<C>>
where
    C::Compiler: 'static,
    <C::Compiler as BindingCompilerExtension>::Artifact: 'static,
{
    type Artifact = <C::Compiler as BindingCompilerExtension>::Artifact;
    type Cursor = <C::Compiler as BindingCompilerExtension>::Cursor;
    fn identity(&self) -> BindingRegistrationIdentity {
        self.registration().identity()
    }
    fn support(&self) -> &ConsumerCompilerSupport {
        self.support()
    }
    fn bounds(&self, i: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.registration().compiler().compiler().bounds(i)
    }
    fn start(&self, i: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        self.registration().compiler().compiler().start(i)
    }
    fn progress(
        &self,
        i: &BindingCompilerInput<'_>,
        c: Self::Cursor,
        b: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        self.registration().compiler().compiler().step(i, c, b)
    }
    fn abort(&self, c: Self::Cursor) {
        self.registration().compiler().compiler().abort(c)
    }
}
#[cfg(feature = "std")]
impl private::Sealed for ConsumerCompilerRegistration<HostConsumerBindingRegistration> {}
#[cfg(feature = "std")]
impl ConsumerPropertyReadRegistration
    for ConsumerCompilerRegistration<HostConsumerBindingRegistration>
{
    type Artifact = HostBindingArtifact;
    type Cursor = HostBindingCompilerCursor;
    fn identity(&self) -> BindingRegistrationIdentity {
        self.registration().identity()
    }
    fn support(&self) -> &ConsumerCompilerSupport {
        self.support()
    }
    fn bounds(&self, i: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.registration().compiler().bounds(i)
    }
    fn start(&self, i: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        self.registration().compiler().start(i)
    }
    fn progress(
        &self,
        i: &BindingCompilerInput<'_>,
        c: Self::Cursor,
        b: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        self.registration().compiler().step(i, c, b)
    }
    fn abort(&self, c: Self::Cursor) {
        assert!(self.registration().compiler().abort(c).is_ok());
    }
}

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
        .with_operation(Operation::ReadProperty)
}
fn invalid() -> CoreError {
    CoreError::Validation(context())
}
fn invariant() -> CoreError {
    CoreError::InternalInvariant(context())
}
fn cancelled() -> CoreError {
    CoreError::Cancelled(context())
}
fn unavailable() -> CoreError {
    // A capped child ledger deliberately exposes no ceiling/provenance getter.
    // Its rejection must not fabricate an exceeded copied policy limit.
    CoreError::Backpressure(
        context().with_redacted_cause(331, "capped aggregate capacity unavailable"),
    )
}
fn limit(k: K, max: u64, requested: u64) -> CoreError {
    CoreError::LimitExceeded {
        resource: k,
        limit: max,
        requested: Some(requested),
        observed: None,
        context: context(),
    }
}
fn add(a: u64, b: u64) -> CoreResult<u64> {
    a.checked_add(b).ok_or_else(invalid)
}
fn mul(a: u64, b: u64) -> CoreResult<u64> {
    a.checked_mul(b).ok_or_else(invalid)
}
fn layout<T>(n: u64) -> CoreResult<Layout> {
    Layout::array::<T>(usize::try_from(n).map_err(|_| invalid())?).map_err(|_| invalid())
}

const FIELDS: [K; 27] = [
    K::DocumentBytesMax,
    K::AffordancesPerThingMax,
    K::FormsPerContextMax,
    K::FormsPerThingMax,
    K::FormBindingCandidatesPerOperationMax,
    K::CompiledPlanBytesMax,
    K::LogicalPlanBytesPerThingMax,
    K::CompiledRuntimeBytesPerThingMax,
    K::CompiledRuntimeBytesGlobalMax,
    K::BindingArtifactsPerThingMax,
    K::BindingArtifactsGlobalMax,
    K::BindingArtifactBytesPerItemMax,
    K::BindingArtifactBytesPerThingMax,
    K::BindingArtifactBytesGlobalMax,
    K::BindingCompilerCursorBytesPerItemMax,
    K::BindingCompilerCursorBytesGlobalMax,
    K::AdmissionTemporaryBytesPerOperationMax,
    K::AdmissionTemporaryBytesGlobalMax,
    K::PlanCompileWorkUnitsPerStepMax,
    K::PlanReclaimBytesPerStepMax,
    K::CleanupWorkItemsPerStepMax,
    K::CleanupBytesMax,
    K::UriTemplateSourceBytesMax,
    K::PeakLiveBytesPerAdmissionMax,
    K::AdmissionPeakLiveBytesGlobalMax,
    K::EngineLiveBytesGlobalMax,
    K::LargestContiguousAllocationBytesMax,
];

/// Fixed checked projection of existing limits; it does not certify compilers.
#[derive(Clone, Copy, Debug)]
pub struct ConsumerPropertyReadBuildConfig {
    values: [u64; 27],
}
impl ConsumerPropertyReadBuildConfig {
    pub fn try_from_limits(limits: &ResourceLimits) -> CoreResult<Self> {
        let mut values = [0; 27];
        for (out, k) in values.iter_mut().zip(FIELDS) {
            *out = limits.get(k).ok_or_else(invalid)?;
        }
        let c = Self { values };
        // Check lifetime products before any source or compiler progress.
        mul(
            c.get(K::FormsPerThingMax),
            c.get(K::PlanCompileWorkUnitsPerStepMax),
        )?;
        add(
            c.get(K::CompiledRuntimeBytesPerThingMax),
            c.get(K::AdmissionTemporaryBytesPerOperationMax),
        )?;
        mul(
            add(c.get(K::AffordancesPerThingMax), c.get(K::FormsPerThingMax))?,
            add(
                c.get(K::DocumentBytesMax),
                c.get(K::UriTemplateSourceBytesMax),
            )?,
        )?;
        Ok(c)
    }
    fn get(&self, k: K) -> u64 {
        self.values[FIELDS
            .iter()
            .position(|&v| v == k)
            .expect("slice projection")]
    }
    fn check(&self, k: K, n: u64) -> CoreResult<()> {
        let max = self.get(k);
        if n > max {
            Err(limit(k, max, n))
        } else {
            Ok(())
        }
    }
}

/// Disjoint measured storage categories. `largest_allocation` is a real
/// checked allocator request, never the sum of an output allowance.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlanFootprint {
    pub owned_text_bytes: u64,
    pub owned_program_bytes: u64,
    pub property_table_bytes: u64,
    pub coordinate_table_bytes: u64,
    pub logical_owner_bytes: u64,
    pub candidate_and_reference_bytes: u64,
    pub artifact_bytes: u64,
    pub fixed_owner_bytes: u64,
    pub reclaim_metadata_bytes: u64,
    pub compiler_cursor_bytes: u64,
    pub compiler_temporary_bytes: u64,
    /// Conservative source-level construction/result transport overlap, not a
    /// measurement of a compiler-generated machine stack frame.
    pub planning_temporary_bytes: u64,
    pub build_owner_bytes: u64,
    pub retained_source_bytes: u64,
    pub lazy_bytes: u64,
    pub registration_delta_bytes: u64,
    pub largest_allocation: u64,
    pub peak_live_bytes: u64,
}
impl PlanFootprint {
    pub fn retained_bytes(&self) -> u64 {
        self.owned_text_bytes
            + self.owned_program_bytes
            + self.property_table_bytes
            + self.coordinate_table_bytes
            + self.logical_owner_bytes
            + self.candidate_and_reference_bytes
            + self.artifact_bytes
            + self.fixed_owner_bytes
            + self.reclaim_metadata_bytes
    }
}

#[derive(Clone, Copy, Debug)]
struct Cost {
    items: u64,
    bytes: u64,
    cleanup: u64,
    binding: u64,
    reclaim: u64,
    resource: K,
    configured: u64,
}
impl Default for Cost {
    fn default() -> Self {
        Self {
            items: 0,
            bytes: 0,
            cleanup: 0,
            binding: 0,
            reclaim: 0,
            resource: K::CompiledPlanBytesMax,
            configured: 0,
        }
    }
}
impl Cost {
    fn bounded(mut self, resource: K, configured: u64) -> Self {
        self.resource = resource;
        self.configured = configured;
        self
    }
    fn pairs(self) -> [(W, u64); 5] {
        [
            (W::PlanningItems, self.items),
            (W::CodecOutputBytes, self.bytes),
            (W::CleanupItems, self.cleanup),
            (W::BindingPolls, self.binding),
            (W::CodecInputBytes, self.reclaim),
        ]
    }
    fn debit(&mut self, b: &mut WorkBudget, c: Self) -> CoreResult<bool> {
        let exhausted = || CoreError::LimitExceeded {
            resource: self.resource,
            limit: self.configured,
            requested: None,
            observed: None,
            context: context().with_redacted_cause(330, "derived Planning work envelope exhausted"),
        };
        let next = Self {
            items: self.items.checked_sub(c.items).ok_or_else(exhausted)?,
            bytes: self.bytes.checked_sub(c.bytes).ok_or_else(exhausted)?,
            cleanup: self.cleanup.checked_sub(c.cleanup).ok_or_else(exhausted)?,
            binding: self.binding.checked_sub(c.binding).ok_or_else(exhausted)?,
            reclaim: self.reclaim.checked_sub(c.reclaim).ok_or_else(exhausted)?,
            resource: self.resource,
            configured: self.configured,
        };
        if c.pairs().into_iter().any(|(w, n)| b.remaining(w) < n) {
            return Ok(false);
        }
        for (w, n) in c.pairs() {
            b.consume(w, n).unwrap();
        }
        *self = next;
        Ok(true)
    }
}

/// Once-emitted requirements remain associated with their unique waiting build.
#[derive(Clone, Copy, Debug)]
pub struct ConsumerPropertyReadPreflight {
    pub property_count: u64,
    pub coordinate_count: u64,
    pub registration: BindingRegistrationIdentity,
    pub output_capacity_bytes: u64,
    pub cursor_capacity_bytes: u64,
    pub temporary_capacity_bytes: u64,
    pub cleanup_capacity_bytes: u64,
    pub remaining_planning_items: u64,
    pub remaining_copy_bytes: u64,
    pub compiler_work_max: u64,
}

struct PropertyRow {
    name: Box<str>,
    start: usize,
    end: usize,
}
#[derive(Default)]
struct Parts {
    id: Option<String>,
    name: Option<Box<str>>,
    target: Option<Box<str>>,
    content: Option<Box<str>>,
    protocol: Option<Box<str>>,
}
enum ArtifactOwner<A> {
    Empty,
    Ready(BindingArtifactEnvelope<A>),
    Rejected(BindingArtifact<A>),
}
impl<A> ArtifactOwner<A> {
    fn as_ref(&self) -> Option<&BindingArtifactEnvelope<A>> {
        match self {
            Self::Ready(a) => Some(a),
            _ => None,
        }
    }
}
struct Coordinate<A> {
    plan: Option<LogicalInteractionPlan>,
    parts: Parts,
    raw: Option<Box<str>>,
    coding: Option<Box<str>>,
    security: Option<Box<str>>,
    scopes: Vec<Box<str>>,
    candidate: BindingCandidate,
    reference: BindingArtifactRef,
    bound: Option<BindingCompilerBounds>,
    artifact: ArtifactOwner<A>,
    unsupported: bool,
    scope_bytes: u64,
    scope_capacity_bytes: u64,
    backing_bytes: u64,
}
impl<A> Coordinate<A> {
    fn new(
        candidate: BindingCandidate,
        identity: BindingArtifactIdentity,
        slot: SlotIndex,
    ) -> Self {
        Self {
            plan: None,
            parts: Parts::default(),
            raw: None,
            coding: None,
            security: None,
            scopes: Vec::new(),
            candidate,
            reference: BindingArtifactRef::new(identity, slot),
            bound: None,
            artifact: ArtifactOwner::Empty,
            unsupported: false,
            scope_bytes: 0,
            scope_capacity_bytes: 0,
            backing_bytes: 0,
        }
    }
}
#[derive(Default)]
struct Reclaim {
    bytes_paid: u64,
    allowance: Cost,
}
struct Data<A: 'static> {
    registration: BindingRegistrationIdentity,
    rows: Vec<PropertyRow>,
    coordinates: Vec<Coordinate<A>>,
    runtime: AdmissionLedger,
    build: Option<AdmissionLedger>,
    runtime_charge: u64,
    temporary_charge: u64,
    config: ConsumerPropertyReadBuildConfig,
    footprint: PlanFootprint,
    cause: CoreError,
    reclaim: Reclaim,
    material: Cost,
    bounds: Cost,
    compile: Cost,
    seal: Cost,
    node_bytes: u64,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum RootState {
    Building,
    Draft,
    Rollback,
    Available,
    Terminal,
}
struct Node<A: 'static> {
    references: Cell<u8>,
    state: Cell<RootState>,
    data: UnsafeCell<ManuallyDrop<Data<A>>>,
}
struct Root<A: 'static> {
    pointer: NonNull<Node<A>>,
    _local: PhantomData<alloc::rc::Rc<()>>,
}
// Safety invariant: the slot and exactly one public state-selected lease own
// this block (a consuming transfer may briefly hold a third reference). No
// public handle can clone a lease. The slot touches only Cells while a lease
// exists; only that lease may access Data, and Draft exposes shared loans only.
// All handles are local/non-Send. Terminal release follows explicit draining,
// with no initialized collection elements or capacities left in the block.
// The std terminal bridge may consume both references and own this same block
// uniquely. While detached, no Root exists; reattachment restores the pair
// before exposing any local handle or data loan.
impl<A: 'static> Root<A> {
    fn node(&self) -> &Node<A> {
        unsafe { self.pointer.as_ref() }
    }
    fn share(&self) -> Self {
        let n = self.node();
        let count = n.references.get();
        assert!(count < 3);
        n.references.set(count + 1);
        Self {
            pointer: self.pointer,
            _local: PhantomData,
        }
    }
    fn data(&self) -> &Data<A> {
        unsafe { &*self.node().data.get() }
    }
    // Only the state-selected build/draft/rollback lease accesses output. The
    // slot owns backing but never mutates output under an outstanding lease.
    fn data_mut(&mut self) -> &mut Data<A> {
        unsafe { &mut *self.node().data.get() }
    }
}
impl<A: 'static> Drop for Root<A> {
    fn drop(&mut self) {
        let node = self.node();
        let count = node.references.get();
        node.references.set(count - 1);
        if count != 1 {
            return;
        }
        // The advanced caller must retain and drive its manual root. Dropping
        // that root is no completion claim and cannot run variable destruction.
        if node.state.get() != RootState::Terminal {
            return;
        }
        let data = unsafe { &*self.pointer.as_ref().data.get() };
        debug_assert!(
            data.rows.is_empty()
                && data.rows.capacity() == 0
                && data.coordinates.is_empty()
                && data.coordinates.capacity() == 0
        );
        let mut runtime = unsafe { core::ptr::read(&data.runtime) };
        let bytes = data.node_bytes;
        unsafe {
            dealloc(self.pointer.as_ptr().cast(), Layout::new::<Node<A>>());
        }
        assert!(runtime.release_persistent_runtime(bytes));
    }
}

/// Reserved inline manual root. Retain it and its committed account until all
/// returned handles have been reclaimed and `take()` observes terminal state.
pub struct ConsumerPropertyReadCleanupSlot<A: 'static> {
    root: Option<Root<A>>,
    fixed: Option<FixedCleanup>,
    owner: (SlotIndex, Generation),
    bytes: u64,
}
impl<A: 'static> ConsumerPropertyReadCleanupSlot<A> {
    /// Inline destination plus conservative fixed cleanup transport. This is a
    /// logical allowance; it is never reported as a contiguous allocation.
    pub fn required_capacity() -> u64 {
        (size_of::<Self>() as u64)
            .checked_add(size_of::<(
                Coordinate<A>,
                PropertyRow,
                ConsumerPropertyReadRollbackStep<A>,
                AdmissionLedger,
            )>() as u64)
            .expect("fixed Rust Layout sum")
    }
    pub fn try_new<'a>(
        config: &ConsumerPropertyReadBuildConfig,
        capacity: ResourceReservation<'a>,
    ) -> Result<Self, (ResourceReservation<'a>, CoreError)> {
        if capacity.kind() != K::CleanupBytesMax
            || capacity.bytes() < Self::required_capacity()
            || capacity.bytes() > config.get(K::CleanupBytesMax)
        {
            return Err((capacity, invalid()));
        }
        let owner = capacity.owner();
        let bytes = capacity.bytes();
        capacity.commit();
        Ok(Self {
            root: None,
            fixed: None,
            owner,
            bytes,
        })
    }
    fn refresh(&mut self) {
        if self
            .root
            .as_ref()
            .is_some_and(|r| r.node().state.get() == RootState::Terminal)
        {
            self.root = None;
        }
    }
    pub fn is_vacant(&self) -> bool {
        self.fixed.is_none()
            && self
                .root
                .as_ref()
                .is_none_or(|r| r.node().state.get() == RootState::Terminal)
    }
    pub fn reservation(&self) -> ((SlotIndex, Generation), u64) {
        (self.owner, self.bytes)
    }
    pub fn take(&mut self) -> Option<ConsumerPropertyReadRollback<A>> {
        self.refresh();
        if let Some(fixed) = self.fixed.take() {
            return Some(ConsumerPropertyReadRollback {
                owner: RollbackOwner::Fixed(Some(fixed)),
            });
        }
        let root = self.root.as_ref()?;
        if root.node().state.get() != RootState::Available {
            return None;
        }
        root.node().state.set(RootState::Rollback);
        Some(ConsumerPropertyReadRollback {
            owner: RollbackOwner::Shared(root.share()),
        })
    }
}
struct FixedCleanup {
    cause: CoreError,
    _build: Option<AdmissionLedger>,
    _runtime: Option<AdmissionLedger>,
}
enum RollbackOwner<A: 'static> {
    Fixed(Option<FixedCleanup>),
    Shared(Root<A>),
}
/// Source-free cleanup lease. Dropping it returns the lease to the reserved root.
#[must_use]
pub struct ConsumerPropertyReadRollback<A: 'static> {
    owner: RollbackOwner<A>,
}
impl<A: 'static> Drop for ConsumerPropertyReadRollback<A> {
    fn drop(&mut self) {
        if let RollbackOwner::Shared(r) = &self.owner {
            if r.node().state.get() == RootState::Rollback {
                r.node().state.set(RootState::Available);
            }
        }
    }
}
#[must_use]
pub enum ConsumerPropertyReadRollbackStep<A: 'static> {
    Pending(ConsumerPropertyReadRollback<A>),
    Complete(CoreError),
}

/// Immutable unpublished output. Drop hands cleanup back to its reserved root;
/// identity and footprint are observations, never an installation permit.
pub struct ConsumerPropertyReadDraft<A: 'static> {
    root: Root<A>,
}
impl<A: 'static> ConsumerPropertyReadDraft<A> {
    pub fn registration_identity(&self) -> BindingRegistrationIdentity {
        self.root.data().registration
    }
    pub fn footprint(&self) -> PlanFootprint {
        self.root.data().footprint
    }
    pub fn property_count(&self) -> usize {
        self.root.data().rows.len()
    }
    pub fn coordinate_count(&self) -> usize {
        self.root.data().coordinates.len()
    }
    pub fn into_rollback(self) -> ConsumerPropertyReadRollback<A> {
        self.root.node().state.set(RootState::Rollback);
        ConsumerPropertyReadRollback {
            owner: RollbackOwner::Shared(self.root.share()),
        }
    }
}
impl<A: 'static> Drop for ConsumerPropertyReadDraft<A> {
    fn drop(&mut self) {
        if self.root.node().state.get() == RootState::Draft {
            self.root.node().state.set(RootState::Available);
        }
    }
}

#[cfg(feature = "std")]
struct OwnedRoot<A: 'static> {
    // Suppress even the Box deallocation: the original manual root may only
    // release its backing after the existing budgeted reclaimer is terminal.
    node: ManuallyDrop<Box<Node<A>>>,
    owner: (SlotIndex, Generation),
    bytes: u64,
}

#[cfg(feature = "std")]
impl<A: 'static> OwnedRoot<A> {
    fn prepay(
        lease: &Root<A>,
        slot: &ConsumerPropertyReadCleanupSlot<A>,
        state: RootState,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        if slot.fixed.is_some()
            || slot.root.as_ref().is_none_or(|anchor| {
                anchor.pointer != lease.pointer
                    || lease.node().state.get() != state
                    || lease.node().references.get() != 2
            })
        {
            return Err(CoreError::Validation(
                context().with_redacted_cause(332, "terminal cleanup root mismatch"),
            ));
        }
        // The coordinator supplies fresh fixed conversion credit, separate
        // from the retained Planning/reclaim lifetime allowances in Data.
        budget.consume(W::PlanningItems, 1).map_err(|_| {
            CoreError::Backpressure(
                context()
                    .with_redacted_cause(333, "terminal ownership conversion credit unavailable"),
            )
        })
    }

    // Called only after prepay, with the consumed state-selected lease's Drop
    // suppressed. No operation between validation and detach can add a loan or
    // reference, change phase, run a callback, or fail.
    fn detach(pointer: NonNull<Node<A>>, slot: ConsumerPropertyReadCleanupSlot<A>) -> Self {
        let owner = slot.owner;
        let bytes = slot.bytes;
        core::mem::forget(slot);
        // SAFETY: pointer is the original alloc(Layout::new::<Node<A>>())
        // block. Both of its only two Root owners have now been forgotten.
        // Consuming the lease ends every selection/data loan. Exactly this Box
        // owns the initialized, aligned block; Data remains ManuallyDrop.
        let node = unsafe { Box::from_raw(pointer.as_ptr()) };
        node.references.set(0);
        Self {
            node: ManuallyDrop::new(node),
            owner,
            bytes,
        }
    }

    fn into_local(self) -> (Root<A>, ConsumerPropertyReadCleanupSlot<A>) {
        let node = ManuallyDrop::into_inner(self.node);
        // Unique ownership ends before either local owner can expose a loan.
        // Box::leak/NonNull preserve the original allocation without allocating
        // or running Data's destructor. Root::drop resumes terminal release.
        node.references.set(2);
        let pointer = NonNull::from(Box::leak(node));
        let root = || Root {
            pointer,
            _local: PhantomData,
        };
        (
            root(),
            ConsumerPropertyReadCleanupSlot {
                root: Some(root()),
                fixed: None,
                owner: self.owner,
                bytes: self.bytes,
            },
        )
    }
}

/// Unique source-free owner of a completed draft and its physical manual root.
///
/// This std-only bridge is `Send` when `A: Send`, and is neither Clone nor Sync.
/// Each successful detach prepays one `PlanningItems` credit for the complete
/// detach/reattach pair. Conversions allocate nothing and drive no Planning,
/// compiler or cleanup work. The coordinator must separately retain the
/// original committed cleanup account while any child or backing remains.
///
/// Retain and explicitly drain this advanced manual owner: dropping it with
/// live output leaks the graph, just as dropping the original manual root
/// does. It supplies no publication permit or executor drop recovery.
///
/// The owner cannot be cloned, forged or shared, and a non-Send artifact keeps
/// the complete graph local:
/// ```compile_fail
/// use clinkz_wot_planning::ConsumerPropertyReadOwnedOutput;
/// fn cloneable<T: Clone>() {}
/// cloneable::<ConsumerPropertyReadOwnedOutput<clinkz_wot_core::ResolvedTargetArtifact>>();
/// ```
/// ```compile_fail
/// use clinkz_wot_core::ResolvedTargetArtifact;
/// use clinkz_wot_planning::ConsumerPropertyReadOwnedOutput;
/// let _ = ConsumerPropertyReadOwnedOutput::<ResolvedTargetArtifact> { root: todo!() };
/// ```
/// ```compile_fail
/// use clinkz_wot_planning::ConsumerPropertyReadOwnedOutput;
/// fn shared<T: Sync>() {}
/// shared::<ConsumerPropertyReadOwnedOutput<clinkz_wot_core::ResolvedTargetArtifact>>();
/// ```
/// ```compile_fail
/// use clinkz_wot_planning::ConsumerPropertyReadOwnedOutput;
/// use std::rc::Rc;
/// fn movable<T: Send>() {}
/// movable::<ConsumerPropertyReadOwnedOutput<Rc<()>>>();
/// ```
#[cfg(feature = "std")]
#[must_use = "retain the manual root and explicitly reclaim its output"]
pub struct ConsumerPropertyReadOwnedOutput<A: 'static> {
    root: OwnedRoot<A>,
}

#[cfg(feature = "std")]
impl<A: 'static> ConsumerPropertyReadOwnedOutput<A> {
    /// Consumes the matching local pair, or returns both owners unchanged.
    /// Invalid roots/phases consume no work; short credit returns Backpressure.
    pub fn try_from_local(
        draft: ConsumerPropertyReadDraft<A>,
        root: ConsumerPropertyReadCleanupSlot<A>,
        budget: &mut WorkBudget,
    ) -> Result<
        Self,
        (
            ConsumerPropertyReadDraft<A>,
            ConsumerPropertyReadCleanupSlot<A>,
            CoreError,
        ),
    > {
        if let Err(cause) = OwnedRoot::prepay(&draft.root, &root, RootState::Draft, budget) {
            return Err((draft, root, cause));
        }
        let pointer = draft.root.pointer;
        core::mem::forget(draft);
        Ok(Self {
            root: OwnedRoot::detach(pointer, root),
        })
    }

    /// Restores exactly one matching local draft/root pair using prepaid work.
    pub fn into_local(
        self,
    ) -> (
        ConsumerPropertyReadDraft<A>,
        ConsumerPropertyReadCleanupSlot<A>,
    ) {
        let (root, slot) = self.root.into_local();
        (ConsumerPropertyReadDraft { root }, slot)
    }
}

/// Unique source-free owner of variable rollback and its physical manual root.
///
/// Has the same fixed credit, auto traits, cleanup-account retention and manual
/// draining obligation as [`ConsumerPropertyReadOwnedOutput`]. Partial byte
/// credit, lifetime allowances and first cause remain in the original backing.
/// ```compile_fail
/// use clinkz_wot_planning::ConsumerPropertyReadOwnedRollback;
/// fn shared<T: Sync>() {}
/// shared::<ConsumerPropertyReadOwnedRollback<clinkz_wot_core::ResolvedTargetArtifact>>();
/// ```
/// ```compile_fail
/// use clinkz_wot_planning::ConsumerPropertyReadOwnedRollback;
/// use std::rc::Rc;
/// fn movable<T: Send>() {}
/// movable::<ConsumerPropertyReadOwnedRollback<Rc<()>>>();
/// ```
#[cfg(feature = "std")]
#[must_use = "retain the manual root and explicitly drive rollback to terminal"]
pub struct ConsumerPropertyReadOwnedRollback<A: 'static> {
    root: OwnedRoot<A>,
}

#[cfg(feature = "std")]
impl<A: 'static> ConsumerPropertyReadOwnedRollback<A> {
    /// Consumes the matching variable rollback/root pair. A fixed rollback is
    /// returned unchanged with UnsupportedOperation for the existing local path.
    pub fn try_from_local(
        rollback: ConsumerPropertyReadRollback<A>,
        root: ConsumerPropertyReadCleanupSlot<A>,
        budget: &mut WorkBudget,
    ) -> Result<
        Self,
        (
            ConsumerPropertyReadRollback<A>,
            ConsumerPropertyReadCleanupSlot<A>,
            CoreError,
        ),
    > {
        let RollbackOwner::Shared(lease) = &rollback.owner else {
            return Err((
                rollback,
                root,
                CoreError::UnsupportedOperation(
                    context().with_redacted_cause(334, "fixed rollback uses local cleanup"),
                ),
            ));
        };
        if let Err(cause) = OwnedRoot::prepay(lease, &root, RootState::Rollback, budget) {
            return Err((rollback, root, cause));
        }
        let pointer = lease.pointer;
        core::mem::forget(rollback);
        Ok(Self {
            root: OwnedRoot::detach(pointer, root),
        })
    }

    /// Restores the original rollback lease/root pair without a second charge.
    pub fn into_local(
        self,
    ) -> (
        ConsumerPropertyReadRollback<A>,
        ConsumerPropertyReadCleanupSlot<A>,
    ) {
        let (root, slot) = self.root.into_local();
        (
            ConsumerPropertyReadRollback {
                owner: RollbackOwner::Shared(root),
            },
            slot,
        )
    }
}

/// A short borrow of exactly one sealed coordinate, never another target range.
pub struct ConsumerPropertyReadSelection<'a, A> {
    coordinate: &'a Coordinate<A>,
}
impl<A> ConsumerPropertyReadSelection<'_, A> {
    pub fn plan(&self) -> &LogicalInteractionPlan {
        self.coordinate.plan.as_ref().unwrap()
    }
    pub fn candidate(&self) -> BindingCandidate {
        self.coordinate.candidate
    }
    pub fn artifact(&self) -> &BindingArtifactEnvelope<A> {
        self.coordinate.artifact.as_ref().unwrap()
    }
    pub fn artifact_ref(&self) -> BindingArtifactRef {
        self.coordinate.reference
    }
    pub fn raw_href(&self) -> &str {
        self.coordinate.raw.as_deref().unwrap()
    }
    pub fn content_coding(&self) -> Option<&str> {
        self.coordinate.coding.as_deref()
    }
    pub fn security_name(&self) -> &str {
        self.coordinate.security.as_deref().unwrap()
    }
    pub fn scopes(&self) -> &[Box<str>] {
        &self.coordinate.scopes
    }
}
pub fn select_consumer_property_read_aggregate<'a, A: 'static>(
    draft: &'a ConsumerPropertyReadDraft<A>,
    name: &str,
    options: &InteractionOptions,
) -> CoreResult<ConsumerPropertyReadSelection<'a, A>> {
    let d = draft.root.data();
    let failure = |reason| CoreError::Selection {
        reason,
        context: ErrorContext::new(ErrorPhase::Selection, RetryClass::Never)
            .with_operation(Operation::ReadProperty),
    };
    let index = d
        .rows
        .binary_search_by(|r| r.name.as_ref().cmp(name))
        .map_err(|_| failure(SelectionFailureReason::AffordanceMissing))?;
    let row = &d.rows[index];
    let range = &d.coordinates[row.start..row.end];
    if range.is_empty() {
        return Err(failure(SelectionFailureReason::NoFormSupportsOperation));
    }
    let coordinate = match options.form_index() {
        None => range
            .iter()
            .find(|c| !c.unsupported)
            .ok_or_else(|| CoreError::UnsupportedOperation(context()))?,
        Some(index) => range
            .iter()
            .find(|c| c.plan.as_ref().unwrap().form_index() as usize == index)
            .ok_or_else(|| failure(SelectionFailureReason::StrictSelectionMismatch))?,
    };
    if coordinate.unsupported {
        let plan = coordinate.plan.as_ref().unwrap();
        return Err(CoreError::UnsupportedOperation(
            context()
                .with_form_index(plan.form_index())
                .with_plan(plan.plan_id()),
        ));
    }
    Ok(ConsumerPropertyReadSelection { coordinate })
}

impl<A: 'static> Data<A> {
    fn check_request(&self, bytes: u64) -> CoreResult<u64> {
        self.config
            .check(K::LargestContiguousAllocationBytesMax, bytes)?;
        let next = add(self.runtime_charge, bytes)?;
        for k in [
            K::CompiledPlanBytesMax,
            K::CompiledRuntimeBytesPerThingMax,
            K::CompiledRuntimeBytesGlobalMax,
        ] {
            self.config.check(k, next)?;
        }
        let peak = add(
            next,
            self.build.as_ref().map_or(0, AdmissionLedger::live_bytes),
        )?;
        for k in [
            K::PeakLiveBytesPerAdmissionMax,
            K::AdmissionPeakLiveBytesGlobalMax,
            K::EngineLiveBytesGlobalMax,
        ] {
            self.config.check(k, peak)?;
        }
        Ok(next)
    }
    fn allocate(&mut self, l: Layout) -> CoreResult<NonNull<u8>> {
        let bytes = l.size() as u64;
        let next = self.check_request(bytes)?;
        let reservation = self
            .runtime
            .try_reserve_persistent_runtime(K::CompiledRuntimeBytesPerThingMax, bytes)
            .ok_or_else(unavailable)?;
        self.footprint.largest_allocation = self.footprint.largest_allocation.max(bytes);
        let pointer =
            NonNull::new(unsafe { alloc(l) }).ok_or_else(|| CoreError::Backpressure(context()))?;
        reservation.commit();
        self.runtime_charge = next;
        self.observe();
        Ok(pointer)
    }
    fn observe(&mut self) {
        self.footprint.peak_live_bytes = self.footprint.peak_live_bytes.max(
            self.runtime.live_bytes() + self.build.as_ref().map_or(0, AdmissionLedger::live_bytes),
        );
    }
    fn text(&mut self, text: &str) -> CoreResult<Box<str>> {
        if text.is_empty() {
            return Ok(Box::from(""));
        }
        let pointer = self.allocate(layout::<u8>(text.len() as u64)?)?;
        unsafe {
            core::ptr::copy_nonoverlapping(text.as_ptr(), pointer.as_ptr(), text.len());
            let slice = core::ptr::slice_from_raw_parts_mut(pointer.as_ptr(), text.len());
            self.footprint.owned_text_bytes += text.len() as u64;
            // Source strings are UTF-8; the paid copy preserves every byte.
            Ok(Box::from_raw(slice as *mut str))
        }
    }
    fn table<T>(&mut self, n: u64) -> CoreResult<Vec<T>> {
        if n == 0 {
            return Ok(Vec::new());
        }
        let l = layout::<T>(n)?;
        let pointer = self.allocate(l)?;
        // Exact allocation/capacity, with no allocator-dependent Vec slack or
        // infallible reserve/shrink operation. Every initialized item is pushed
        // only after checking the sealed census capacity.
        Ok(unsafe { Vec::from_raw_parts(pointer.as_ptr().cast::<T>(), 0, n as usize) })
    }
    fn release(&mut self, n: u64) {
        assert!(self.runtime.release_persistent_runtime(n));
        self.runtime_charge -= n;
    }
    fn release_temporary(&mut self) {
        if self.temporary_charge != 0 {
            assert!(
                self.build
                    .as_mut()
                    .unwrap()
                    .release_temporary(self.temporary_charge)
            );
            self.temporary_charge = 0;
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Phase {
    Initialize,
    Preflight,
    Waiting,
    Root,
    Tables,
    Materialize,
    Bounds,
    Compile,
    SealRows,
    SealCoordinates,
}
#[derive(Default)]
struct Census {
    properties: u64,
    forms: u64,
    text: u64,
    scope_tables: u64,
    scopes: u64,
    allocations: u64,
    property_forms: u64,
}

/// Unique consumed owner of the TD continuation and complete registration loan.
/// Raw Thing input cannot substitute for TD admission:
/// ```compile_fail
/// use clinkz_wot_planning::*;
/// use clinkz_wot_core::PlanSetGeneration;
/// use clinkz_wot_foundation::AdmissionLedger;
/// use clinkz_wot_td::thing::Thing;
/// fn invalid<R: ConsumerPropertyReadRegistration>(source: &Thing, r: &R,
///     config: &ConsumerPropertyReadBuildConfig, ledger: AdmissionLedger,
///     slot: &mut ConsumerPropertyReadCleanupSlot<R::Artifact>) {
///     let _ = build_consumer_property_read_aggregate(source, r, config,
///         PlanSetGeneration::INITIAL, ledger, slot);
/// }
/// ```
/// A suspended build retains its source loan:
/// ```compile_fail
/// use clinkz_wot_planning::*;
/// use clinkz_wot_core::PlanSetGeneration;
/// use clinkz_wot_foundation::{AdmissionLedger, WorkBudget};
/// use clinkz_wot_td::{thing::Thing, ValidatedThingAdmissionConfig,
///     ValidatedThingCursor, ValidatedThingProgress};
/// fn invalid<R: ConsumerPropertyReadRegistration>(source: Thing, r: &R,
///     config: &ConsumerPropertyReadBuildConfig, td_config: &ValidatedThingAdmissionConfig,
///     td_ledger: AdmissionLedger, ledger: AdmissionLedger,
///     slot: &mut ConsumerPropertyReadCleanupSlot<R::Artifact>) {
///     let proof = match ValidatedThingCursor::from_thing(&source, td_config, td_ledger)
///         .step(&mut WorkBudget::new(), false) {
///         ValidatedThingProgress::Complete(proof) => proof, _ => return,
///     };
///     let build = build_consumer_property_read_aggregate(proof, r, config,
///         PlanSetGeneration::INITIAL, ledger, slot);
///     drop(source);
///     drop(build);
/// }
/// ```
/// A suspended build retains its complete registration loan:
/// ```compile_fail
/// use clinkz_wot_planning::*;
/// use clinkz_wot_core::PlanSetGeneration;
/// use clinkz_wot_foundation::AdmissionLedger;
/// use clinkz_wot_td::ValidatedThing;
/// fn invalid<R: ConsumerPropertyReadRegistration>(proof: ValidatedThing<'_>, r: R,
///     config: &ConsumerPropertyReadBuildConfig, ledger: AdmissionLedger,
///     slot: &mut ConsumerPropertyReadCleanupSlot<R::Artifact>) {
///     let build = build_consumer_property_read_aggregate(proof, &r, config,
///         PlanSetGeneration::INITIAL, ledger, slot);
///     drop(r);
///     drop(build);
/// }
/// ```
#[must_use]
pub struct ConsumerPropertyReadBuild<'td, 'environment, R: ConsumerPropertyReadRegistration> {
    td: Option<ValidatedPropertyReadCursor<'td>>,
    #[cfg(feature = "std")]
    semantics: Option<clinkz_wot_td::semantic::ResolvedSemanticDocument<'td>>,
    registration: &'environment R,
    cleanup: &'environment mut ConsumerPropertyReadCleanupSlot<R::Artifact>,
    config: ConsumerPropertyReadBuildConfig,
    generation: PlanSetGeneration,
    identity: BindingRegistrationIdentity,
    phase: Phase,
    census: Census,
    preflight: Option<ConsumerPropertyReadPreflight>,
    initial: Option<AdmissionLedger>,
    runtime: Option<AdmissionLedger>,
    fixed_charge: u64,
    planning_temporary: u64,
    allowance: Cost,
    root: Option<Root<R::Artifact>>,
    cursor: Option<R::Cursor>,
    position: usize,
    compiler_remaining: u64,
    host_backing: u64,
}
#[must_use]
pub enum ConsumerPropertyReadBuildStep<'td, 'environment, R: ConsumerPropertyReadRegistration> {
    Pending(ConsumerPropertyReadBuild<'td, 'environment, R>),
    Preflight {
        build: ConsumerPropertyReadBuild<'td, 'environment, R>,
        requirements: ConsumerPropertyReadPreflight,
    },
    Complete(ConsumerPropertyReadDraft<R::Artifact>),
    Failed {
        cause: CoreError,
        cleanup: Option<ConsumerPropertyReadRollback<R::Artifact>>,
    },
}

pub fn build_consumer_property_read_aggregate<
    'td,
    'environment,
    R: ConsumerPropertyReadRegistration,
>(
    td: ValidatedThing<'td>,
    registration: &'environment R,
    config: &ConsumerPropertyReadBuildConfig,
    generation: PlanSetGeneration,
    ledger: AdmissionLedger,
    cleanup: &'environment mut ConsumerPropertyReadCleanupSlot<R::Artifact>,
) -> CoreResult<ConsumerPropertyReadBuild<'td, 'environment, R>> {
    cleanup.refresh();
    if !cleanup.is_vacant() {
        return Err(CoreError::Backpressure(context()));
    }
    config.check(K::CleanupBytesMax, cleanup.bytes)?;
    if config.get(K::FormBindingCandidatesPerOperationMax) == 0
        || config.get(K::PlanReclaimBytesPerStepMax) == 0
        || config.get(K::CleanupWorkItemsPerStepMax) == 0
        || config.get(K::PlanCompileWorkUnitsPerStepMax) == 0
    {
        return Err(CoreError::UnsupportedOperation(context()));
    }
    let support = registration.support();
    config.check(
        K::BindingArtifactBytesPerItemMax,
        support.artifact_layout().size() as u64,
    )?;
    config.check(
        K::BindingCompilerCursorBytesPerItemMax,
        support.cursor_layout().size() as u64,
    )?;
    config.check(
        K::BindingCompilerCursorBytesGlobalMax,
        support.cursor_layout().size() as u64,
    )?;
    let planning_temporary = size_of::<(
        Data<R::Artifact>,
        ConsumerPropertyReadBuildStep<'td, 'environment, R>,
        PropertyRow,
        Coordinate<R::Artifact>,
        ConsumerPropertyReadPreflight,
        CoreError,
    )>() as u64;
    let fixed_charge = add(
        size_of::<ConsumerPropertyReadBuild<'td, 'environment, R>>() as u64,
        planning_temporary,
    )?;
    #[cfg(feature = "std")]
    let fixed_charge = if support.uses_semantics() {
        let bytes = clinkz_wot_td::semantic::ResolvedSemanticDocument::capacity_bytes();
        config.check(K::LargestContiguousAllocationBytesMax, bytes)?;
        add(fixed_charge, bytes)?
    } else {
        fixed_charge
    };
    let temporary = add(fixed_charge, support.temporary_layout().size() as u64)?;
    config.check(K::AdmissionTemporaryBytesPerOperationMax, temporary)?;
    config.check(K::AdmissionTemporaryBytesGlobalMax, temporary)?;
    // Generic table Layout arithmetic is fixed, not a source scan/allocation.
    layout::<PropertyRow>(config.get(K::AffordancesPerThingMax))?;
    layout::<Coordinate<R::Artifact>>(config.get(K::FormsPerThingMax))?;
    let items = add(
        add(
            config.get(K::AffordancesPerThingMax),
            config.get(K::FormsPerThingMax),
        )?,
        3,
    )?;
    #[cfg(feature = "std")]
    let host_backing = support.host_slot_layout().map_or(0, |l| l.size() as u64);
    #[cfg(not(feature = "std"))]
    let host_backing = 0;
    Ok(ConsumerPropertyReadBuild {
        td: Some(td.into_property_read()),
        #[cfg(feature = "std")]
        semantics: None,
        registration,
        cleanup,
        config: *config,
        generation,
        identity: registration.identity(),
        phase: Phase::Initialize,
        census: Census::default(),
        preflight: None,
        initial: Some(ledger),
        runtime: None,
        fixed_charge,
        planning_temporary,
        allowance: Cost {
            items,
            cleanup: 4,
            ..Cost::default()
        }
        .bounded(K::FormsPerThingMax, config.get(K::FormsPerThingMax)),
        root: None,
        cursor: None,
        position: 0,
        compiler_remaining: 0,
        host_backing,
    })
}

fn validate_bound(bound: &BindingCompilerBounds, count: u64, max: u64) -> CoreResult<u64> {
    let units = bound.work().remaining(W::BindingPolls);
    if units == 0
        || units > max
        || W::ALL
            .into_iter()
            .any(|w| w != W::BindingPolls && bound.work().remaining(w) != 0)
    {
        return Err(invalid());
    }
    mul(count, max)
}
fn form_size(form: &ValidatedPropertyReadForm<'_>) -> CoreResult<(u64, u64, u64)> {
    if form.security_count() != 1
        || form.security_scheme() != Some("nosec")
        || form.property_name().is_empty()
        || form.resolved_href().is_empty()
    {
        return Err(invalid());
    }
    let scopes = form.scopes();
    Ok((
        add(
            form.copy_bytes(),
            form.security_name().unwrap().len() as u64,
        )?,
        layout::<Box<str>>(scopes.len() as u64)?.size() as u64,
        scopes.len() as u64,
    ))
}
impl<'td, 'environment, R: ConsumerPropertyReadRegistration>
    ConsumerPropertyReadBuild<'td, 'environment, R>
{
    pub fn admit_runtime(
        mut self,
        ledger: AdmissionLedger,
    ) -> Result<Self, (Self, AdmissionLedger, CoreError)> {
        if self.phase != Phase::Waiting {
            return Err((self, ledger, invalid()));
        }
        self.runtime = Some(ledger);
        self.phase = Phase::Root;
        Ok(self)
    }
    fn requirements(&self) -> CoreResult<ConsumerPropertyReadPreflight> {
        let c = &self.census;
        let support = self.registration.support();
        for k in [
            K::FormsPerThingMax,
            K::BindingArtifactsPerThingMax,
            K::BindingArtifactsGlobalMax,
        ] {
            self.config.check(k, c.forms)?;
        }
        let artifacts = mul(c.forms, support.artifact_layout().size() as u64)?;
        self.config
            .check(K::BindingArtifactBytesPerThingMax, artifacts)?;
        self.config
            .check(K::BindingArtifactBytesGlobalMax, artifacts)?;
        let mut output = size_of::<Node<R::Artifact>>() as u64;
        for n in [
            layout::<PropertyRow>(c.properties)?.size() as u64,
            layout::<Coordinate<R::Artifact>>(c.forms)?.size() as u64,
            c.text,
            c.scope_tables,
            mul(c.forms, self.host_backing)?,
        ] {
            output = add(output, n)?;
        }
        for k in [
            K::CompiledPlanBytesMax,
            K::CompiledRuntimeBytesPerThingMax,
            K::CompiledRuntimeBytesGlobalMax,
        ] {
            self.config.check(k, output)?;
        }
        self.config.check(
            K::LogicalPlanBytesPerThingMax,
            add(
                add(c.text, c.scope_tables)?,
                mul(c.forms, size_of::<LogicalInteractionPlan>() as u64)?,
            )?,
        )?;
        let peak = add(
            output,
            add(self.fixed_charge, support.temporary_layout().size() as u64)?,
        )?;
        for k in [
            K::PeakLiveBytesPerAdmissionMax,
            K::AdmissionPeakLiveBytesGlobalMax,
            K::EngineLiveBytesGlobalMax,
        ] {
            self.config.check(k, peak)?;
        }
        let material_items = add(add(c.properties, c.forms)?, add(c.scopes, 5)?)?;
        let items = add(
            material_items,
            add(
                add(c.forms, 1)?,
                add(c.forms, add(add(c.properties, c.forms)?, 2)?)?,
            )?,
        )?;
        // Every allocation prepays acquisition and fixed release. The root and
        // both outer tables are included even when a zero count skips a request.
        let _cleanup = mul(add(c.allocations, 3)?, 2)?;
        let compiler = mul(c.forms, self.config.get(K::PlanCompileWorkUnitsPerStepMax))?;
        mul(
            c.forms,
            add(
                support.start_work(),
                self.config.get(K::PlanCompileWorkUnitsPerStepMax),
            )?,
        )?;
        mul(c.forms, support.bounds_work())?;
        mul(
            c.forms,
            support.allocation_work()
                + support.abort_work()
                + support.destruction_work()
                + support.release_work(),
        )?;
        Ok(ConsumerPropertyReadPreflight {
            property_count: c.properties,
            coordinate_count: c.forms,
            registration: self.identity,
            output_capacity_bytes: output,
            cursor_capacity_bytes: support.cursor_layout().size() as u64,
            temporary_capacity_bytes: add(
                self.fixed_charge,
                support.temporary_layout().size() as u64,
            )?,
            cleanup_capacity_bytes:
                ConsumerPropertyReadCleanupSlot::<R::Artifact>::required_capacity(),
            remaining_planning_items: items,
            remaining_copy_bytes: c.text,
            compiler_work_max: compiler,
        })
    }
    fn stop_inputs(&mut self) {
        if let Some(cursor) = self.cursor.take() {
            self.registration.abort(cursor);
            if self.host_backing != 0 {
                let d = self.root.as_mut().unwrap().data_mut();
                d.release(self.host_backing);
                d.coordinates[self.position].backing_bytes = 0;
                d.footprint.artifact_bytes -= self.host_backing;
            }
        }
        self.td = None;
        #[cfg(feature = "std")]
        {
            self.semantics = None;
        }
        if let Some(r) = self.root.as_mut() {
            let d = r.data_mut();
            d.release_temporary();
            if self.fixed_charge != 0 {
                assert!(
                    d.build
                        .as_mut()
                        .unwrap()
                        .release_temporary(self.fixed_charge)
                );
                self.fixed_charge = 0;
            }
        } else if self.fixed_charge != 0 && self.phase != Phase::Initialize {
            assert!(
                self.initial
                    .as_mut()
                    .unwrap()
                    .release_temporary(self.fixed_charge)
            );
            self.fixed_charge = 0;
        }
    }
    fn fail(mut self, cause: CoreError) -> ConsumerPropertyReadBuildStep<'td, 'environment, R> {
        self.stop_inputs();
        let cleanup = if let Some(mut root) = self.root.take() {
            root.data_mut().cause = cause.clone();
            root.node().state.set(RootState::Rollback);
            ConsumerPropertyReadRollback {
                owner: RollbackOwner::Shared(root),
            }
        } else {
            ConsumerPropertyReadRollback {
                owner: RollbackOwner::Fixed(Some(FixedCleanup {
                    cause: cause.clone(),
                    _build: self.initial.take(),
                    _runtime: self.runtime.take(),
                })),
            }
        };
        ConsumerPropertyReadBuildStep::Failed {
            cause,
            cleanup: Some(cleanup),
        }
    }
    pub fn step(
        mut self,
        budget: &mut WorkBudget,
        cancel_requested: bool,
    ) -> ConsumerPropertyReadBuildStep<'td, 'environment, R> {
        if cancel_requested {
            return self.fail(cancelled());
        }
        match self.advance(budget) {
            Err(cause) => self.fail(cause),
            Ok(Advance::Pending) => ConsumerPropertyReadBuildStep::Pending(self),
            Ok(Advance::Preflight(requirements)) => ConsumerPropertyReadBuildStep::Preflight {
                build: self,
                requirements,
            },
            Ok(Advance::Complete) => {
                self.stop_inputs();
                let root = self.root.take().unwrap();
                root.node().state.set(RootState::Draft);
                ConsumerPropertyReadBuildStep::Complete(ConsumerPropertyReadDraft { root })
            }
        }
    }
    fn advance(&mut self, budget: &mut WorkBudget) -> CoreResult<Advance> {
        match self.phase {
            Phase::Initialize => {
                if !self.allowance.debit(
                    budget,
                    Cost {
                        items: 1,
                        cleanup: 4,
                        ..Cost::default()
                    },
                )? {
                    return Ok(Advance::Pending);
                }
                let guard = self
                    .initial
                    .as_mut()
                    .unwrap()
                    .try_reserve_temporary(
                        K::AdmissionTemporaryBytesPerOperationMax,
                        self.fixed_charge,
                    )
                    .ok_or_else(unavailable)?;
                guard.commit();
                self.phase = Phase::Preflight;
                self.td.as_ref().unwrap().id().ok_or_else(invalid)?;
            }
            Phase::Preflight => {
                match self
                    .td
                    .as_mut()
                    .unwrap()
                    .step(budget, false)
                    .map_err(td_error)?
                {
                    TdStep::Pending => return Ok(Advance::Pending),
                    TdStep::Ready(event) => {
                        if !self.allowance.debit(
                            budget,
                            Cost {
                                items: 1,
                                ..Cost::default()
                            },
                        )? {
                            return Ok(Advance::Pending);
                        }
                        match event {
                            Event::Property { ordinal, name } => {
                                if u64::from(ordinal) != self.census.properties {
                                    return Err(invariant());
                                }
                                self.census.properties = add(self.census.properties, 1)?;
                                self.config
                                    .check(K::AffordancesPerThingMax, self.census.properties)?;
                                self.census.text = add(self.census.text, name.len() as u64)?;
                                self.census.allocations = add(self.census.allocations, 1)?;
                                self.census.property_forms = 0;
                            }
                            Event::Form(form) => {
                                if !self
                                    .registration
                                    .support()
                                    .accepts_target(form.resolved_href())
                                {
                                    self.td.as_mut().unwrap().acknowledge();
                                    return Ok(Advance::Pending);
                                }
                                let (text, table, scopes) = form_size(&form)?;
                                self.census.forms = add(self.census.forms, 1)?;
                                self.config.check(K::FormsPerThingMax, self.census.forms)?;
                                self.census.property_forms = add(self.census.property_forms, 1)?;
                                self.config
                                    .check(K::FormsPerContextMax, self.census.property_forms)?;
                                self.census.text = add(self.census.text, text)?;
                                self.census.scope_tables = add(self.census.scope_tables, table)?;
                                self.census.scopes = add(self.census.scopes, scopes)?;
                                self.census.allocations =
                                    add(self.census.allocations, add(9, scopes)?)?;
                            }
                        }
                        self.td.as_mut().unwrap().acknowledge();
                    }
                    TdStep::Done => {
                        if !self.allowance.debit(
                            budget,
                            Cost {
                                items: 1,
                                ..Cost::default()
                            },
                        )? {
                            return Ok(Advance::Pending);
                        }
                        let requirements = self.requirements()?;
                        self.td = Some(self.td.take().unwrap().rewind());
                        self.phase = Phase::Waiting;
                        self.preflight = Some(requirements);
                        return Ok(Advance::Preflight(requirements));
                    }
                }
            }
            Phase::Waiting => return Ok(Advance::Pending),
            Phase::Root => self.make_root(budget)?,
            Phase::Tables => self.make_tables(budget)?,
            Phase::Materialize => self.materialize(budget)?,
            Phase::Bounds => self.collect_bound(budget)?,
            Phase::Compile => self.compile_coordinate(budget)?,
            Phase::SealRows | Phase::SealCoordinates => return self.seal(budget),
        }
        Ok(Advance::Pending)
    }
    fn make_root(&mut self, budget: &mut WorkBudget) -> CoreResult<()> {
        let c = &self.census;
        let requirements = self.preflight.unwrap();
        let support = self.registration.support();
        let mut material = Cost {
            items: add(add(c.properties, c.forms)?, add(c.scopes, 5)?)?,
            bytes: c.text,
            cleanup: mul(add(c.allocations, 3)?, 2)?,
            ..Cost::default()
        }
        .bounded(
            K::CompiledPlanBytesMax,
            self.config.get(K::CompiledPlanBytesMax),
        );
        if !material.debit(
            budget,
            Cost {
                items: 1,
                cleanup: 2,
                ..Cost::default()
            },
        )? {
            return Ok(());
        }
        let l = Layout::new::<Node<R::Artifact>>();
        let n = l.size() as u64;
        self.config
            .check(K::LargestContiguousAllocationBytesMax, n)?;
        let peak = add(n, self.initial.as_ref().unwrap().live_bytes())?;
        for k in [
            K::PeakLiveBytesPerAdmissionMax,
            K::AdmissionPeakLiveBytesGlobalMax,
            K::EngineLiveBytesGlobalMax,
        ] {
            self.config.check(k, peak)?;
        }
        for k in [
            K::CompiledPlanBytesMax,
            K::CompiledRuntimeBytesPerThingMax,
            K::CompiledRuntimeBytesGlobalMax,
        ] {
            self.config.check(k, n)?;
        }
        let reclaim = Reclaim {
            bytes_paid: 0,
            allowance: Cost {
                items: add(add(c.properties, c.forms)?, add(c.scopes, 4)?)?,
                cleanup: add(add(c.properties, c.forms)?, add(c.scopes, 4)?)?,
                reclaim: requirements.output_capacity_bytes,
                ..Cost::default()
            }
            .bounded(
                K::CompiledPlanBytesMax,
                self.config.get(K::CompiledPlanBytesMax),
            ),
        };
        let footprint = PlanFootprint {
            fixed_owner_bytes: n - size_of::<Reclaim>() as u64,
            reclaim_metadata_bytes: size_of::<Reclaim>() as u64,
            compiler_cursor_bytes: size_of::<R::Cursor>() as u64,
            compiler_temporary_bytes: support.temporary_layout().size() as u64,
            build_owner_bytes: self.fixed_charge
                - self.planning_temporary
                - size_of::<R::Cursor>() as u64,
            planning_temporary_bytes: self.planning_temporary,
            largest_allocation: n,
            ..PlanFootprint::default()
        };
        let bounds = Cost {
            items: add(c.forms, 1)?,
            binding: mul(c.forms, support.bounds_work())?,
            ..Cost::default()
        }
        .bounded(K::FormsPerThingMax, self.config.get(K::FormsPerThingMax));
        let compile = Cost {
            items: c.forms,
            binding: mul(
                c.forms,
                add(
                    support.start_work(),
                    self.config.get(K::PlanCompileWorkUnitsPerStepMax),
                )?,
            )?,
            cleanup: mul(
                c.forms,
                support.allocation_work()
                    + support.abort_work()
                    + support.destruction_work()
                    + support.release_work(),
            )?,
            ..Cost::default()
        }
        .bounded(K::FormsPerThingMax, self.config.get(K::FormsPerThingMax));
        let seal = Cost {
            items: add(add(c.properties, c.forms)?, 2)?,
            ..Cost::default()
        }
        .bounded(K::FormsPerThingMax, self.config.get(K::FormsPerThingMax));
        let reservation = self
            .runtime
            .as_mut()
            .unwrap()
            .try_reserve_persistent_runtime(K::CompiledRuntimeBytesPerThingMax, n)
            .ok_or_else(unavailable)?;
        let pointer = NonNull::new(unsafe { alloc(l) })
            .ok_or_else(|| CoreError::Backpressure(context()))?
            .cast::<Node<R::Artifact>>();
        reservation.commit();
        let data = Data {
            registration: self.identity,
            rows: Vec::new(),
            coordinates: Vec::new(),
            runtime: self.runtime.take().unwrap(),
            build: self.initial.take(),
            runtime_charge: n,
            temporary_charge: 0,
            config: self.config,
            footprint,
            cause: cancelled(),
            reclaim,
            material,
            bounds,
            compile,
            seal,
            node_bytes: n,
        };
        unsafe {
            pointer.as_ptr().write(Node {
                references: Cell::new(1),
                state: Cell::new(RootState::Building),
                data: UnsafeCell::new(ManuallyDrop::new(data)),
            });
        }
        let mut root = Root {
            pointer,
            _local: PhantomData,
        };
        root.data_mut().observe();
        self.cleanup.root = Some(root.share());
        self.root = Some(root);
        self.phase = Phase::Tables;
        Ok(())
    }
    fn make_tables(&mut self, budget: &mut WorkBudget) -> CoreResult<()> {
        let d = self.root.as_mut().unwrap().data_mut();
        if !d.material.debit(
            budget,
            Cost {
                items: 1,
                cleanup: 4,
                ..Cost::default()
            },
        )? {
            return Ok(());
        }
        d.rows = d.table(self.census.properties)?;
        d.footprint.property_table_bytes =
            layout::<PropertyRow>(self.census.properties)?.size() as u64;
        d.coordinates = d.table(self.census.forms)?;
        let n = self.census.forms;
        let logical = mul(n, size_of::<LogicalInteractionPlan>() as u64)?;
        let candidates = mul(
            n,
            (size_of::<BindingCandidate>() + size_of::<BindingArtifactRef>()) as u64,
        )?;
        let inline = if self.host_backing == 0 {
            mul(
                n,
                self.registration.support().artifact_layout().size() as u64,
            )?
        } else {
            0
        };
        d.footprint.logical_owner_bytes = logical;
        d.footprint.candidate_and_reference_bytes = candidates;
        d.footprint.artifact_bytes = inline;
        d.footprint.coordinate_table_bytes =
            layout::<Coordinate<R::Artifact>>(n)?.size() as u64 - logical - candidates - inline;
        self.phase = Phase::Materialize;
        Ok(())
    }
    fn materialize(&mut self, budget: &mut WorkBudget) -> CoreResult<()> {
        let td = self.td.as_mut().unwrap();
        let id = td.id().ok_or_else(invalid)?;
        // TD's id() points into the borrowed immutable Thing, never into its
        // mutable cursor/scratch. Polling changes only that cursor, not the Thing
        // or its String backing. Keep this pointer only within this call while
        // the original source loan remains live; no suspended owner stores it.
        let id_pointer = id as *const str;
        let event = td.step(budget, false).map_err(td_error)?;
        let d = self.root.as_mut().unwrap().data_mut();
        match event {
            TdStep::Pending => return Ok(()),
            TdStep::Done => {
                if !d.material.debit(
                    budget,
                    Cost {
                        items: 1,
                        ..Cost::default()
                    },
                )? {
                    return Ok(());
                }
                if let Some(row) = d.rows.last_mut() {
                    row.end = d.coordinates.len();
                }
                if d.rows.len() as u64 != self.census.properties
                    || d.coordinates.len() as u64 != self.census.forms
                {
                    return Err(invariant());
                }
                // Native compilers still need one semantic closure per coordinate.
                // The immutable TD loan is released before draft publication.
                if !self.registration.support().uses_semantics() {
                    self.td = None;
                }
                self.phase = Phase::Bounds;
                self.position = 0;
                if self.census.forms != 0 {
                    let bytes = self.registration.support().temporary_layout().size() as u64;
                    let guard = d
                        .build
                        .as_mut()
                        .unwrap()
                        .try_reserve_temporary(K::AdmissionTemporaryBytesPerOperationMax, bytes)
                        .ok_or_else(unavailable)?;
                    guard.commit();
                    d.temporary_charge = bytes;
                    d.observe();
                }
                return Ok(());
            }
            TdStep::Ready(Event::Property { ordinal, name }) => {
                if !d.material.debit(
                    budget,
                    Cost {
                        items: 1,
                        bytes: name.len() as u64,
                        cleanup: 2,
                        ..Cost::default()
                    },
                )? {
                    return Ok(());
                }
                if ordinal as usize != d.rows.len() || d.rows.len() == d.rows.capacity() {
                    return Err(invariant());
                }
                if let Some(row) = d.rows.last_mut() {
                    row.end = d.coordinates.len();
                }
                let name = d.text(name)?;
                let start = d.coordinates.len();
                d.rows.push(PropertyRow {
                    name,
                    start,
                    end: start,
                });
            }
            TdStep::Ready(Event::Form(form)) => {
                if !self
                    .registration
                    .support()
                    .accepts_target(form.resolved_href())
                {
                    td.acknowledge();
                    return Ok(());
                }
                let (text, scope_table, scopes) = form_size(&form)?;
                if !d.material.debit(
                    budget,
                    Cost {
                        items: add(1, scopes)?,
                        bytes: text,
                        cleanup: mul(add(9, scopes)?, 2)?,
                        ..Cost::default()
                    },
                )? {
                    return Ok(());
                }
                if d.coordinates.len() == d.coordinates.capacity()
                    || form.property_ordinal() as usize + 1 != d.rows.len()
                {
                    return Err(invariant());
                }
                let position = d.coordinates.len();
                let plan_id = PlanId::new(SlotIndex::new(position as u32), self.generation.get());
                let candidate = BindingCandidate::new(
                    self.identity.binding_id(),
                    self.identity.binding_generation(),
                    self.identity.configuration(),
                    self.identity.artifact_compatibility(),
                    0,
                    0,
                );
                let identity = BindingArtifactIdentity::new(
                    self.generation,
                    plan_id,
                    candidate.binding_id(),
                    candidate.binding_generation(),
                    candidate.configuration(),
                    candidate.compatibility(),
                    BindingArtifactRole::ConsumerCall,
                );
                d.coordinates.push(Coordinate::new(
                    candidate,
                    identity,
                    SlotIndex::new(position as u32),
                ));
                // All allocations are stored in the source-free owner as soon as
                // acquired. An allocation failure cannot trigger partial Vec Drop.
                let value = d.text(unsafe { &*id_pointer })?.into_string();
                d.coordinates[position].parts.id = Some(value);
                let value = d.text(form.property_name())?;
                d.coordinates[position].parts.name = Some(value);
                let value = d.text(form.resolved_href())?;
                d.coordinates[position].parts.target = Some(value);
                let value = d.text(form.content_type())?;
                d.coordinates[position].parts.content = Some(value);
                if let Some(s) = form.subprotocol() {
                    let value = d.text(s)?;
                    d.coordinates[position].parts.protocol = Some(value);
                }
                let value = d.text(form.href())?;
                d.coordinates[position].raw = Some(value);
                if let Some(s) = form.content_coding() {
                    let value = d.text(s)?;
                    d.coordinates[position].coding = Some(value);
                }
                let value = d.text(form.security_name().unwrap())?;
                d.coordinates[position].security = Some(value);
                let values = d.table::<Box<str>>(scopes)?;
                d.coordinates[position].scopes = values;
                d.coordinates[position].scope_capacity_bytes = scope_table;
                d.footprint.owned_program_bytes += scope_table;
                for scope in form.scopes().iter() {
                    let value = d.text(scope)?;
                    d.coordinates[position].scope_bytes += scope.len() as u64;
                    d.coordinates[position].scopes.push(value);
                }
                let c = &mut d.coordinates[position];
                c.plan = Some(
                    LogicalInteractionPlan::try_property_read(
                        plan_id,
                        ThingId::new(c.parts.id.take().unwrap()),
                        c.parts.name.take().unwrap(),
                        form.original_index(),
                        c.parts.target.take().unwrap(),
                        c.parts.content.take(),
                        c.parts.protocol.take(),
                    )
                    .expect("paid nonempty TD facts checked before allocation")
                    .with_additional_form_metadata(form.has_additional_form_metadata()),
                );
            }
        }
        td.acknowledge();
        Ok(())
    }
}
enum Advance {
    Pending,
    Preflight(ConsumerPropertyReadPreflight),
    Complete,
}

impl<'td, 'environment, R: ConsumerPropertyReadRegistration>
    ConsumerPropertyReadBuild<'td, 'environment, R>
{
    fn collect_bound(&mut self, budget: &mut WorkBudget) -> CoreResult<()> {
        let d = self.root.as_mut().unwrap().data_mut();
        if self.position == d.coordinates.len() {
            if !d.bounds.debit(
                budget,
                Cost {
                    items: 1,
                    ..Cost::default()
                },
            )? {
                return Ok(());
            }
            self.phase = Phase::Compile;
            self.position = 0;
            return Ok(());
        }
        if !d.bounds.debit(
            budget,
            Cost {
                items: 1,
                binding: self.registration.support().bounds_work(),
                ..Cost::default()
            },
        )? {
            return Ok(());
        }
        let c = &d.coordinates[self.position];
        let input = BindingCompilerInput::new(
            c.plan.as_ref().unwrap(),
            c.candidate,
            BindingArtifactRole::ConsumerCall,
        );
        #[cfg(feature = "std")]
        let input = match &self.semantics {
            Some(s) => input.with_semantics(s),
            None => input,
        };
        let bound = self.registration.bounds(&input)?;
        validate_bound(
            &bound,
            self.census.forms,
            self.config.get(K::PlanCompileWorkUnitsPerStepMax),
        )?;
        self.config.check(
            K::BindingArtifactBytesPerItemMax,
            bound.artifact().retained_bytes(),
        )?;
        self.config.check(
            K::BindingCompilerCursorBytesPerItemMax,
            bound.cursor_bytes(),
        )?;
        self.config
            .check(K::BindingCompilerCursorBytesGlobalMax, bound.cursor_bytes())?;
        if bound.cursor_bytes() > self.registration.support().cursor_layout().size() as u64
            || bound.temporary_bytes()
                > self.registration.support().temporary_layout().size() as u64
        {
            return Err(invariant());
        }
        d.coordinates[self.position].bound = Some(bound);
        self.position += 1;
        Ok(())
    }
    fn compile_coordinate(&mut self, budget: &mut WorkBudget) -> CoreResult<()> {
        let d = self.root.as_mut().unwrap().data_mut();
        if self.position == d.coordinates.len() {
            self.phase = Phase::SealRows;
            self.position = 0;
            return Ok(());
        }
        #[cfg(feature = "std")]
        if self.registration.support().uses_semantics() && self.semantics.is_none() {
            use clinkz_wot_td::{ValidatedSemanticError as E, semantic::SemanticError};
            let plan = d.coordinates[self.position].plan.as_ref().unwrap();
            match self.td.as_mut().unwrap().resolve_extensions(
                plan.property_name(),
                plan.form_index(),
                budget,
            ) {
                Ok(Some(semantics)) => self.semantics = Some(semantics),
                Ok(None) => return Ok(()),
                Err(E::Admission(cause)) => return Err(td_error(cause)),
                Err(E::Resolution(SemanticError::Allocation)) => {
                    return Err(CoreError::Backpressure(context()));
                }
                Err(E::Resolution(SemanticError::Limit | SemanticError::UnsupportedContext)) => {
                    // Preparation profile/capacity refusal applies to this
                    // coordinate. It is not a whole-TD WoT validity judgment.
                    d.coordinates[self.position].unsupported = true;
                    self.position += 1;
                    return Ok(());
                }
            }
        }
        let support = self.registration.support();
        if self.cursor.is_none() {
            if !d.compile.debit(
                budget,
                Cost {
                    items: 1,
                    binding: support.start_work(),
                    cleanup: support.allocation_work()
                        + support.abort_work()
                        + support.destruction_work()
                        + support.release_work(),
                    ..Cost::default()
                },
            )? {
                return Ok(());
            }
            let next = d.check_request(self.host_backing)?;
            // Core's closed Host adapter has precisely one fallible backing
            // acquisition. Authorize its actual Layout before invoking start.
            let cursor = {
                let guard = if self.host_backing != 0 {
                    Some(
                        d.runtime
                            .try_reserve_persistent_runtime(
                                K::CompiledRuntimeBytesPerThingMax,
                                self.host_backing,
                            )
                            .ok_or_else(unavailable)?,
                    )
                } else {
                    None
                };
                let c = &d.coordinates[self.position];
                let input = BindingCompilerInput::new(
                    c.plan.as_ref().unwrap(),
                    c.candidate,
                    BindingArtifactRole::ConsumerCall,
                );
                #[cfg(feature = "std")]
                let input = match &self.semantics {
                    Some(s) => input.with_semantics(s),
                    None => input,
                };
                let cursor = self.registration.start(&input)?;
                match guard {
                    Some(guard) => guard.commit(),
                    None => {}
                }
                cursor
            };
            d.runtime_charge = next;
            d.coordinates[self.position].backing_bytes = self.host_backing;
            d.footprint.artifact_bytes += self.host_backing;
            d.footprint.largest_allocation = d.footprint.largest_allocation.max(self.host_backing);
            d.observe();
            self.compiler_remaining = d.coordinates[self.position]
                .bound
                .as_ref()
                .unwrap()
                .work()
                .remaining(W::BindingPolls);
            self.cursor = Some(cursor);
            return Ok(());
        }
        let step = support.step_work();
        if self.compiler_remaining < step || d.compile.binding < step {
            return Err(invariant());
        }
        let available = budget
            .remaining(W::BindingPolls)
            .min(self.compiler_remaining)
            .min(d.compile.binding)
            .min(self.config.get(K::PlanCompileWorkUnitsPerStepMax));
        if available < step {
            return Ok(());
        }
        let c = &d.coordinates[self.position];
        let input = BindingCompilerInput::new(
            c.plan.as_ref().unwrap(),
            c.candidate,
            BindingArtifactRole::ConsumerCall,
        );
        #[cfg(feature = "std")]
        let input = match &self.semantics {
            Some(s) => input.with_semantics(s),
            None => input,
        };
        let mut child = WorkBudget::new().with_remaining(W::BindingPolls, available);
        let result = self
            .registration
            .progress(&input, self.cursor.take().unwrap(), &mut child);
        let used = available
            .checked_sub(child.remaining(W::BindingPolls))
            .ok_or_else(invariant)?;
        budget.consume(W::BindingPolls, used).unwrap();
        d.compile.binding -= used;
        self.compiler_remaining -= used;
        match result {
            BindingCompilerStep::Pending(cursor) => self.cursor = Some(cursor),
            BindingCompilerStep::Failed(failure) => {
                let (cause, cursor) = failure.into_parts();
                if support.uses_semantics() && matches!(cause, CoreError::UnsupportedOperation(_)) {
                    self.registration.abort(cursor);
                    d.release(self.host_backing);
                    let c = &mut d.coordinates[self.position];
                    c.backing_bytes = 0;
                    c.unsupported = true;
                    d.footprint.artifact_bytes -= self.host_backing;
                    self.finish_coordinate();
                    return Ok(());
                }
                self.cursor = Some(cursor);
                return Err(cause);
            }
            BindingCompilerStep::Complete(output) => {
                let c = &mut d.coordinates[self.position];
                // The closed issuer guarantees compatibility and exact payload.
                // Preserve any rejected output in the cleanup owner nevertheless.
                let artifact = output.into_artifact();
                let envelope = match BindingArtifactEnvelope::try_new(
                    c.reference.identity(),
                    c.bound.as_ref().unwrap().artifact(),
                    artifact,
                ) {
                    Ok(envelope) => envelope,
                    Err(rejection) => {
                        c.artifact = ArtifactOwner::Rejected(rejection.into_artifact());
                        return Err(invariant());
                    }
                };
                c.artifact = ArtifactOwner::Ready(envelope);
                self.finish_coordinate();
            }
        }
        Ok(())
    }
    fn finish_coordinate(&mut self) {
        #[cfg(feature = "std")]
        {
            self.semantics = None;
        }
        self.position += 1;
        self.compiler_remaining = 0;
    }
    fn seal(&mut self, budget: &mut WorkBudget) -> CoreResult<Advance> {
        let d = self.root.as_mut().unwrap().data_mut();
        if !d.seal.debit(
            budget,
            Cost {
                items: 1,
                ..Cost::default()
            },
        )? {
            return Ok(Advance::Pending);
        }
        if self.phase == Phase::SealRows {
            if self.position == d.rows.len() {
                self.phase = Phase::SealCoordinates;
                self.position = 0;
                return Ok(Advance::Pending);
            }
            let row = &d.rows[self.position];
            if row.start > row.end
                || row.end > d.coordinates.len()
                || (self.position == 0 && row.start != 0)
                || (self.position != 0 && d.rows[self.position - 1].end != row.start)
            {
                return Err(invariant());
            }
            self.position += 1;
        } else {
            if self.position == d.coordinates.len() {
                if d.footprint.retained_bytes() != d.runtime_charge {
                    return Err(invariant());
                }
                return Ok(Advance::Complete);
            }
            let c = &d.coordinates[self.position];
            if c.unsupported {
                self.position += 1;
                return Ok(Advance::Pending);
            }
            let artifact = c.artifact.as_ref().ok_or_else(invariant)?;
            if c.reference.identity() != artifact.identity()
                || c.reference.artifact_slot() != SlotIndex::new(self.position as u32)
                || artifact.route_reservation().is_some()
                || c.plan.as_ref().unwrap().plan_id() != artifact.identity().plan_id()
            {
                return Err(invariant());
            }
            self.position += 1;
        }
        Ok(Advance::Pending)
    }
}

fn coordinate_text<A>(c: &Coordinate<A>) -> u64 {
    let mut bytes = c.raw.as_deref().map_or(0, str::len)
        + c.coding.as_deref().map_or(0, str::len)
        + c.security.as_deref().map_or(0, str::len);
    if let Some(p) = &c.plan {
        bytes += p.thing_id().as_str().len()
            + p.property_name().len()
            + p.resolved_target().len()
            + p.content_type().map_or(0, str::len)
            + p.subprotocol().map_or(0, str::len);
    } else {
        bytes += c.parts.id.as_ref().map_or(0, String::len)
            + c.parts.name.as_deref().map_or(0, str::len)
            + c.parts.target.as_deref().map_or(0, str::len)
            + c.parts.content.as_deref().map_or(0, str::len)
            + c.parts.protocol.as_deref().map_or(0, str::len);
    }
    bytes as u64
}
impl<A: 'static> ConsumerPropertyReadRollback<A> {
    pub fn cause(&self) -> &CoreError {
        match &self.owner {
            RollbackOwner::Fixed(f) => &f.as_ref().unwrap().cause,
            RollbackOwner::Shared(r) => &r.data().cause,
        }
    }
    pub fn live_bytes(&self) -> u64 {
        match &self.owner {
            RollbackOwner::Fixed(_) => 0,
            RollbackOwner::Shared(r) => r.data().runtime_charge,
        }
    }
    pub fn observations(&self) -> Option<(u64, u64, u64)> {
        match &self.owner {
            RollbackOwner::Fixed(f) => f.as_ref().and_then(|f| f._runtime.as_ref()).map(|l| {
                (
                    l.live_bytes(),
                    l.peak_live_bytes(),
                    l.largest_contiguous_allocation(),
                )
            }),
            RollbackOwner::Shared(r) => {
                let d = r.data();
                Some((
                    d.runtime.live_bytes(),
                    d.runtime.peak_live_bytes(),
                    d.runtime.largest_contiguous_allocation(),
                ))
            }
        }
    }
    pub fn step(mut self, budget: &mut WorkBudget) -> ConsumerPropertyReadRollbackStep<A> {
        if let RollbackOwner::Fixed(f) = &mut self.owner {
            return ConsumerPropertyReadRollbackStep::Complete(f.take().unwrap().cause);
        }
        let RollbackOwner::Shared(root) = &mut self.owner else {
            unreachable!()
        };
        let d = root.data_mut();
        let cap = d.config.get(K::PlanReclaimBytesPerStepMax);
        // Exactly one item operation per call, bounded by the existing cleanup
        // item cap. Byte progress can span calls; it never refills its lifetime.
        let (bytes, kind) = if let Some(c) = d.coordinates.last() {
            if let Some(scope) = c.scopes.last() {
                (scope.len() as u64, 0)
            } else {
                (
                    coordinate_text(c) + c.scope_capacity_bytes + c.backing_bytes,
                    1,
                )
            }
        } else if let Some(row) = d.rows.last() {
            (row.name.len() as u64, 2)
        } else if d.coordinates.capacity() != 0 {
            (
                d.coordinates.capacity() as u64 * size_of::<Coordinate<A>>() as u64,
                3,
            )
        } else if d.rows.capacity() != 0 {
            (
                d.rows.capacity() as u64 * size_of::<PropertyRow>() as u64,
                4,
            )
        } else {
            (0, 5)
        };
        let left = bytes - d.reclaim.bytes_paid;
        let chunk = left.min(cap).min(budget.remaining(W::CodecInputBytes));
        if left != 0 && chunk == 0 {
            return ConsumerPropertyReadRollbackStep::Pending(self);
        }
        if chunk != 0 {
            if !d
                .reclaim
                .allowance
                .debit(
                    budget,
                    Cost {
                        reclaim: chunk,
                        ..Cost::default()
                    },
                )
                .expect("prepaid output capacity")
            {
                return ConsumerPropertyReadRollbackStep::Pending(self);
            }
            d.reclaim.bytes_paid += chunk;
        }
        if d.reclaim.bytes_paid < bytes {
            return ConsumerPropertyReadRollbackStep::Pending(self);
        }
        if !d
            .reclaim
            .allowance
            .debit(
                budget,
                Cost {
                    items: 1,
                    cleanup: 1,
                    ..Cost::default()
                },
            )
            .expect("fixed census cleanup allowance")
        {
            return ConsumerPropertyReadRollbackStep::Pending(self);
        }
        d.reclaim.bytes_paid = 0;
        match kind {
            0 => {
                let c = d.coordinates.last_mut().unwrap();
                let scope = c.scopes.pop().unwrap();
                let n = scope.len() as u64;
                drop(scope);
                c.scope_bytes -= n;
                d.footprint.owned_text_bytes -= n;
                d.release(n);
            }
            1 => {
                let c = d.coordinates.pop().unwrap();
                let text = coordinate_text(&c);
                let backing = c.backing_bytes;
                let program = c.scope_capacity_bytes;
                debug_assert!(c.scopes.is_empty());
                // At most the fixed Core plan fields and fixed supplementary
                // fields remain. Their release primitives were prepaid before
                // acquisition; variable scopes were drained one at a time.
                drop(c);
                d.release(text + backing + program);
                d.footprint.owned_text_bytes -= text;
                d.footprint.owned_program_bytes -= program;
            }
            2 => {
                let row = d.rows.pop().unwrap();
                let n = row.name.len() as u64;
                drop(row);
                d.release(n);
                d.footprint.owned_text_bytes -= n;
            }
            3 => {
                drop(core::mem::take(&mut d.coordinates));
                d.release(bytes);
            }
            4 => {
                drop(core::mem::take(&mut d.rows));
                d.release(bytes);
            }
            _ => {
                debug_assert_eq!(d.runtime_charge, d.node_bytes);
                let cause = d.cause.clone();
                root.node().state.set(RootState::Terminal);
                return ConsumerPropertyReadRollbackStep::Complete(cause);
            }
        }
        ConsumerPropertyReadRollbackStep::Pending(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn work_debits_are_atomic_and_lifetime_products_are_checked() {
        let original = Cost {
            items: 2,
            bytes: 3,
            ..Cost::default()
        };
        let mut life = original;
        let mut short = WorkBudget::new()
            .with_remaining(W::PlanningItems, 2)
            .with_remaining(W::CodecOutputBytes, 2);
        assert!(!life.debit(&mut short, original).unwrap());
        assert_eq!(short.remaining(W::PlanningItems), 2);
        assert_eq!(life.items, 2);
        short.set_remaining(W::CodecOutputBytes, 3);
        assert!(life.debit(&mut short, original).unwrap());
        assert!(life.debit(&mut short, original).is_err());
        let mut life = original;
        let mut spare = WorkBudget::new()
            .with_remaining(W::PlanningItems, 3)
            .with_remaining(W::CodecOutputBytes, 4);
        assert!(life.debit(&mut spare, original).unwrap());
        assert_eq!(spare.remaining(W::PlanningItems), 1);
        assert_eq!(spare.remaining(W::CodecOutputBytes), 1);
        let bound =
            |work| BindingCompilerBounds::new(BindingArtifactFootprint::new(1, 65), 1, 1, work);
        assert!(validate_bound(&bound(WorkBudget::new()), 1, 16).is_err());
        for w in W::ALL.into_iter().filter(|w| *w != W::BindingPolls) {
            assert!(
                validate_bound(
                    &bound(
                        WorkBudget::new()
                            .with_remaining(W::BindingPolls, 4)
                            .with_remaining(w, 1)
                    ),
                    1,
                    16
                )
                .is_err()
            );
        }
        assert!(
            validate_bound(
                &bound(WorkBudget::new().with_remaining(W::BindingPolls, 17)),
                1,
                16
            )
            .is_err()
        );
        assert!(
            validate_bound(
                &bound(WorkBudget::new().with_remaining(W::BindingPolls, 4)),
                u64::MAX,
                16
            )
            .is_err()
        );
    }
}

// The external fixture includes this exact source under cfg(test) for
// observation/injection; its public tests also use the normal Planning crate.
#[cfg(test)]
#[path = "../tests/support/consumer_property_read.rs"]
#[allow(dead_code)]
pub(crate) mod test_support;

impl<R: ConsumerPropertyReadRegistration> Drop for ConsumerPropertyReadBuild<'_, '_, R> {
    fn drop(&mut self) {
        self.stop_inputs();
        if let Some(root) = self.root.take() {
            root.node().state.set(RootState::Available);
        } else if self.initial.is_some() || self.runtime.is_some() {
            debug_assert!(self.cleanup.fixed.is_none());
            self.cleanup.fixed = Some(FixedCleanup {
                cause: cancelled(),
                _build: self.initial.take(),
                _runtime: self.runtime.take(),
            });
        }
    }
}
