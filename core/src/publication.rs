//! Producer notification publication is independent of finite exchanges and Consumer
//! subscription conversations. Native distribution never enumerates Engine
//! subscribers. These are manually driven Host contracts.
use crate::{BindingGeneration, BindingId, BindingInputRejection, CoreError, CoreResult, Payload};
use alloc::rc::Rc;
use clinkz_wot_foundation::WorkBudget;
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
    EventData(Payload),
}
impl PublicationData {
    pub fn kind(&self) -> crate::notification::NotificationKind {
        match self {
            Self::PropertyValue(_) => crate::notification::NotificationKind::PropertyObservation,
            Self::EventData(_) => crate::notification::NotificationKind::Event,
        }
    }
    pub fn payload(&self) -> &Payload {
        match self {
            Self::PropertyValue(p) | Self::EventData(p) => p,
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
    fn notification_route(&self, target: &crate::AffordanceTarget) -> Option<&NotificationRoute>;
    /// Before acceptance, refusal returns the original complete data carrier.
    /// After acceptance the publication owns the native slot through disposal.
    fn start_publication(
        &mut self,
        route: &NotificationRoute,
        data: PublicationData,
    ) -> Result<Self::Publication, BindingInputRejection<PublicationData>>;
}
