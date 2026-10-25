//! Source-free, exclusive finite ingress. A native Binding matches and retains
//! the wire opportunity; it never receives an application-dispatch capability.
//! This raw-value slice makes no DataSchema or Action-lifetime claim.

use alloc::vec::Vec;
use core::task::{Context, Poll};

use crate::producer::ProducerInstanceKey;
use crate::{
    AffordanceTarget, BindingGeneration, BindingId, CoreResult, HandlerContext, InteractionInput,
    InteractionOutput, Operation,
};

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

/// The original reply opportunity stays with its result through pending sends,
/// revocation, failure and native close. Success here is explicitly *raw* data;
/// no structural or schema validation is implied by the carrier.
pub struct RawFiniteReply<R> {
    pub opportunity: R,
    pub result: CoreResult<InteractionOutput>,
}

/// Binding-owned matching, I/O, correlation and physical retirement for finite
/// exchanges with one exclusive reply. `poll_start` may acquire native resources
/// but MUST keep semantic ingress closed until the runtime activates locally.
/// The Binding bounds input/metadata and candidate backing before returning an
/// ingress. A pending or failed send retains its original opportunity until
/// `poll_close` has disposed native work. Drop performs finite local disposal.
pub trait FiniteIngressBinding {
    type Reply;
    type Configuration: Eq;

    fn identity(&self) -> (BindingId, BindingGeneration);
    /// Actual native configuration, borrowed without rebuilding a matching tag.
    fn configuration(&self) -> &Self::Configuration;
    fn poll_start(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>>;
    fn poll_receive(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<CoreResult<FiniteIngress<Self::Reply>>>;
    fn poll_send(
        &mut self,
        cx: &mut Context<'_>,
        reply: &RawFiniteReply<Self::Reply>,
    ) -> Poll<CoreResult<()>>;
    fn poll_close(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>>;
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
