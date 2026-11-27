//! Pure binding preparation. Runtime envelopes and storage live in Core.
#[cfg(feature = "std")]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use clinkz_wot_core::artifact::try_box;
use clinkz_wot_core::*;
#[cfg(test)]
use clinkz_wot_foundation::SlotIndex;
use clinkz_wot_foundation::WorkBudget;
#[cfg(feature = "std")]
use clinkz_wot_foundation::WorkClass;
#[cfg(feature = "std")]
use core::any::Any;
use core::fmt;
#[cfg(feature = "std")]
use core::{alloc::Layout, mem::size_of};
#[derive(Debug, Eq, PartialEq)]
pub struct BindingCompilerBounds {
    artifact: BindingArtifactFootprint,
    cursor_bytes: u64,
    temporary_bytes: u64,
    work: WorkBudget,
}

impl BindingCompilerBounds {
    /// Creates the complete compiler bound.
    pub const fn new(
        artifact: BindingArtifactFootprint,
        cursor_bytes: u64,
        temporary_bytes: u64,
        work: WorkBudget,
    ) -> Self {
        Self {
            artifact,
            cursor_bytes,
            temporary_bytes,
            work,
        }
    }

    /// Returns the admitted final artifact footprint.
    pub const fn artifact(&self) -> BindingArtifactFootprint {
        self.artifact
    }

    /// Returns the declared cursor byte count.
    pub const fn cursor_bytes(&self) -> u64 {
        self.cursor_bytes
    }

    /// Returns the declared peak temporary byte count.
    pub const fn temporary_bytes(&self) -> u64 {
        self.temporary_bytes
    }

    /// Returns the declared typed work allowance.
    pub const fn work(&self) -> &WorkBudget {
        &self.work
    }

    /// Consumes the declaration and returns its work allowance.
    pub fn into_work(self) -> WorkBudget {
        self.work
    }
}

/// Complete generation-qualified identity of one admitted binding artifact.
#[derive(Clone, Copy)]
pub struct BindingCompilerInput<'a> {
    plan: PlanId,
    candidate: BindingCandidate,
    role: BindingArtifactRole,
    operation: Operation,
    property: &'a str,
    form: u32,
    target: &'a str,
    content_type: Option<&'a str>,
    subprotocol: Option<&'a str>,
    #[cfg(feature = "std")]
    semantics: Option<ResolvedBindingInput<'a>>,
}

impl<'a> BindingCompilerInput<'a> {
    /// Restricted logical-record adapter. General preparation uses `operation_input`.
    pub fn new(
        plan: &'a LogicalInteractionPlan,
        candidate: BindingCandidate,
        role: BindingArtifactRole,
    ) -> Self {
        Self {
            plan: plan.plan_id(),
            candidate,
            role,
            operation: plan.operation(),
            property: plan.property_name(),
            form: plan.form_index(),
            target: plan.resolved_target(),
            content_type: plan.content_type(),
            subprotocol: plan.subprotocol(),
            #[cfg(feature = "std")]
            semantics: None,
        }
    }
    pub const fn plan_id(&self) -> PlanId {
        self.plan
    }
    pub const fn candidate(&self) -> BindingCandidate {
        self.candidate
    }
    pub const fn role(&self) -> BindingArtifactRole {
        self.role
    }
    pub const fn operation(&self) -> Operation {
        self.operation
    }
    pub const fn form_index(&self) -> u32 {
        self.form
    }
    pub const fn resolved_target(&self) -> &'a str {
        self.target
    }
    pub const fn property_name(&self) -> &'a str {
        self.property
    }
    pub const fn content_type(&self) -> Option<&'a str> {
        self.content_type
    }
    pub const fn subprotocol(&self) -> Option<&'a str> {
        self.subprotocol
    }
    /// Resolved Form/operation input, independent of source TD identity or a logical record.
    #[cfg(feature = "std")]
    pub fn operation_input(
        plan: PlanId,
        candidate: BindingCandidate,
        role: BindingArtifactRole,
        facts: ResolvedBindingInput<'a>,
    ) -> Self {
        let property = match facts.owner {
            clinkz_wot_td::semantic::FormOwner::Property(n) => n,
            _ => "",
        };
        Self {
            plan,
            candidate,
            role,
            operation: facts.operation,
            property,
            form: facts.form_index,
            target: facts.target,
            content_type: facts.content_type,
            subprotocol: facts.subprotocol,
            semantics: Some(facts),
        }
    }
    #[cfg(feature = "std")]
    pub const fn resolved(&self) -> Option<ResolvedBindingInput<'a>> {
        self.semantics
    }
}

/// Native compilation input: effective WoT facts plus resolved scoped values.
/// No TD/context parser, security decision, handler or live resource is lent.
#[cfg(feature = "std")]
#[derive(Clone, Copy)]
pub struct ResolvedBindingInput<'a> {
    /// Exact complete Consumer conjunction, qualified against the effective
    /// expression. None requires complete expression qualification.
    pub selected_security: Option<&'a clinkz_wot_core::security::SecurityRequirement>,
    pub operation: clinkz_wot_td::data_type::Operation,
    pub owner: clinkz_wot_td::semantic::FormOwner<'a>,
    /// Property-only inline adapter convenience; use `owner` for general input.
    pub property: &'a str,
    pub form_index: u32,
    pub target: &'a str,
    pub content_type: Option<&'a str>,
    pub subprotocol: Option<&'a str>,
    pub document: &'a clinkz_wot_td::semantic::ResolvedSemanticDocument<'a>,
}

impl fmt::Debug for BindingCompilerInput<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BindingCompilerInput")
            .field("plan", &self.plan)
            .field("candidate", &self.candidate)
            .field("role", &self.role)
            .finish()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingCompilerOutput<A> {
    artifact: BindingArtifact<A>,
}

impl<A> BindingCompilerOutput<A> {
    /// Wraps one completed artifact.
    pub const fn new(artifact: BindingArtifact<A>) -> Self {
        Self { artifact }
    }

    /// Borrows the completed artifact.
    pub const fn artifact(&self) -> &BindingArtifact<A> {
        &self.artifact
    }

    /// Consumes the output and returns the completed artifact.
    pub fn into_artifact(self) -> BindingArtifact<A> {
        self.artifact
    }
}

/// Compiler failure that preserves its caller-owned cursor.
#[derive(Debug, Eq, PartialEq)]
pub struct BindingCompilerFailure<C> {
    error: CoreError,
    cursor: C,
}

impl<C> BindingCompilerFailure<C> {
    /// Creates an ownership-preserving compiler failure.
    pub const fn new(error: CoreError, cursor: C) -> Self {
        Self { error, cursor }
    }

    /// Borrows the structured failure.
    pub const fn error(&self) -> &CoreError {
        &self.error
    }

    /// Borrows the unchanged or resumable cursor.
    pub const fn cursor(&self) -> &C {
        &self.cursor
    }

    /// Consumes the failure and returns both owned values.
    pub fn into_parts(self) -> (CoreError, C) {
        (self.error, self.cursor)
    }
}

/// One incremental compiler step.
#[derive(Debug, Eq, PartialEq)]
#[must_use]
pub enum BindingCompilerStep<C, A> {
    /// More work remains and the owned cursor is returned.
    Pending(C),
    /// Compilation completed with one measured artifact.
    Complete(BindingCompilerOutput<A>),
    /// Compilation failed and the owned cursor is returned.
    Failed(BindingCompilerFailure<C>),
}

/// Portable protocol-specific binding compiler extension.
pub trait BindingCompilerExtension {
    /// Pure caller-owned resumable state.
    type Cursor;
    /// Immutable protocol-specific artifact payload.
    type Artifact;

    /// Returns the stable compatibility identity of produced artifacts.
    fn compatibility(&self) -> BindingArtifactCompatibility;

    /// Declares final, cursor, temporary, and typed-work bounds before progress.
    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds>;

    /// Creates pure cursor state without externally chargeable progress.
    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor>;

    /// Performs bounded progress while preserving cursor ownership.
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact>;

    /// Consumes pure in-memory cursor state.
    fn abort(&self, cursor: Self::Cursor);
}

/// Trusted portable compiler contract, independent of compiler name, atomicity
/// or output Copy semantics. Costs include cursor/output/indirect destruction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsumerCompilerContract {
    pub inspection_work: u64,
    pub start_work: u64,
    pub step_work: u64,
    pub max_steps: u64,
    pub abort_work: u64,
    pub artifact_drop_work: u64,
    /// Maximum temporary backing during configuration/bounds inspection.
    pub inspection_temporary_bytes: u64,
}
/// Native compiler callbacks charge BindingPolls. Their complete lifetime
/// allowance is prepaid by admission; per-step credit cannot renew it.
#[cfg(feature = "std")]
pub trait ConsumerCompiler: BindingCompilerExtension {
    const CONTRACT: ConsumerCompilerContract;
    /// Actual immutable policy paired with the executor, not a support tag.
    fn configuration(&self) -> ConsumerBindingConfiguration;
}
#[cfg(feature = "std")]
#[derive(Clone, Copy, Debug)]
pub struct ConsumerCompilerMetadata {
    pub configuration: ConsumerBindingConfiguration,
    pub contract: ConsumerCompilerContract,
    pub slot: Layout,
}

/// Independently authored, pure, bounded atomic Host lowering for Property Read.
/// WORK bounds native inspection/copy/destruction under the TD arena limits.
/// Implementations are trusted linked code and require executable qualification;
/// numeric declarations and safe erasure cannot prove their behavior. Native
/// private backing follows the selected Host policy. Strict Embedded compilers
/// retain their separate incremental storage/work contract.
///
/// This is a restricted adapter, not the target general compiler SPI. Generic
/// `BindingCompilerExtension` erasure supports owned artifacts and resumable
/// jobs. ConsumerCompiler declares their bounded Consumer admission contract.
#[cfg(feature = "std")]
pub trait HostSemanticCompiler: Send + Sync + 'static {
    /// This first Host slice stores source-free inline native instructions.
    /// Copy excludes owned heap descendants and native resource destructors.
    type Artifact: Copy + Send + Sync + 'static;
    const WORK: u64;
    fn compatibility(&self) -> BindingArtifactCompatibility;
    fn configuration(&self) -> ConsumerBindingConfiguration;
    fn compile(&self, input: ResolvedBindingInput<'_>) -> CoreResult<Self::Artifact>;
}

#[cfg(feature = "std")]
struct SemanticCompilerAdapter<C>(C);
#[cfg(feature = "std")]
struct SemanticCompilerCursor {
    plan: PlanId,
    candidate: BindingCandidate,
}
#[cfg(feature = "std")]
impl<C: HostSemanticCompiler> BindingCompilerExtension for SemanticCompilerAdapter<C> {
    type Cursor = SemanticCompilerCursor;
    type Artifact = C::Artifact;
    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.0.compatibility()
    }
    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        // This adapter's bounds depend only on captured types/configuration.
        // Mapping inspection waits for the coordinate's prepared semantics.
        self.check_identity(input)?;
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, size_of::<C::Artifact>() as u64),
            size_of::<SemanticCompilerCursor>() as u64,
            size_of::<(BindingCompilerStep<Self::Cursor, C::Artifact>, CoreError)>() as u64,
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, C::WORK),
        ))
    }
    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        self.check(input)?;
        Ok(SemanticCompilerCursor {
            plan: input.plan_id(),
            candidate: input.candidate(),
        })
    }
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        if budget.consume(WorkClass::BindingPolls, C::WORK).is_err() {
            return BindingCompilerStep::Pending(cursor);
        }
        let result = self.check(input).and_then(|()| {
            if cursor.plan != input.plan_id() || cursor.candidate != input.candidate() {
                return Err(host_cursor_mismatch(input));
            }
            self.0.compile(
                input
                    .resolved()
                    .ok_or_else(|| host_cursor_mismatch(input))?,
            )
        });
        match result {
            Ok(artifact) => {
                BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                    self.0.compatibility(),
                    BindingArtifactFootprint::new(1, size_of::<C::Artifact>() as u64),
                    artifact,
                )))
            }
            Err(cause) => BindingCompilerStep::Failed(BindingCompilerFailure::new(cause, cursor)),
        }
    }
    fn abort(&self, _: Self::Cursor) {}
}
#[cfg(feature = "std")]
impl<C: HostSemanticCompiler> SemanticCompilerAdapter<C> {
    fn check_identity(&self, input: &BindingCompilerInput<'_>) -> CoreResult<()> {
        if input.role() != BindingArtifactRole::ConsumerCall
            || input.candidate().configuration() != self.0.configuration().configuration()
            || input.candidate().compatibility() != self.0.compatibility()
        {
            return Err(host_cursor_mismatch(input));
        }
        Ok(())
    }
    fn check(&self, input: &BindingCompilerInput<'_>) -> CoreResult<()> {
        self.check_identity(input)?;
        if input.resolved().is_none() {
            return Err(host_cursor_mismatch(input));
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<C: HostSemanticCompiler> ConsumerCompiler for SemanticCompilerAdapter<C> {
    const CONTRACT: ConsumerCompilerContract = ConsumerCompilerContract {
        inspection_work: 1,
        start_work: 1,
        step_work: C::WORK,
        max_steps: 1,
        abort_work: 1,
        artifact_drop_work: 1,
        inspection_temporary_bytes: 0,
    };
    fn configuration(&self) -> ConsumerBindingConfiguration {
        self.0.configuration()
    }
}

/// Validated typed compiler registration for the portable Producer path.
pub struct StaticBindingCompilerRegistration<C> {
    compiler: C,
}

impl<C> StaticBindingCompilerRegistration<C> {
    /// Creates a typed compiler component.
    pub const fn new(compiler: C) -> Self {
        Self { compiler }
    }

    /// Borrows the concrete or application-enum compiler.
    pub const fn compiler(&self) -> &C {
        &self.compiler
    }

    /// Consumes the component and returns the compiler.
    pub fn into_compiler(self) -> C {
        self.compiler
    }
}

impl<C: fmt::Debug> fmt::Debug for StaticBindingCompilerRegistration<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StaticBindingCompilerRegistration")
            .field("compiler", &self.compiler)
            .finish()
    }
}

#[cfg(feature = "std")]
/// Planning-erased host cursor. Its concrete type remains ownership-preserving.
pub struct HostBindingCompilerCursor {
    storage: Box<dyn Any + Send>,
    // Preserve the admitted closed Consumer's prepaid transport allowance.
    // Generic compilers retain their native work contract.
}

#[cfg(feature = "std")]
impl fmt::Debug for HostBindingCompilerCursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostBindingCompilerCursor")
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "std")]
// Only exclusive access to the cursor is exposed. This allows a Send, !Sync
// native cursor to share an allocation with a later Send + Sync artifact,
// without requiring a lock or strengthening the public compiler bounds.
#[cfg(feature = "std")]
struct Exclusive<T>(T);

#[cfg(feature = "std")]
// SAFETY: Exclusive has no shared projection of T. Its value can only be moved
// out through ownership; shared artifact access never projects the cursor.
unsafe impl<T: Send> Sync for Exclusive<T> {}

#[cfg(feature = "std")]
enum HostCompilerSlot<C: BindingCompilerExtension> {
    Vacant,
    Cursor(Exclusive<C::Cursor>),
    Complete(BindingCompilerOutput<C::Artifact>),
}
#[cfg(feature = "std")]
impl<C> HostArtifactStorage for HostCompilerSlot<C>
where
    C: BindingCompilerExtension + 'static,
    C::Cursor: Send + 'static,
    C::Artifact: Send + Sync + 'static,
{
    fn storage_overhead(&self) -> u64 {
        // The native footprint already includes the inline payload. Charge the
        // rest of the retained slot once, including unused cursor capacity.
        (size_of::<Self>() - size_of::<C::Artifact>()) as u64
    }

    fn payload(&self) -> &dyn Any {
        let Self::Complete(output) = self else {
            unreachable!("only a completed slot becomes a Host artifact")
        };
        output.artifact().payload()
    }

    fn take_payload(&mut self, destination: &mut dyn Any) {
        // Type-check before moving the output; no new erasure allocation is
        // needed to extract a native payload, including a zero-sized payload.
        let destination = destination
            .downcast_mut::<Option<C::Artifact>>()
            .expect("payload type checked before extraction");
        let Self::Complete(output) = core::mem::replace(self, Self::Vacant) else {
            unreachable!()
        };
        *destination = Some(output.into_artifact().into_payload());
    }
}

#[cfg(feature = "std")]
fn host_allocation_error() -> CoreError {
    CoreError::Backpressure(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Safe)
            .with_redacted_cause(322, "Host compiler allocation failed"),
    )
}

#[cfg(feature = "std")]
trait ErasedBindingCompiler: Send + Sync {
    fn slot_layout(&self) -> Layout;
    fn compatibility(&self) -> BindingArtifactCompatibility;
    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds>;
    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<HostBindingCompilerCursor>;
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: HostBindingCompilerCursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<HostBindingCompilerCursor, HostBindingArtifact>;
    fn abort(&self, cursor: HostBindingCompilerCursor) -> Result<(), HostBindingCompilerCursor>;
}

#[cfg(feature = "std")]
struct HostCompilerAdapter<C>(C);

#[cfg(feature = "std")]
impl<C> ErasedBindingCompiler for HostCompilerAdapter<C>
where
    C: BindingCompilerExtension + Send + Sync + 'static,
    C::Cursor: Send + 'static,
    C::Artifact: Send + Sync + 'static,
{
    fn slot_layout(&self) -> Layout {
        Layout::new::<HostCompilerSlot<C>>()
    }

    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.0.compatibility()
    }

    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.0.bounds(input)
    }

    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<HostBindingCompilerCursor> {
        // Acquire the complete cursor/output slot before invoking native code.
        // A completed artifact never needs another engine allocation.
        let mut slot =
            try_box(HostCompilerSlot::<C>::Vacant).map_err(|_| host_allocation_error())?;
        *slot = HostCompilerSlot::Cursor(Exclusive(self.0.start(input)?));
        Ok(HostBindingCompilerCursor { storage: slot })
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: HostBindingCompilerCursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<HostBindingCompilerCursor, HostBindingArtifact> {
        let HostBindingCompilerCursor { storage } = cursor;
        let mut slot = match storage.downcast::<HostCompilerSlot<C>>() {
            Ok(slot) => slot,
            Err(storage) => {
                return BindingCompilerStep::Failed(BindingCompilerFailure::new(
                    host_cursor_mismatch(input),
                    HostBindingCompilerCursor { storage },
                ));
            }
        };
        let HostCompilerSlot::Cursor(Exclusive(native)) =
            core::mem::replace(&mut *slot, HostCompilerSlot::Vacant)
        else {
            unreachable!("only a live cursor is returned to its compiler")
        };
        match self.0.step(input, native, budget) {
            BindingCompilerStep::Pending(native) => {
                *slot = HostCompilerSlot::Cursor(Exclusive(native));
                BindingCompilerStep::Pending(HostBindingCompilerCursor { storage: slot })
            }
            BindingCompilerStep::Complete(output) => {
                let compatibility = output.artifact().compatibility();
                let footprint = output.artifact().footprint();
                let reservation = output.artifact().route_reservation();
                *slot = HostCompilerSlot::Complete(output);
                let payload = HostBindingArtifact::from_storage(slot);
                let artifact = match reservation {
                    Some(reservation) => BindingArtifact::producer_route(
                        compatibility,
                        footprint,
                        reservation,
                        payload,
                    ),
                    None => BindingArtifact::new(compatibility, footprint, payload),
                };
                BindingCompilerStep::Complete(BindingCompilerOutput::new(artifact))
            }
            BindingCompilerStep::Failed(failure) => {
                let (error, native) = failure.into_parts();
                *slot = HostCompilerSlot::Cursor(Exclusive(native));
                BindingCompilerStep::Failed(BindingCompilerFailure::new(
                    error,
                    HostBindingCompilerCursor { storage: slot },
                ))
            }
        }
    }

    fn abort(&self, cursor: HostBindingCompilerCursor) -> Result<(), HostBindingCompilerCursor> {
        let HostBindingCompilerCursor { storage } = cursor;
        match storage.downcast::<HostCompilerSlot<C>>() {
            Ok(slot) => {
                let HostCompilerSlot::Cursor(Exclusive(native)) = *slot else {
                    unreachable!()
                };
                self.0.abort(native);
                Ok(())
            }
            Err(storage) => Err(HostBindingCompilerCursor { storage }),
        }
    }
}

#[cfg(feature = "std")]
fn host_cursor_mismatch(input: &BindingCompilerInput<'_>) -> CoreError {
    let candidate = input.candidate();
    CoreError::InternalInvariant(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
            .with_operation(input.operation())
            .with_form_index(input.form_index())
            .with_plan(input.plan_id())
            .with_binding(candidate.binding_id(), candidate.binding_generation()),
    )
}

#[cfg(feature = "std")]
/// Host compiler component with Planning-owned compiler erasure.
pub struct HostBindingCompilerRegistration {
    compiler: Box<dyn ErasedBindingCompiler>,
    open_consumer: Option<ConsumerCompilerMetadata>,
}

#[cfg(feature = "std")]
impl HostBindingCompilerRegistration {
    /// Erases one portable compiler using safe standard-library type identity.
    /// Startup allocation follows the Host deployment failure policy. Use
    /// [`Self::try_new`] when the original compiler must survive refusal.
    pub fn new<C>(compiler: C) -> Self
    where
        C: BindingCompilerExtension + Send + Sync + 'static,
        C::Cursor: Send + 'static,
        C::Artifact: Send + Sync + 'static,
    {
        Self {
            compiler: Box::new(HostCompilerAdapter(compiler)),

            open_consumer: None,
        }
    }

    /// Fallibly erases any portable compiler, returning it unchanged on refusal.
    /// No native callback runs during adapter construction.
    pub fn try_new<C>(compiler: C) -> Result<Self, clinkz_wot_core::BindingInputRejection<C>>
    where
        C: BindingCompilerExtension + Send + Sync + 'static,
        C::Cursor: Send + 'static,
        C::Artifact: Send + Sync + 'static,
    {
        match try_box(HostCompilerAdapter(compiler)) {
            Ok(compiler) => Ok(Self {
                compiler,

                open_consumer: None,
            }),
            Err(HostCompilerAdapter(compiler)) => Err(clinkz_wot_core::BindingInputRejection::new(
                compiler,
                clinkz_wot_core::BindingOperationalError::new(host_allocation_error()),
            )),
        }
    }

    /// General semantic input with the explicitly atomic adapter's own bounds.
    pub fn try_new_semantic<C: HostSemanticCompiler>(
        compiler: C,
    ) -> Result<Self, BindingInputRejection<C>> {
        Self::try_new_consumer_extension(SemanticCompilerAdapter(compiler)).map_err(|rejected| {
            let (adapter, error) = rejected.into_parts();
            BindingInputRejection::new(adapter.0, error)
        })
    }

    /// Capture an independently authored owned/incremental compiler. No callback
    /// eligibility certificate or builtin compiler type is involved.
    pub fn try_new_consumer_extension<C>(compiler: C) -> Result<Self, BindingInputRejection<C>>
    where
        C: ConsumerCompiler + Send + Sync + 'static,
        C::Cursor: Send + 'static,
        C::Artifact: Send + Sync + 'static,
    {
        let configuration = compiler.configuration();
        if configuration.native_compatibility() != compiler.compatibility() {
            return Err(BindingInputRejection::new(
                compiler,
                BindingOperationalError::new(CoreError::Validation(ErrorContext::new(
                    ErrorPhase::Admission,
                    RetryClass::Never,
                ))),
            ));
        }
        let metadata = ConsumerCompilerMetadata {
            configuration,
            contract: C::CONTRACT,
            slot: Layout::new::<HostCompilerSlot<C>>(),
        };
        let mut registration = Self::try_new(compiler)?;
        registration.open_consumer = Some(metadata);
        Ok(registration)
    }
    pub const fn consumer_metadata(&self) -> Option<ConsumerCompilerMetadata> {
        self.open_consumer
    }

    /// Actual retained allocation for one cursor and its completed output.
    /// Includes native alignment, tag and artifact metadata. Native bounds and
    /// private backing are separate; this is not a total-memory declaration.
    pub fn slot_layout(&self) -> Layout {
        self.compiler.slot_layout()
    }

    /// Returns the erased compiler's stable compatibility identity.
    pub fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compiler.compatibility()
    }

    /// Obtains native bounds without beginning compiler progress.
    pub fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.compiler.bounds(input)
    }

    /// Acquires the complete slot fallibly before invoking native start.
    pub fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<HostBindingCompilerCursor> {
        self.compiler.start(input)
    }

    /// Performs one erased compiler step without allocating engine backing.
    pub fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: HostBindingCompilerCursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<HostBindingCompilerCursor, HostBindingArtifact> {
        self.compiler.step(input, cursor, budget)
    }

    /// Aborts a matching cursor, returning an original mismatched cursor.
    pub fn abort(
        &self,
        cursor: HostBindingCompilerCursor,
    ) -> Result<(), HostBindingCompilerCursor> {
        self.compiler.abort(cursor)
    }
}

#[cfg(feature = "std")]
impl fmt::Debug for HostBindingCompilerRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostBindingCompilerRegistration")
            .field("compatibility", &self.compatibility())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use clinkz_wot_foundation::Generation;
    #[cfg(feature = "std")]
    use clinkz_wot_foundation::WorkClass;

    fn plan_and_candidate(
        compatibility: BindingArtifactCompatibility,
    ) -> (LogicalInteractionPlan, BindingCandidate) {
        let plan = LogicalInteractionPlan::try_property_read(
            PlanId::new(SlotIndex::new(1), Generation::INITIAL),
            clinkz_wot_core::ThingId::from("urn:test:artifact"),
            Box::from("temperature"),
            0,
            Box::from("mock://sensor/temperature"),
            Some(Box::from("application/json")),
            None,
        )
        .expect("valid plan");
        let candidate = BindingCandidate::new(
            BindingId::new(2),
            BindingGeneration::INITIAL,
            BindingConfigurationDigest::new([3; 32]),
            compatibility,
            0,
            0,
        );
        (plan, candidate)
    }

    fn route_reservation() -> RouteReservationIdentity {
        RouteReservationIdentity::new(
            clinkz_wot_core::CollisionDomainId::new([21; 16]),
            clinkz_wot_core::EndpointReservationKey::new([22; 32]),
        )
    }

    fn artifact_identity(
        compatibility: BindingArtifactCompatibility,
        role: BindingArtifactRole,
    ) -> BindingArtifactIdentity {
        let (plan, candidate) = plan_and_candidate(compatibility);
        BindingArtifactIdentity::new(
            PlanSetGeneration::INITIAL,
            plan.plan_id(),
            candidate.binding_id(),
            candidate.binding_generation(),
            candidate.configuration(),
            compatibility,
            role,
        )
    }

    #[test]
    fn envelope_rejection_preserves_original_artifact() {
        let expected = BindingArtifactCompatibility::new([4; 16]);
        let other = BindingArtifactCompatibility::new([5; 16]);
        let (plan, candidate) = plan_and_candidate(expected);
        let identity = BindingArtifactIdentity::new(
            PlanSetGeneration::INITIAL,
            plan.plan_id(),
            candidate.binding_id(),
            candidate.binding_generation(),
            candidate.configuration(),
            expected,
            BindingArtifactRole::ConsumerCall,
        );
        let artifact = BindingArtifact::new(other, BindingArtifactFootprint::new(1, 2), 17_u8);
        let rejected = BindingArtifactEnvelope::try_new(
            identity,
            BindingArtifactFootprint::new(1, 2),
            artifact,
        )
        .expect_err("compatibility mismatch must fail");
        assert_eq!(
            rejected.reason(),
            BindingArtifactRejectionReason::CompatibilityMismatch
        );
        let artifact = rejected.into_artifact();
        assert_eq!(artifact.payload(), &17);

        let artifact = BindingArtifact::new(expected, BindingArtifactFootprint::new(2, 3), 17_u8);
        let rejected = BindingArtifactEnvelope::try_new(
            identity,
            BindingArtifactFootprint::new(1, 3),
            artifact,
        )
        .expect_err("footprint mismatch must fail");
        assert_eq!(
            rejected.reason(),
            BindingArtifactRejectionReason::FootprintExceeded
        );
        assert_eq!(rejected.into_artifact().payload(), &17);
    }

    #[test]
    fn envelope_enforces_role_scoped_route_reservation_metadata() {
        let compatibility = BindingArtifactCompatibility::new([4; 16]);
        let footprint = BindingArtifactFootprint::new(1, 2);
        let missing = BindingArtifactEnvelope::try_new(
            artifact_identity(compatibility, BindingArtifactRole::ProducerRoute),
            footprint,
            BindingArtifact::new(compatibility, footprint, 17_u8),
        )
        .expect_err("Producer route without reservation must fail");
        assert_eq!(
            missing.reason(),
            BindingArtifactRejectionReason::MissingRouteReservation
        );
        assert_eq!(missing.into_artifact().route_reservation(), None);

        let unexpected = BindingArtifactEnvelope::try_new(
            artifact_identity(compatibility, BindingArtifactRole::ConsumerCall),
            footprint,
            BindingArtifact::producer_route(compatibility, footprint, route_reservation(), 17_u8),
        )
        .expect_err("Consumer artifact with route reservation must fail");
        assert_eq!(
            unexpected.reason(),
            BindingArtifactRejectionReason::UnexpectedRouteReservation
        );
        assert_eq!(
            unexpected.into_artifact().route_reservation(),
            Some(route_reservation())
        );

        let admitted = BindingArtifactEnvelope::try_new(
            artifact_identity(compatibility, BindingArtifactRole::ProducerRoute),
            footprint,
            BindingArtifact::producer_route(compatibility, footprint, route_reservation(), 17_u8),
        )
        .expect("complete Producer-route metadata");
        assert_eq!(admitted.route_reservation(), Some(route_reservation()));
        assert_eq!(
            admitted.into_artifact().into_route_parts(),
            (compatibility, footprint, Some(route_reservation()), 17_u8)
        );
    }

    #[cfg(feature = "std")]
    #[derive(Clone, Copy)]
    struct OneStepCompiler {
        compatibility: BindingArtifactCompatibility,
    }

    #[cfg(feature = "std")]
    impl BindingCompilerExtension for OneStepCompiler {
        type Cursor = u8;
        type Artifact = u8;

        fn compatibility(&self) -> BindingArtifactCompatibility {
            self.compatibility
        }

        fn bounds(&self, _input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
            Ok(BindingCompilerBounds::new(
                BindingArtifactFootprint::new(1, 1),
                1,
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
            let footprint = BindingArtifactFootprint::new(1, 1);
            let artifact = if input.role() == BindingArtifactRole::ProducerRoute {
                BindingArtifact::producer_route(
                    self.compatibility,
                    footprint,
                    route_reservation(),
                    cursor,
                )
            } else {
                BindingArtifact::new(self.compatibility, footprint, cursor)
            };
            BindingCompilerStep::Complete(BindingCompilerOutput::new(artifact))
        }

        fn abort(&self, _cursor: Self::Cursor) {}
    }

    #[cfg(feature = "std")]
    #[derive(Clone, Copy)]
    struct AlternateOneStepCompiler {
        compatibility: BindingArtifactCompatibility,
    }

    #[cfg(feature = "std")]
    impl BindingCompilerExtension for AlternateOneStepCompiler {
        type Cursor = u16;
        type Artifact = u16;

        fn compatibility(&self) -> BindingArtifactCompatibility {
            self.compatibility
        }

        fn bounds(&self, _input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
            Ok(BindingCompilerBounds::new(
                BindingArtifactFootprint::new(1, 2),
                2,
                0,
                WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1),
            ))
        }

        fn start(&self, _input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
            Ok(7)
        }

        fn step(
            &self,
            _input: &BindingCompilerInput<'_>,
            cursor: Self::Cursor,
            budget: &mut WorkBudget,
        ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return BindingCompilerStep::Pending(cursor);
            }
            BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                self.compatibility,
                BindingArtifactFootprint::new(1, 2),
                cursor,
            )))
        }

        fn abort(&self, _cursor: Self::Cursor) {}
    }

    #[cfg(feature = "std")]
    #[test]
    fn host_erasure_returns_mismatched_cursor_and_payload() {
        let first_compatibility = BindingArtifactCompatibility::new([6; 16]);
        let second_compatibility = BindingArtifactCompatibility::new([7; 16]);
        let first = HostBindingCompilerRegistration::new(OneStepCompiler {
            compatibility: first_compatibility,
        });
        let second = HostBindingCompilerRegistration::new(AlternateOneStepCompiler {
            compatibility: second_compatibility,
        });
        let (plan, candidate) = plan_and_candidate(first_compatibility);
        let input = BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);
        let cursor = second.start(&input).expect("second cursor");
        let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 2);

        let cursor = match first.step(&input, cursor, &mut budget) {
            BindingCompilerStep::Failed(failure) => failure.into_parts().1,
            _ => panic!("mismatched cursor was not returned"),
        };
        let artifact = match second.step(&input, cursor, &mut budget) {
            BindingCompilerStep::Complete(output) => output.into_artifact(),
            _ => panic!("returned cursor no longer worked with its owner"),
        };
        assert!(artifact.try_payload::<u8>(first_compatibility).is_none());
        let artifact = artifact
            .try_into_payload::<u8>(second_compatibility)
            .expect_err("payload type mismatch must preserve artifact");
        assert_eq!(
            artifact
                .try_into_payload::<u16>(second_compatibility)
                .expect("matching payload"),
            7
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn host_erasure_preserves_route_reservation_metadata() {
        let compatibility = BindingArtifactCompatibility::new([6; 16]);
        let compiler = HostBindingCompilerRegistration::new(OneStepCompiler { compatibility });
        let (plan, candidate) = plan_and_candidate(compatibility);
        let input = BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ProducerRoute);
        let cursor = compiler.start(&input).expect("Producer-route cursor");
        let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
        let artifact = match compiler.step(&input, cursor, &mut budget) {
            BindingCompilerStep::Complete(output) => output.into_artifact(),
            _ => panic!("Producer-route host compiler did not complete"),
        };
        assert_eq!(artifact.route_reservation(), Some(route_reservation()));
        let artifact = artifact
            .try_into_payload::<u16>(compatibility)
            .expect_err("payload mismatch must preserve complete artifact");
        assert_eq!(artifact.route_reservation(), Some(route_reservation()));
        let (_, _, reservation, payload) = artifact.into_route_parts();
        assert_eq!(reservation, Some(route_reservation()));
        assert_eq!(
            payload
                .into_payload::<u8>()
                .expect("matching erased payload"),
            7
        );
    }
}
