//! Portable binding-compiler and immutable artifact contracts.

#[cfg(feature = "std")]
use alloc::boxed::Box;
use core::{alloc::Layout, fmt, mem::size_of};

#[cfg(feature = "std")]
use std::any::Any;

use clinkz_wot_foundation::{SlotIndex, WorkBudget, WorkClass};

use crate::{
    BindingCandidate, BindingConfigurationDigest, BindingGeneration, BindingId,
    BindingResourceContract, CoreError, CoreResult, LogicalInteractionPlan, PlanId,
    PlanSetGeneration, RouteReservationIdentity,
};
use crate::{ErrorContext, ErrorPhase, RetryClass};

/// Stable compatibility identity shared by one compiler and its artifacts.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct BindingArtifactCompatibility([u8; 16]);

impl BindingArtifactCompatibility {
    /// Creates an identity from its complete fixed-width representation.
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the complete fixed-width representation.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// Execution role for which an immutable artifact was compiled.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BindingArtifactRole {
    /// Consumer request/call preparation.
    ConsumerCall,
    /// Consumer subscription preparation.
    ConsumerSubscription,
    /// Producer inbound route preparation.
    ProducerRoute,
    /// Producer publication preparation.
    ProducerPublication,
}

/// Measured retained lifetime footprint of one compiled artifact.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BindingArtifactFootprint {
    retained_items: u32,
    retained_bytes: u64,
}

impl BindingArtifactFootprint {
    /// Creates an exact measured or admitted footprint.
    pub const fn new(retained_items: u32, retained_bytes: u64) -> Self {
        Self {
            retained_items,
            retained_bytes,
        }
    }

    /// Returns the retained item count.
    pub const fn retained_items(self) -> u32 {
        self.retained_items
    }

    /// Returns the retained byte count.
    pub const fn retained_bytes(self) -> u64 {
        self.retained_bytes
    }

    /// Returns whether this measured footprint fits the admitted ceiling.
    pub const fn fits_within(self, admitted: Self) -> bool {
        self.retained_items <= admitted.retained_items
            && self.retained_bytes <= admitted.retained_bytes
    }
}

/// Pre-progress resource declaration for one compiler input.
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
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingArtifactIdentity {
    plan_set_generation: PlanSetGeneration,
    plan_id: PlanId,
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    configuration: BindingConfigurationDigest,
    compatibility: BindingArtifactCompatibility,
    role: BindingArtifactRole,
}

impl BindingArtifactIdentity {
    /// Creates the complete immutable artifact identity.
    pub const fn new(
        plan_set_generation: PlanSetGeneration,
        plan_id: PlanId,
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        configuration: BindingConfigurationDigest,
        compatibility: BindingArtifactCompatibility,
        role: BindingArtifactRole,
    ) -> Self {
        Self {
            plan_set_generation,
            plan_id,
            binding_id,
            binding_generation,
            configuration,
            compatibility,
            role,
        }
    }

    /// Returns the plan-set generation.
    pub const fn plan_set_generation(&self) -> PlanSetGeneration {
        self.plan_set_generation
    }

    /// Returns the logical-plan identity.
    pub const fn plan_id(&self) -> PlanId {
        self.plan_id
    }

    /// Returns the binding identity.
    pub const fn binding_id(&self) -> BindingId {
        self.binding_id
    }

    /// Returns the binding generation.
    pub const fn binding_generation(&self) -> BindingGeneration {
        self.binding_generation
    }

    /// Returns the captured binding configuration digest.
    pub const fn configuration(&self) -> BindingConfigurationDigest {
        self.configuration
    }

    /// Returns the compiler/artifact compatibility identity.
    pub const fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    /// Returns the artifact's execution role.
    pub const fn role(&self) -> BindingArtifactRole {
        self.role
    }
}

/// Read-only resolved input passed to a binding compiler.
#[derive(Clone, Copy)]
pub struct BindingCompilerInput<'a> {
    logical_plan: &'a LogicalInteractionPlan,
    candidate: BindingCandidate,
    role: BindingArtifactRole,
    #[cfg(feature = "std")]
    semantics: Option<&'a clinkz_wot_td::semantic::ResolvedSemanticDocument<'a>>,
}

impl<'a> BindingCompilerInput<'a> {
    /// Creates a compiler input from one resolved plan and indexed candidate.
    pub const fn new(
        logical_plan: &'a LogicalInteractionPlan,
        candidate: BindingCandidate,
        role: BindingArtifactRole,
    ) -> Self {
        Self {
            logical_plan,
            candidate,
            role,
            #[cfg(feature = "std")]
            semantics: None,
        }
    }

    /// Returns the resolved immutable logical plan.
    pub const fn logical_plan(&self) -> &'a LogicalInteractionPlan {
        self.logical_plan
    }

    /// Returns the captured candidate identity.
    pub const fn candidate(&self) -> BindingCandidate {
        self.candidate
    }

    /// Returns the required artifact role.
    pub const fn role(&self) -> BindingArtifactRole {
        self.role
    }

    /// Attaches common resolved semantics for a native Host compiler. The
    /// legacy logical record remains engine storage, not the native input API.
    #[cfg(feature = "std")]
    pub fn with_semantics(
        mut self,
        semantics: &'a clinkz_wot_td::semantic::ResolvedSemanticDocument<'a>,
    ) -> Self {
        self.semantics = Some(semantics);
        self
    }

    #[cfg(feature = "std")]
    pub fn resolved(&self) -> Option<ResolvedBindingInput<'a>> {
        Some(ResolvedBindingInput {
            operation: self.logical_plan.operation(),
            property: self.logical_plan.property_name(),
            form_index: self.logical_plan.form_index(),
            target: self.logical_plan.resolved_target(),
            content_type: self.logical_plan.content_type(),
            subprotocol: self.logical_plan.subprotocol(),
            document: self.semantics?,
        })
    }
}

/// Native compilation input: effective WoT facts plus resolved scoped values.
/// No TD/context parser, security decision, handler or live resource is lent.
#[cfg(feature = "std")]
#[derive(Clone, Copy)]
pub struct ResolvedBindingInput<'a> {
    pub operation: clinkz_wot_td::data_type::Operation,
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
            .field("logical_plan", &self.logical_plan)
            .field("candidate", &self.candidate)
            .field("role", &self.role)
            .finish()
    }
}

/// Typed protocol-specific artifact plus its measured admission properties.
#[derive(Debug, Eq, PartialEq)]
pub struct BindingArtifact<A> {
    compatibility: BindingArtifactCompatibility,
    footprint: BindingArtifactFootprint,
    route_reservation: Option<RouteReservationIdentity>,
    payload: A,
}

impl<A> BindingArtifact<A> {
    /// Creates a measured typed artifact.
    pub const fn new(
        compatibility: BindingArtifactCompatibility,
        footprint: BindingArtifactFootprint,
        payload: A,
    ) -> Self {
        Self {
            compatibility,
            footprint,
            route_reservation: None,
            payload,
        }
    }

    /// Creates a measured Producer-route artifact with its canonical endpoint identity.
    pub const fn producer_route(
        compatibility: BindingArtifactCompatibility,
        footprint: BindingArtifactFootprint,
        reservation: RouteReservationIdentity,
        payload: A,
    ) -> Self {
        Self {
            compatibility,
            footprint,
            route_reservation: Some(reservation),
            payload,
        }
    }

    /// Returns the payload compatibility identity.
    pub const fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    /// Returns the measured retained footprint.
    pub const fn footprint(&self) -> BindingArtifactFootprint {
        self.footprint
    }

    /// Returns the canonical endpoint identity carried by a Producer-route artifact.
    pub const fn route_reservation(&self) -> Option<RouteReservationIdentity> {
        self.route_reservation
    }

    /// Borrows the typed payload.
    pub const fn payload(&self) -> &A {
        &self.payload
    }

    /// Consumes the wrapper and returns the typed payload.
    pub fn into_payload(self) -> A {
        self.payload
    }

    /// Consumes the wrapper and returns every captured part.
    pub fn into_parts(self) -> (BindingArtifactCompatibility, BindingArtifactFootprint, A) {
        (self.compatibility, self.footprint, self.payload)
    }

    /// Consumes the wrapper and returns every part, including route metadata.
    pub fn into_route_parts(
        self,
    ) -> (
        BindingArtifactCompatibility,
        BindingArtifactFootprint,
        Option<RouteReservationIdentity>,
        A,
    ) {
        (
            self.compatibility,
            self.footprint,
            self.route_reservation,
            self.payload,
        )
    }
}

/// Successful compiler result before artifact admission.
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

/// Independently authored, pure, bounded atomic Host lowering for Property Read.
/// WORK bounds native inspection/copy/destruction under the TD arena limits.
/// Implementations are trusted linked code and require executable qualification;
/// numeric declarations and safe erasure cannot prove their behavior. Native
/// private backing follows the selected Host policy. Strict Embedded compilers
/// retain their separate incremental storage/work contract.
///
/// This is a restricted adapter, not the target general compiler SPI. Generic
/// `BindingCompilerExtension` erasure supports owned artifacts and resumable
/// jobs, but those generic registrations cannot enter Consumer planning yet.
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
            plan: input.logical_plan().plan_id(),
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
            if cursor.plan != input.logical_plan().plan_id()
                || cursor.candidate != input.candidate()
            {
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

/// Checked immutable eligibility of the closed Consumer compiler and client.
///
/// This metadata identifies configuration, not program content or a live lease.
/// The private-policy key must describe the captured client's actual policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsumerBindingConfiguration {
    contract: BindingResourceContract,
    capacity: u8,
    cell: u8,
    default_json_forms: bool,
    #[cfg(feature = "std")]
    native: Option<BindingArtifactCompatibility>,
    #[cfg(feature = "std")]
    native_capacity: usize,
    private_policy: BindingConfigurationDigest,
    result: (u64, u64),
    key: BindingConfigurationDigest,
}

impl ConsumerBindingConfiguration {
    /// Checks the current compilation cell, capacity, and complete result backing.
    /// Derivation performs fixed, allocation-free work without client callbacks.
    pub fn try_new(
        contract: BindingResourceContract,
        target_capacity: usize,
        private_policy: BindingConfigurationDigest,
        body_bytes: u64,
        media_capacity: u64,
    ) -> CoreResult<Self> {
        let error = || {
            CoreError::Validation(
                ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
                    .with_redacted_cause(326, "unsupported Consumer binding configuration"),
            )
        };
        if contract == BindingResourceContract::StrictV1
            || (contract == BindingResourceContract::ConsumerOwnedHostV2 && !cfg!(feature = "std"))
            || !(1..=RESOLVED_TARGET_CAPACITY).contains(&target_capacity)
            || crate::response::consumer_result_backing(body_bytes, media_capacity).is_none()
        {
            return Err(error());
        }
        let cell = if cfg!(feature = "std") {
            2
        } else if cfg!(feature = "async") {
            1
        } else {
            0
        };
        let tag = match contract {
            BindingResourceContract::ConsumerOwnedV2 => 1,
            BindingResourceContract::ConsumerOwnedHostV2 => 2,
            BindingResourceContract::StrictV1 => unreachable!(),
        };
        let key = BindingConfigurationDigest::new(
            sha2_const_stable::Sha256::new()
                .update(b"clinkz-consumer-binding-configuration-v1\0")
                .update(&[tag, cell, target_capacity as u8])
                .update(RESOLVED_TARGET_COMPATIBILITY.as_bytes())
                .update(private_policy.as_bytes())
                .update(&body_bytes.to_le_bytes())
                .update(&media_capacity.to_le_bytes())
                .finalize(),
        );
        // Exclude the entire strict namespace even in the event of collision.
        if (1..=64).any(|capacity| key == strict_configuration(capacity)) {
            return Err(error());
        }
        Ok(Self {
            contract,
            capacity: target_capacity as u8,
            cell,
            default_json_forms: false,
            #[cfg(feature = "std")]
            native: None,
            #[cfg(feature = "std")]
            native_capacity: 0,
            private_policy,
            result: (body_bytes, media_capacity),
            key,
        })
    }
    /// Returns the explicitly selected resource and disposal contract.
    pub const fn resource_contract(&self) -> BindingResourceContract {
        self.contract
    }
    /// Returns the supported target capacity.
    pub const fn target_capacity(&self) -> usize {
        #[cfg(feature = "std")]
        if self.native.is_some() {
            return self.native_capacity;
        }
        self.capacity as usize
    }
    /// Returns the singleton compilation cell captured by this configuration.
    pub const fn compilation_cell(&self) -> &'static str {
        match self.cell {
            2 => "std",
            1 => "async-no-std",
            _ => "no-default",
        }
    }
    /// Returns the actual client's private-policy identity.
    pub const fn private_policy(&self) -> BindingConfigurationDigest {
        self.private_policy
    }
    /// Returns maximum body bytes and capacity of each media String.
    pub const fn result_limits(&self) -> (u64, u64) {
        self.result
    }
    /// Returns the derived eligibility key, shared by compiler and artifacts.
    pub const fn configuration(&self) -> BindingConfigurationDigest {
        self.key
    }

    /// Native Host target capacity is Binding policy, independent of the old
    /// inline copy helper's ceiling. Result backing remains engine-checked.
    #[cfg(feature = "std")]
    pub fn for_semantic_host(
        target_capacity: usize,
        private_policy: BindingConfigurationDigest,
        compatibility: BindingArtifactCompatibility,
        body_bytes: u64,
        media_capacity: u64,
    ) -> CoreResult<Self> {
        if target_capacity == 0
            || crate::response::consumer_result_backing(body_bytes, media_capacity).is_none()
        {
            return Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Admission,
                RetryClass::Never,
            )));
        }
        let key = BindingConfigurationDigest::new(
            sha2_const_stable::Sha256::new()
                .update(b"clinkz-native-consumer-configuration-v1\0")
                .update(compatibility.as_bytes())
                .update(private_policy.as_bytes())
                .update(&(target_capacity as u64).to_le_bytes())
                .update(&body_bytes.to_le_bytes())
                .update(&media_capacity.to_le_bytes())
                .finalize(),
        );
        Ok(Self {
            contract: BindingResourceContract::ConsumerOwnedHostV2,
            capacity: 0,
            cell: 2,
            default_json_forms: false,
            native: Some(compatibility),
            native_capacity: target_capacity,
            private_policy,
            result: (body_bytes, media_capacity),
            key,
        })
    }

    /// Restricts the closed compiler to known metadata-free default JSON Forms.
    /// The derived key preserves every original configuration operand. Repeated
    /// application is idempotent and performs no allocation or client callback.
    pub fn with_default_json_forms(mut self) -> CoreResult<Self> {
        #[cfg(feature = "std")]
        if self.native.is_some() {
            return Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Admission,
                RetryClass::Never,
            )));
        }
        if self.default_json_forms {
            return Ok(self);
        }
        let key = BindingConfigurationDigest::new(
            sha2_const_stable::Sha256::new()
                .update(b"clinkz-consumer-default-json-forms-v1\0")
                .update(self.key.as_bytes())
                .finalize(),
        );
        if key == self.key || (1..=64).any(|capacity| key == strict_configuration(capacity)) {
            return Err(CoreError::Validation(
                ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
                    .with_redacted_cause(327, "Consumer Form restriction identity collision"),
            ));
        }
        self.key = key;
        self.default_json_forms = true;
        Ok(self)
    }

    /// Whether the client and compiler require paid default JSON Form facts.
    pub const fn requires_default_json_forms(&self) -> bool {
        self.default_json_forms
    }
}

// One additional fixed predicate unit covers presence and the bounded media
// comparison (at most the length of "application/json"). Strict/unrestricted
// callbacks retain their existing costs.
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
            capacity: configuration.capacity,
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
            || input.logical_plan().resolved_target().len() > usize::from(self.capacity)
        {
            return Err(resolved_target_error(input));
        }
        if form_eligibility_work(self.owned) != 0
            && (input.logical_plan().has_additional_form_metadata() != Some(false)
                || input.logical_plan().content_type() != Some("application/json"))
        {
            return Err(CoreError::UnsupportedOperation(
                resolved_target_context(input)
                    .with_redacted_cause(328, "Consumer compiler requires default JSON Forms"),
            ));
        }
        Ok(())
    }

    pub(crate) fn support(&self, host_erased: bool) -> ConsumerCompilerSupport {
        ConsumerCompilerSupport {
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
        .with_operation(input.logical_plan().operation())
        .with_form_index(input.logical_plan().form_index())
        .with_plan(input.logical_plan().plan_id())
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
            plan: input.logical_plan().plan_id(),
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
        if cursor.plan != input.logical_plan().plan_id() || cursor.candidate != input.candidate() {
            return BindingCompilerStep::Failed(BindingCompilerFailure::new(
                resolved_target_error(input),
                cursor,
            ));
        }
        if cursor.pending {
            cursor.pending = false;
            return BindingCompilerStep::Pending(cursor);
        }
        let target = input.logical_plan().resolved_target().as_bytes();
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
/// # use clinkz_wot_core::ConsumerCompilerSupport;
/// let unsupported = ConsumerCompilerSupport { capacity: 64, host_erased: false };
/// ```
#[derive(Debug)]
pub struct ConsumerCompilerSupport {
    #[cfg(feature = "std")]
    native: Option<NativeConsumerSupport>,
    capacity: u8,
    host_erased: bool,
    owned: Option<ConsumerBindingConfiguration>,
}

impl ConsumerCompilerSupport {
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
            if configuration.native.is_some() {
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

/// Reason a measured artifact could not enter an immutable plan set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingArtifactRejectionReason {
    /// Compiler and artifact compatibility identities differ.
    CompatibilityMismatch,
    /// Measured retained items or bytes exceed the admitted bound.
    FootprintExceeded,
    /// A Producer-route artifact omitted its canonical endpoint identity.
    MissingRouteReservation,
    /// A non-Producer-route artifact supplied Producer-only endpoint metadata.
    UnexpectedRouteReservation,
}

/// Artifact-admission failure that returns the original typed artifact.
#[derive(Debug, Eq, PartialEq)]
pub struct BindingArtifactRejection<A> {
    reason: BindingArtifactRejectionReason,
    artifact: BindingArtifact<A>,
}

impl<A> BindingArtifactRejection<A> {
    /// Returns the exact rejection class.
    pub const fn reason(&self) -> BindingArtifactRejectionReason {
        self.reason
    }

    /// Borrows the rejected typed artifact.
    pub const fn artifact(&self) -> &BindingArtifact<A> {
        &self.artifact
    }

    /// Consumes the rejection and returns the original artifact.
    pub fn into_artifact(self) -> BindingArtifact<A> {
        self.artifact
    }
}

/// Admitted immutable artifact with complete identity and measured bounds.
#[derive(Debug, Eq, PartialEq)]
pub struct BindingArtifactEnvelope<A> {
    identity: BindingArtifactIdentity,
    admitted: BindingArtifactFootprint,
    artifact: BindingArtifact<A>,
}

impl<A> BindingArtifactEnvelope<A> {
    /// Validates compatibility and retained footprint without losing ownership.
    pub fn try_new(
        identity: BindingArtifactIdentity,
        admitted: BindingArtifactFootprint,
        artifact: BindingArtifact<A>,
    ) -> Result<Self, BindingArtifactRejection<A>> {
        if identity.compatibility() != artifact.compatibility() {
            return Err(BindingArtifactRejection {
                reason: BindingArtifactRejectionReason::CompatibilityMismatch,
                artifact,
            });
        }
        if !artifact.footprint().fits_within(admitted) {
            return Err(BindingArtifactRejection {
                reason: BindingArtifactRejectionReason::FootprintExceeded,
                artifact,
            });
        }
        match (identity.role(), artifact.route_reservation()) {
            (BindingArtifactRole::ProducerRoute, None) => {
                return Err(BindingArtifactRejection {
                    reason: BindingArtifactRejectionReason::MissingRouteReservation,
                    artifact,
                });
            }
            (BindingArtifactRole::ProducerRoute, Some(_)) | (_, None) => {}
            (_, Some(_)) => {
                return Err(BindingArtifactRejection {
                    reason: BindingArtifactRejectionReason::UnexpectedRouteReservation,
                    artifact,
                });
            }
        }
        Ok(Self {
            identity,
            admitted,
            artifact,
        })
    }

    /// Returns the complete generation-qualified identity.
    pub const fn identity(&self) -> BindingArtifactIdentity {
        self.identity
    }

    /// Returns the admitted retained-footprint ceiling.
    pub const fn admitted(&self) -> BindingArtifactFootprint {
        self.admitted
    }

    /// Borrows the admitted typed artifact.
    pub const fn artifact(&self) -> &BindingArtifact<A> {
        &self.artifact
    }

    /// Returns the admitted canonical endpoint identity for a Producer route.
    pub const fn route_reservation(&self) -> Option<RouteReservationIdentity> {
        self.artifact.route_reservation()
    }

    /// Consumes the envelope and returns the typed artifact.
    pub fn into_artifact(self) -> BindingArtifact<A> {
        self.artifact
    }
}

/// Compact reference to one immutable artifact slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BindingArtifactRef {
    identity: BindingArtifactIdentity,
    artifact_slot: SlotIndex,
}

impl BindingArtifactRef {
    /// Creates a generation-qualified artifact reference.
    pub const fn new(identity: BindingArtifactIdentity, artifact_slot: SlotIndex) -> Self {
        Self {
            identity,
            artifact_slot,
        }
    }

    /// Returns the complete referenced artifact identity.
    pub const fn identity(&self) -> BindingArtifactIdentity {
        self.identity
    }

    /// Returns the plan-set-local artifact slot.
    pub const fn artifact_slot(&self) -> SlotIndex {
        self.artifact_slot
    }
}

/// Typed compiler component for an application-owned static compiler universe.
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
/// Core-erased host cursor. Its concrete type remains ownership-preserving.
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
/// Core-erased immutable host artifact payload.
pub struct HostBindingArtifact(Box<dyn ErasedHostArtifact>);

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
trait ErasedHostArtifact: Send + Sync {
    fn storage_overhead(&self) -> u64;
    fn payload(&self) -> &dyn Any;
    fn take_payload(&mut self, destination: &mut dyn Any);
}

#[cfg(feature = "std")]
impl<C> ErasedHostArtifact for HostCompilerSlot<C>
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
impl HostBindingArtifact {
    /// Engine-owned slot bytes beyond the native inline payload.
    /// Add these to the native footprint when charging retained output; outer
    /// owners and native private backing remain separate physical costs.
    pub fn storage_overhead(&self) -> u64 {
        self.0.storage_overhead()
    }

    fn payload(&self) -> &dyn Any {
        self.0.payload()
    }

    fn into_payload<T: Send + Sync + 'static>(mut self) -> Result<T, Self> {
        if !self.payload().is::<T>() {
            return Err(self);
        }
        let mut payload: Option<T> = None;
        self.0.take_payload(&mut payload);
        Ok(payload.expect("completed slot supplied its payload"))
    }
}

#[cfg(feature = "std")]
pub(crate) fn try_box<T>(value: T) -> Result<Box<T>, T> {
    let layout = Layout::new::<T>();
    if layout.size() == 0 {
        return Ok(Box::new(value));
    }
    // SAFETY: use the exact nonzero Layout for T and check null before writing.
    // A successful pointer is initialized once and transferred to Box, whose
    // destructor uses the same global allocator and Layout. Failure returns T.
    let pointer = unsafe { alloc::alloc::alloc(layout) }.cast::<T>();
    if pointer.is_null() {
        return Err(value);
    }
    unsafe {
        pointer.write(value);
        Ok(Box::from_raw(pointer))
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
impl fmt::Debug for HostBindingArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostBindingArtifact")
            .finish_non_exhaustive()
    }
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
                let payload = HostBindingArtifact(slot);
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
            .with_operation(input.logical_plan().operation())
            .with_form_index(input.logical_plan().form_index())
            .with_plan(input.logical_plan().plan_id())
            .with_binding(candidate.binding_id(), candidate.binding_generation()),
    )
}

#[cfg(feature = "std")]
/// Host compiler component with Core-owned safe type erasure.
pub struct HostBindingCompilerRegistration {
    compiler: Box<dyn ErasedBindingCompiler>,
    consumer_support: Option<ConsumerCompilerSupport>,
}

#[cfg(feature = "std")]
impl HostBindingCompilerRegistration {
    /// Captures an independently authored native compiler with actual layouts.
    /// The complete registration subsequently checks its client/config pairing.
    pub fn try_new_semantic<C: HostSemanticCompiler>(
        compiler: C,
    ) -> Result<Self, crate::BindingInputRejection<C>> {
        let configuration = compiler.configuration();
        let compatibility = compiler.compatibility();
        if C::WORK == 0
            || C::WORK == u64::MAX
            || compiler.schemes().is_empty()
            || configuration.native != Some(compatibility)
        {
            return Err(crate::BindingInputRejection::new(
                compiler,
                crate::BindingOperationalError::new(CoreError::Validation(ErrorContext::new(
                    ErrorPhase::Admission,
                    RetryClass::Never,
                ))),
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
        let support = ConsumerCompilerSupport {
            native: Some(native),
            capacity: configuration.capacity,
            host_erased: true,
            owned: Some(configuration),
        };
        match Self::try_new(SemanticCompilerAdapter(compiler)) {
            Ok(mut registration) => {
                registration.consumer_support = Some(support);
                Ok(registration)
            }
            Err(rejected) => {
                let (adapter, error) = rejected.into_parts();
                Err(crate::BindingInputRejection::new(adapter.0, error))
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
        }
    }

    /// Fallibly erases any portable compiler, returning it unchanged on refusal.
    /// No native callback runs during adapter construction.
    pub fn try_new<C>(compiler: C) -> Result<Self, crate::BindingInputRejection<C>>
    where
        C: BindingCompilerExtension + Send + Sync + 'static,
        C::Cursor: Send + 'static,
        C::Artifact: Send + Sync + 'static,
    {
        match try_box(HostCompilerAdapter(compiler)) {
            Ok(compiler) => Ok(Self {
                compiler,
                consumer_support: None,
            }),
            Err(HostCompilerAdapter(compiler)) => Err(crate::BindingInputRejection::new(
                compiler,
                crate::BindingOperationalError::new(host_allocation_error()),
            )),
        }
    }

    /// Captures the existing closed Consumer contract using the generic adapter.
    /// This constructor does not widen Consumer compiler eligibility.
    pub fn try_new_consumer(
        compiler: ResolvedTargetCompiler,
    ) -> Result<Self, crate::BindingInputRejection<ResolvedTargetCompiler>> {
        let support = compiler.support(true);
        let mut registration = Self::try_new(compiler)?;
        registration.consumer_support = Some(support);
        Ok(registration)
    }

    pub(crate) fn consumer_support(&self) -> Option<ConsumerCompilerSupport> {
        self.consumer_support
            .as_ref()
            .map(|support| ConsumerCompilerSupport {
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

#[cfg(feature = "std")]
impl BindingArtifact<HostBindingArtifact> {
    /// Borrows a matching concrete payload after compatibility/type checks.
    pub fn try_payload<T>(&self, expected: BindingArtifactCompatibility) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        if self.compatibility() != expected {
            return None;
        }
        self.payload.payload().downcast_ref::<T>()
    }

    /// Consumes and extracts a matching concrete payload.
    ///
    /// Either mismatch returns the original erased artifact unchanged.
    pub fn try_into_payload<T>(self, expected: BindingArtifactCompatibility) -> Result<T, Self>
    where
        T: Send + Sync + 'static,
    {
        if self.compatibility != expected {
            return Err(self);
        }
        let Self {
            compatibility,
            footprint,
            route_reservation,
            payload,
        } = self;
        match payload.into_payload::<T>() {
            Ok(payload) => Ok(payload),
            Err(payload) => Err(Self {
                compatibility,
                footprint,
                route_reservation,
                payload,
            }),
        }
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
            crate::ThingId::from("urn:test:artifact"),
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
            crate::CollisionDomainId::new([21; 16]),
            crate::EndpointReservationKey::new([22; 32]),
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
            crate::ThingId::from("urn:trace:compiler"),
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
        let rejection = HostBindingCompilerRegistration::try_new_consumer(compiler).unwrap_err();
        assert_eq!(calls(), [0; 5]);
        let host =
            HostBindingCompilerRegistration::try_new_consumer(rejection.into_input()).unwrap();
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
