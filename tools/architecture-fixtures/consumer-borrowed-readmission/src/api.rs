//! Frozen portable signatures, wrapping private candidate mechanics.
use super::*;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingPhase {
    Inspect,
    Basic,
    Semantics,
    Rollback,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingInvalidKind {
    MissingRequiredField,
    InvalidOperation,
    InvalidSchema,
    InvalidSecurity,
    InvalidUri,
    InvalidReference,
    InvalidContext,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedThingInvalid {
    kind: ValidatedThingInvalidKind,
    phase: ValidatedThingPhase,
    node_ordinal: u64,
    source: Cause,
}
impl ValidatedThingInvalid {
    pub fn kind(&self) -> ValidatedThingInvalidKind {
        self.kind
    }
    pub fn phase(&self) -> ValidatedThingPhase {
        self.phase
    }
    pub fn node_ordinal(&self) -> u64 {
        self.node_ordinal
    }
    pub fn diagnostic_for_fixture(&self) -> Cause {
        self.source
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedThingLimit {
    kind: R,
    configured: u64,
    observed: u64,
    phase: ValidatedThingPhase,
}
impl ValidatedThingLimit {
    pub fn kind(&self) -> R {
        self.kind
    }
    pub fn configured(&self) -> u64 {
        self.configured
    }
    pub fn observed(&self) -> u64 {
        self.observed
    }
    pub fn phase(&self) -> ValidatedThingPhase {
        self.phase
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingFailureKind {
    CheckedArithmetic,
    AllocationFailed,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedThingFailure {
    kind: ValidatedThingFailureKind,
    phase: ValidatedThingPhase,
    requested_bytes: u64,
}
impl ValidatedThingFailure {
    pub fn kind(&self) -> ValidatedThingFailureKind {
        self.kind
    }
    pub fn phase(&self) -> ValidatedThingPhase {
        self.phase
    }
    pub fn requested_bytes(&self) -> u64 {
        self.requested_bytes
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingCause {
    Invalid(ValidatedThingInvalid),
    Limit(ValidatedThingLimit),
    Cancelled { phase: ValidatedThingPhase },
    Failed(ValidatedThingFailure),
}
fn cause(
    source: Cause,
    phase: ValidatedThingPhase,
    policy: Policy,
    trace: Trace,
) -> ValidatedThingCause {
    use ValidatedThingInvalidKind as I;
    match source {
        Cause::Resource {
            kind,
            configured,
            observed,
        } => ValidatedThingCause::Limit(ValidatedThingLimit {
            kind,
            configured,
            observed,
            phase,
        }),
        Cause::Cancelled => ValidatedThingCause::Cancelled { phase },
        Cause::Allocation { requested_bytes } => {
            ValidatedThingCause::Failed(ValidatedThingFailure {
                kind: ValidatedThingFailureKind::AllocationFailed,
                phase,
                requested_bytes,
            })
        }
        Cause::Arithmetic | Cause::Configuration => {
            ValidatedThingCause::Failed(ValidatedThingFailure {
                kind: ValidatedThingFailureKind::CheckedArithmetic,
                phase,
                requested_bytes: 0,
            })
        }
        Cause::Nodes
        | Cause::Members
        | Cause::Depth
        | Cause::Text
        | Cause::Content
        | Cause::Number
        | Cause::Lifetime
        | Cause::Memory => {
            let (kind, observed) = match source {
                Cause::Nodes => (R::JsonValueNodesPerDocumentMax, trace.nodes as u64),
                Cause::Members => (
                    R::JsonMembersPerObjectMax,
                    policy.get(R::JsonMembersPerObjectMax) + 1,
                ),
                Cause::Depth => (R::JsonNestingDepthMax, trace.max_depth as u64),
                Cause::Text => (R::StringBytesMax, trace.content as u64),
                Cause::Content => (R::DocumentBytesMax, trace.content as u64),
                Cause::Number => (
                    R::NumberLexemeBytesMax,
                    policy.get(R::NumberLexemeBytesMax) + 1,
                ),
                Cause::Lifetime => (
                    R::DocumentValidationWorkUnitsMax,
                    trace.work.iter().sum::<u64>() + 1,
                ),
                _ => (R::AdmissionTemporaryBytesPerOperationMax, trace.live),
            };
            ValidatedThingCause::Limit(ValidatedThingLimit {
                kind,
                configured: policy.get(kind),
                observed,
                phase,
            })
        }
        _ => {
            let kind = match source {
                Cause::Invalid(v) => match v.rule {
                    b::InlineRule::Schema(_) => I::InvalidSchema,
                    b::InlineRule::Basic(b::RuleKind::Operation) => I::InvalidOperation,
                    b::InlineRule::Basic(b::RuleKind::Missing) => I::MissingRequiredField,
                    b::InlineRule::Basic(b::RuleKind::Undefined) => I::InvalidReference,
                    _ => I::InvalidSecurity,
                },
                Cause::MissingId => I::MissingRequiredField,
                Cause::Security => I::InvalidSecurity,
                Cause::Uri => I::InvalidUri,
                _ => unreachable!(),
            };
            let node_ordinal = if let Cause::Invalid(v) = source {
                v.schema_ordinal.unwrap_or(0)
            } else {
                0
            };
            ValidatedThingCause::Invalid(ValidatedThingInvalid {
                kind,
                phase,
                node_ordinal,
                source,
            })
        }
    }
}
pub struct ValidatedThingCursor<'td>(Validation<'td>);
pub struct ValidatedThing<'td>(Validated<'td>);
pub enum ValidatedThingProgress<'td> {
    Pending(ValidatedThingCursor<'td>),
    Complete(ValidatedThing<'td>),
    Failed(ValidatedThingCause),
}
impl<'td> ValidatedThingCursor<'td> {
    pub fn from_thing(
        thing: &'td Thing,
        config: &ValidatedThingAdmissionConfig,
        ledger: AdmissionLedger,
    ) -> Self {
        Self(Validation::new(thing, config.policy, ledger))
    }
    pub fn step(
        self,
        budget: &mut WorkBudget,
        cancel_requested: bool,
    ) -> ValidatedThingProgress<'td> {
        let policy = self.0.policy;
        match self.0.step(budget, cancel_requested) {
            Progress::Pending(v) => ValidatedThingProgress::Pending(Self(v)),
            Progress::Complete(v) => ValidatedThingProgress::Complete(ValidatedThing(v)),
            Progress::Failed(c, t, basic) => ValidatedThingProgress::Failed(cause(
                c,
                if basic {
                    ValidatedThingPhase::Basic
                } else {
                    ValidatedThingPhase::Inspect
                },
                policy,
                t,
            )),
        }
    }
    pub fn trace(&self) -> Trace {
        self.0.trace()
    }
    pub fn lifetime_remaining(&self) -> u64 {
        self.0.lifetime_remaining()
    }
    pub fn transferring(&self) -> bool {
        self.0.transferring()
    }
    pub fn frame_request_pending(&self) -> bool {
        self.0.frame_request_pending()
    }
    pub fn next_atomic_number_bytes(&mut self) -> Option<u64> {
        self.0.numeric_charge()
    }
}
impl<'td> ValidatedThing<'td> {
    pub fn into_property_read(self) -> ValidatedPropertyReadCursor<'td> {
        ValidatedPropertyReadCursor(Read::new(self.0))
    }
    pub fn trace(&self) -> Trace {
        self.0.trace()
    }
}
pub struct ValidatedPropertyReadCursor<'td>(Read<'td>);
pub enum ValidatedPropertyReadStep<'step> {
    Pending,
    Ready(ValidatedPropertyReadEvent<'step>),
    Done,
}
pub enum ValidatedPropertyReadEvent<'step> {
    Property { ordinal: u32, name: &'step str },
    Form(ValidatedPropertyReadForm<'step>),
}
pub struct ValidatedPropertyReadForm<'step>(Fact<'step>);
pub struct ValidatedTextSequence<'step>(TextSequence<'step>);
impl<'step> ValidatedTextSequence<'step> {
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn byte_len(&self) -> u64 {
        self.0.byte_len()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &'step str> + 'step {
        self.0.iter()
    }
}
impl<'step> ValidatedPropertyReadForm<'step> {
    pub fn property_ordinal(&self) -> u32 {
        self.0.property as u32
    }
    pub fn property_name(&self) -> &'step str {
        self.0.name
    }
    pub fn original_index(&self) -> u32 {
        self.0.original as u32
    }
    pub fn href(&self) -> &'step str {
        self.0.raw
    }
    pub fn resolved_href(&self) -> &'step str {
        self.0.resolved
    }
    pub fn content_type(&self) -> &'step str {
        self.0.content_type
    }
    pub fn content_coding(&self) -> Option<&'step str> {
        self.0.content_coding
    }
    pub fn subprotocol(&self) -> Option<&'step str> {
        self.0.subprotocol
    }
    pub fn scopes(&self) -> ValidatedTextSequence<'step> {
        ValidatedTextSequence(self.0.scopes)
    }
    pub fn readable(&self) -> bool {
        true
    }
    pub fn security_count(&self) -> u64 {
        match self.0.security {
            SecurityFact::Empty => 0,
            SecurityFact::Multiple(n) => n as u64,
            _ => 1,
        }
    }
    pub fn security_name(&self) -> Option<&'step str> {
        if let SecurityFact::Single { name, .. } = self.0.security {
            Some(name)
        } else {
            None
        }
    }
    pub fn security_scheme(&self) -> Option<&'step str> {
        if let SecurityFact::Single { scheme, .. } = self.0.security {
            Some(scheme)
        } else {
            None
        }
    }
    pub fn copy_bytes(&self) -> u64 {
        self.0.copy_bytes
    }
}
impl<'td> ValidatedPropertyReadCursor<'td> {
    pub fn id(&self) -> Option<&str> {
        self.0.id()
    }
    pub fn step(
        &mut self,
        budget: &mut WorkBudget,
        cancel_requested: bool,
    ) -> Result<ValidatedPropertyReadStep<'_>, ValidatedThingCause> {
        let policy = self.0.proof.policy;
        let trace = self.0.trace();
        if let Some(c) = self.0.failure {
            return Err(cause(c, ValidatedThingPhase::Semantics, policy, trace));
        }
        // First cause is stored by private progress, before lending any result.
        match self.0.step(budget, cancel_requested) {
            Ok(Event::Pending) => Ok(ValidatedPropertyReadStep::Pending),
            Ok(Event::Done) => Ok(ValidatedPropertyReadStep::Done),
            Ok(Event::Property { ordinal, name }) => Ok(ValidatedPropertyReadStep::Ready(
                ValidatedPropertyReadEvent::Property {
                    ordinal: ordinal as u32,
                    name,
                },
            )),
            Ok(Event::Form(f)) => Ok(ValidatedPropertyReadStep::Ready(
                ValidatedPropertyReadEvent::Form(ValidatedPropertyReadForm(f)),
            )),
            Err(c) => Err(cause(c, ValidatedThingPhase::Semantics, policy, trace)),
        }
    }
    pub fn acknowledge(&mut self) {
        self.0.acknowledge();
    }
    pub fn rewind(self) -> Self {
        if self.0.stage == 7 {
            Self(Read::new(self.0.finish()))
        } else {
            self
        }
    }
    pub fn trace(&self) -> Trace {
        self.0.trace()
    }
    pub fn lifetime_remaining(&self) -> u64 {
        self.0.lifetime_remaining()
    }
    pub fn uri_validation_bytes(&self) -> u64 {
        self.0.uri_validation_bytes()
    }
    pub fn uri_observations(&self) -> UriObservations {
        let mut o = self.0.uri_observed;
        if let Some(u) = &self.0.uri {
            let n = u.observations();
            o.component_bytes += n.component_bytes;
            o.merge_bytes += n.merge_bytes;
            o.segment_bytes += n.segment_bytes;
            o.pop_bytes += n.pop_bytes;
            o.emitted_bytes += n.emitted_bytes;
            o.reversed_bytes += n.reversed_bytes;
            o.shifted_bytes += n.shifted_bytes;
            o.utf8_bytes += n.utf8_bytes;
        }
        o
    }
    pub fn scope_visits(&self) -> u64 {
        self.0.scope_visits()
    }
    pub fn ready(&self) -> bool {
        self.0.ready()
    }
}

/// Actual layouts, including the inline observation state used by this fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayoutRow {
    pub name: &'static str,
    pub size: usize,
    pub alignment: usize,
}
pub const fn layout_catalog() -> [LayoutRow; 6] {
    const fn row<T>(name: &'static str) -> LayoutRow {
        LayoutRow {
            name,
            size: core::mem::size_of::<T>(),
            alignment: core::mem::align_of::<T>(),
        }
    }
    [
        row::<ValidatedThingCursor<'static>>("inspect"),
        row::<ValidatedThing<'static>>("proof"),
        row::<ValidatedPropertyReadCursor<'static>>("semantic"),
        row::<ValidatedThingProgress<'static>>("inspect-result"),
        row::<super::Job<'static>>("frame"),
        row::<u8>("URI-byte"),
    ]
}
impl ValidatedThingAdmissionConfig {
    pub fn frame_capacity(&self) -> usize {
        self.policy.frames
    }
    pub fn controlled_heap_envelope(&self) -> u64 {
        let frames =
            2 * self.policy.frames as u64 * core::mem::size_of::<super::Job<'static>>() as u64;
        frames.max(self.policy.get(R::UriTemplateSourceBytesMax))
    }
}
impl ValidatedThingCursor<'_> {
    pub fn allocation_events(&self) -> &[super::storage::AllocationEvent] {
        self.0.storage.events()
    }
}
impl ValidatedThing<'_> {
    pub fn allocation_events(&self) -> &[super::storage::AllocationEvent] {
        self.0.storage.events()
    }
}
impl ValidatedPropertyReadCursor<'_> {
    pub fn allocation_events(&self) -> &[super::storage::AllocationEvent] {
        self.0.proof.storage.events()
    }
}
pub use super::storage::{AllocationEvent, AllocationEventKind};
