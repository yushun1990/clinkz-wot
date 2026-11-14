//! Consumer notification conversations, independent of native subscriptions.
//! This initial Host facet is raw: it preserves data roles, not TD qualification
//! or DataSchema validation. Native matching, ACKs and distribution stay native.
use crate::{
    collection::PropertyCollectionSchema, consumer::ConsumerDataService,
    data_schema::PreparedSchema,
};
use alloc::{rc::Rc, sync::Arc};
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
    /// One Thing-root conversation carrying named Property collection data.
    ObserveAllProperties,
    Event,
    /// One Thing-root conversation carrying individually attributed Events.
    SubscribeAllEvents,
}
impl NotificationKind {
    /// Scope alone does not select a notification family: Thing-root Forms
    /// can describe Property or Event collections. Capture the operation kind.
    pub fn for_operation(operation: Operation) -> Option<Self> {
        match operation {
            Operation::ObserveProperty | Operation::UnobserveProperty => {
                Some(Self::PropertyObservation)
            }
            Operation::ObserveAllProperties | Operation::UnobserveAllProperties => {
                Some(Self::ObserveAllProperties)
            }
            Operation::SubscribeEvent | Operation::UnsubscribeEvent => Some(Self::Event),
            Operation::SubscribeAllEvents | Operation::UnsubscribeAllEvents => {
                Some(Self::SubscribeAllEvents)
            }
            _ => None,
        }
    }
    pub fn accepts_target(self, target: &AffordanceTarget) -> bool {
        matches!(
            (self, target),
            (Self::PropertyObservation, AffordanceTarget::Property(_))
                | (Self::ObserveAllProperties, AffordanceTarget::Thing)
                | (Self::Event, AffordanceTarget::Event(_))
                | (Self::SubscribeAllEvents, AffordanceTarget::Thing)
        )
    }
    pub const fn start_operation(self) -> Operation {
        match self {
            Self::PropertyObservation => Operation::ObserveProperty,
            Self::ObserveAllProperties => Operation::ObserveAllProperties,
            Self::Event => Operation::SubscribeEvent,
            Self::SubscribeAllEvents => Operation::SubscribeAllEvents,
        }
    }
    pub const fn stop_operation(self) -> Operation {
        match self {
            Self::PropertyObservation => Operation::UnobserveProperty,
            Self::ObserveAllProperties => Operation::UnobserveAllProperties,
            Self::Event => Operation::UnsubscribeEvent,
            Self::SubscribeAllEvents => Operation::UnsubscribeAllEvents,
        }
    }
    pub const fn is_event(self) -> bool {
        matches!(self, Self::Event | Self::SubscribeAllEvents)
    }
    /// Occurrence identity is independent of the root conversation target.
    pub fn accepts_occurrence(
        self,
        conversation: &AffordanceTarget,
        item: &AffordanceTarget,
    ) -> bool {
        if self == Self::SubscribeAllEvents {
            matches!(item, AffordanceTarget::Event(_))
        } else {
            conversation == item
        }
    }
}

/// A Property's value schema is not Event data. Event control/response roles
/// stay distinct; native SUBACK/UNSUBACK packets never become application data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotificationSchemas {
    PropertyValue(PreparedSchema),
    /// Complete named readable values; shared backing survives the source TD.
    ObserveAllProperties(Arc<PropertyCollectionSchema>),
    SubscribeAllEvents(Arc<crate::collection::EventCollectionSchema>),
    Event {
        subscription: Option<PreparedSchema>,
        data: Option<PreparedSchema>,
        data_response: Option<PreparedSchema>,
        cancellation: Option<PreparedSchema>,
    },
}
impl NotificationSchemas {
    /// Complete indirect backing. Runtime capture borrows this original owner;
    /// native compilation must not silently deep-clone its variable trees.
    pub fn retained_bytes(&self) -> usize {
        match self {
            Self::ObserveAllProperties(s) => {
                s.retained_bytes()
                    + core::mem::size_of_val(&**s)
                    + 2 * core::mem::size_of::<usize>()
            }
            Self::SubscribeAllEvents(s) => {
                s.retained_bytes()
                    + core::mem::size_of_val(&**s)
                    + 2 * core::mem::size_of::<usize>()
            }
            _ => [
                self.subscription(),
                self.data(),
                self.data_response(),
                self.cancellation(),
            ]
            .into_iter()
            .flatten()
            .map(PreparedSchema::retained_bytes)
            .sum(),
        }
    }
    pub fn kind(&self) -> crate::notification::NotificationKind {
        match self {
            Self::PropertyValue(_) => crate::notification::NotificationKind::PropertyObservation,
            Self::ObserveAllProperties(_) => {
                crate::notification::NotificationKind::ObserveAllProperties
            }
            Self::Event { .. } => crate::notification::NotificationKind::Event,
            Self::SubscribeAllEvents(_) => {
                crate::notification::NotificationKind::SubscribeAllEvents
            }
        }
    }
    /// Scalar roles only. Event collections retain roles per named member;
    /// None here does not assert that every collection member lacks that role.
    pub fn subscription(&self) -> Option<&PreparedSchema> {
        match self {
            Self::Event { subscription, .. } => subscription.as_ref(),
            _ => None,
        }
    }
    pub fn data(&self) -> Option<&PreparedSchema> {
        match self {
            Self::PropertyValue(s) => Some(s),
            Self::Event { data, .. } => data.as_ref(),
            Self::ObserveAllProperties(_) => None,
            Self::SubscribeAllEvents(_) => None,
        }
    }
    pub fn data_response(&self) -> Option<&PreparedSchema> {
        match self {
            Self::Event { data_response, .. } => data_response.as_ref(),
            _ => None,
        }
    }
    pub fn cancellation(&self) -> Option<&PreparedSchema> {
        match self {
            Self::Event { cancellation, .. } => cancellation.as_ref(),
            _ => None,
        }
    }
    /// Scalar support cannot qualify a named collection decoder.
    pub fn supports(&self, data: &dyn ConsumerDataService) -> bool {
        match self {
            Self::ObserveAllProperties(s) => data.supports_property_map(s),
            Self::SubscribeAllEvents(s) => s.members().iter().all(|m| {
                [&m.subscription, &m.data, &m.data_response, &m.cancellation]
                    .into_iter()
                    .flatten()
                    .all(|s| data.supports_schema(s))
            }),
            _ => [
                self.subscription(),
                self.data(),
                self.data_response(),
                self.cancellation(),
            ]
            .into_iter()
            .flatten()
            .all(|s| data.supports_schema(s)),
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
    pub kind: NotificationKind,
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
    /// Logical source of THIS occurrence, never a native topic or route token.
    pub target: AffordanceTarget,
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
    /// Bound complete data AND occurrence-target backing BEFORE handoff.
    /// `notification_item_backing` computes the recipient allowance. Pending registers wake
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
/// Original logical context for one policy check. `target` is the conversation
/// scope; `occurrence` is present for acceptance/delivery/response of an owned
/// item, including delayed checks. It is the mapping's logical affordance coordinate, not
/// an authenticated sender, topic, principal or device identity. A check without
/// an occurrence grants no permission to deliver a future item.
#[derive(Clone, Copy)]
pub struct NotificationAuthorization<'a> {
    pub recipient: &'a NotificationIdentity,
    pub target: &'a AffordanceTarget,
    pub kind: NotificationKind,
    pub occurrence: Option<&'a AffordanceTarget>,
    pub permission: NotificationPermission,
}
impl NotificationAuthorization<'_> {
    pub fn operation(&self) -> Operation {
        if self.permission == NotificationPermission::Cancel {
            self.kind.stop_operation()
        } else {
            self.kind.start_operation()
        }
    }
}
/// Actual captured application policy, rechecked at long-lived delivery and
/// response/cancel. Native trust/credential services remain Binding-owned.
pub trait NotificationAuthorizer {
    fn work(&self) -> u64 {
        1
    }
    fn authorize(&self, request: NotificationAuthorization<'_>) -> CoreResult<()>;
}

/// Optional captured validation for the raw lifecycle. Data work is prepaid
/// before native handoff; validation cannot strand an unowned late candidate.
/// Implementations are trusted, pure and finite, like ConsumerDataService.
pub trait NotificationValidation {
    fn subscription(
        &self,
        input: &SubscriptionInput,
        budget: &mut clinkz_wot_foundation::WorkBudget,
    ) -> CoreResult<()>;
    fn cancellation(
        &self,
        input: &CancellationInput,
        budget: &mut clinkz_wot_foundation::WorkBudget,
    ) -> CoreResult<()>;
    fn response(
        &self,
        target: &AffordanceTarget,
        input: &EventDataResponse,
        budget: &mut clinkz_wot_foundation::WorkBudget,
    ) -> CoreResult<()>;
    fn prepay_data(&self, budget: &mut clinkz_wot_foundation::WorkBudget) -> CoreResult<()>;
    fn data(&self, target: &AffordanceTarget, payload: Option<&Payload>) -> CoreResult<()>;
}

/// Complete recipient allowance includes attribution backing as well as data.
pub fn notification_item_backing(
    target: &AffordanceTarget,
    payload: Option<&Payload>,
) -> Option<usize> {
    let name = target.name().map_or(Some(0), |n| {
        let layout = core::alloc::Layout::new::<[usize; 2]>()
            .extend(core::alloc::Layout::array::<u8>(n.len()).ok()?)
            .ok()?
            .0
            .pad_to_align();
        Some(layout.size())
    })?;
    notification_payload_backing(payload)?.checked_add(name)
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
