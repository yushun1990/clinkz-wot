//! Producer notification publication is independent of finite exchanges and Consumer
//! subscription conversations. Native distribution never enumerates Engine
//! subscribers. These are manually driven Host contracts.
use crate::{
    BindingGeneration, BindingId, BindingInputRejection, CoreError, CoreResult, Operation, Payload,
    action::{ActionRoute, ActionValidator},
    producer::ProducerInstanceKey,
};
use alloc::rc::{Rc, Weak};
use clinkz_wot_foundation::WorkBudget;
use core::cell::Cell;
use core::task::{Context, Poll};

/// Actual native mapping identity. Copying its reference grants no logical
/// authority; publication captures the original mapping and services together.
#[derive(Clone, Debug)]
pub struct NotificationRoute {
    binding: (BindingId, BindingGeneration),
    identity: Rc<()>,
}
impl PartialEq for NotificationRoute {
    fn eq(&self, other: &Self) -> bool {
        self.same_mapping(other)
    }
}
impl Eq for NotificationRoute {}
impl NotificationRoute {
    pub fn new(binding: (BindingId, BindingGeneration)) -> Self {
        Self {
            binding,
            identity: Rc::new(()),
        }
    }
    pub fn binding(&self) -> (BindingId, BindingGeneration) {
        self.binding
    }
    pub fn same_mapping(&self, other: &Self) -> bool {
        self.binding == other.binding && Rc::ptr_eq(&self.identity, &other.identity)
    }
}

/// Property values and Event data remain distinct through admission and native
/// publication. Neither variant carries subscription/cancellation or dataResponse.
#[derive(Debug)]
pub enum PublicationData {
    PropertyValue(Payload),
    /// One named Property collection sample under Thing-root authority.
    PropertyValues(Payload),
    EventData(Payload),
    /// One Event occurrence under a root collection publication authority.
    EventOccurrence {
        event: alloc::sync::Arc<str>,
        data: Payload,
    },
}
impl PublicationData {
    pub fn kind(&self) -> crate::notification::NotificationKind {
        match self {
            Self::PropertyValue(_) => crate::notification::NotificationKind::PropertyObservation,
            Self::PropertyValues(_) => crate::notification::NotificationKind::ObserveAllProperties,
            Self::EventData(_) => crate::notification::NotificationKind::Event,
            Self::EventOccurrence { .. } => {
                crate::notification::NotificationKind::SubscribeAllEvents
            }
        }
    }
    pub fn payload(&self) -> &Payload {
        match self {
            Self::PropertyValue(p) | Self::PropertyValues(p) | Self::EventData(p) => p,
            Self::EventOccurrence { data, .. } => data,
        }
    }
    pub fn backing_bytes(&self) -> Option<usize> {
        match self {
            Self::EventOccurrence { event, data } => {
                crate::notification::notification_item_backing(
                    &crate::AffordanceTarget::Event(event.clone()),
                    Some(data),
                )
            }
            _ => crate::notification::notification_payload_backing(Some(self.payload())),
        }
    }
}

/// Actual prepared data and security services. The Engine invokes both before
/// admission. A native implementation cannot substitute services per emission.
pub trait ProducerNotificationServices {
    fn authorize(&self, data: &PublicationData, budget: &mut WorkBudget) -> CoreResult<()>;
    fn validate(&self, data: &PublicationData, budget: &mut WorkBudget) -> CoreResult<()>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationEffect {
    NotPublished,
    /// A native publication may have escaped; cancellation is not rollback.
    Uncertain,
    /// The mapping's local publication completed. No subscriber delivery claim.
    LocallyPublished,
}

#[derive(Clone, Debug)]
pub struct NativePublicationOutcome {
    pub effect: PublicationEffect,
    pub cause: Option<CoreError>,
}

/// One admitted native publication, including late work and final disposal.
/// Pending/error retains its backing and original capacity. Drop must perform
/// bounded local disposal, without claiming successful delivery or drain.
pub trait NativeNotificationPublication {
    fn poll_publish(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
        cancel: bool,
    ) -> Poll<NativePublicationOutcome>;
    /// Pending/error retains the complete owner. Only Ready(Ok) proves local
    /// disposal; an error must not authorize settlement acknowledgement.
    fn poll_retire(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<()>>;
}

pub trait ProducerNotificationBinding {
    type Publication: NativeNotificationPublication;
    fn producer_instance(&self) -> crate::producer::ProducerInstanceKey;
    /// Native readiness, health and lease retirement only. Servient consumes
    /// failure evidence separately; this callback has no logical authority.
    fn poll_participation(
        &mut self,
        _cx: &mut Context<'_>,
        _budget: &mut WorkBudget,
    ) -> Poll<CoreResult<()>> {
        Poll::Pending
    }
    fn notification_route(
        &self,
        target: &crate::AffordanceTarget,
        kind: crate::notification::NotificationKind,
    ) -> Option<&NotificationRoute>;
    /// Before acceptance, refusal returns the original complete data carrier.
    /// After acceptance the publication owns the native slot through disposal.
    fn start_publication(
        &mut self,
        route: &NotificationRoute,
        data: PublicationData,
    ) -> Result<Self::Publication, BindingInputRejection<PublicationData>>;
}

/// Passive origin check for a native participant; cannot dispatch or publish.
#[derive(Clone)]
pub struct ProducerOwnerToken(Weak<()>);
impl ProducerOwnerToken {
    pub fn new(issuer: &Rc<()>) -> Self {
        Self(Rc::downgrade(issuer))
    }
    pub fn same_owner(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
    pub fn matches_issuer(&self, issuer: &alloc::rc::Weak<()>) -> bool {
        self.0.ptr_eq(issuer)
    }
    pub fn is_alive(&self) -> bool {
        self.0.strong_count() != 0
    }
}

/// Move-only identity of one native participation owner. It is neither a
/// logical permission nor infrastructure shutdown authority. Implementations
/// retain it until that participation's complete local disposal.
pub struct ProducerParticipationId(Rc<()>);
impl ProducerParticipationId {
    pub fn new() -> Self {
        Self(Rc::new(()))
    }
    pub fn same_owner(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub fn downgrade(&self) -> Weak<()> {
        Rc::downgrade(&self.0)
    }
    pub fn matches(&self, other: &Weak<()>) -> bool {
        self.downgrade().ptr_eq(other)
    }
}
impl Default for ProducerParticipationId {
    fn default() -> Self {
        Self::new()
    }
}

/// Closed native contribution to one logical publication. Readiness and
/// membership are passive; the separate Servient commit opens its ingress gate.
/// A lease may use shared physical infrastructure without sharing logical authority.
pub trait ClosedProducerParticipation {
    /// Native publication slots prepaid by managed admission. Servient adds
    /// its own logical publication metadata; native backing is declared below.
    /// A qualified notification contributor must declare at least one slot;
    /// wrappers preserve the original declaration.
    fn notification_publication_slots(&self) -> usize {
        0
    }
    /// Passive issuer/key association, never proof of committed participation
    /// membership or permission to dispatch or revoke.
    fn associated_with(&self, _owner: &ProducerOwnerToken, _key: ProducerInstanceKey) -> bool {
        false
    }
    /// Native failure evidence. Servient decides logical revocation.
    fn native_failure(&self) -> Option<&CoreError> {
        None
    }
    /// Immutable for this owner's entire lifetime, including retirement.
    fn participation_id(&self) -> &ProducerParticipationId;
    /// Retain the actual transferred capacity owner until native work, captured
    /// backing and unread results are disposed. This cannot dispatch or do I/O.
    fn retain_owner(&mut self, owner: Rc<dyn core::any::Any>);
    /// Managed ownership handoff: the complete additional controlled backing
    /// plus promised input/result capacity, only before this participation has
    /// acquired unreserved native work. SDK/shared infrastructure stays native.
    /// None means this participant has no managed Host admission contract.
    fn admission_backing_bytes(
        &self,
        _owner: &ProducerOwnerToken,
        _key: ProducerInstanceKey,
    ) -> Option<usize> {
        None
    }
    /// Bounded local retirement intent, without polling I/O or calling an
    /// application. Progress remains specific to the native participant.
    fn begin_retire(&mut self);
    /// Passive readiness for this exact logical instance and Servient owner.
    /// Readiness alone does not establish qualified Form/operation coverage.
    fn ready_for(&self, owner: &ProducerOwnerToken, key: ProducerInstanceKey) -> bool;
    /// Exact native mapping/definition support. One participant may cover many
    /// Forms and Things, but each selected operation has exactly one owner at
    /// publication. This is not an Engine protocol matcher.
    fn covers(
        &self,
        _key: ProducerInstanceKey,
        _binding: (BindingId, BindingGeneration),
        _site: crate::thing::AffordanceSite<'_>,
        _form: u32,
        _operation: Operation,
        _definition: &dyn core::any::Any,
    ) -> bool {
        false
    }
    /// Exact number of prepared operation associations retained by this
    /// participation, when its managed profile supplies a complete inventory.
    /// This is not physical connection, route or subscription cardinality.
    /// Shared prepared-backing deductions require this complete inventory;
    /// wrappers must propagate it. None makes no such inventory claim.
    fn qualified_operation_count(&self) -> Option<usize> {
        None
    }
    /// Supply the actual source-free validation service for an invoke Form
    /// which this participant covers. Missing service refuses publication,
    /// including when the Action has no explicit input/output DataSchema.
    /// Mapping and service are captured before the local commit, not borrowed
    /// from an arbitrary RawActionRequest at ingress.
    fn action_validation(
        &self,
        _key: ProducerInstanceKey,
        _binding: (BindingId, BindingGeneration),
        _site: crate::thing::AffordanceSite<'_>,
        _form: u32,
        _definition: &dyn core::any::Any,
    ) -> Option<(ActionRoute, Rc<dyn ActionValidator>)> {
        None
    }
    /// Supply the actual mapping for a covered query/cancel Form. This route
    /// is captured with the invoke service before publication; native requests
    /// cannot manufacture a control route by supplying their own key.
    fn action_control_route(
        &self,
        _key: ProducerInstanceKey,
        _binding: (BindingId, BindingGeneration),
        _site: crate::thing::AffordanceSite<'_>,
        _form: u32,
        _operation: Operation,
        _definition: &dyn core::any::Any,
    ) -> Option<ActionRoute> {
        None
    }
    /// Actual selected publication services and mapping for a covered Property observe
    /// or Event subscribe Form. No native subscription or Engine subscriber is created.
    fn notification_publication(
        &self,
        _key: ProducerInstanceKey,
        _binding: (BindingId, BindingGeneration),
        _site: crate::thing::AffordanceSite<'_>,
        _form: u32,
        _definition: &dyn core::any::Any,
    ) -> Option<(
        crate::publication::NotificationRoute,
        Rc<dyn crate::publication::ProducerNotificationServices>,
        usize,
    )> {
        None
    }
    /// The native owner must consult this gate before offering semantic ingress.
    /// It must be closed until publication and must not be shared between
    /// independently published participations. Returning a gate is a passive
    /// loan; all callbacks, including this one, run before the commit.
    fn ingress_gate(&self) -> &Rc<Cell<bool>>;
    /// True only after this participation's native obligations and unread
    /// settlements have retired. The physical infrastructure may remain shared.
    fn retired(&self) -> bool;
}
