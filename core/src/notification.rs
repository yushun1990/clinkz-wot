//! Consumer notification conversations, independent of native subscriptions.
//! This initial Host facet is raw: it preserves data roles, not TD qualification
//! or DataSchema validation. Native matching, ACKs and distribution stay native.
use alloc::rc::Rc;
use core::{
    convert::Infallible,
    task::{Context, Poll},
};

use crate::consumer::ConsumerInstanceRef;
use crate::{
    AffordanceTarget, BindingGeneration, BindingId, BindingInputRejection, CoreResult, Operation,
    Payload, SubscriptionId,
};

/// A retained issuer distinguishes even identical slots in different roots.
/// Copying an identity grants neither delivery permission nor shutdown power.
#[derive(Clone)]
pub struct NotificationIdentity {
    issuer: Rc<()>,
    pub consumer: ConsumerInstanceRef,
    pub subscription: SubscriptionId,
}
impl PartialEq for NotificationIdentity {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.issuer, &other.issuer)
            && self.consumer == other.consumer
            && self.subscription == other.subscription
    }
}
impl Eq for NotificationIdentity {}
impl core::fmt::Debug for NotificationIdentity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NotificationIdentity")
            .field("consumer", &self.consumer)
            .field("subscription", &self.subscription)
            .finish_non_exhaustive()
    }
}
pub struct NotificationIssuer(Rc<()>);
impl Default for NotificationIssuer {
    fn default() -> Self {
        Self(Rc::new(()))
    }
}
impl NotificationIssuer {
    pub fn identity(
        &self,
        consumer: ConsumerInstanceRef,
        subscription: SubscriptionId,
    ) -> NotificationIdentity {
        NotificationIdentity {
            issuer: self.0.clone(),
            consumer,
            subscription,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationKind {
    PropertyObservation,
    Event,
}
impl NotificationKind {
    pub fn for_target(target: &AffordanceTarget) -> Option<Self> {
        match target {
            AffordanceTarget::Property(_) => Some(Self::PropertyObservation),
            AffordanceTarget::Event(_) => Some(Self::Event),
            _ => None,
        }
    }
    pub const fn start_operation(self) -> Operation {
        match self {
            Self::PropertyObservation => Operation::ObserveProperty,
            Self::Event => Operation::SubscribeEvent,
        }
    }
    pub const fn stop_operation(self) -> Operation {
        match self {
            Self::PropertyObservation => Operation::UnobserveProperty,
            Self::Event => Operation::UnsubscribeEvent,
        }
    }
}

/// These are different application-data roles, not protocol control packets.
pub struct SubscriptionInput(pub Option<Payload>);
pub struct CancellationInput(pub Option<Payload>);
pub struct EventDataResponse(pub Option<Payload>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseRequirement {
    None,
    Required,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationOverflow {
    DropNewest,
    Terminate,
}

pub struct RawNotificationRequest {
    pub target: AffordanceTarget,
    pub subscription: SubscriptionInput,
    /// Chosen from the actual required semantics, not inferred from the mere
    /// presence of an Event dataResponse schema. Property observations use None.
    pub responses: ResponseRequirement,
    pub overflow: NotificationOverflow,
}

/// A supported per-notification response means an actual exclusive opportunity
/// for THIS logical recipient. A Binding must refuse incompatible physical
/// sharing/cardinality before acceptance; it may not duplicate one reply right.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotificationSupport {
    pub event_responses: bool,
    pub cancellation: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteCancellation {
    NotAttempted,
    Confirmed,
    Unknown,
    Unsupported,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventResponseDelivery {
    NotAttempted,
    Delivered,
    Unknown,
}

/// An original native response opportunity has its own owner. Pending/error
/// retains both correlation and local backing. Ready(Ok) from poll_deliver
/// certifies delivery of the ORIGINAL response at the declared native edge.
/// poll_retire must dispose all local obligations before Ready(Ok); it reports
/// remote delivery separately. It may retire an unused optional opportunity
/// without any poll_deliver call; retirement must not synthesize an application
/// dataResponse. abort_local consumes the entire owner in finite,
/// nonblocking local work, with no escaped tasks/callbacks or remote claim.
/// Trusted linked code must fund its own indirect backing and disposal envelope.
pub trait NotificationResponse {
    fn poll_deliver(
        &mut self,
        cx: &mut Context<'_>,
        response: &EventDataResponse,
    ) -> Poll<CoreResult<()>>;
    fn poll_retire(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<EventResponseDelivery>>;
    fn abort_local(self);
}
impl NotificationResponse for Infallible {
    fn poll_deliver(&mut self, _: &mut Context<'_>, _: &EventDataResponse) -> Poll<CoreResult<()>> {
        match *self {}
    }
    fn poll_retire(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<EventResponseDelivery>> {
        match *self {}
    }
    fn abort_local(self) {
        match self {}
    }
}

/// A native candidate, not permission to deliver. Even a late or refused value
/// retains its original recipient and response owner until disposal.
pub struct NativeNotification<R> {
    pub recipient: NotificationIdentity,
    pub data: Option<Payload>,
    pub response: Option<R>,
}
pub enum NotificationSignal<R> {
    Data(NativeNotification<R>),
    /// Visible native loss. The native owner has disposed any lost opportunities
    /// according to its declared policy; this is not an Engine replay request.
    Loss(u64),
}

/// A lease/participation, not the physical service. Close releases only this
/// logical participation; replies may retain independent physical work leases.
/// Native queues, matching, distribution, sharing compatibility, idle retention,
/// reconnect and final infrastructure shutdown belong to the Binding.
pub trait NotificationParticipation {
    type Response: NotificationResponse;
    fn poll_start(
        &mut self,
        cx: &mut Context<'_>,
        input: &SubscriptionInput,
    ) -> Poll<CoreResult<()>>;
    /// Bound complete payload backing BEFORE handoff. Pending registers wake
    /// interest and rechecks readiness; callbacks cannot deliver to applications.
    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
        max_payload_backing: usize,
    ) -> Poll<CoreResult<NotificationSignal<Self::Response>>>;
    /// None requests local removal without application cancellation authority.
    /// Some is a separately authorized mapped cancellation input. Pending/error
    /// retains the entire participant. Ready(Ok) means complete LOCAL disposal,
    /// not physical shutdown or necessarily confirmed remote cancellation.
    fn poll_close(
        &mut self,
        cx: &mut Context<'_>,
        cancellation: Option<&CancellationInput>,
    ) -> Poll<CoreResult<RemoteCancellation>>;
    /// Mandatory finite nonblocking whole-owner local abort at Host-root Drop.
    /// No remote callback, network wait, detached task or remote success claim.
    fn abort_local(self);
}

/// Startup-registered source-free capability. No compiler or TD is retained.
/// support is passive and bounded for the actual program/configuration.
/// prepare is local construction ONLY: no external effects or escaped work.
/// Rejection returns the exact program; native establishment begins in the
/// returned participant's poll_start, including all provisional cleanup owners.
/// Registration/configuration/program destructors must provide finite local
/// disposal too; implementations requiring a blocking service shutdown are not
/// qualified for this Host facet. External service lifetime is application-owned.
pub trait ConsumerNotificationBinding {
    type Program;
    type Configuration: Eq;
    type Participation: NotificationParticipation;
    fn identity(&self) -> (BindingId, BindingGeneration);
    fn configuration(&self) -> &Self::Configuration;
    fn support(
        &self,
        program: &Self::Program,
        kind: NotificationKind,
    ) -> CoreResult<NotificationSupport>;
    fn prepare(
        &mut self,
        program: Self::Program,
        recipient: &NotificationIdentity,
    ) -> Result<Self::Participation, BindingInputRejection<Self::Program>>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationPermission {
    Establish,
    Deliver,
    Respond,
    Cancel,
}
/// Actual captured application policy, rechecked at long-lived delivery and
/// response/cancel. Native trust/credential services remain Binding-owned.
pub trait NotificationAuthorizer {
    fn authorize(
        &self,
        recipient: &NotificationIdentity,
        target: &AffordanceTarget,
        permission: NotificationPermission,
    ) -> CoreResult<()>;
}

/// Full retained payload backing, including both String capacities and the Arc
/// slice allocation. Shared bodies need no copy; this conservative recipient
/// allowance is not an additive physical-RAM measurement of shared allocations.
pub fn notification_payload_backing(payload: Option<&Payload>) -> Option<usize> {
    use core::{alloc::Layout, mem::size_of};
    let Some(p) = payload else { return Some(0) };
    let body = Layout::new::<[usize; 2]>()
        .extend(Layout::array::<u8>(p.body.len()).ok()?)
        .ok()?
        .0
        .pad_to_align();
    size_of::<Payload>()
        .checked_add(body.size())?
        .checked_add(p.content_type.capacity())?
        .checked_add(p.content_coding.as_ref().map_or(0, |s| s.capacity()))
}
