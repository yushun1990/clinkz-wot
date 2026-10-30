//! Producer Event emission is independent of finite exchanges and Consumer
//! subscription conversations. Native distribution never enumerates Engine
//! subscribers. These are manually driven Host contracts.
use crate::{BindingGeneration, BindingId, BindingInputRejection, CoreError, CoreResult, Payload};
use alloc::rc::Rc;
use clinkz_wot_foundation::WorkBudget;
use core::task::{Context, Poll};

/// Actual native mapping identity. Copying its reference grants no logical
/// authority; publication captures the original mapping and services together.
#[derive(Clone, Debug)]
pub struct EventRoute {
    binding: (BindingId, BindingGeneration),
    identity: Rc<()>,
}
impl PartialEq for EventRoute {
    fn eq(&self, other: &Self) -> bool {
        self.same_mapping(other)
    }
}
impl Eq for EventRoute {}
impl EventRoute {
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

/// Event data only, never subscription/cancellation input or dataResponse.
#[derive(Debug)]
pub struct EventData(pub Payload);

/// Actual prepared data and security services. The Engine invokes both before
/// admission. A native implementation cannot substitute services per emission.
pub trait ProducerEventServices {
    fn authorize(&self, data: &EventData, budget: &mut WorkBudget) -> CoreResult<()>;
    fn validate(&self, data: &EventData, budget: &mut WorkBudget) -> CoreResult<()>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventPublishEffect {
    NotPublished,
    /// A native publication may have escaped; cancellation is not rollback.
    Uncertain,
    /// The mapping's local publication completed. No subscriber delivery claim.
    LocallyPublished,
}

#[derive(Clone, Debug)]
pub struct NativeEventOutcome {
    pub effect: EventPublishEffect,
    pub cause: Option<CoreError>,
}

/// One admitted native publication, including late work and final disposal.
/// Pending/error retains its backing and original capacity. Drop must perform
/// bounded local disposal, without claiming successful delivery or drain.
pub trait NativeEventPublication {
    fn poll_publish(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
        cancel: bool,
    ) -> Poll<NativeEventOutcome>;
    /// Pending/error retains the complete owner. Only Ready(Ok) proves local
    /// disposal; an error must not authorize settlement acknowledgement.
    fn poll_retire(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<()>>;
}

pub trait ProducerEventBinding {
    type Publication: NativeEventPublication;
    fn event_route(&self, event: &str) -> Option<&EventRoute>;
    /// Before acceptance, refusal returns the original complete data carrier.
    /// After acceptance the publication owns the native slot through disposal.
    fn start_event(
        &mut self,
        route: &EventRoute,
        data: EventData,
    ) -> Result<Self::Publication, BindingInputRejection<EventData>>;
}
