//! Source-free, exclusive finite ingress. A native Binding matches and retains
//! the wire opportunity; it never receives an application-dispatch capability.
//! Raw activation makes no TD-publication or Action-lifetime claim. A runtime
//! can additionally require the actual provider's prepared output validator.

use alloc::vec::Vec;
use core::any::Any;
use core::task::{Context, Poll};

use crate::Deadline;
use crate::producer::ProducerInstanceKey;
use crate::{
    AffordanceTarget, BindingGeneration, BindingId, CoreResult, HandlerContext, InteractionInput,
    InteractionOutput, InteractionStatus, Operation, ResponsePayloadRole,
};
use clinkz_wot_foundation::MonotonicInstant;

/// One *candidate*, not a grant to dispatch. A copied key cannot keep a revoked
/// instance alive or redirect an accepted exchange to its replacement.
#[derive(Debug)]
pub struct FiniteCandidate {
    pub instance: ProducerInstanceKey,
    pub target: AffordanceTarget,
    pub operation: Operation,
}

/// One owned native input and its single exclusive reply opportunity. A Binding
/// may match zero or many candidates; this facet refuses ambiguity instead of
/// inventing a unique replier or distributing one reply among recipients.
pub struct FiniteIngress<R> {
    pub candidates: Vec<FiniteCandidate>,
    pub input: InteractionInput,
    pub reply: R,
}

/// The complete original input, candidates and reply opportunity stay together
/// through pending sends, refusal, revocation, failure and native close. Success
/// here is explicitly *raw* data; DataSchema conformance is not implied by the
/// carrier. Handler invocation is never proof of write effect certainty.
pub struct RawFiniteReply<R> {
    pub ingress: FiniteIngress<R>,
    pub result: CoreResult<InteractionOutput>,
    pub effect: FiniteEffect,
    pub recipient: Option<ProducerInstanceKey>,
    pub operation: Option<Operation>,
}

/// Semantic-effect certainty, independent of the native reply outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FiniteEffect {
    /// The handler was not entered; no application effect was attempted.
    NotInvoked,
    /// The handler was entered. Even an error or invalid/oversized result may
    /// follow a committed write; implicit replay is not safe.
    Unknown,
}

/// Whether the original result reached the Binding's declared delivery edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FiniteDelivery {
    Delivered,
    NotDelivered,
    Unknown,
}

/// One terminal native and semantic outcome, including the complete owned late
/// result (even if delivery was refused or uncertain) and original mapping
/// identity. Each native runtime retains one unread settlement; another exchange
/// on that runtime cannot be received until it is taken. Siblings are independent.
pub struct RawFiniteSettlement {
    /// Actual native runtime association, retained even after independent close.
    pub binding: (BindingId, BindingGeneration),
    /// Shared view of the original registered target backing. Refused/unaccepted
    /// input has no accepted target; its native input is settled separately.
    pub target: Option<AffordanceTarget>,
    pub recipient: Option<ProducerInstanceKey>,
    pub operation: Option<Operation>,
    pub effect: FiniteEffect,
    pub delivery: FiniteDelivery,
    pub result: CoreResult<InteractionOutput>,
}

impl core::fmt::Debug for RawFiniteSettlement {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("RawFiniteSettlement")
            .field("binding", &self.binding)
            .field("has_target", &self.target.is_some())
            .field("recipient", &self.recipient)
            .field("operation", &self.operation)
            .field("effect", &self.effect)
            .field("delivery", &self.delivery)
            // CoreError Debug is redacted and bounded. A successful output is
            // application data; never format its body or media metadata.
            .field("error", &self.result.as_ref().err())
            .field("has_output", &self.result.is_ok())
            .finish()
    }
}

/// Basic Property/collection-read result shape. Raw mode permits unvalidated data bytes, not
/// Action status/invocation metadata, untrusted Binding response claims, or a
/// missing Read value. This initial Write facet supports empty success only.
pub fn raw_finite_property_shape(operation: Operation, output: &InteractionOutput) -> bool {
    match operation {
        Operation::ReadProperty | Operation::ReadAllProperties => {
            crate::response::property_read_handler_shape(output)
        }
        Operation::WriteProperty => {
            let metadata = output.metadata();
            output.data().is_none()
                && output.status() == InteractionStatus::Ok
                && metadata.payload_role() == ResponsePayloadRole::Application
                && metadata.action_invocation().is_none()
                && metadata.binding_response().is_none()
        }
        _ => false,
    }
}

/// Binding-owned matching, I/O, correlation and physical retirement for finite
/// exchanges with one exclusive reply. `poll_start` may acquire native resources
/// but MUST keep semantic ingress closed until the runtime activates locally.
/// The Binding bounds input/metadata and candidate backing before returning an
/// ingress. A pending or failed send retains its complete original exchange until
/// `poll_close` has disposed native work. Close receives the *complete* unmatched
/// input or accepted reply, including the original opportunity. It MUST perform
/// the supported native refusal/abort or report an uncertain delivery; it must
/// not silently discard the opportunity. Drop of Binding/Reply performs finite
/// local disposal without network waits, reporting no completed drain.
pub trait FiniteIngressBinding {
    type Reply;
    type Configuration: Eq;

    fn identity(&self) -> (BindingId, BindingGeneration);
    /// Actual native configuration, borrowed without rebuilding a matching tag.
    /// Equality with the captured value must be passive and bounded on each
    /// progress step; an unbounded native configuration needs a different
    /// immutable revision-bearing representation.
    fn configuration(&self) -> &Self::Configuration;
    /// Additional Engine-owned backing for the captured configuration pair
    /// and admitted input, excluding their inline Rust layouts. A managed Host
    /// composition requires this actual bound before native setup. Native
    /// socket/parser/SDK backing keeps its separate Binding policy. None makes
    /// no managed-account claim; raw execution remains independently available.
    fn engine_backing_bytes(&self) -> Option<usize> {
        None
    }
    /// Passive closed-participation recipient check. This is a native mapping
    /// claim, not logical permission; a shared Binding may prepare several
    /// independent Things. Raw-only Bindings gain no publication readiness.
    fn prepared_for_instance(&self, _key: ProducerInstanceKey) -> bool {
        false
    }
    /// Passive exact mapping claim for a qualified Producer publication. The
    /// default is *no* coverage: existing raw-only finite Bindings do not gain
    /// advertised TD support by implementing the I/O facet alone.
    fn covers_definition(
        &self,
        _key: ProducerInstanceKey,
        _site: crate::thing::AffordanceSite<'_>,
        _form: u32,
        _operation: Operation,
        _definition: &dyn Any,
    ) -> bool {
        false
    }
    /// Actual source-independent service for this captured native mapping set.
    /// None is an absent facet, not a successful validation. A runtime requiring
    /// validation must fence progress if this service disappears. Providers with
    /// multiple mappings must check both data roles for each original candidate;
    /// a service for a different operation cannot satisfy the selected mapping.
    /// The service and its operands belong to the captured Configuration;
    /// substituting them must invalidate that pairing, not preserve its tag.
    fn validator(&self) -> Option<&dyn FiniteValidator> {
        None
    }
    /// Passive deadline of the original retained native exchange, including
    /// unfinished ingress and pending delivery. Clock identity belongs to the
    /// caller; native code must not substitute a wall clock or reset on polls.
    fn request_deadline(&self) -> Deadline {
        Deadline::NONE
    }
    fn poll_start(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>>;
    fn poll_receive(
        &mut self,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> Poll<CoreResult<FiniteIngress<Self::Reply>>>;
    /// Ready(Ok) certifies that the original result reached this Binding's
    /// declared delivery boundary. Pending and error do not. Raw mode waives
    /// schema checks, not this guarantee: substituting an error response cannot
    /// certify delivery of a retained successful value.
    fn poll_send(
        &mut self,
        cx: &mut Context<'_>,
        reply: &RawFiniteReply<Self::Reply>,
    ) -> Poll<CoreResult<()>>;
    /// The borrowed unmatched ingress or accepted reply stays owned by Servient
    /// until terminal success. `cause` describes the first local close reason.
    /// Report whether its original result/refusal was delivered; a partially
    /// sent result MUST be classified Unknown. Pending/error retains the complete
    /// owner for a subsequent close poll.
    fn poll_close(
        &mut self,
        cx: &mut Context<'_>,
        ingress: Option<&FiniteIngress<Self::Reply>>,
        reply: Option<&RawFiniteReply<Self::Reply>>,
        cause: Option<&crate::CoreError>,
    ) -> Poll<CoreResult<FiniteDelivery>>;
}

/// Captured source-independent validation for this participant's mappings.
/// Input checks precede application authorization and behavior effects. Output
/// checks do not establish whether an errored/late write committed. The exact
/// original candidate selects both roles; no TD or operation switch is needed
/// in the Engine. Callbacks are pure and bounded by the runtime input/output
/// envelope and must not acquire native effects or application authority.
pub trait FiniteValidator {
    /// Complete additional callback work beyond the runtime's declared maximum
    /// input/output body and media traversals. Passive bounded metadata, captured
    /// at admission and rechecked before effects. Zero retains that base policy.
    fn additional_validation_work(&self) -> u64 {
        0
    }
    fn validate_input(
        &self,
        candidate: &FiniteCandidate,
        input: &InteractionInput,
    ) -> CoreResult<()>;
    fn validate_output(
        &self,
        candidate: &FiniteCandidate,
        output: &InteractionOutput,
    ) -> CoreResult<()>;
}

/// Application behavior for a *raw* finite operation. Authorization runs at
/// each semantic acceptance, independently of native matching/verification.
/// Implementations that promise schema-validated data must supply a separate
/// executable validation boundary before making that claim.
pub trait RawFiniteHandler {
    fn authorize(&self, input: &InteractionInput) -> CoreResult<()>;
    fn handle(
        &self,
        context: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput>;
}
