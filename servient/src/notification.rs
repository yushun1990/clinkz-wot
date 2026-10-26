//! Caller-driven raw Host Consumer notifications. One root represents one
//! logical Consumer incarnation and one captured native capability. Roots may
//! share Binding-owned infrastructure, but never application cursors/authority.
use alloc::{collections::VecDeque, rc::Rc, vec::Vec};
use clinkz_wot_core::{notification::*, *};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use core::{
    cell::Cell,
    mem::size_of,
    task::{Context, Poll},
};

/// Owner-scoped Engine reservations, not a transitive SDK/RAM profile.
/// Each admitted slot prepays its queue, one extra native handoff (even when
/// full), one reciprocal response, both controls and fixed terminal/cleanup
/// storage. Native participation/reply indirect backing has its Binding owner.
#[derive(Clone, Copy, Debug)]
pub struct RawNotificationLimits {
    pub subscriptions: usize,
    pub queue_items: usize,
    pub queue_payload_bytes: usize,
    pub payload_backing_bytes: usize,
    pub control_backing_bytes: usize,
    pub target_name_bytes: usize,
    pub engine_bytes: usize,
}

/// A unique receive cursor. It cannot be cloned. Drop fences delivery immediately
/// and leaves the complete owner in its prepaid root slot for manual progress.
///
/// ```compile_fail,E0599
/// fn duplicate(cursor: clinkz_wot_servient::notification::NotificationCursor) {
///     let _other_receiver = cursor.clone();
/// }
/// ```
pub struct NotificationCursor {
    identity: NotificationIdentity,
    stopped: Rc<Cell<bool>>,
}
impl NotificationCursor {
    pub fn identity(&self) -> &NotificationIdentity {
        &self.identity
    }
    pub fn stop_handle(&self) -> NotificationStop {
        NotificationStop(self.stopped.clone())
    }
}
impl Drop for NotificationCursor {
    fn drop(&mut self) {
        self.stopped.set(true);
    }
}
/// Cloneable cancellation intent only; it confers no receive/response or native
/// infrastructure-shutdown authority. No user/native callback runs here.
#[derive(Clone)]
pub struct NotificationStop(Rc<Cell<bool>>);
impl NotificationStop {
    pub fn request_stop(&self) {
        self.0.set(true);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationState {
    Establishing,
    Active,
    Closing,
    Closed,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventResponseState {
    Unavailable,
    Awaiting,
    Pending,
    Delivered,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NotificationLoss {
    pub items: u64,
    pub saturated: bool,
}
impl NotificationLoss {
    fn add(&mut self, n: u64) {
        match self.items.checked_add(n) {
            Some(total) => self.items = total,
            None => {
                self.items = u64::MAX;
                self.saturated = true;
            }
        }
    }
}
/// Borrowed raw data, retained by the original root until advance or cleanup.
/// Sequence counts logical admitted items, not a wire order/replay guarantee.
pub struct RawNotificationView<'a> {
    pub sequence: u64,
    pub data: Option<&'a Payload>,
    pub response: EventResponseState,
}
pub struct NotificationStatus<'a> {
    pub state: NotificationState,
    pub loss: NotificationLoss,
    pub queued_items: usize,
    pub queued_payload_bytes: usize,
    pub has_retained_handoff: bool,
    pub native_started: bool,
    pub first_cause: Option<&'a CoreError>,
    pub cleanup_error: Option<&'a CoreError>,
    pub cancellation_error: Option<&'a CoreError>,
}
/// Available only after all local participants, reply jobs and retained data
/// are disposed. A remote Unknown/Unsupported is not a local cleanup failure.
pub struct NotificationSettlement {
    pub identity: NotificationIdentity,
    pub target: AffordanceTarget,
    pub binding: (BindingId, BindingGeneration),
    pub first_cause: Option<CoreError>,
    pub cleanup_error: Option<CoreError>,
    pub cancellation_error: Option<CoreError>,
    pub native_started: bool,
    pub remote_cancellation: RemoteCancellation,
    pub loss: NotificationLoss,
    pub delivered_items: u64,
    pub responses_delivered: u64,
    pub responses_uncertain: u64,
}

struct Buffered<R> {
    native: NativeNotification<R>,
    bytes: usize,
    sequence: u64,
    exposed: bool,
    response_input: Option<EventDataResponse>,
    response_state: EventResponseState,
}
impl<R: NotificationResponse> Buffered<R> {
    fn abort(mut self) {
        if let Some(response) = self.native.response.take() {
            response.abort_local();
        }
    }
}
struct Entry<P: NotificationParticipation> {
    identity: NotificationIdentity,
    request: RawNotificationRequest,
    stopped: Rc<Cell<bool>>,
    state: NotificationState,
    support: NotificationSupport,
    participant: Option<P>,
    native_started: bool,
    close_started: bool,
    cancel_admitted: bool,
    queue: VecDeque<Buffered<P::Response>>,
    staged: Option<Buffered<P::Response>>,
    queue_bytes: usize,
    reservation: usize,
    cancellation: Option<CancellationInput>,
    first_cause: Option<CoreError>,
    cleanup_error: Option<CoreError>,
    cancellation_error: Option<CoreError>,
    remote: RemoteCancellation,
    loss: NotificationLoss,
    next_sequence: u64,
    delivered: u64,
    responses_delivered: u64,
    responses_uncertain: u64,
}
impl<P: NotificationParticipation> Entry<P> {
    fn fence(&mut self, cause: CoreError) {
        if self.state == NotificationState::Closed {
            return;
        }
        // Stop intent linearizes at the shared flag, even if a later revoke or
        // callback failure is the next operation that observes this entry.
        let cause = if self.stopped.get() {
            cancelled()
        } else {
            cause
        };
        self.first_cause.get_or_insert(cause);
        self.stopped.set(true);
        self.state = NotificationState::Closing;
    }
    fn sync_stop(&mut self) {
        if self.stopped.get() {
            self.fence(cancelled());
        }
    }
    fn check_live(&mut self, paired: bool) -> CoreResult<()> {
        self.sync_stop();
        if !paired {
            self.fence(stale());
        }
        if matches!(
            self.state,
            NotificationState::Establishing | NotificationState::Active
        ) {
            Ok(())
        } else {
            Err(self.first_cause.clone().unwrap_or_else(cancelled))
        }
    }
    fn cleanup_error(&mut self, error: CoreError) {
        self.cleanup_error.get_or_insert(error);
    }
}
impl<P: NotificationParticipation> Drop for Entry<P> {
    fn drop(&mut self) {
        // Explicit qualified finite local abort, never a successful drain claim.
        if let Some(item) = self.staged.take() {
            item.abort();
        }
        for item in self.queue.drain(..) {
            item.abort();
        }
        if let Some(participant) = self.participant.take() {
            participant.abort_local();
        }
    }
}
struct Slot<P: NotificationParticipation> {
    generation: Option<Generation>,
    entry: Option<Entry<P>>,
}

/// Exclusive manual access linearizes progress, delivery, stop and settlement.
/// Policy and native callbacks execute without locks. Linked callbacks and all
/// local destructors must be finite/cooperative; this is no preemption claim.
pub struct RawNotificationServient<B: ConsumerNotificationBinding, A: NotificationAuthorizer> {
    binding: B,
    configuration: B::Configuration,
    binding_identity: (BindingId, BindingGeneration),
    authorizer: A,
    consumer: ThingSlotId,
    issuer: NotificationIssuer,
    slots: Vec<Slot<B::Participation>>,
    limits: RawNotificationLimits,
    reserved_bytes: usize,
    current: bool,
    cleanup_cursor: usize,
    retained: usize,
}
fn context(phase: ErrorPhase) -> ErrorContext {
    ErrorContext::new(phase, RetryClass::Never)
}
fn capacity() -> CoreError {
    CoreError::Backpressure(context(ErrorPhase::Admission))
}
fn stale() -> CoreError {
    CoreError::StaleHandle(context(ErrorPhase::Delivery))
}
fn cancelled() -> CoreError {
    CoreError::Cancelled(context(ErrorPhase::Cleanup))
}
fn invalid() -> CoreError {
    CoreError::Validation(context(ErrorPhase::Validate))
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(context(ErrorPhase::Selection))
}
fn reject<T>(input: T, error: CoreError) -> BindingInputRejection<T> {
    BindingInputRejection::new(input, BindingOperationalError::new(error))
}
fn debit(budget: &mut WorkBudget, class: WorkClass, n: usize) -> CoreResult<()> {
    budget.consume(class, n as u64).map_err(|_| capacity())
}
fn binding_paired<B: ConsumerNotificationBinding>(
    binding: &B,
    configuration: &B::Configuration,
    identity: (BindingId, BindingGeneration),
) -> bool {
    binding.identity() == identity && binding.configuration() == configuration
}

impl<B: ConsumerNotificationBinding, A: NotificationAuthorizer> RawNotificationServient<B, A> {
    #[allow(clippy::type_complexity)]
    pub fn try_new(
        binding: B,
        configuration: B::Configuration,
        authorizer: A,
        consumer: ThingSlotId,
        limits: RawNotificationLimits,
    ) -> Result<Self, BindingInputRejection<(B, B::Configuration, A)>> {
        let needed = limits
            .subscriptions
            .checked_mul(size_of::<Slot<B::Participation>>())
            .and_then(|n| n.checked_add(size_of::<Self>() + 2 * size_of::<usize>()));
        if binding.configuration() != &configuration {
            return Err(reject((binding, configuration, authorizer), invalid()));
        }
        if limits.subscriptions > u32::MAX as usize
            || needed.is_none_or(|n| n > limits.engine_bytes)
        {
            return Err(reject((binding, configuration, authorizer), capacity()));
        }
        let mut slots = Vec::new();
        if slots.try_reserve_exact(limits.subscriptions).is_err() {
            return Err(reject((binding, configuration, authorizer), capacity()));
        }
        let reserved_bytes = slots
            .capacity()
            .checked_mul(size_of::<Slot<B::Participation>>())
            .and_then(|n| n.checked_add(size_of::<Self>() + 2 * size_of::<usize>()));
        let Some(reserved_bytes) = reserved_bytes.filter(|n| *n <= limits.engine_bytes) else {
            return Err(reject((binding, configuration, authorizer), capacity()));
        };
        slots.resize_with(limits.subscriptions, || Slot {
            generation: Some(Generation::INITIAL),
            entry: None,
        });
        let binding_identity = binding.identity();
        Ok(Self {
            binding,
            configuration,
            binding_identity,
            authorizer,
            consumer,
            issuer: NotificationIssuer::default(),
            slots,
            limits,
            reserved_bytes,
            current: true,
            cleanup_cursor: 0,
            retained: 0,
        })
    }
    pub fn reserved_engine_bytes(&self) -> usize {
        self.reserved_bytes
    }
    pub fn retained_subscriptions(&self) -> usize {
        self.retained
    }
    fn paired(&self) -> bool {
        self.current && binding_paired(&self.binding, &self.configuration, self.binding_identity)
    }
    fn index(&self, identity: &NotificationIdentity) -> CoreResult<usize> {
        let i = identity.subscription.slot().get() as usize;
        self.slots
            .get(i)
            .and_then(|s| s.entry.as_ref())
            .filter(|e| e.identity == *identity)
            .map(|_| i)
            .ok_or_else(stale)
    }
    fn entry(&self, id: &NotificationIdentity) -> CoreResult<&Entry<B::Participation>> {
        Ok(self.slots[self.index(id)?].entry.as_ref().unwrap())
    }
    fn reservation(&self, queue_capacity: usize, target_bytes: usize) -> Option<usize> {
        use core::alloc::Layout;
        let target = Layout::new::<[usize; 2]>()
            .extend(Layout::array::<u8>(target_bytes).ok()?)
            .ok()?
            .0
            .pad_to_align()
            .size();
        // Entry/staging/terminal inline storage is already in the root slab.
        queue_capacity
            .checked_mul(size_of::<
                Buffered<<B::Participation as NotificationParticipation>::Response>,
            >())?
            .checked_add(self.limits.queue_payload_bytes)?
            .checked_add(self.limits.payload_backing_bytes.checked_mul(2)?)?
            .checked_add(self.limits.control_backing_bytes.checked_mul(2)?)?
            .checked_add(target)?
            .checked_add(3 * size_of::<usize>()) // Rc<Cell<bool>>, including alignment
    }

    /// Complete pre-effect rejection returns the exact request and native program.
    /// Missing response support refuses here, before native establishment.
    #[allow(clippy::type_complexity)]
    pub fn subscribe(
        &mut self,
        request: RawNotificationRequest,
        program: B::Program,
        budget: &mut WorkBudget,
    ) -> Result<NotificationCursor, BindingInputRejection<(RawNotificationRequest, B::Program)>>
    {
        let check = (|| {
            if !self.paired() {
                return Err(stale());
            }
            let kind = NotificationKind::for_target(&request.target).ok_or_else(unsupported)?;
            if request
                .target
                .name()
                .is_none_or(|s| s.len() > self.limits.target_name_bytes)
                || notification_payload_backing(request.subscription.0.as_ref())
                    .is_none_or(|n| n > self.limits.control_backing_bytes)
            {
                return Err(capacity());
            }
            if kind == NotificationKind::PropertyObservation
                && (request.responses != ResponseRequirement::None
                    || request.subscription.0.is_some())
            {
                return Err(invalid());
            }
            if self.limits.queue_items == 0 {
                return Err(capacity());
            }
            if kind == NotificationKind::PropertyObservation
                && (self.limits.queue_payload_bytes == 0 || self.limits.payload_backing_bytes == 0)
            {
                return Err(capacity());
            }
            debit(budget, WorkClass::QueueOperations, self.slots.len())?;
            let i = self
                .slots
                .iter()
                .position(|s| s.entry.is_none() && s.generation.is_some())
                .ok_or_else(capacity)?;
            let id = self.issuer.identity(
                self.consumer,
                SubscriptionId::new(SlotIndex::new(i as u32), self.slots[i].generation.unwrap()),
            );
            debit(budget, WorkClass::ProviderProbes, 2)?;
            self.authorizer
                .authorize(&id, &request.target, NotificationPermission::Establish)?;
            let support = self.binding.support(&program, kind)?;
            if !self.paired() {
                return Err(stale());
            }
            if request.responses == ResponseRequirement::Required && !support.event_responses {
                return Err(unsupported());
            }
            let reservation = self
                .reservation(
                    self.limits.queue_items,
                    request.target.name().unwrap().len(),
                )
                .ok_or_else(capacity)?;
            if self
                .reserved_bytes
                .checked_add(reservation)
                .is_none_or(|n| n > self.limits.engine_bytes)
            {
                return Err(capacity());
            }
            debit(budget, WorkClass::BindingPolls, 1)?;
            Ok((i, id, support))
        })();
        let (i, identity, support) = match check {
            Ok(v) => v,
            Err(e) => return Err(reject((request, program), e)),
        };
        // Prepaid before any external effect. Actual allocator capacity is
        // checked too; process OOM remains an environment failure.
        let mut queue = VecDeque::new();
        if queue.try_reserve_exact(self.limits.queue_items).is_err() {
            return Err(reject((request, program), capacity()));
        }
        let Some(reservation) = self
            .reservation(queue.capacity(), request.target.name().unwrap().len())
            .filter(|n| {
                self.reserved_bytes
                    .checked_add(*n)
                    .is_some_and(|v| v <= self.limits.engine_bytes)
            })
        else {
            return Err(reject((request, program), capacity()));
        };
        let stopped = Rc::new(Cell::new(false));
        let participant = match self.binding.prepare(program, &identity) {
            Ok(p) => p,
            Err(e) => {
                let (program, error) = e.into_parts();
                return Err(BindingInputRejection::new((request, program), error));
            }
        };
        self.reserved_bytes += reservation;
        self.retained += 1;
        let cursor = NotificationCursor {
            identity: identity.clone(),
            stopped: stopped.clone(),
        };
        self.slots[i].entry = Some(Entry {
            identity,
            request,
            stopped,
            support,
            state: NotificationState::Establishing,
            participant: Some(participant),
            native_started: false,
            close_started: false,
            cancel_admitted: false,
            queue,
            staged: None,
            queue_bytes: 0,
            reservation,
            cancellation: None,
            first_cause: None,
            cleanup_error: None,
            cancellation_error: None,
            remote: RemoteCancellation::NotAttempted,
            loss: NotificationLoss::default(),
            next_sequence: 0,
            delivered: 0,
            responses_delivered: 0,
            responses_uncertain: 0,
        });
        Ok(cursor)
    }

    fn authorize(
        entry: &mut Entry<B::Participation>,
        authorizer: &A,
        paired: bool,
        permission: NotificationPermission,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        entry.check_live(paired)?;
        debit(budget, WorkClass::ProviderProbes, 1)?;
        let result = authorizer.authorize(&entry.identity, &entry.request.target, permission);
        entry.sync_stop();
        if entry.state == NotificationState::Closing {
            return Err(entry.first_cause.clone().unwrap());
        }
        if let Err(error) = result {
            entry.fence(error.clone());
            return Err(error);
        }
        Ok(())
    }

    /// Revocation is local and immediate, even with zero native work budget.
    /// The root cannot be reactivated as a replacement Consumer incarnation.
    pub fn revoke(&mut self) {
        self.current = false;
        for slot in &mut self.slots {
            if let Some(e) = &mut slot.entry {
                e.fence(stale());
            }
        }
    }
    /// Whole logical-owner close; native service shutdown remains Binding-owned.
    pub fn begin_close(&mut self) {
        self.current = false;
        for slot in &mut self.slots {
            if let Some(e) = &mut slot.entry {
                e.fence(cancelled());
            }
        }
    }
    /// One paid round-robin cleanup visit, usable after every cursor/identity
    /// has been lost. Returning the complete terminal report acknowledges it;
    /// pending/error retains the original slot. No native global scheduler is
    /// introduced: this only services this root's already retained obligations.
    pub fn step_cleanup(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<NotificationSettlement>> {
        if self.slots.is_empty() {
            return Ok(None);
        }
        debit(budget, WorkClass::CleanupItems, 1)?;
        let i = self.cleanup_cursor;
        self.cleanup_cursor = (i + 1) % self.slots.len();
        let Some(e) = &self.slots[i].entry else {
            return Ok(None);
        };
        if !e.stopped.get() && e.state != NotificationState::Closed {
            return Ok(None);
        }
        let identity = e.identity.clone();
        if self.progress(&identity, cx, budget)? == NotificationState::Closed {
            self.take_settlement(&identity).map(Some)
        } else {
            Ok(None)
        }
    }
    /// Stop ALWAYS fences local delivery first. A malformed/unsupported/denied
    /// mapped cancellation returns its complete input and does not undo stop.
    pub fn stop(
        &mut self,
        cursor: &NotificationCursor,
        cancellation: Option<CancellationInput>,
        budget: &mut WorkBudget,
    ) -> Result<(), BindingInputRejection<Option<CancellationInput>>> {
        let i = match self.index(cursor.identity()) {
            Ok(i) => i,
            Err(e) => return Err(reject(cancellation, e)),
        };
        let paired = self.paired();
        let e = self.slots[i].entry.as_mut().unwrap();
        e.fence(cancelled());
        if cancellation.is_none() {
            return Ok(());
        }
        let result = (|| {
            if e.state == NotificationState::Closed || e.close_started || e.cancellation.is_some() {
                return Err(invalid());
            }
            if !paired {
                return Err(stale());
            }
            if !e.support.cancellation {
                return Err(unsupported());
            }
            if matches!(e.request.target, AffordanceTarget::Property(_))
                && cancellation.as_ref().unwrap().0.is_some()
            {
                return Err(invalid());
            }
            if notification_payload_backing(cancellation.as_ref().unwrap().0.as_ref())
                .is_none_or(|n| n > self.limits.control_backing_bytes)
            {
                return Err(capacity());
            }
            debit(budget, WorkClass::ProviderProbes, 1)?;
            self.authorizer.authorize(
                &e.identity,
                &e.request.target,
                NotificationPermission::Cancel,
            )
        })();
        if let Err(error) = result {
            e.cancellation_error.get_or_insert(error.clone());
            return Err(reject(cancellation, error));
        }
        e.cancellation = cancellation;
        Ok(())
    }

    pub fn status(&self, identity: &NotificationIdentity) -> CoreResult<NotificationStatus<'_>> {
        let e = self.entry(identity)?;
        let state = if e.stopped.get() && e.state != NotificationState::Closed {
            NotificationState::Closing
        } else {
            e.state
        };
        Ok(NotificationStatus {
            state,
            loss: e.loss,
            queued_items: e.queue.len(),
            queued_payload_bytes: e.queue_bytes,
            has_retained_handoff: e.staged.is_some(),
            native_started: e.native_started,
            first_cause: e.first_cause.as_ref(),
            cleanup_error: e.cleanup_error.as_ref(),
            cancellation_error: e.cancellation_error.as_ref(),
        })
    }
    pub fn front(
        &mut self,
        cursor: &mut NotificationCursor,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<RawNotificationView<'_>>> {
        let i = self.index(cursor.identity())?;
        let paired = self.paired();
        let e = self.slots[i].entry.as_mut().unwrap();
        Self::authorize(
            e,
            &self.authorizer,
            paired,
            NotificationPermission::Deliver,
            budget,
        )?;
        e.check_live(binding_paired(
            &self.binding,
            &self.configuration,
            self.binding_identity,
        ))?;
        if e.state != NotificationState::Active {
            return Ok(None);
        }
        let Some(item) = e.queue.front_mut() else {
            return Ok(None);
        };
        if !item.exposed {
            item.exposed = true;
            e.delivered += 1;
        }
        Ok(Some(RawNotificationView {
            sequence: item.sequence,
            data: item.native.data.as_ref(),
            response: item.response_state,
        }))
    }
    pub fn respond(
        &mut self,
        cursor: &mut NotificationCursor,
        input: EventDataResponse,
        budget: &mut WorkBudget,
    ) -> Result<(), BindingInputRejection<EventDataResponse>> {
        let result = (|| {
            let i = self.index(cursor.identity())?;
            let paired = self.paired();
            let e = self.slots[i].entry.as_mut().unwrap();
            Self::authorize(
                e,
                &self.authorizer,
                paired,
                NotificationPermission::Respond,
                budget,
            )?;
            e.check_live(binding_paired(
                &self.binding,
                &self.configuration,
                self.binding_identity,
            ))?;
            if notification_payload_backing(input.0.as_ref())
                .is_none_or(|n| n > self.limits.payload_backing_bytes)
            {
                return Err(capacity());
            }
            let item = e.queue.front_mut().ok_or_else(invalid)?;
            if !item.exposed || item.response_state != EventResponseState::Awaiting {
                return Err(unsupported());
            }
            item.response_state = EventResponseState::Pending;
            Ok(i)
        })();
        match result {
            Ok(i) => {
                self.slots[i]
                    .entry
                    .as_mut()
                    .unwrap()
                    .queue
                    .front_mut()
                    .unwrap()
                    .response_input = Some(input);
                Ok(())
            }
            Err(e) => Err(reject(input, e)),
        }
    }
    pub fn advance(
        &mut self,
        cursor: &mut NotificationCursor,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        let i = self.index(cursor.identity())?;
        let e = self.slots[i].entry.as_mut().unwrap();
        e.sync_stop();
        if e.state != NotificationState::Active {
            return Err(cancelled());
        }
        let item = e.queue.front().ok_or_else(invalid)?;
        if !item.exposed || item.native.response.is_some() {
            return Err(CoreError::Lifecycle(context(ErrorPhase::Delivery)));
        }
        debit(budget, WorkClass::QueueOperations, 1)?;
        let item = e.queue.pop_front().unwrap();
        e.queue_bytes -= item.bytes;
        Ok(())
    }

    /// At most one native/response poll or one retained item disposal per call.
    /// Callers select independent subscriptions; a stalled one cannot hold a
    /// registry lock or consume a sibling's progress lane. Zero work is passive
    /// except for the allocation-free local stop fence.
    pub fn progress(
        &mut self,
        identity: &NotificationIdentity,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<NotificationState> {
        let i = self.index(identity)?;
        let paired = self.paired();
        let e = self.slots[i].entry.as_mut().unwrap();
        e.sync_stop();
        if !paired {
            e.fence(stale());
        }
        if e.state == NotificationState::Closed {
            return Ok(e.state);
        }
        if e.state == NotificationState::Closing {
            if !e.close_started && e.cancellation.is_some() && !e.cancel_admitted {
                debit(budget, WorkClass::ProviderProbes, 1)?;
                match if paired {
                    self.authorizer.authorize(
                        &e.identity,
                        &e.request.target,
                        NotificationPermission::Cancel,
                    )
                } else {
                    Err(stale())
                } {
                    Ok(())
                        if binding_paired(
                            &self.binding,
                            &self.configuration,
                            self.binding_identity,
                        ) =>
                    {
                        e.cancel_admitted = true
                    }
                    Ok(()) => {
                        e.cancellation_error.get_or_insert_with(stale);
                    }
                    Err(error) => {
                        e.cancellation_error.get_or_insert(error);
                    }
                }
            }
            return Self::cleanup(e, cx, budget);
        }
        if e.state == NotificationState::Establishing {
            Self::authorize(
                e,
                &self.authorizer,
                paired,
                NotificationPermission::Establish,
                budget,
            )?;
            e.check_live(binding_paired(
                &self.binding,
                &self.configuration,
                self.binding_identity,
            ))?;
            debit(budget, WorkClass::BindingPolls, 1)?;
            e.native_started = true;
            let result = e
                .participant
                .as_mut()
                .unwrap()
                .poll_start(cx, &e.request.subscription);
            e.sync_stop();
            match result {
                Poll::Ready(Ok(())) if e.state != NotificationState::Closing => {
                    e.state = NotificationState::Active
                }
                Poll::Ready(Err(error)) => e.fence(error),
                _ => {}
            }
            e.sync_stop();
            if !binding_paired(&self.binding, &self.configuration, self.binding_identity) {
                e.fence(stale());
            }
            return Ok(e.state);
        }
        // A reciprocal response is independent of data delivery and native ACKs.
        if e.queue
            .front()
            .is_some_and(|v| v.response_state == EventResponseState::Pending)
        {
            Self::authorize(
                e,
                &self.authorizer,
                paired,
                NotificationPermission::Respond,
                budget,
            )?;
            e.check_live(binding_paired(
                &self.binding,
                &self.configuration,
                self.binding_identity,
            ))?;
            debit(budget, WorkClass::BindingPolls, 1)?;
            let item = e.queue.front_mut().unwrap();
            let result = item
                .native
                .response
                .as_mut()
                .unwrap()
                .poll_deliver(cx, item.response_input.as_ref().unwrap());
            e.sync_stop();
            match result {
                Poll::Ready(Ok(())) => {
                    e.queue.front_mut().unwrap().response_state = EventResponseState::Delivered;
                    e.responses_delivered += 1;
                }
                Poll::Ready(Err(error)) => e.fence(error),
                Poll::Pending => {}
            }
            e.sync_stop();
            if !binding_paired(&self.binding, &self.configuration, self.binding_identity) {
                e.fence(stale());
            }
            return Ok(e.state);
        }
        if e.queue.front().is_some_and(|v| {
            v.response_state == EventResponseState::Delivered && v.native.response.is_some()
        }) {
            debit(budget, WorkClass::BindingPolls, 1)?;
            let item = e.queue.front_mut().unwrap();
            let result = item.native.response.as_mut().unwrap().poll_retire(cx);
            e.sync_stop();
            match result {
                Poll::Ready(Ok(_)) => e
                    .queue
                    .front_mut()
                    .unwrap()
                    .native
                    .response
                    .take()
                    .unwrap()
                    .abort_local(),
                Poll::Ready(Err(error)) => {
                    e.cleanup_error(error.clone());
                    e.fence(error);
                }
                Poll::Pending => {}
            }
            e.sync_stop();
            return Ok(e.state);
        }
        Self::authorize(
            e,
            &self.authorizer,
            paired,
            NotificationPermission::Deliver,
            budget,
        )?;
        e.check_live(binding_paired(
            &self.binding,
            &self.configuration,
            self.binding_identity,
        ))?;
        // Prepay the independent post-callback policy probe before asking the
        // Binding for an owned value. A short work quantum never strands a
        // staged value or accidentally terminates a healthy conversation.
        debit(budget, WorkClass::ProviderProbes, 1)?;
        debit(budget, WorkClass::QueueOperations, 1)?;
        debit(budget, WorkClass::BindingPolls, 1)?;
        let signal = e
            .participant
            .as_mut()
            .unwrap()
            .poll_next(cx, self.limits.payload_backing_bytes);
        e.sync_stop();
        // Native callbacks may request stop synchronously. Own their late value
        // before checking current authority again; never expose it to the cursor.
        match signal {
            Poll::Ready(Ok(NotificationSignal::Data(native))) => {
                let bytes = notification_payload_backing(native.data.as_ref());
                let sequence = e.next_sequence;
                let has_response = native.response.is_some();
                e.staged = Some(Buffered {
                    native,
                    bytes: bytes.unwrap_or(usize::MAX),
                    sequence,
                    exposed: false,
                    response_input: None,
                    response_state: if has_response {
                        EventResponseState::Awaiting
                    } else {
                        EventResponseState::Unavailable
                    },
                });
                e.sync_stop();
                if e.state == NotificationState::Closing {
                    return Ok(e.state);
                }
                // Fresh policy AFTER native progress, independently for each
                // original recipient, precedes acceptance into its logical queue.
                let still_paired =
                    binding_paired(&self.binding, &self.configuration, self.binding_identity);
                let result = if still_paired {
                    self.authorizer.authorize(
                        &e.identity,
                        &e.request.target,
                        NotificationPermission::Deliver,
                    )
                } else {
                    Err(stale())
                };
                e.sync_stop();
                if e.state == NotificationState::Closing {
                    return Ok(e.state);
                }
                if let Err(error) = result {
                    e.fence(error);
                    return Ok(e.state);
                }
                if !binding_paired(&self.binding, &self.configuration, self.binding_identity) {
                    e.fence(stale());
                    return Ok(e.state);
                }
                let item = e.staged.as_ref().unwrap();
                let kind = NotificationKind::for_target(&e.request.target).unwrap();
                if item.native.recipient != e.identity {
                    e.fence(stale());
                } else if bytes.is_none_or(|n| n > self.limits.payload_backing_bytes)
                    || (kind == NotificationKind::PropertyObservation
                        && (item.native.data.is_none() || has_response))
                    || (has_response && !e.support.event_responses)
                    || (e.request.responses == ResponseRequirement::Required && !has_response)
                {
                    e.fence(CoreError::Payload(context(ErrorPhase::Delivery)));
                } else if e.queue.len() == self.limits.queue_items
                    || e.queue_bytes
                        .checked_add(item.bytes)
                        .is_none_or(|n| n > self.limits.queue_payload_bytes)
                {
                    e.loss.add(1);
                    // A response-bearing value cannot be silently dropped. Keep
                    // its exact reply owner in the funded staging cleanup slot.
                    if e.request.overflow == NotificationOverflow::Terminate || has_response {
                        e.fence(CoreError::Backpressure(context(ErrorPhase::Delivery)));
                    } else {
                        e.staged.take().unwrap().abort();
                    }
                } else if let Some(next) = e.next_sequence.checked_add(1) {
                    let item = e.staged.take().unwrap();
                    e.queue_bytes += item.bytes;
                    e.queue.push_back(item);
                    e.next_sequence = next;
                } else {
                    e.fence(CoreError::Backpressure(context(ErrorPhase::Delivery)));
                }
            }
            Poll::Ready(Ok(NotificationSignal::Loss(n))) => {
                if n == 0 {
                    e.fence(invalid());
                } else {
                    e.loss.add(n);
                    if e.request.overflow == NotificationOverflow::Terminate {
                        e.fence(CoreError::Backpressure(context(ErrorPhase::Delivery)));
                    }
                }
            }
            Poll::Ready(Err(error)) => e.fence(error),
            Poll::Pending => {}
        }
        e.sync_stop();
        Ok(e.state)
    }

    fn cleanup(
        e: &mut Entry<B::Participation>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<NotificationState> {
        debit(budget, WorkClass::CleanupItems, 1)?;
        let staged = e.staged.is_some();
        if let Some(item) = if staged {
            e.staged.as_mut()
        } else {
            e.queue.front_mut()
        } {
            if let Some(response) = item.native.response.as_mut() {
                debit(budget, WorkClass::BindingPolls, 1)?;
                match response.poll_retire(cx) {
                    Poll::Ready(Ok(delivery)) => {
                        if item.response_state != EventResponseState::Delivered {
                            match delivery {
                                EventResponseDelivery::Delivered => {
                                    e.responses_delivered = e.responses_delivered.saturating_add(1)
                                }
                                EventResponseDelivery::Unknown => {
                                    e.responses_uncertain = e.responses_uncertain.saturating_add(1)
                                }
                                EventResponseDelivery::NotAttempted => {}
                            }
                        }
                        item.native.response.take().unwrap().abort_local();
                    }
                    Poll::Ready(Err(error)) => e.cleanup_error(error),
                    Poll::Pending => {}
                }
                return Ok(e.state);
            }
            debit(budget, WorkClass::QueueOperations, 1)?;
            if staged {
                e.staged.take();
            } else {
                e.queue_bytes -= e.queue.pop_front().unwrap().bytes;
            }
            return Ok(e.state);
        }
        if !e.native_started {
            e.participant.take().unwrap().abort_local();
            e.state = NotificationState::Closed;
            return Ok(e.state);
        }
        debit(budget, WorkClass::BindingPolls, 1)?;
        e.close_started = true;
        match e.participant.as_mut().unwrap().poll_close(
            cx,
            if e.cancel_admitted {
                e.cancellation.as_ref()
            } else {
                None
            },
        ) {
            Poll::Ready(Ok(remote)) => {
                e.remote = remote;
                e.participant.take().unwrap().abort_local();
                e.state = NotificationState::Closed;
            }
            Poll::Ready(Err(error)) => e.cleanup_error(error),
            Poll::Pending => {}
        }
        Ok(e.state)
    }

    /// The root owner may acknowledge a closed report even after cursor Drop.
    /// The numeric/native identity alone cannot receive data or acknowledge live
    /// cleanup. No slot/capacity is recycled before this terminal boundary.
    pub fn take_settlement(
        &mut self,
        identity: &NotificationIdentity,
    ) -> CoreResult<NotificationSettlement> {
        let i = self.index(identity)?;
        if self.slots[i].entry.as_ref().unwrap().state != NotificationState::Closed {
            return Err(CoreError::Lifecycle(context(ErrorPhase::Cleanup)));
        }
        let mut e = self.slots[i].entry.take().unwrap();
        self.slots[i].generation = self.slots[i].generation.and_then(|g| g.checked_next());
        self.reserved_bytes -= e.reservation;
        self.retained -= 1;
        Ok(NotificationSettlement {
            identity: e.identity.clone(),
            target: e.request.target.clone(),
            binding: self.binding_identity,
            first_cause: e.first_cause.take(),
            cleanup_error: e.cleanup_error.take(),
            cancellation_error: e.cancellation_error.take(),
            native_started: e.native_started,
            remote_cancellation: e.remote,
            loss: e.loss,
            delivered_items: e.delivered,
            responses_delivered: e.responses_delivered,
            responses_uncertain: e.responses_uncertain,
        })
    }
}
