#![allow(dead_code)]

use std::any::Any;
use std::boxed::Box;
use std::string::String;
use std::vec;
use std::vec::Vec;

#[derive(Debug, Eq, PartialEq)]
struct CoreError(&'static str);

type CoreResult<T> = Result<T, CoreError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BindingId(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BindingGeneration(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlanId(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlanSetGeneration(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SlotIndex(u32);

#[derive(Clone, Debug, Eq, PartialEq)]
struct ThingId(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
struct BindingConfigurationDigest([u8; 32]);

impl BindingConfigurationDigest {
    const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
struct BindingArtifactCompatibility([u8; 16]);

impl BindingArtifactCompatibility {
    const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operation {
    ReadProperty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BindingArtifactRole {
    ConsumerCall,
    ConsumerSubscription,
    ProducerRoute,
    ProducerPublication,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BindingArtifactFootprint {
    retained_items: u32,
    retained_bytes: u64,
}

impl BindingArtifactFootprint {
    const fn new(retained_items: u32, retained_bytes: u64) -> Self {
        Self {
            retained_items,
            retained_bytes,
        }
    }

    const fn retained_items(self) -> u32 {
        self.retained_items
    }

    const fn retained_bytes(self) -> u64 {
        self.retained_bytes
    }

    const fn fits_within(self, admitted: Self) -> bool {
        self.retained_items <= admitted.retained_items
            && self.retained_bytes <= admitted.retained_bytes
    }
}

#[derive(Debug, Eq, PartialEq)]
struct WorkBudget {
    binding_polls: u64,
}

impl WorkBudget {
    const fn new() -> Self {
        Self { binding_polls: 0 }
    }

    const fn with_binding_polls(mut self, binding_polls: u64) -> Self {
        self.binding_polls = binding_polls;
        self
    }

    const fn remaining_binding_polls(&self) -> u64 {
        self.binding_polls
    }

    fn try_consume_binding_poll(&mut self) -> bool {
        if self.binding_polls == 0 {
            return false;
        }
        self.binding_polls -= 1;
        true
    }
}

#[derive(Debug, Eq, PartialEq)]
struct BindingCompilerBounds {
    artifact: BindingArtifactFootprint,
    cursor_bytes: u64,
    temporary_bytes: u64,
    work: WorkBudget,
}

impl BindingCompilerBounds {
    fn new(
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

    const fn artifact(&self) -> BindingArtifactFootprint {
        self.artifact
    }

    const fn cursor_bytes(&self) -> u64 {
        self.cursor_bytes
    }

    const fn temporary_bytes(&self) -> u64 {
        self.temporary_bytes
    }

    const fn work(&self) -> &WorkBudget {
        &self.work
    }

    fn into_work(self) -> WorkBudget {
        self.work
    }
}

#[derive(Debug, Eq, PartialEq)]
struct LogicalInteractionPlan {
    plan_id: PlanId,
    thing_id: ThingId,
    property_name: Box<str>,
    form_index: u32,
    resolved_target: Box<str>,
    content_type: Option<Box<str>>,
    subprotocol: Option<Box<str>>,
}

impl LogicalInteractionPlan {
    fn try_property_read(
        plan_id: PlanId,
        thing_id: ThingId,
        property_name: Box<str>,
        form_index: u32,
        resolved_target: Box<str>,
        content_type: Option<Box<str>>,
        subprotocol: Option<Box<str>>,
    ) -> CoreResult<Self> {
        if property_name.is_empty() || resolved_target.is_empty() {
            return Err(CoreError("empty property-read plan field"));
        }
        Ok(Self {
            plan_id,
            thing_id,
            property_name,
            form_index,
            resolved_target,
            content_type,
            subprotocol,
        })
    }

    const fn plan_id(&self) -> PlanId {
        self.plan_id
    }

    fn thing_id(&self) -> &ThingId {
        &self.thing_id
    }

    const fn operation(&self) -> Operation {
        Operation::ReadProperty
    }

    fn property_name(&self) -> &str {
        &self.property_name
    }

    const fn form_index(&self) -> u32 {
        self.form_index
    }

    fn resolved_target(&self) -> &str {
        &self.resolved_target
    }

    fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    fn subprotocol(&self) -> Option<&str> {
        self.subprotocol.as_deref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BindingCandidate {
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    configuration: BindingConfigurationDigest,
    compatibility: BindingArtifactCompatibility,
    registration_ordinal: u32,
    candidate_order: u32,
}

impl BindingCandidate {
    const fn new(
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        configuration: BindingConfigurationDigest,
        compatibility: BindingArtifactCompatibility,
        registration_ordinal: u32,
        candidate_order: u32,
    ) -> Self {
        Self {
            binding_id,
            binding_generation,
            configuration,
            compatibility,
            registration_ordinal,
            candidate_order,
        }
    }

    const fn binding_id(self) -> BindingId {
        self.binding_id
    }

    const fn binding_generation(self) -> BindingGeneration {
        self.binding_generation
    }

    const fn configuration(self) -> BindingConfigurationDigest {
        self.configuration
    }

    const fn compatibility(self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    const fn registration_ordinal(self) -> u32 {
        self.registration_ordinal
    }

    const fn candidate_order(self) -> u32 {
        self.candidate_order
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BindingArtifactIdentity {
    plan_set_generation: PlanSetGeneration,
    plan_id: PlanId,
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    configuration: BindingConfigurationDigest,
    compatibility: BindingArtifactCompatibility,
    role: BindingArtifactRole,
}

impl BindingArtifactIdentity {
    const fn new(
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

    const fn compatibility(self) -> BindingArtifactCompatibility {
        self.compatibility
    }
}

#[derive(Clone, Copy)]
struct BindingCompilerInput<'a> {
    logical_plan: &'a LogicalInteractionPlan,
    candidate: BindingCandidate,
    role: BindingArtifactRole,
}

impl<'a> BindingCompilerInput<'a> {
    const fn new(
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

    const fn logical_plan(self) -> &'a LogicalInteractionPlan {
        self.logical_plan
    }

    const fn candidate(self) -> BindingCandidate {
        self.candidate
    }

    const fn role(self) -> BindingArtifactRole {
        self.role
    }
}

#[derive(Debug, Eq, PartialEq)]
struct BindingArtifact<A> {
    compatibility: BindingArtifactCompatibility,
    footprint: BindingArtifactFootprint,
    payload: A,
}

impl<A> BindingArtifact<A> {
    const fn new(
        compatibility: BindingArtifactCompatibility,
        footprint: BindingArtifactFootprint,
        payload: A,
    ) -> Self {
        Self {
            compatibility,
            footprint,
            payload,
        }
    }

    const fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    const fn footprint(&self) -> BindingArtifactFootprint {
        self.footprint
    }

    const fn payload(&self) -> &A {
        &self.payload
    }

    fn into_payload(self) -> A {
        self.payload
    }

    fn into_parts(self) -> (BindingArtifactCompatibility, BindingArtifactFootprint, A) {
        (self.compatibility, self.footprint, self.payload)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct BindingCompilerOutput<A> {
    artifact: BindingArtifact<A>,
}

impl<A> BindingCompilerOutput<A> {
    const fn new(artifact: BindingArtifact<A>) -> Self {
        Self { artifact }
    }

    const fn artifact(&self) -> &BindingArtifact<A> {
        &self.artifact
    }

    fn into_artifact(self) -> BindingArtifact<A> {
        self.artifact
    }
}

#[derive(Debug, Eq, PartialEq)]
struct BindingCompilerFailure<C> {
    error: CoreError,
    cursor: C,
}

impl<C> BindingCompilerFailure<C> {
    const fn new(error: CoreError, cursor: C) -> Self {
        Self { error, cursor }
    }

    const fn error(&self) -> &CoreError {
        &self.error
    }

    const fn cursor(&self) -> &C {
        &self.cursor
    }

    fn into_parts(self) -> (CoreError, C) {
        (self.error, self.cursor)
    }
}

#[derive(Debug, Eq, PartialEq)]
#[must_use]
enum BindingCompilerStep<C, A> {
    Pending(C),
    Complete(BindingCompilerOutput<A>),
    Failed(BindingCompilerFailure<C>),
}

trait BindingCompilerExtension {
    type Cursor;
    type Artifact;

    fn compatibility(&self) -> BindingArtifactCompatibility;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BindingArtifactRejectionReason {
    CompatibilityMismatch,
    FootprintExceeded,
}

#[derive(Debug, Eq, PartialEq)]
struct BindingArtifactRejection<A> {
    reason: BindingArtifactRejectionReason,
    artifact: BindingArtifact<A>,
}

impl<A> BindingArtifactRejection<A> {
    const fn reason(&self) -> BindingArtifactRejectionReason {
        self.reason
    }

    fn into_artifact(self) -> BindingArtifact<A> {
        self.artifact
    }
}

#[derive(Debug, Eq, PartialEq)]
struct BindingArtifactEnvelope<A> {
    identity: BindingArtifactIdentity,
    admitted: BindingArtifactFootprint,
    artifact: BindingArtifact<A>,
}

impl<A> BindingArtifactEnvelope<A> {
    fn try_new(
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
        Ok(Self {
            identity,
            admitted,
            artifact,
        })
    }

    const fn identity(&self) -> BindingArtifactIdentity {
        self.identity
    }

    const fn admitted(&self) -> BindingArtifactFootprint {
        self.admitted
    }

    const fn artifact(&self) -> &BindingArtifact<A> {
        &self.artifact
    }

    fn into_artifact(self) -> BindingArtifact<A> {
        self.artifact
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BindingArtifactRef {
    identity: BindingArtifactIdentity,
    artifact_slot: SlotIndex,
}

impl BindingArtifactRef {
    const fn new(identity: BindingArtifactIdentity, artifact_slot: SlotIndex) -> Self {
        Self {
            identity,
            artifact_slot,
        }
    }

    const fn identity(self) -> BindingArtifactIdentity {
        self.identity
    }

    const fn artifact_slot(self) -> SlotIndex {
        self.artifact_slot
    }
}

struct StaticBindingCompilerRegistration<C> {
    compiler: C,
}

impl<C> StaticBindingCompilerRegistration<C> {
    const fn new(compiler: C) -> Self {
        Self { compiler }
    }

    const fn compiler(&self) -> &C {
        &self.compiler
    }

    fn into_compiler(self) -> C {
        self.compiler
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MockCursor {
    remaining_steps: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MockArtifact {
    target: Box<str>,
}

#[derive(Clone, Copy)]
struct MockCompiler {
    compatibility: BindingArtifactCompatibility,
}

impl BindingCompilerExtension for MockCompiler {
    type Cursor = MockCursor;
    type Artifact = MockArtifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        let bytes = input.logical_plan().resolved_target().len() as u64;
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, bytes),
            size_of::<MockCursor>() as u64,
            0,
            WorkBudget::new().with_binding_polls(2),
        ))
    }

    fn start(&self, _input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        Ok(MockCursor { remaining_steps: 2 })
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        mut cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        if !budget.try_consume_binding_poll() {
            return BindingCompilerStep::Pending(cursor);
        }
        cursor.remaining_steps -= 1;
        if cursor.remaining_steps != 0 {
            return BindingCompilerStep::Pending(cursor);
        }
        let target: Box<str> = input.logical_plan().resolved_target().into();
        let footprint = BindingArtifactFootprint::new(1, target.len() as u64);
        BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
            self.compatibility(),
            footprint,
            MockArtifact { target },
        )))
    }

    fn abort(&self, _cursor: Self::Cursor) {}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AlternateCursor(u16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AlternateArtifact(u16);

#[derive(Clone, Copy)]
struct AlternateCompiler {
    compatibility: BindingArtifactCompatibility,
}

impl BindingCompilerExtension for AlternateCompiler {
    type Cursor = AlternateCursor;
    type Artifact = AlternateArtifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    fn bounds(&self, _input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, 2),
            size_of::<AlternateCursor>() as u64,
            0,
            WorkBudget::new().with_binding_polls(1),
        ))
    }

    fn start(&self, _input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        Ok(AlternateCursor(7))
    }

    fn step(
        &self,
        _input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        if !budget.try_consume_binding_poll() {
            return BindingCompilerStep::Pending(cursor);
        }
        BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
            self.compatibility(),
            BindingArtifactFootprint::new(1, 2),
            AlternateArtifact(cursor.0),
        )))
    }

    fn abort(&self, _cursor: Self::Cursor) {}
}

enum AppCompiler {
    Mock(MockCompiler),
    Alternate(AlternateCompiler),
}

#[derive(Debug, Eq, PartialEq)]
enum AppCursor {
    Mock(MockCursor),
    Alternate(AlternateCursor),
}

#[derive(Debug, Eq, PartialEq)]
enum AppArtifact {
    Mock(MockArtifact),
    Alternate(AlternateArtifact),
}

impl BindingCompilerExtension for AppCompiler {
    type Cursor = AppCursor;
    type Artifact = AppArtifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        match self {
            Self::Mock(compiler) => compiler.compatibility(),
            Self::Alternate(compiler) => compiler.compatibility(),
        }
    }

    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        match self {
            Self::Mock(compiler) => compiler.bounds(input),
            Self::Alternate(compiler) => compiler.bounds(input),
        }
    }

    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        match self {
            Self::Mock(compiler) => compiler.start(input).map(AppCursor::Mock),
            Self::Alternate(compiler) => compiler.start(input).map(AppCursor::Alternate),
        }
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        match (self, cursor) {
            (Self::Mock(compiler), AppCursor::Mock(cursor)) => {
                map_mock_step(compiler.step(input, cursor, budget))
            }
            (Self::Alternate(compiler), AppCursor::Alternate(cursor)) => {
                map_alternate_step(compiler.step(input, cursor, budget))
            }
            (_, cursor) => BindingCompilerStep::Failed(BindingCompilerFailure::new(
                CoreError("static compiler/cursor variant mismatch"),
                cursor,
            )),
        }
    }

    fn abort(&self, cursor: Self::Cursor) {
        match (self, cursor) {
            (Self::Mock(compiler), AppCursor::Mock(cursor)) => compiler.abort(cursor),
            (Self::Alternate(compiler), AppCursor::Alternate(cursor)) => compiler.abort(cursor),
            _ => {}
        }
    }
}

fn map_mock_step(
    step: BindingCompilerStep<MockCursor, MockArtifact>,
) -> BindingCompilerStep<AppCursor, AppArtifact> {
    match step {
        BindingCompilerStep::Pending(cursor) => {
            BindingCompilerStep::Pending(AppCursor::Mock(cursor))
        }
        BindingCompilerStep::Complete(output) => {
            let (compatibility, footprint, payload) = output.into_artifact().into_parts();
            BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                compatibility,
                footprint,
                AppArtifact::Mock(payload),
            )))
        }
        BindingCompilerStep::Failed(failure) => {
            let (error, cursor) = failure.into_parts();
            BindingCompilerStep::Failed(BindingCompilerFailure::new(error, AppCursor::Mock(cursor)))
        }
    }
}

fn map_alternate_step(
    step: BindingCompilerStep<AlternateCursor, AlternateArtifact>,
) -> BindingCompilerStep<AppCursor, AppArtifact> {
    match step {
        BindingCompilerStep::Pending(cursor) => {
            BindingCompilerStep::Pending(AppCursor::Alternate(cursor))
        }
        BindingCompilerStep::Complete(output) => {
            let (compatibility, footprint, payload) = output.into_artifact().into_parts();
            BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                compatibility,
                footprint,
                AppArtifact::Alternate(payload),
            )))
        }
        BindingCompilerStep::Failed(failure) => {
            let (error, cursor) = failure.into_parts();
            BindingCompilerStep::Failed(BindingCompilerFailure::new(
                error,
                AppCursor::Alternate(cursor),
            ))
        }
    }
}

struct HostBindingCompilerCursor(Box<dyn Any + Send>);

struct HostBindingArtifact(Box<dyn Any + Send + Sync>);

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

struct HostCompilerAdapter<C>(C);

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
                    CoreError("host compiler cursor type mismatch"),
                    HostBindingCompilerCursor(cursor),
                ));
            }
        };
        match self.0.step(input, cursor, budget) {
            BindingCompilerStep::Pending(cursor) => {
                BindingCompilerStep::Pending(HostBindingCompilerCursor(Box::new(cursor)))
            }
            BindingCompilerStep::Complete(output) => {
                let (compatibility, footprint, payload) = output.into_artifact().into_parts();
                BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                    compatibility,
                    footprint,
                    HostBindingArtifact(Box::new(payload)),
                )))
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

struct HostBindingCompilerRegistration {
    compiler: Box<dyn ErasedBindingCompiler>,
}

impl HostBindingCompilerRegistration {
    fn new<C>(compiler: C) -> Self
    where
        C: BindingCompilerExtension + Send + Sync + 'static,
        C::Cursor: Send + 'static,
        C::Artifact: Send + Sync + 'static,
    {
        Self {
            compiler: Box::new(HostCompilerAdapter(compiler)),
        }
    }

    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compiler.compatibility()
    }

    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.compiler.bounds(input)
    }

    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<HostBindingCompilerCursor> {
        self.compiler.start(input)
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: HostBindingCompilerCursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<HostBindingCompilerCursor, HostBindingArtifact> {
        self.compiler.step(input, cursor, budget)
    }

    fn abort(&self, cursor: HostBindingCompilerCursor) -> Result<(), HostBindingCompilerCursor> {
        self.compiler.abort(cursor)
    }
}

impl BindingArtifact<HostBindingArtifact> {
    fn try_payload<T>(&self, expected: BindingArtifactCompatibility) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        if self.compatibility() != expected {
            return None;
        }
        self.payload.0.downcast_ref::<T>()
    }

    fn try_into_payload<T>(self, expected: BindingArtifactCompatibility) -> Result<T, Self>
    where
        T: Send + Sync + 'static,
    {
        if self.compatibility != expected {
            return Err(self);
        }
        let Self {
            compatibility,
            footprint,
            payload,
        } = self;
        match payload.0.downcast::<T>() {
            Ok(payload) => Ok(*payload),
            Err(payload) => Err(Self {
                compatibility,
                footprint,
                payload: HostBindingArtifact(payload),
            }),
        }
    }
}

#[derive(Debug)]
struct Thing {
    id: String,
    property_name: String,
    target: String,
}

struct PlanBuildInput<'a, R: ?Sized> {
    validated_td: &'a Thing,
    registrations: &'a R,
    plan_set_generation: PlanSetGeneration,
}

impl<'a, R: ?Sized> PlanBuildInput<'a, R> {
    const fn new(
        validated_td: &'a Thing,
        registrations: &'a R,
        plan_set_generation: PlanSetGeneration,
    ) -> Self {
        Self {
            validated_td,
            registrations,
            plan_set_generation,
        }
    }

    const fn validated_td(&self) -> &'a Thing {
        self.validated_td
    }

    const fn registrations(&self) -> &'a R {
        self.registrations
    }

    const fn plan_set_generation(&self) -> PlanSetGeneration {
        self.plan_set_generation
    }
}

#[derive(Debug, Eq, PartialEq)]
struct PlanBuildOutput<A> {
    logical_plans: Vec<LogicalInteractionPlan>,
    artifacts: Vec<BindingArtifactEnvelope<A>>,
    artifact_refs: Vec<BindingArtifactRef>,
}

impl<A> PlanBuildOutput<A> {
    fn new(
        logical_plans: Vec<LogicalInteractionPlan>,
        artifacts: Vec<BindingArtifactEnvelope<A>>,
        artifact_refs: Vec<BindingArtifactRef>,
    ) -> Self {
        Self {
            logical_plans,
            artifacts,
            artifact_refs,
        }
    }

    fn logical_plans(&self) -> &[LogicalInteractionPlan] {
        &self.logical_plans
    }

    fn artifacts(&self) -> &[BindingArtifactEnvelope<A>] {
        &self.artifacts
    }

    fn artifact_refs(&self) -> &[BindingArtifactRef] {
        &self.artifact_refs
    }

    fn into_parts(
        self,
    ) -> (
        Vec<LogicalInteractionPlan>,
        Vec<BindingArtifactEnvelope<A>>,
        Vec<BindingArtifactRef>,
    ) {
        (self.logical_plans, self.artifacts, self.artifact_refs)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct PlanBuildFailure<C> {
    error: CoreError,
    cursor: C,
}

#[derive(Debug, Eq, PartialEq)]
#[must_use]
enum PlanBuildStep<C, A> {
    Pending(C),
    Complete(PlanBuildOutput<A>),
    Failed(PlanBuildFailure<C>),
}

trait PlanCompiler<R: ?Sized> {
    type Cursor;
    type Artifact;

    fn start(&self, input: &PlanBuildInput<'_, R>) -> CoreResult<Self::Cursor>;

    fn step(
        &self,
        input: &PlanBuildInput<'_, R>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> PlanBuildStep<Self::Cursor, Self::Artifact>;

    fn abort(&self, cursor: Self::Cursor);
}

#[derive(Debug, Eq, PartialEq)]
enum PropertyReadBuildCursor {
    Start,
    Compiling {
        plan: LogicalInteractionPlan,
        candidate: BindingCandidate,
        admitted: BindingArtifactFootprint,
        compiler_cursor: AppCursor,
    },
    ArtifactReady {
        plan: LogicalInteractionPlan,
        candidate: BindingCandidate,
        admitted: BindingArtifactFootprint,
        artifact: BindingArtifact<AppArtifact>,
    },
}

struct PropertyReadPlanCompiler;

impl PlanCompiler<[StaticBindingCompilerRegistration<AppCompiler>]> for PropertyReadPlanCompiler {
    type Cursor = PropertyReadBuildCursor;
    type Artifact = AppArtifact;

    fn start(
        &self,
        input: &PlanBuildInput<'_, [StaticBindingCompilerRegistration<AppCompiler>]>,
    ) -> CoreResult<Self::Cursor> {
        if input.registrations().is_empty() {
            return Err(CoreError("no compiler registration"));
        }
        Ok(PropertyReadBuildCursor::Start)
    }

    fn step(
        &self,
        input: &PlanBuildInput<'_, [StaticBindingCompilerRegistration<AppCompiler>]>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> PlanBuildStep<Self::Cursor, Self::Artifact> {
        if budget.remaining_binding_polls() == 0 {
            return PlanBuildStep::Pending(cursor);
        }
        let (plan, candidate, admitted, compiler_cursor) = match cursor {
            PropertyReadBuildCursor::Start => {
                let td = input.validated_td();
                let plan = match LogicalInteractionPlan::try_property_read(
                    PlanId(3),
                    ThingId(td.id.clone()),
                    td.property_name.clone().into_boxed_str(),
                    0,
                    td.target.clone().into_boxed_str(),
                    Some("application/json".into()),
                    None,
                ) {
                    Ok(plan) => plan,
                    Err(error) => {
                        return PlanBuildStep::Failed(PlanBuildFailure {
                            error,
                            cursor: PropertyReadBuildCursor::Start,
                        });
                    }
                };
                let registration = &input.registrations()[0];
                let compatibility = registration.compiler().compatibility();
                let candidate = BindingCandidate::new(
                    BindingId(11),
                    BindingGeneration(12),
                    BindingConfigurationDigest::new([13; 32]),
                    compatibility,
                    0,
                    0,
                );
                let compiler_input =
                    BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);
                let admitted = match registration.compiler().bounds(&compiler_input) {
                    Ok(bounds) => bounds.artifact(),
                    Err(error) => {
                        return PlanBuildStep::Failed(PlanBuildFailure {
                            error,
                            cursor: PropertyReadBuildCursor::Start,
                        });
                    }
                };
                let compiler_cursor = match registration.compiler().start(&compiler_input) {
                    Ok(cursor) => cursor,
                    Err(error) => {
                        return PlanBuildStep::Failed(PlanBuildFailure {
                            error,
                            cursor: PropertyReadBuildCursor::Start,
                        });
                    }
                };
                (plan, candidate, admitted, compiler_cursor)
            }
            PropertyReadBuildCursor::Compiling {
                plan,
                candidate,
                admitted,
                compiler_cursor,
            } => (plan, candidate, admitted, compiler_cursor),
            PropertyReadBuildCursor::ArtifactReady {
                plan,
                candidate,
                admitted,
                artifact,
            } => {
                return finish_property_read_build(
                    input.plan_set_generation(),
                    plan,
                    candidate,
                    admitted,
                    artifact,
                );
            }
        };

        let registration = &input.registrations()[0];
        let compiler_input =
            BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);
        match registration
            .compiler()
            .step(&compiler_input, compiler_cursor, budget)
        {
            BindingCompilerStep::Pending(compiler_cursor) => {
                PlanBuildStep::Pending(PropertyReadBuildCursor::Compiling {
                    plan,
                    candidate,
                    admitted,
                    compiler_cursor,
                })
            }
            BindingCompilerStep::Complete(output) => finish_property_read_build(
                input.plan_set_generation(),
                plan,
                candidate,
                admitted,
                output.into_artifact(),
            ),
            BindingCompilerStep::Failed(failure) => {
                let (error, compiler_cursor) = failure.into_parts();
                PlanBuildStep::Failed(PlanBuildFailure {
                    error,
                    cursor: PropertyReadBuildCursor::Compiling {
                        plan,
                        candidate,
                        admitted,
                        compiler_cursor,
                    },
                })
            }
        }
    }

    fn abort(&self, _cursor: Self::Cursor) {}
}

fn finish_property_read_build(
    plan_set_generation: PlanSetGeneration,
    plan: LogicalInteractionPlan,
    candidate: BindingCandidate,
    admitted: BindingArtifactFootprint,
    artifact: BindingArtifact<AppArtifact>,
) -> PlanBuildStep<PropertyReadBuildCursor, AppArtifact> {
    let identity = BindingArtifactIdentity::new(
        plan_set_generation,
        plan.plan_id(),
        candidate.binding_id(),
        candidate.binding_generation(),
        candidate.configuration(),
        candidate.compatibility(),
        BindingArtifactRole::ConsumerCall,
    );
    let envelope = match BindingArtifactEnvelope::try_new(identity, admitted, artifact) {
        Ok(envelope) => envelope,
        Err(rejection) => {
            return PlanBuildStep::Failed(PlanBuildFailure {
                error: CoreError("artifact admission failed"),
                cursor: PropertyReadBuildCursor::ArtifactReady {
                    plan,
                    candidate,
                    admitted,
                    artifact: rejection.into_artifact(),
                },
            });
        }
    };
    PlanBuildStep::Complete(PlanBuildOutput::new(
        vec![plan],
        vec![envelope],
        vec![BindingArtifactRef::new(identity, SlotIndex(0))],
    ))
}

fn plan_and_input(
    compatibility: BindingArtifactCompatibility,
) -> (LogicalInteractionPlan, BindingCandidate) {
    let plan = LogicalInteractionPlan::try_property_read(
        PlanId(1),
        ThingId("urn:test:thing".into()),
        "temperature".into(),
        0,
        "mock://sensor/temperature".into(),
        Some("application/json".into()),
        None,
    )
    .unwrap();
    let candidate = BindingCandidate::new(
        BindingId(2),
        BindingGeneration(3),
        BindingConfigurationDigest::new([4; 32]),
        compatibility,
        5,
        0,
    );
    (plan, candidate)
}

#[test]
fn portable_step_and_envelope_preserve_ownership_and_bounds() {
    let compatibility = BindingArtifactCompatibility::new([7; 16]);
    let compiler = MockCompiler { compatibility };
    let (plan, candidate) = plan_and_input(compatibility);
    let input = BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);

    let cursor = compiler.start(&input).unwrap();
    let mut zero = WorkBudget::new();
    let cursor = match compiler.step(&input, cursor, &mut zero) {
        BindingCompilerStep::Pending(cursor) => cursor,
        other => panic!("zero budget did not preserve pending cursor: {other:?}"),
    };
    assert_eq!(cursor, MockCursor { remaining_steps: 2 });

    let mut budget = WorkBudget::new().with_binding_polls(2);
    let cursor = match compiler.step(&input, cursor, &mut budget) {
        BindingCompilerStep::Pending(cursor) => cursor,
        other => panic!("first charged step did not remain pending: {other:?}"),
    };
    let artifact = match compiler.step(&input, cursor, &mut budget) {
        BindingCompilerStep::Complete(output) => output.into_artifact(),
        other => panic!("second charged step did not complete: {other:?}"),
    };
    assert_eq!(artifact.payload().target.as_ref(), plan.resolved_target());

    let identity = BindingArtifactIdentity::new(
        PlanSetGeneration(8),
        plan.plan_id(),
        candidate.binding_id(),
        candidate.binding_generation(),
        candidate.configuration(),
        compatibility,
        BindingArtifactRole::ConsumerCall,
    );
    let rejected =
        BindingArtifactEnvelope::try_new(identity, BindingArtifactFootprint::new(1, 1), artifact)
            .unwrap_err();
    assert_eq!(
        rejected.reason(),
        BindingArtifactRejectionReason::FootprintExceeded
    );
    let artifact = rejected.into_artifact();
    let envelope = BindingArtifactEnvelope::try_new(
        identity,
        BindingArtifactFootprint::new(1, 1024),
        artifact,
    )
    .unwrap();
    assert_eq!(
        envelope.artifact().payload().target.as_ref(),
        "mock://sensor/temperature"
    );
}

#[test]
fn application_closed_static_enum_is_typed_and_rejects_variant_mismatch() {
    let mock_compatibility = BindingArtifactCompatibility::new([9; 16]);
    let alternate_compatibility = BindingArtifactCompatibility::new([10; 16]);
    let table = [
        StaticBindingCompilerRegistration::new(AppCompiler::Mock(MockCompiler {
            compatibility: mock_compatibility,
        })),
        StaticBindingCompilerRegistration::new(AppCompiler::Alternate(AlternateCompiler {
            compatibility: alternate_compatibility,
        })),
    ];
    let (plan, candidate) = plan_and_input(mock_compatibility);
    let input = BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);
    let mut budget = WorkBudget::new().with_binding_polls(1);
    let mismatch = table[0].compiler().step(
        &input,
        AppCursor::Alternate(AlternateCursor(44)),
        &mut budget,
    );
    match mismatch {
        BindingCompilerStep::Failed(failure) => {
            let (error, cursor) = failure.into_parts();
            assert_eq!(error, CoreError("static compiler/cursor variant mismatch"));
            assert_eq!(cursor, AppCursor::Alternate(AlternateCursor(44)));
        }
        _ => panic!("static variant mismatch lost its owned cursor"),
    }
}

#[test]
fn host_erasure_returns_original_cursor_and_artifact_on_mismatch() {
    let mock_compatibility = BindingArtifactCompatibility::new([11; 16]);
    let alternate_compatibility = BindingArtifactCompatibility::new([12; 16]);
    let mock = HostBindingCompilerRegistration::new(MockCompiler {
        compatibility: mock_compatibility,
    });
    let alternate = HostBindingCompilerRegistration::new(AlternateCompiler {
        compatibility: alternate_compatibility,
    });
    let (plan, candidate) = plan_and_input(mock_compatibility);
    let input = BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);

    let alternate_cursor = alternate.start(&input).unwrap();
    let mut budget = WorkBudget::new().with_binding_polls(3);
    let alternate_cursor = match mock.step(&input, alternate_cursor, &mut budget) {
        BindingCompilerStep::Failed(failure) => {
            assert_eq!(
                failure.error(),
                &CoreError("host compiler cursor type mismatch")
            );
            failure.into_parts().1
        }
        _ => panic!("host cursor mismatch was not ownership-preserving"),
    };
    let alternate_artifact = match alternate.step(&input, alternate_cursor, &mut budget) {
        BindingCompilerStep::Complete(output) => output.into_artifact(),
        _ => panic!("returned cursor no longer belonged to its original compiler"),
    };
    assert_eq!(
        alternate_artifact
            .try_payload::<AlternateArtifact>(alternate_compatibility)
            .copied(),
        Some(AlternateArtifact(7))
    );

    let cursor = mock.start(&input).unwrap();
    let cursor = match mock.step(&input, cursor, &mut budget) {
        BindingCompilerStep::Pending(cursor) => cursor,
        _ => panic!("mock host compiler skipped pending"),
    };
    let artifact = match mock.step(&input, cursor, &mut budget) {
        BindingCompilerStep::Complete(output) => output.into_artifact(),
        _ => panic!("mock host compiler did not complete"),
    };
    assert!(
        artifact
            .try_payload::<MockArtifact>(alternate_compatibility)
            .is_none()
    );
    let artifact = artifact
        .try_into_payload::<MockArtifact>(alternate_compatibility)
        .unwrap_err();
    let artifact = artifact
        .try_into_payload::<AlternateArtifact>(mock_compatibility)
        .unwrap_err();
    let payload = match artifact.try_into_payload::<MockArtifact>(mock_compatibility) {
        Ok(payload) => payload,
        Err(_) => panic!("matching host payload was not recoverable"),
    };
    assert_eq!(payload.target.as_ref(), plan.resolved_target());
}

#[test]
fn plan_output_owns_every_runtime_value_after_td_and_registrations_drop() {
    let output = {
        let compatibility = BindingArtifactCompatibility::new([13; 16]);
        let td = Thing {
            id: "urn:test:owned-output".into(),
            property_name: "level".into(),
            target: "mock://tank/level".into(),
        };
        let registrations = [StaticBindingCompilerRegistration::new(AppCompiler::Mock(
            MockCompiler { compatibility },
        ))];
        let input = PlanBuildInput::new(&td, &registrations[..], PlanSetGeneration(21));
        let compiler = PropertyReadPlanCompiler;
        let cursor = compiler.start(&input).unwrap();
        let mut zero = WorkBudget::new();
        let cursor = match compiler.step(&input, cursor, &mut zero) {
            PlanBuildStep::Pending(cursor) => cursor,
            other => panic!("zero budget advanced the plan build: {other:?}"),
        };
        assert_eq!(cursor, PropertyReadBuildCursor::Start);

        let mut first_step = WorkBudget::new().with_binding_polls(1);
        let cursor = match compiler.step(&input, cursor, &mut first_step) {
            PlanBuildStep::Pending(cursor @ PropertyReadBuildCursor::Compiling { .. }) => cursor,
            other => panic!("first plan step did not retain compiler progress: {other:?}"),
        };

        let mut second_step = WorkBudget::new().with_binding_polls(1);
        match compiler.step(&input, cursor, &mut second_step) {
            PlanBuildStep::Complete(output) => output,
            other => panic!("resumed property-read plan did not complete: {other:?}"),
        }
    };

    assert_eq!(output.logical_plans()[0].property_name(), "level");
    assert_eq!(
        output.logical_plans()[0].resolved_target(),
        "mock://tank/level"
    );
    match output.artifacts()[0].artifact().payload() {
        AppArtifact::Mock(payload) => {
            assert_eq!(payload.target.as_ref(), "mock://tank/level");
        }
        AppArtifact::Alternate(_) => panic!("wrong static artifact variant"),
    }
    assert_eq!(output.artifact_refs()[0].artifact_slot(), SlotIndex(0));
}
