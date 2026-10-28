//! Pure binding preparation. Runtime envelopes and storage live in Core.
#[cfg(feature = "std")]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use clinkz_wot_core::artifact::try_box;
use clinkz_wot_core::*;
#[cfg(test)]
use clinkz_wot_foundation::SlotIndex;
use clinkz_wot_foundation::{WorkBudget, WorkClass};
#[cfg(feature = "std")]
use core::any::Any;
use core::{alloc::Layout, fmt, mem::size_of};
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
    additional_metadata: Option<bool>,
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
            additional_metadata: plan.has_additional_form_metadata(),
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
    pub const fn has_additional_form_metadata(&self) -> Option<bool> {
        self.additional_metadata
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
            additional_metadata: None,
            semantics: Some(facts),
        }
    }
    #[cfg(feature = "std")]
    pub fn with_semantics(
        mut self,
        document: &'a clinkz_wot_td::semantic::ResolvedSemanticDocument<'a>,
    ) -> Self {
        if document.form_facts(self.property, self.form).is_some() {
            self.semantics = Some(ResolvedBindingInput {
                operation: self.operation,
                owner: clinkz_wot_td::semantic::FormOwner::Property(self.property),
                property: self.property,
                form_index: self.form,
                target: self.target,
                content_type: self.content_type,
                subprotocol: self.subprotocol,
                document,
            });
        }
        self
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
    fn schemes(&self) -> &'static [&'static str];
    fn compile(&self, input: ResolvedBindingInput<'_>) -> CoreResult<Self::Artifact>;
}

#[cfg(feature = "std")]
#[derive(Clone, Copy, Debug)]
struct NativeConsumerSupport {
    compatibility: BindingArtifactCompatibility,
    schemes: &'static [&'static str],
    work: u64,
    cursor: Layout,
    artifact: Layout,
    output: Layout,
    temporary: Layout,
    adapter: Layout,
    slot: Layout,
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
                return Err(resolved_target_error(input));
            }
            self.0.compile(
                input
                    .resolved()
                    .ok_or_else(|| resolved_target_error(input))?,
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
            return Err(resolved_target_error(input));
        }
        Ok(())
    }
    fn check(&self, input: &BindingCompilerInput<'_>) -> CoreResult<()> {
        self.check_identity(input)?;
        if input.resolved().is_none() {
            return Err(resolved_target_error(input));
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

const RESOLVED_TARGET_CAPACITY: usize = 64;
const RESOLVED_TARGET_COMPATIBILITY: BindingArtifactCompatibility =
    BindingArtifactCompatibility::new(*b"clinkz-target-v1");
const RESOLVED_TARGET_STEP_WORK: u64 = 2;

// A conservative ownership-overlap allowance, not a measured machine stack
// frame. Include the input cursor, result transport, output under construction,
// inline copy and structured error even though not all branches need all five.
type ConsumerCompilerTemporary = (
    ResolvedTargetCompilerCursor,
    BindingCompilerStep<ResolvedTargetCompilerCursor, ResolvedTargetArtifact>,
    BindingCompilerOutput<ResolvedTargetArtifact>,
    ResolvedTargetArtifact,
    CoreError,
);

pub(crate) const fn resolved_target_compatibility() -> BindingArtifactCompatibility {
    RESOLVED_TARGET_COMPATIBILITY
}
const fn form_eligibility_work(configuration: Option<ConsumerBindingConfiguration>) -> u64 {
    match configuration {
        Some(configuration) => configuration.requires_default_json_forms() as u64,
        None => 0,
    }
}

const fn strict_configuration(capacity: u8) -> BindingConfigurationDigest {
    let mut bytes = [0; 32];
    bytes[0] = 0x63;
    bytes[1] = 0x74;
    bytes[2] = 1;
    bytes[3] = capacity;
    BindingConfigurationDigest::new(bytes)
}

/// Closed, allocation-free copy of an already resolved Consumer target.
///
/// The capacity is immutable and participates in the configuration digest.
/// This primitive supplies no TD, URI, selection, or protocol interpretation.
#[derive(Debug)]
pub struct ResolvedTargetCompiler {
    capacity: u8,
    owned: Option<ConsumerBindingConfiguration>,
}

impl ResolvedTargetCompiler {
    /// Checks the actual target capacity, in `1..=64` UTF-8 bytes.
    pub fn try_new(capacity: usize) -> CoreResult<Self> {
        if !(1..=RESOLVED_TARGET_CAPACITY).contains(&capacity) {
            return Err(CoreError::Validation(
                ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
                    .with_redacted_cause(320, "resolved target capacity must be in 1..=64"),
            ));
        }
        Ok(Self {
            capacity: capacity as u8,
            owned: None,
        })
    }

    /// Constructs the same target-copy compiler from checked owned metadata.
    pub const fn new_owned(configuration: ConsumerBindingConfiguration) -> Self {
        Self {
            capacity: configuration.target_capacity() as u8,
            owned: Some(configuration),
        }
    }

    /// Returns the digest of the actual private configuration and format version.
    pub const fn configuration(&self) -> BindingConfigurationDigest {
        match self.owned {
            Some(configuration) => configuration.configuration(),
            None => strict_configuration(self.capacity),
        }
    }

    fn check(&self, input: &BindingCompilerInput<'_>) -> CoreResult<()> {
        if input.role() != BindingArtifactRole::ConsumerCall
            || input.candidate().configuration() != self.configuration()
            || input.candidate().compatibility() != RESOLVED_TARGET_COMPATIBILITY
            || input.resolved_target().len() > usize::from(self.capacity)
        {
            return Err(resolved_target_error(input));
        }
        if form_eligibility_work(self.owned) != 0
            && (input.has_additional_form_metadata() != Some(false)
                || input.content_type() != Some("application/json"))
        {
            return Err(CoreError::UnsupportedOperation(
                resolved_target_context(input)
                    .with_redacted_cause(328, "Consumer compiler requires default JSON Forms"),
            ));
        }
        Ok(())
    }

    pub(crate) fn support(&self, host_erased: bool) -> InlineConsumerCompilerSupport {
        InlineConsumerCompilerSupport {
            capacity: self.capacity,
            host_erased,
            owned: self.owned,
            #[cfg(feature = "std")]
            native: None,
        }
    }
}

/// Fixed native continuation and coordinate identity; never an input pointer.
#[derive(Debug, Eq, PartialEq)]
pub struct ResolvedTargetCompilerCursor {
    plan: PlanId,
    candidate: BindingCandidate,
    remaining: u8,
    pending: bool,
}

/// Owned resolved UTF-8 target in fixed native storage.
#[derive(Debug, Eq, PartialEq)]
pub struct ResolvedTargetArtifact {
    bytes: [u8; RESOLVED_TARGET_CAPACITY],
    len: u8,
}

impl ResolvedTargetArtifact {
    /// Borrows the target after the input and registration have been destroyed.
    pub fn target(&self) -> &str {
        core::str::from_utf8(&self.bytes[..usize::from(self.len)])
            .expect("closed compiler copies a complete valid UTF-8 string")
    }
}

fn resolved_target_error(input: &BindingCompilerInput<'_>) -> CoreError {
    CoreError::Validation(
        resolved_target_context(input)
            .with_redacted_cause(321, "input does not match the closed Consumer compiler"),
    )
}

fn resolved_target_context(input: &BindingCompilerInput<'_>) -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
        .with_operation(input.operation())
        .with_form_index(input.form_index())
        .with_plan(input.plan_id())
        .with_binding(
            input.candidate().binding_id(),
            input.candidate().binding_generation(),
        )
}

impl BindingCompilerExtension for ResolvedTargetCompiler {
    type Cursor = ResolvedTargetCompilerCursor;
    type Artifact = ResolvedTargetArtifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        #[cfg(all(test, feature = "std"))]
        consumer_trace::hit(0);
        RESOLVED_TARGET_COMPATIBILITY
    }

    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        #[cfg(all(test, feature = "std"))]
        consumer_trace::hit(1);
        self.check(input)?;
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, size_of::<ResolvedTargetArtifact>() as u64),
            size_of::<ResolvedTargetCompilerCursor>() as u64,
            size_of::<ConsumerCompilerTemporary>() as u64,
            WorkBudget::new().with_remaining(
                WorkClass::BindingPolls,
                2 * (RESOLVED_TARGET_STEP_WORK + form_eligibility_work(self.owned)),
            ),
        ))
    }

    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        #[cfg(all(test, feature = "std"))]
        consumer_trace::hit(2);
        self.check(input)?;
        Ok(ResolvedTargetCompilerCursor {
            plan: input.plan_id(),
            candidate: input.candidate(),
            remaining: 2,
            pending: true,
        })
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        mut cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        #[cfg(all(test, feature = "std"))]
        consumer_trace::hit(3);
        // Unpaid retries cannot inspect/advance continuation or refill its
        // lifetime allowance. Even a fresh caller budget cannot reset it.
        let step_work = RESOLVED_TARGET_STEP_WORK + form_eligibility_work(self.owned);
        if budget.remaining(WorkClass::BindingPolls) < step_work {
            return BindingCompilerStep::Pending(cursor);
        }
        if cursor.remaining == 0 {
            return BindingCompilerStep::Failed(BindingCompilerFailure::new(
                resolved_target_error(input),
                cursor,
            ));
        }
        budget.consume(WorkClass::BindingPolls, step_work).unwrap();
        cursor.remaining -= 1;
        if let Err(cause) = self.check(input) {
            return BindingCompilerStep::Failed(BindingCompilerFailure::new(cause, cursor));
        }
        if cursor.plan != input.plan_id() || cursor.candidate != input.candidate() {
            return BindingCompilerStep::Failed(BindingCompilerFailure::new(
                resolved_target_error(input),
                cursor,
            ));
        }
        if cursor.pending {
            cursor.pending = false;
            return BindingCompilerStep::Pending(cursor);
        }
        let target = input.resolved_target().as_bytes();
        let mut payload = ResolvedTargetArtifact {
            bytes: [0; RESOLVED_TARGET_CAPACITY],
            len: target.len() as u8,
        };
        payload.bytes[..target.len()].copy_from_slice(target);
        BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
            RESOLVED_TARGET_COMPATIBILITY,
            BindingArtifactFootprint::new(1, size_of::<ResolvedTargetArtifact>() as u64),
            payload,
        )))
    }

    fn abort(&self, _cursor: Self::Cursor) {
        #[cfg(all(test, feature = "std"))]
        consumer_trace::hit(4);
    }
}

/// Read-only source-owned costs and layouts, attached to one complete owner.
///
/// This value cannot be constructed by a binding, transferred to certify a
/// different registration, or used to attest an arbitrary compiler. Costs are
/// fixed work units, not timings. Callers prepay bounds/compatibility/start in
/// `BindingPolls`, and allocation/abort/destruction/release in `CleanupItems`.
/// Only step debits its supplied budget; other SPI signatures are unchanged.
/// Startup storage and per-coordinate storage are separate accounts.
///
/// ```compile_fail
/// # use clinkz_wot_planning::InlineConsumerCompilerSupport;
/// let unsupported = InlineConsumerCompilerSupport { capacity: 64, host_erased: false };
/// ```
#[derive(Debug)]
pub struct InlineConsumerCompilerSupport {
    #[cfg(feature = "std")]
    native: Option<NativeConsumerSupport>,
    capacity: u8,
    host_erased: bool,
    owned: Option<ConsumerBindingConfiguration>,
}

impl InlineConsumerCompilerSupport {
    #[cfg(feature = "std")]
    pub(crate) const fn configuration(&self) -> BindingConfigurationDigest {
        match self.owned {
            Some(configuration) => configuration.configuration(),
            None => strict_configuration(self.capacity),
        }
    }
    /// Whether this compiler consumes the common Host semantic view.
    pub const fn uses_semantics(&self) -> bool {
        #[cfg(feature = "std")]
        {
            return self.native.is_some();
        }
        #[cfg(not(feature = "std"))]
        {
            false
        }
    }
    /// Coarse scheme applicability only; no vocabulary ranking or interpretation.
    pub fn accepts_target(&self, target: &str) -> bool {
        #[cfg(feature = "std")]
        if let Some(native) = &self.native {
            return target.split_once(':').is_some_and(|(scheme, _)| {
                native
                    .schemes
                    .iter()
                    .any(|s| scheme.eq_ignore_ascii_case(s))
            });
        }
        let _ = target;
        true
    }
    #[cfg(feature = "std")]
    pub(crate) const fn compatibility(&self) -> BindingArtifactCompatibility {
        #[cfg(feature = "std")]
        if let Some(native) = &self.native {
            return native.compatibility;
        }
        RESOLVED_TARGET_COMPATIBILITY
    }
    /// Projects the actual immutable owned configuration; strict support has none.
    pub const fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
        self.owned
    }
    /// Returns the actual checked target capacity in bytes.
    pub const fn target_capacity(&self) -> usize {
        #[cfg(feature = "std")]
        if let Some(configuration) = self.owned {
            if configuration.native_compatibility().is_some() {
                return configuration.target_capacity();
            }
        }
        self.capacity as usize
    }
    /// Fixed cost of an explicitly invoked compatibility callback.
    pub const fn compatibility_work(&self) -> u64 {
        1
    }
    /// Fixed pre-bounds cost, known before invoking bounds.
    pub const fn bounds_work(&self) -> u64 {
        #[cfg(feature = "std")]
        if self.native.is_some() {
            return 1;
        }
        1 + form_eligibility_work(self.owned)
    }
    /// Fixed native start cost, separate from Host allocation work.
    pub const fn start_work(&self) -> u64 {
        #[cfg(feature = "std")]
        if self.native.is_some() {
            return 1;
        }
        1 + form_eligibility_work(self.owned)
    }
    /// Total work per paid step, including supported Host transport when present.
    pub const fn step_work(&self) -> u64 {
        #[cfg(feature = "std")]
        if let Some(native) = &self.native {
            return native.work + 1;
        }
        RESOLVED_TARGET_STEP_WORK + form_eligibility_work(self.owned) + self.host_erased as u64
    }
    /// Fixed native abort cost for an owned cursor.
    pub const fn abort_work(&self) -> u64 {
        1
    }
    /// Fixed destruction cost of the native, allocation-free output.
    pub const fn destruction_work(&self) -> u64 {
        1
    }
    /// Fixed per-coordinate physical acquisition work; zero in the static cell.
    pub const fn allocation_work(&self) -> u64 {
        self.host_erased as u64
    }
    /// Fixed per-coordinate backing release work; zero in the static cell.
    pub const fn release_work(&self) -> u64 {
        self.host_erased as u64
    }
    /// Exact fixed native cursor layout.
    pub const fn cursor_layout(&self) -> Layout {
        #[cfg(feature = "std")]
        if let Some(native) = &self.native {
            return native.cursor;
        }
        Layout::new::<ResolvedTargetCompilerCursor>()
    }
    /// Exact binding-authored payload layout (included once in a Host slot).
    pub const fn artifact_layout(&self) -> Layout {
        #[cfg(feature = "std")]
        if let Some(native) = &self.native {
            return native.artifact;
        }
        Layout::new::<ResolvedTargetArtifact>()
    }
    /// Exact native output layout, including Core artifact metadata.
    pub const fn output_layout(&self) -> Layout {
        #[cfg(feature = "std")]
        if let Some(native) = &self.native {
            return native.output;
        }
        Layout::new::<BindingCompilerOutput<ResolvedTargetArtifact>>()
    }
    /// Conservative callback/transport overlap in addition to retained backing.
    /// This includes native output while the Host slot still exists. It is a
    /// fixed ownership bound, not target-specific machine stack measurement.
    pub const fn temporary_layout(&self) -> Layout {
        #[cfg(feature = "std")]
        if let Some(native) = &self.native {
            return native.temporary;
        }
        #[cfg(feature = "std")]
        if self.host_erased {
            return Layout::new::<(
                ConsumerCompilerTemporary,
                BindingCompilerStep<HostBindingCompilerCursor, HostBindingArtifact>,
            )>();
        }
        Layout::new::<ConsumerCompilerTemporary>()
    }
    /// Exact startup adapter allocation, separate from per-coordinate admission.
    #[cfg(feature = "std")]
    pub const fn host_adapter_layout(&self) -> Option<Layout> {
        #[cfg(feature = "std")]
        if let Some(native) = &self.native {
            return Some(native.adapter);
        }
        if self.host_erased {
            Some(Layout::new::<ResolvedTargetCompiler>())
        } else {
            None
        }
    }
    /// Exact retained Host allocation, including tag, metadata, and payload.
    #[cfg(feature = "std")]
    pub const fn host_slot_layout(&self) -> Option<Layout> {
        #[cfg(feature = "std")]
        if let Some(native) = &self.native {
            return Some(native.slot);
        }
        if self.host_erased {
            Some(Layout::new::<HostCompilerSlot<ResolvedTargetCompiler>>())
        } else {
            None
        }
    }
    /// Exact outer Host cursor owner; additional to, not inside, its slot.
    #[cfg(feature = "std")]
    pub const fn host_cursor_owner_layout(&self) -> Option<Layout> {
        if self.host_erased {
            Some(Layout::new::<HostBindingCompilerCursor>())
        } else {
            None
        }
    }
    /// Exact outer completed artifact owner; additional to its retained slot.
    #[cfg(feature = "std")]
    pub const fn host_output_owner_layout(&self) -> Option<Layout> {
        if self.host_erased {
            Some(Layout::new::<BindingCompilerOutput<HostBindingArtifact>>())
        } else {
            None
        }
    }
}

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
    consumer_steps: Option<u8>,
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
        Ok(HostBindingCompilerCursor {
            storage: slot,
            consumer_steps: None,
        })
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: HostBindingCompilerCursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<HostBindingCompilerCursor, HostBindingArtifact> {
        let HostBindingCompilerCursor {
            storage,
            consumer_steps,
        } = cursor;
        let mut slot = match storage.downcast::<HostCompilerSlot<C>>() {
            Ok(slot) => slot,
            Err(storage) => {
                return BindingCompilerStep::Failed(BindingCompilerFailure::new(
                    host_cursor_mismatch(input),
                    HostBindingCompilerCursor {
                        storage,
                        consumer_steps,
                    },
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
                BindingCompilerStep::Pending(HostBindingCompilerCursor {
                    storage: slot,
                    consumer_steps,
                })
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
                    HostBindingCompilerCursor {
                        storage: slot,
                        consumer_steps,
                    },
                ))
            }
        }
    }

    fn abort(&self, cursor: HostBindingCompilerCursor) -> Result<(), HostBindingCompilerCursor> {
        let HostBindingCompilerCursor {
            storage,
            consumer_steps,
        } = cursor;
        match storage.downcast::<HostCompilerSlot<C>>() {
            Ok(slot) => {
                let HostCompilerSlot::Cursor(Exclusive(native)) = *slot else {
                    unreachable!()
                };
                self.0.abort(native);
                Ok(())
            }
            Err(storage) => Err(HostBindingCompilerCursor {
                storage,
                consumer_steps,
            }),
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
    consumer_support: Option<InlineConsumerCompilerSupport>,
    open_consumer: Option<ConsumerCompilerMetadata>,
}

#[cfg(feature = "std")]
impl HostBindingCompilerRegistration {
    /// Captures an independently authored native compiler with actual layouts.
    /// The complete registration subsequently checks its client/config pairing.
    pub fn try_new_inline_semantic<C: HostSemanticCompiler>(
        compiler: C,
    ) -> Result<Self, clinkz_wot_core::BindingInputRejection<C>> {
        let configuration = compiler.configuration();
        let compatibility = compiler.compatibility();
        if C::WORK == 0
            || C::WORK == u64::MAX
            || compiler.schemes().is_empty()
            || configuration.native_compatibility() != Some(compatibility)
        {
            return Err(clinkz_wot_core::BindingInputRejection::new(
                compiler,
                clinkz_wot_core::BindingOperationalError::new(CoreError::Validation(
                    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never),
                )),
            ));
        }
        let native = NativeConsumerSupport {
            compatibility,
            schemes: compiler.schemes(),
            work: C::WORK,
            cursor: Layout::new::<SemanticCompilerCursor>(),
            artifact: Layout::new::<C::Artifact>(),
            output: Layout::new::<BindingCompilerOutput<C::Artifact>>(),
            temporary: Layout::new::<(
                BindingCompilerStep<SemanticCompilerCursor, C::Artifact>,
                BindingCompilerStep<HostBindingCompilerCursor, HostBindingArtifact>,
                CoreError,
            )>(),
            adapter: Layout::new::<SemanticCompilerAdapter<C>>(),
            slot: Layout::new::<HostCompilerSlot<SemanticCompilerAdapter<C>>>(),
        };
        let support = InlineConsumerCompilerSupport {
            native: Some(native),
            capacity: configuration.target_capacity() as u8,
            host_erased: true,
            owned: Some(configuration),
        };
        match Self::try_new_consumer_extension(SemanticCompilerAdapter(compiler)) {
            Ok(mut registration) => {
                registration.consumer_support = Some(support);
                // The inline wrapper adds different progress costs. Its
                // original generic metadata must not claim open eligibility.
                registration.open_consumer = None;
                Ok(registration)
            }
            Err(rejected) => {
                let (adapter, error) = rejected.into_parts();
                Err(clinkz_wot_core::BindingInputRejection::new(
                    adapter.0, error,
                ))
            }
        }
    }
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
            consumer_support: None,
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
                consumer_support: None,
                open_consumer: None,
            }),
            Err(HostCompilerAdapter(compiler)) => Err(clinkz_wot_core::BindingInputRejection::new(
                compiler,
                clinkz_wot_core::BindingOperationalError::new(host_allocation_error()),
            )),
        }
    }

    /// General semantic input with the explicitly atomic adapter's own bounds.
    /// Consumer admission does not depend on its private inline support record.
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
        #[cfg(feature = "std")]
        if configuration
            .native_compatibility()
            .is_some_and(|native| native != compiler.compatibility())
        {
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

    /// Captures the existing closed Consumer contract using the generic adapter.
    /// This constructor does not widen Consumer compiler eligibility.
    pub fn try_new_inline_consumer(
        compiler: ResolvedTargetCompiler,
    ) -> Result<Self, clinkz_wot_core::BindingInputRejection<ResolvedTargetCompiler>> {
        let support = compiler.support(true);
        let mut registration = Self::try_new(compiler)?;
        registration.consumer_support = Some(support);
        Ok(registration)
    }

    pub(crate) fn consumer_support(&self) -> Option<InlineConsumerCompilerSupport> {
        self.consumer_support
            .as_ref()
            .map(|support| InlineConsumerCompilerSupport {
                capacity: support.capacity,
                host_erased: support.host_erased,
                owned: support.owned,
                native: support.native,
            })
    }

    /// Actual retained allocation for one cursor and its completed output.
    /// Includes native alignment, tag and artifact metadata. Native bounds and
    /// private backing are separate; this is not a total-memory declaration.
    pub fn slot_layout(&self) -> Layout {
        self.compiler.slot_layout()
    }

    /// Returns the erased compiler's stable compatibility identity.
    pub fn compatibility(&self) -> BindingArtifactCompatibility {
        // Closed compatibility is already captured; preserve callback costs.
        if let Some(support) = &self.consumer_support {
            support.compatibility()
        } else {
            self.compiler.compatibility()
        }
    }

    /// Obtains native bounds without beginning compiler progress.
    pub fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        let native = self.compiler.bounds(input)?;
        match &self.consumer_support {
            None => Ok(native),
            Some(support) => Ok(BindingCompilerBounds::new(
                native.artifact(),
                native.cursor_bytes(),
                support.temporary_layout().size() as u64,
                WorkBudget::new().with_remaining(
                    WorkClass::BindingPolls,
                    (if support.uses_semantics() { 1 } else { 2 }) * support.step_work(),
                ),
            )),
        }
    }

    /// Acquires the complete slot fallibly before invoking native start.
    pub fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<HostBindingCompilerCursor> {
        let mut cursor = self.compiler.start(input)?;
        cursor.consumer_steps = self
            .consumer_support
            .as_ref()
            .map(|s| if s.uses_semantics() { 1 } else { 2 });
        Ok(cursor)
    }

    /// Performs one erased compiler step without allocating engine backing.
    pub fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        mut cursor: HostBindingCompilerCursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<HostBindingCompilerCursor, HostBindingArtifact> {
        if let Some(support) = &self.consumer_support {
            if budget.remaining(WorkClass::BindingPolls) < support.step_work() {
                return BindingCompilerStep::Pending(cursor);
            }
        }
        if cursor.consumer_steps.is_some() != self.consumer_support.is_some() {
            return BindingCompilerStep::Failed(BindingCompilerFailure::new(
                host_cursor_mismatch(input),
                cursor,
            ));
        }
        if let Some(remaining) = &mut cursor.consumer_steps {
            if *remaining == 0 {
                return BindingCompilerStep::Failed(BindingCompilerFailure::new(
                    resolved_target_error(input),
                    cursor,
                ));
            }
            budget.consume(WorkClass::BindingPolls, 1).unwrap();
            *remaining -= 1;
        }
        self.compiler.step(input, cursor, budget)
    }

    /// Aborts a matching cursor, returning an original mismatched cursor.
    pub fn abort(
        &self,
        cursor: HostBindingCompilerCursor,
    ) -> Result<(), HostBindingCompilerCursor> {
        if cursor.consumer_steps.is_some() != self.consumer_support.is_some() {
            return Err(cursor);
        }
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

// Private instrumentation is compiled only into Core's std unit-test binary.
// External completion tests independently inject failure through the actual
// global allocator and observe exact production Layouts and release addresses.
#[cfg(all(test, feature = "std"))]
mod consumer_trace {
    use super::*;
    use std::{
        alloc::{GlobalAlloc, System},
        cell::Cell,
    };

    std::thread_local! {
        static CALLS: Cell<[usize; 5]> = const { Cell::new([0; 5]) };
        static FAIL_NEXT: Cell<bool> = const { Cell::new(false) };
    }
    pub(super) fn hit(index: usize) {
        CALLS.with(|cell| {
            let mut calls = cell.get();
            calls[index] += 1;
            cell.set(calls);
        });
    }
    fn calls() -> [usize; 5] {
        CALLS.with(Cell::get)
    }
    struct Allocator;
    #[global_allocator]
    static ALLOCATOR: Allocator = Allocator;
    // SAFETY: System delegation preserves every pointer and Layout. The armed
    // thread-local failure returns null at one known fallible allocation only.
    unsafe impl GlobalAlloc for Allocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            if FAIL_NEXT
                .try_with(|cell| cell.replace(false))
                .unwrap_or(false)
            {
                std::ptr::null_mut()
            } else {
                unsafe { System.alloc(layout) }
            }
        }
        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            unsafe { System.dealloc(pointer, layout) }
        }
    }

    #[test]
    fn allocation_precedes_native_start_and_short_host_retries_invoke_no_callback() {
        let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
        let plan = LogicalInteractionPlan::try_property_read(
            PlanId::new(
                SlotIndex::new(7),
                clinkz_wot_foundation::Generation::INITIAL,
            ),
            clinkz_wot_core::ThingId::from("urn:trace:compiler"),
            "temperature".into(),
            0,
            "mock://s/temperature".into(),
            None,
            None,
        )
        .unwrap();
        let candidate = BindingCandidate::new(
            BindingId::new(9),
            BindingGeneration::INITIAL,
            compiler.configuration(),
            RESOLVED_TARGET_COMPATIBILITY,
            5,
            7,
        );
        let input = BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);
        CALLS.with(|cell| cell.set([0; 5]));
        FAIL_NEXT.with(|cell| cell.set(true));
        let rejection =
            HostBindingCompilerRegistration::try_new_inline_consumer(compiler).unwrap_err();
        assert_eq!(calls(), [0; 5]);
        let host = HostBindingCompilerRegistration::try_new_inline_consumer(rejection.into_input())
            .unwrap();
        let support = host.consumer_support().unwrap();
        assert_eq!(host.compatibility(), RESOLVED_TARGET_COMPATIBILITY);
        assert_eq!(support.target_capacity(), 64);
        assert_eq!(calls(), [0; 5]); // Captured immutable metadata, no callback.
        FAIL_NEXT.with(|cell| cell.set(true));
        assert!(host.start(&input).is_err());
        assert_eq!(calls(), [0; 5]); // Null acquisition never reaches native start.
        let mut cursor = host.start(&input).unwrap();
        assert_eq!(calls(), [0, 0, 1, 0, 0]);
        let slot = cursor
            .storage
            .downcast_ref::<HostCompilerSlot<ResolvedTargetCompiler>>()
            .unwrap();
        let address = slot as *const HostCompilerSlot<ResolvedTargetCompiler>;
        for _ in 0..4 {
            for units in 0..support.step_work() {
                let BindingCompilerStep::Pending(returned) = host.step(
                    &input,
                    cursor,
                    &mut WorkBudget::new().with_remaining(WorkClass::BindingPolls, units),
                ) else {
                    panic!("unpaid Pending")
                };
                cursor = returned;
                assert_eq!(calls(), [0, 0, 1, 0, 0]);
                let slot = cursor
                    .storage
                    .downcast_ref::<HostCompilerSlot<ResolvedTargetCompiler>>()
                    .unwrap();
                assert_eq!(
                    slot as *const HostCompilerSlot<ResolvedTargetCompiler>,
                    address
                );
            }
        }
        let BindingCompilerStep::Pending(cursor) = host.step(
            &input,
            cursor,
            &mut WorkBudget::new().with_remaining(WorkClass::BindingPolls, support.step_work()),
        ) else {
            panic!("paid Pending")
        };
        assert_eq!(calls(), [0, 0, 1, 1, 0]);
        let slot = cursor
            .storage
            .downcast_ref::<HostCompilerSlot<ResolvedTargetCompiler>>()
            .unwrap();
        assert_eq!(
            slot as *const HostCompilerSlot<ResolvedTargetCompiler>,
            address
        );
        let changed =
            BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ProducerRoute);
        let BindingCompilerStep::Failed(failure) = host.step(
            &changed,
            cursor,
            &mut WorkBudget::new().with_remaining(WorkClass::BindingPolls, support.step_work()),
        ) else {
            panic!("paid failure")
        };
        assert_eq!(calls(), [0, 0, 1, 2, 0]);
        let mut cursor = failure.into_parts().1;
        for _ in 0..3 {
            let BindingCompilerStep::Failed(failure) = host.step(
                &input,
                cursor,
                &mut WorkBudget::new().with_remaining(WorkClass::BindingPolls, support.step_work()),
            ) else {
                panic!("exhausted lifetime remainder")
            };
            cursor = failure.into_parts().1;
            assert_eq!(calls(), [0, 0, 1, 2, 0]);
            let slot = cursor
                .storage
                .downcast_ref::<HostCompilerSlot<ResolvedTargetCompiler>>()
                .unwrap();
            assert_eq!(
                slot as *const HostCompilerSlot<ResolvedTargetCompiler>,
                address
            );
        }
        host.abort(cursor).unwrap();
        assert_eq!(calls(), [0, 0, 1, 2, 1]);
        // Native cursor and payload have no destructor callback, allocation or
        // external cleanup; abort consumes pure state once, release frees once.
        assert!(!core::mem::needs_drop::<ResolvedTargetCompilerCursor>());
        assert!(!core::mem::needs_drop::<ResolvedTargetArtifact>());
    }
}
