//! Logical Producer Event authority and settlement. Native matching, fan-out
//! and physical resources belong exclusively to the selected Binding.
use super::*;
use clinkz_wot_core::{event::*, notification::notification_payload_backing};

/// A registered application Event source, not a request handler.
pub struct ProducerEventSource {
    name: Arc<str>,
}
impl ProducerEventSource {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self { name: name.into() }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub(crate) fn backing_bytes(&self) -> usize {
        self.name.len() + 2 * size_of::<usize>()
    }
}

pub struct CapturedEventRoute {
    pub event: Arc<str>,
    pub route: EventRoute,
    pub services: Rc<dyn ProducerEventServices>,
    /// Complete retained application data carrier allowance, prepaid by the
    /// native participant's managed admission contract (one slot here).
    pub data_bytes: usize,
}
impl CapturedEventRoute {
    pub fn backing_bytes(routes: &[Self], capacity: usize) -> Option<usize> {
        routes
            .iter()
            .try_fold(capacity.checked_mul(size_of::<Self>())?, |n, r| {
                n.checked_add(r.event.len() + 2 * size_of::<usize>())
            })
    }
}

#[derive(Clone, Debug)]
pub struct ProducerEventSettlement {
    pub instance: ProducerInstanceKey,
    pub event: Arc<str>,
    pub route: EventRoute,
    pub cause: Option<CoreError>,
    pub cleanup_error: Option<CoreError>,
    pub native: NativeEventOutcome,
}

/// One application emission. Its original logical instance, native work,
/// unread settlement and funding survive replacement or loss of the exposure.
pub struct ProducerEventEmission<P: NativeEventPublication> {
    native: Option<P>,
    current: Rc<Cell<bool>>,
    issuer: Weak<()>,
    route: EventRoute,
    event: Arc<str>,
    deadline: Deadline,
    first_cause: Option<CoreError>,
    cleanup_error: Option<CoreError>,
    native_outcome: Option<NativeEventOutcome>,
    settlement: Option<ProducerEventSettlement>,
    instance: Rc<RawProducerInstance>,
}
impl<P: NativeEventPublication> ProducerEventEmission<P> {
    pub fn cancel(&mut self) {
        if self.settlement.is_none() && self.first_cause.is_none() {
            self.first_cause = Some(CoreError::Cancelled(self.progress_context()));
        }
    }
    pub fn step(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
        now: MonotonicInstant,
    ) -> Poll<()> {
        if self.settlement.is_some() {
            return Poll::Ready(());
        }
        self.observe_authority(now);
        if self.native_outcome.is_none() {
            let progress = self
                .native
                .as_mut()
                .expect("unacknowledged publication")
                .poll_publish(cx, budget, self.first_cause.is_some());
            let Poll::Ready(outcome) = progress else {
                self.observe_authority(now);
                return Poll::Pending;
            };
            if self.first_cause.is_none() {
                self.first_cause = outcome.cause.clone();
            }
            if outcome.cause.is_some() {
                self.current.set(false);
            }
            self.native_outcome = Some(outcome);
        }
        self.observe_authority(now);
        let retirement = self
            .native
            .as_mut()
            .expect("unacknowledged publication")
            .poll_retire(cx, budget);
        if let Poll::Ready(Err(error)) = retirement {
            if self.first_cause.is_none() {
                self.first_cause = Some(error.clone());
            }
            if self.cleanup_error.is_none() {
                self.cleanup_error = Some(error);
            }
            self.current.set(false);
            return Poll::Pending;
        }
        self.observe_authority(now);
        if retirement.is_pending() {
            return Poll::Pending;
        }
        self.settlement = Some(ProducerEventSettlement {
            instance: self.instance.key(),
            event: Arc::clone(&self.event),
            route: self.route.clone(),
            cause: self.first_cause.clone(),
            cleanup_error: self.cleanup_error.clone(),
            native: self.native_outcome.take().expect("terminal native outcome"),
        });
        Poll::Ready(())
    }
    fn observe_authority(&mut self, now: MonotonicInstant) {
        if self.first_cause.is_none() {
            self.first_cause = if !self.current.get() || self.issuer.strong_count() == 0 {
                Some(CoreError::StaleHandle(self.progress_context()))
            } else {
                match self.deadline.checked_is_elapsed_at(now) {
                    Some(false) => None,
                    Some(true) => Some(CoreError::TimedOut(self.progress_context())),
                    None => Some(CoreError::Validation(self.progress_context())),
                }
            };
        }
    }
    fn progress_context(&self) -> ErrorContext {
        let (binding, generation) = self.route.binding();
        ErrorContext::new(ErrorPhase::Delivery, RetryClass::Never)
            .with_thing(self.instance.key().slot())
            .with_binding(binding, generation)
    }
    pub fn settlement(&self) -> Option<&ProducerEventSettlement> {
        self.settlement.as_ref()
    }
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.first_cause.as_ref()
    }
    pub fn cleanup_error(&self) -> Option<&CoreError> {
        self.cleanup_error.as_ref()
    }
    /// Terminal acknowledgement releases the original native slot and funding.
    pub fn acknowledge(mut self) -> Result<ProducerEventSettlement, Self> {
        match self.settlement.take() {
            Some(s) => Ok(s),
            None => Err(self),
        }
    }
}
impl<P: NativeEventPublication> Drop for ProducerEventEmission<P> {
    fn drop(&mut self) {
        // Dispose native work before releasing logical work/funding retention.
        self.native = None;
        self.instance
            .event_work
            .set(self.instance.event_work.get() - 1);
    }
}

impl RawProducerServient {
    /// Admit data against a qualified published Event source and the actual
    /// captured native mapping/services. No subscriber count or delivery ACK
    /// is implied. Refusal has no native publication effect and returns data.
    #[allow(clippy::too_many_arguments)]
    pub fn emit_event<B: ProducerEventBinding>(
        &mut self,
        binding: &mut B,
        key: ProducerInstanceKey,
        event: &str,
        data: EventData,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Result<ProducerEventEmission<B::Publication>, BindingInputRejection<EventData>> {
        let admit = (|| -> CoreResult<_> {
            budget
                .consume(
                    WorkClass::HandlerSteps,
                    (self.limits.instances
                        + (self.limits.behaviors_per_instance + MAX_PRODUCER_PARTICIPANTS)
                            * (self.limits.target_name_bytes + 1)) as u64,
                )
                .map_err(|_| CoreError::Backpressure(context()))?;
            if event.len() > self.limits.target_name_bytes {
                return Err(CoreError::Backpressure(context()));
            }
            match deadline.checked_is_elapsed_at(now) {
                Some(false) => {}
                Some(true) => return Err(CoreError::TimedOut(context())),
                None => return Err(CoreError::Validation(context())),
            }
            let entry = self
                .instances
                .iter()
                .find(|e| e.current.get() && e.instance.key() == key)
                .ok_or_else(|| CoreError::StaleHandle(context()))?;
            let route = binding
                .event_route(event)
                .ok_or_else(|| CoreError::UnsupportedOperation(context()))?;
            let captured = entry
                .event_routes
                .as_ref()
                .and_then(|rs| {
                    rs.iter()
                        .find(|r| r.event.as_ref() == event && r.route.same_mapping(route))
                })
                .ok_or_else(|| CoreError::UnsupportedOperation(context()))?;
            let bytes = notification_payload_backing(Some(&data.0))
                .ok_or_else(|| CoreError::Backpressure(context()))?;
            if bytes > captured.data_bytes || entry.instance.event_work.get() == usize::MAX {
                return Err(CoreError::Backpressure(context()));
            }
            captured.services.authorize(&data, budget)?;
            captured.services.validate(&data, budget)?;
            // Trusted services may revoke a publication through its guard.
            if !entry.current.get() {
                return Err(CoreError::StaleHandle(context()));
            }
            Ok((
                Rc::clone(&entry.instance),
                Rc::clone(&entry.current),
                captured.route.clone(),
                Arc::clone(&captured.event),
            ))
        })();
        let (instance, current, route, event) = match admit {
            Ok(value) => value,
            Err(error) => {
                return Err(BindingInputRejection::new(
                    data,
                    BindingOperationalError::new(error),
                ));
            }
        };
        let native = binding.start_event(&route, data)?;
        instance.event_work.set(instance.event_work.get() + 1);
        Ok(ProducerEventEmission {
            native: Some(native),
            current,
            issuer: Rc::downgrade(&self.issuer),
            route,
            event,
            deadline,
            first_cause: None,
            cleanup_error: None,
            native_outcome: None,
            settlement: None,
            instance,
        })
    }
}
