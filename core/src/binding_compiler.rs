//! Portable binding-compiler and immutable artifact contracts.

#[cfg(feature = "std")]
use alloc::boxed::Box;
use core::fmt;

#[cfg(feature = "std")]
use std::any::Any;

use clinkz_wot_foundation::{SlotIndex, WorkBudget};

use crate::{
    BindingCandidate, BindingConfigurationDigest, BindingGeneration, BindingId, CoreError,
    CoreResult, LogicalInteractionPlan, PlanId, PlanSetGeneration, RouteReservationIdentity,
};
#[cfg(feature = "std")]
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
pub struct HostBindingCompilerCursor(Box<dyn Any + Send>);

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
pub struct HostBindingArtifact(Box<dyn Any + Send + Sync>);

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
    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.0.compatibility()
    }

    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.0.bounds(input)
    }

    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<HostBindingCompilerCursor> {
        self.0
            .start(input)
            .map(|cursor| HostBindingCompilerCursor(Box::new(cursor)))
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: HostBindingCompilerCursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<HostBindingCompilerCursor, HostBindingArtifact> {
        let cursor = match cursor.0.downcast::<C::Cursor>() {
            Ok(cursor) => *cursor,
            Err(cursor) => {
                return BindingCompilerStep::Failed(BindingCompilerFailure::new(
                    host_cursor_mismatch(input),
                    HostBindingCompilerCursor(cursor),
                ));
            }
        };

        match self.0.step(input, cursor, budget) {
            BindingCompilerStep::Pending(cursor) => {
                BindingCompilerStep::Pending(HostBindingCompilerCursor(Box::new(cursor)))
            }
            BindingCompilerStep::Complete(output) => {
                let (compatibility, footprint, reservation, payload) =
                    output.into_artifact().into_route_parts();
                let artifact = match reservation {
                    Some(reservation) => BindingArtifact::producer_route(
                        compatibility,
                        footprint,
                        reservation,
                        HostBindingArtifact(Box::new(payload)),
                    ),
                    None => BindingArtifact::new(
                        compatibility,
                        footprint,
                        HostBindingArtifact(Box::new(payload)),
                    ),
                };
                BindingCompilerStep::Complete(BindingCompilerOutput::new(artifact))
            }
            BindingCompilerStep::Failed(failure) => {
                let (error, cursor) = failure.into_parts();
                BindingCompilerStep::Failed(BindingCompilerFailure::new(
                    error,
                    HostBindingCompilerCursor(Box::new(cursor)),
                ))
            }
        }
    }

    fn abort(&self, cursor: HostBindingCompilerCursor) -> Result<(), HostBindingCompilerCursor> {
        match cursor.0.downcast::<C::Cursor>() {
            Ok(cursor) => {
                self.0.abort(*cursor);
                Ok(())
            }
            Err(cursor) => Err(HostBindingCompilerCursor(cursor)),
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
}

#[cfg(feature = "std")]
impl HostBindingCompilerRegistration {
    /// Erases one portable compiler using safe standard-library type identity.
    pub fn new<C>(compiler: C) -> Self
    where
        C: BindingCompilerExtension + Send + Sync + 'static,
        C::Cursor: Send + 'static,
        C::Artifact: Send + Sync + 'static,
    {
        Self {
            compiler: Box::new(HostCompilerAdapter(compiler)),
        }
    }

    /// Returns the erased compiler's stable compatibility identity.
    pub fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compiler.compatibility()
    }

    /// Obtains bounds without beginning compiler progress.
    pub fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.compiler.bounds(input)
    }

    /// Creates an erased pure cursor.
    pub fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<HostBindingCompilerCursor> {
        self.compiler.start(input)
    }

    /// Performs one erased compiler step.
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
        self.payload.0.downcast_ref::<T>()
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
        match payload.0.downcast::<T>() {
            Ok(payload) => Ok(*payload),
            Err(payload) => Err(Self {
                compatibility,
                footprint,
                route_reservation,
                payload: HostBindingArtifact(payload),
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
            *payload.0.downcast::<u8>().expect("matching erased payload"),
            7
        );
    }
}
