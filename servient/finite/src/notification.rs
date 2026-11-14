//! Caller-driven raw Host notification capacity. Each admitted conversation
//! retains its original logical association. Native infrastructure, persistent
//! association count and execution capacity have independent lifetimes.
use alloc::{collections::VecDeque, rc::Rc, vec::Vec};
use clinkz_wot_core::consumer::ConsumerInstanceRef;
use clinkz_wot_core::thing::AffordanceSite;
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
/// fn duplicate(cursor: clinkz_wot_servient_finite::notification::NotificationCursor) {
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
    /// An unused optional opportunity is being retired without a dataResponse.
    Declining,
    /// Native retirement completed; the cursor may advance without a send.
    Declined,
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
/// Payload/media and occurrence names are slices, never cloneable allocation
/// owners. Keep the root borrowed while using them; explicit copies belong to
/// the caller. An outstanding loan prevents advance and cleanup of that owner.
///
/// ```compile_fail,E0308
/// use clinkz_wot_servient_finite::notification::RawNotificationView;
/// use std::sync::Arc;
/// fn detach(view: RawNotificationView<'_>) -> Arc<[u8]> {
///     Arc::clone(&view.data.unwrap().body)
/// }
/// ```
///
/// ```compile_fail,E0308
/// use clinkz_wot_servient_finite::notification::RawNotificationView;
/// use clinkz_wot_core::AffordanceTarget;
/// fn detach(view: RawNotificationView<'_>) -> AffordanceTarget {
///     view.target.clone()
/// }
/// ```
///
/// ```compile_fail
/// use clinkz_wot_servient_finite::notification::RawNotificationView;
/// fn escape(view: RawNotificationView<'_>) -> &'static [u8] {
///     view.data.unwrap().body
/// }
/// ```
///
/// ```compile_fail
/// use clinkz_wot_servient_finite::notification::RawNotificationView;
/// use clinkz_wot_core::thing::AffordanceSite;
/// fn escape(view: RawNotificationView<'_>) -> &'static str {
///     match view.target { AffordanceSite::Event(name) => name, _ => panic!() }
/// }
/// ```
///
/// ```compile_fail,E0499
/// use clinkz_wot_core::notification::{ConsumerNotificationBinding, NotificationAuthorizer};
/// use clinkz_wot_foundation::WorkBudget;
/// use clinkz_wot_servient_finite::notification::{NotificationCursor, RawNotificationServient};
/// fn advance_while_borrowed<B: ConsumerNotificationBinding, A: NotificationAuthorizer>(
///     root: &mut RawNotificationServient<B, A>, cursor: &mut NotificationCursor,
///     budget: &mut WorkBudget,
/// ) {
///     let bytes = root.front(cursor, budget).unwrap().unwrap().data.unwrap().body;
///     root.advance(cursor, budget).unwrap();
///     assert!(!bytes.is_empty());
/// }
/// ```
pub struct RawNotificationView<'a> {
    pub sequence: u64,
    pub target: AffordanceSite<'a>,
    pub data: Option<PayloadView<'a>>,
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
    // Composed reports can retain original coordinate backing. Keep its
    // original prepared/association owners until this report is disposed.
    _validation: Option<Rc<dyn NotificationValidation>>,
}

struct Buffered<R> {
    native: NativeNotification<R>,
    bytes: usize,
    sequence: u64,
    exposed: bool,
    rejected: bool,
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
struct Entry<B: ConsumerNotificationBinding, A: NotificationAuthorizer> {
    context: RawNotificationContext<B, A>,
    identity: NotificationIdentity,
    request: RawNotificationRequest,
    stopped: Rc<Cell<bool>>,
    state: NotificationState,
    support: NotificationSupport,
    participant: Option<B::Participation>,
    native_started: bool,
    close_started: bool,
    cancel_admitted: bool,
    queue: VecDeque<Buffered<<B::Participation as NotificationParticipation>::Response>>,
    staged: Option<Buffered<<B::Participation as NotificationParticipation>::Response>>,
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
impl<B: ConsumerNotificationBinding, A: NotificationAuthorizer> Entry<B, A> {
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
        if !self.identity.consumer.is_current() {
            self.fence(stale());
        }
    }
    fn check_live(&mut self, paired: bool) -> CoreResult<()> {
        self.sync_stop();
        if !paired || !self.context.paired() {
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
    fn has_rejected_item(&self) -> bool {
        self.staged.as_ref().is_some_and(|v| v.rejected)
            || self.queue.front().is_some_and(|v| v.rejected)
    }
    fn cleanup_error(&mut self, error: CoreError) {
        self.cleanup_error.get_or_insert(error);
    }
}
impl<B: ConsumerNotificationBinding, A: NotificationAuthorizer> Drop for Entry<B, A> {
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
struct Slot<B: ConsumerNotificationBinding, A: NotificationAuthorizer> {
    generation: Option<Generation>,
    entry: Option<Entry<B, A>>,
}

/// Exclusive manual access linearizes progress, delivery, stop and settlement.
/// Policy and native callbacks execute without locks. Linked callbacks and all
/// local destructors must be finite/cooperative; this is no preemption claim.
pub struct RawNotificationServient<B: ConsumerNotificationBinding, A: NotificationAuthorizer> {
    issuer: NotificationIssuer,
    slots: Vec<Slot<B, A>>,
    limits: RawNotificationLimits,
    reserved_bytes: usize,
    current: bool,
    cleanup_cursor: usize,
    retained: usize,
}

/// Source-free admission context, retained by the original work. Construction
/// allocates no conversation slots and establishes no native subscription.
/// The Binding owns its physical backing; validation/policy retain their own
/// original owners. This raw context itself is not a TD qualification proof.
pub struct RawNotificationContext<B: ConsumerNotificationBinding, A: NotificationAuthorizer> {
    binding: B,
    configuration: B::Configuration,
    binding_identity: (BindingId, BindingGeneration),
    authorizer: A,
    validation: Option<Rc<dyn NotificationValidation>>,
    consumer: ConsumerInstanceRef,
}
impl<B: ConsumerNotificationBinding, A: NotificationAuthorizer> RawNotificationContext<B, A> {
    #[allow(clippy::type_complexity)]
    pub fn try_new(
        binding: B,
        configuration: B::Configuration,
        authorizer: A,
        consumer: ConsumerInstanceRef,
        validation: Option<Rc<dyn NotificationValidation>>,
    ) -> Result<
        Self,
        BindingInputRejection<(
            B,
            B::Configuration,
            A,
            Option<Rc<dyn NotificationValidation>>,
        )>,
    > {
        if !consumer.is_current() || binding.configuration() != &configuration {
            return Err(reject(
                (binding, configuration, authorizer, validation),
                invalid(),
            ));
        }
        Ok(Self {
            binding_identity: binding.identity(),
            binding,
            configuration,
            authorizer,
            consumer,
            validation,
        })
    }
    fn paired(&self) -> bool {
        self.consumer.is_current()
            && binding_paired(&self.binding, &self.configuration, self.binding_identity)
    }
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
fn require_subscription(result: CoreResult<NotificationDecision>) -> CoreResult<()> {
    match result? {
        NotificationDecision::Allow => Ok(()),
        NotificationDecision::RejectOccurrence => Err(invalid()),
    }
}
fn occurrence_denied() -> CoreError {
    CoreError::Security {
        reason: SecurityFailureReason::AuthorizationDenied,
        context: context(ErrorPhase::Delivery),
    }
}
fn reject<T>(input: T, error: CoreError) -> BindingInputRejection<T> {
    BindingInputRejection::new(input, BindingOperationalError::new(error))
}
fn debit(budget: &mut WorkBudget, class: WorkClass, n: impl TryInto<u64>) -> CoreResult<()> {
    budget
        .consume(class, n.try_into().map_err(|_| capacity())?)
        .map_err(|_| capacity())
}
fn binding_paired<B: ConsumerNotificationBinding>(
    binding: &B,
    configuration: &B::Configuration,
    identity: (BindingId, BindingGeneration),
) -> bool {
    binding.identity() == identity && binding.configuration() == configuration
}

impl<B: ConsumerNotificationBinding, A: NotificationAuthorizer> RawNotificationServient<B, A> {
    pub fn try_new(limits: RawNotificationLimits) -> CoreResult<Self> {
        let needed = limits
            .subscriptions
            .checked_mul(size_of::<Slot<B, A>>())
            .and_then(|n| n.checked_add(size_of::<Self>() + 2 * size_of::<usize>()));
        if limits.subscriptions > u32::MAX as usize
            || needed.is_none_or(|n| n > limits.engine_bytes)
        {
            return Err(capacity());
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(limits.subscriptions)
            .map_err(|_| capacity())?;
        let reserved_bytes = slots
            .capacity()
            .checked_mul(size_of::<Slot<B, A>>())
            .and_then(|n| n.checked_add(size_of::<Self>() + 2 * size_of::<usize>()))
            .filter(|n| *n <= limits.engine_bytes)
            .ok_or_else(capacity)?;
        slots.resize_with(limits.subscriptions, || Slot {
            generation: Some(Generation::INITIAL),
            entry: None,
        });
        Ok(Self {
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
    fn index(&self, identity: &NotificationIdentity) -> CoreResult<usize> {
        let i = identity.subscription.slot().get() as usize;
        self.slots
            .get(i)
            .and_then(|s| s.entry.as_ref())
            .filter(|e| e.identity == *identity)
            .map(|_| i)
            .ok_or_else(stale)
    }
    fn entry(&self, id: &NotificationIdentity) -> CoreResult<&Entry<B, A>> {
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
        mut context: RawNotificationContext<B, A>,
        budget: &mut WorkBudget,
    ) -> Result<
        NotificationCursor,
        BindingInputRejection<(
            RawNotificationRequest,
            B::Program,
            RawNotificationContext<B, A>,
        )>,
    > {
        let check = (|| {
            if !self.current || !context.paired() {
                return Err(stale());
            }
            let kind = request.kind;
            if !kind.accepts_target(&request.target) {
                return Err(unsupported());
            }
            if request
                .target
                .name()
                .is_some_and(|s| s.len() > self.limits.target_name_bytes)
                || notification_payload_backing(request.subscription.0.as_ref())
                    .is_none_or(|n| n > self.limits.control_backing_bytes)
            {
                return Err(capacity());
            }
            if !kind.is_event()
                && (request.responses != ResponseRequirement::None
                    || request.subscription.0.is_some())
            {
                return Err(invalid());
            }
            if self.limits.queue_items == 0 {
                return Err(capacity());
            }
            if !kind.is_event()
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
                context.consumer.clone(),
                SubscriptionId::new(SlotIndex::new(i as u32), self.slots[i].generation.unwrap()),
            );
            debit(
                budget,
                WorkClass::ProviderProbes,
                context
                    .authorizer
                    .work()
                    .checked_add(1)
                    .ok_or_else(capacity)?,
            )?;
            require_subscription(context.authorizer.authorize(NotificationAuthorization {
                recipient: &id,
                target: &request.target,
                kind,
                occurrence: None,
                permission: NotificationPermission::Establish,
            }))?;
            let support = context.binding.support(&program, kind)?;
            if !self.current || !context.paired() {
                return Err(stale());
            }
            if request.responses == ResponseRequirement::Required && !support.event_responses {
                return Err(unsupported());
            }
            if let Some(validation) = &context.validation {
                validation.subscription(&request.subscription, budget)?;
            }
            if !self.current || !context.paired() {
                return Err(stale());
            }
            let reservation = self
                .reservation(
                    self.limits.queue_items,
                    request.target.name().map_or(0, str::len),
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
            Err(e) => return Err(reject((request, program, context), e)),
        };
        // Prepaid before any external effect. Actual allocator capacity is
        // checked too; process OOM remains an environment failure.
        let mut queue = VecDeque::new();
        if queue.try_reserve_exact(self.limits.queue_items).is_err() {
            return Err(reject((request, program, context), capacity()));
        }
        let Some(reservation) = self
            .reservation(queue.capacity(), request.target.name().map_or(0, str::len))
            .filter(|n| {
                self.reserved_bytes
                    .checked_add(*n)
                    .is_some_and(|v| v <= self.limits.engine_bytes)
            })
        else {
            return Err(reject((request, program, context), capacity()));
        };
        let stopped = Rc::new(Cell::new(false));
        let participant = match context.binding.prepare(program, &identity) {
            Ok(p) => p,
            Err(e) => {
                let (program, error) = e.into_parts();
                return Err(BindingInputRejection::new(
                    (request, program, context),
                    error,
                ));
            }
        };
        self.reserved_bytes += reservation;
        self.retained += 1;
        let cursor = NotificationCursor {
            identity: identity.clone(),
            stopped: stopped.clone(),
        };
        self.slots[i].entry = Some(Entry {
            context,
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
        entry: &mut Entry<B, A>,
        paired: bool,
        permission: NotificationPermission,
        budget: &mut WorkBudget,
    ) -> CoreResult<NotificationDecision> {
        entry.check_live(paired)?;
        let is_item = matches!(
            permission,
            NotificationPermission::Deliver | NotificationPermission::Respond
        );
        if is_item {
            let item = entry.queue.front().ok_or_else(invalid)?;
            if item.rejected {
                return Ok(NotificationDecision::RejectOccurrence);
            }
        }
        let probes = entry
            .context
            .authorizer
            .work()
            .checked_mul(if is_item { 2 } else { 1 })
            .ok_or_else(capacity)?;
        debit(budget, WorkClass::ProviderProbes, probes)?;
        let mut result = entry
            .context
            .authorizer
            .authorize(NotificationAuthorization {
                recipient: &entry.identity,
                target: &entry.request.target,
                kind: entry.request.kind,
                occurrence: if is_item {
                    Some(&entry.queue.front().unwrap().native.target)
                } else {
                    None
                },
                permission,
            });
        entry.check_live(paired)?;
        if is_item && result.is_ok() {
            // The item callback can synchronously withdraw the independent
            // continuing grant. Its post-check was paid before either callback.
            let grant = require_subscription(entry.context.authorizer.authorize(
                NotificationAuthorization {
                    recipient: &entry.identity,
                    target: &entry.request.target,
                    kind: entry.request.kind,
                    occurrence: None,
                    permission: NotificationPermission::Maintain,
                },
            ));
            entry.check_live(paired)?;
            if let Err(error) = grant {
                result = Err(error);
            }
        }
        match result {
            Ok(NotificationDecision::Allow) => Ok(NotificationDecision::Allow),
            Ok(NotificationDecision::RejectOccurrence) if is_item => {
                entry.queue.front_mut().unwrap().rejected = true;
                Ok(NotificationDecision::RejectOccurrence)
            }
            other => {
                let error = match other {
                    Err(error) => error,
                    Ok(_) => invalid(),
                };
                entry.fence(error.clone());
                Err(error)
            }
        }
    }

    /// Fence this capacity domain's admission and retained work immediately,
    /// even with zero native work credit. This does not revoke any Consumer
    /// authority, another capacity owner or shared native infrastructure.
    pub fn revoke(&mut self) {
        self.current = false;
        for slot in &mut self.slots {
            if let Some(e) = &mut slot.entry {
                e.fence(stale());
            }
        }
    }
    /// Close this work owner. Persistent associations, other capacity domains
    /// and shared native infrastructure retain their independent owners.
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
        let Some(e) = &mut self.slots[i].entry else {
            return Ok(None);
        };
        // Independent authority may end while the receive cursor is retained.
        // Observe that fence before deciding whether this entry needs cleanup.
        e.sync_stop();
        if !e.stopped.get() && e.state != NotificationState::Closed && !e.has_rejected_item() {
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
        let paired = self.current;
        let e = self.slots[i].entry.as_mut().unwrap();
        e.fence(cancelled());
        if cancellation.is_none() {
            return Ok(());
        }
        let result = (|| {
            if e.state == NotificationState::Closed || e.close_started || e.cancellation.is_some() {
                return Err(invalid());
            }
            if !paired || !e.context.paired() {
                return Err(stale());
            }
            if !e.support.cancellation {
                return Err(unsupported());
            }
            if matches!(
                e.request.target,
                AffordanceTarget::Thing | AffordanceTarget::Property(_)
            ) && cancellation.as_ref().unwrap().0.is_some()
            {
                return Err(invalid());
            }
            if notification_payload_backing(cancellation.as_ref().unwrap().0.as_ref())
                .is_none_or(|n| n > self.limits.control_backing_bytes)
            {
                return Err(capacity());
            }
            debit(
                budget,
                WorkClass::ProviderProbes,
                e.context.authorizer.work(),
            )?;
            if let Some(validation) = &e.context.validation {
                validation.cancellation(cancellation.as_ref().unwrap(), budget)?;
            }
            require_subscription(e.context.authorizer.authorize(NotificationAuthorization {
                recipient: &e.identity,
                target: &e.request.target,
                kind: e.request.kind,
                occurrence: None,
                permission: NotificationPermission::Cancel,
            }))?;
            if !e.identity.consumer.is_current() {
                return Err(stale());
            }
            Ok(())
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
        let state = if (e.stopped.get() || !e.identity.consumer.is_current())
            && e.state != NotificationState::Closed
        {
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
        let paired = self.current;
        let e = self.slots[i].entry.as_mut().unwrap();
        Self::authorize(e, paired, NotificationPermission::Maintain, budget)?;
        e.check_live(binding_paired(
            &e.context.binding,
            &e.context.configuration,
            e.context.binding_identity,
        ))?;
        if e.state != NotificationState::Active {
            return Ok(None);
        }
        if e.queue.is_empty() {
            return Ok(None);
        }
        if Self::authorize(e, paired, NotificationPermission::Deliver, budget)?
            == NotificationDecision::RejectOccurrence
        {
            return Ok(None);
        }
        let item = e.queue.front_mut().unwrap();
        if !item.exposed {
            item.exposed = true;
            e.delivered += 1;
        }
        Ok(Some(RawNotificationView {
            target: match &item.native.target {
                AffordanceTarget::Thing => AffordanceSite::Thing,
                AffordanceTarget::Property(name) => AffordanceSite::Property(name),
                AffordanceTarget::Action(name) => AffordanceSite::Action(name),
                AffordanceTarget::Event(name) => AffordanceSite::Event(name),
            },
            sequence: item.sequence,
            data: item.native.data.as_ref().map(Payload::view),
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
            let paired = self.current;
            let e = self.slots[i].entry.as_mut().unwrap();
            Self::authorize(e, paired, NotificationPermission::Maintain, budget)?;
            e.check_live(binding_paired(
                &e.context.binding,
                &e.context.configuration,
                e.context.binding_identity,
            ))?;
            if notification_payload_backing(input.0.as_ref())
                .is_none_or(|n| n > self.limits.payload_backing_bytes)
            {
                return Err(capacity());
            }
            let item = e.queue.front_mut().ok_or_else(invalid)?;
            if item.rejected || !item.exposed || item.response_state != EventResponseState::Awaiting
            {
                return Err(unsupported());
            }
            if Self::authorize(e, paired, NotificationPermission::Respond, budget)?
                == NotificationDecision::RejectOccurrence
            {
                return Err(occurrence_denied());
            }
            let item = e.queue.front_mut().unwrap();
            if let Some(validation) = &e.context.validation {
                validation.response(&item.native.target, &input, budget)?;
            }
            e.check_live(binding_paired(
                &e.context.binding,
                &e.context.configuration,
                e.context.binding_identity,
            ))?;
            e.queue.front_mut().unwrap().response_state = EventResponseState::Pending;
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
        if item.rejected || !item.exposed || item.native.response.is_some() {
            return Err(CoreError::Lifecycle(context(ErrorPhase::Delivery)));
        }
        debit(budget, WorkClass::QueueOperations, 1)?;
        let item = e.queue.pop_front().unwrap();
        e.queue_bytes -= item.bytes;
        Ok(())
    }

    /// Decline the exposed front item's unused OPTIONAL response opportunity.
    /// This is a paid local retirement intent, not an application response or
    /// cursor advance. Progress retains the exact reply owner, data and capacity
    /// through Pending/error; successful retirement leaves the subscription
    /// Active and the item available for advance. Required or already submitted
    /// responses cannot be skipped. Like advance, releasing this accepted owner
    /// does not require permission to send a new reciprocal response.
    pub fn decline_response(
        &mut self,
        cursor: &mut NotificationCursor,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        let i = self.index(cursor.identity())?;
        let paired = self.current;
        let e = self.slots[i].entry.as_mut().unwrap();
        e.check_live(paired)?;
        let item = e.queue.front_mut().ok_or_else(invalid)?;
        if e.request.responses != ResponseRequirement::None
            || item.rejected
            || !item.exposed
            || item.response_state != EventResponseState::Awaiting
        {
            return Err(unsupported());
        }
        debit(budget, WorkClass::QueueOperations, 1)?;
        item.response_state = EventResponseState::Declining;
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
        let paired = self.current;
        let e = self.slots[i].entry.as_mut().unwrap();
        e.sync_stop();
        if !paired || !e.context.paired() {
            e.fence(stale());
        }
        if e.state == NotificationState::Closed {
            return Ok(e.state);
        }
        if e.state == NotificationState::Closing {
            if !e.close_started && e.cancellation.is_some() && !e.cancel_admitted {
                debit(
                    budget,
                    WorkClass::ProviderProbes,
                    e.context.authorizer.work(),
                )?;
                match if paired {
                    e.context.authorizer.authorize(NotificationAuthorization {
                        recipient: &e.identity,
                        target: &e.request.target,
                        kind: e.request.kind,
                        occurrence: None,
                        permission: NotificationPermission::Cancel,
                    })
                } else {
                    Err(stale())
                } {
                    Ok(NotificationDecision::Allow)
                        if e.identity.consumer.is_current()
                            && binding_paired(
                                &e.context.binding,
                                &e.context.configuration,
                                e.context.binding_identity,
                            ) =>
                    {
                        e.cancel_admitted = true
                    }
                    Ok(NotificationDecision::Allow) => {
                        e.cancellation_error.get_or_insert_with(stale);
                    }
                    Ok(NotificationDecision::RejectOccurrence) => {
                        e.cancellation_error.get_or_insert_with(invalid);
                    }
                    Err(error) => {
                        e.cancellation_error.get_or_insert(error);
                    }
                }
            }
            return Self::cleanup(e, cx, budget);
        }
        if e.state == NotificationState::Establishing {
            Self::authorize(e, paired, NotificationPermission::Establish, budget)?;
            e.check_live(binding_paired(
                &e.context.binding,
                &e.context.configuration,
                e.context.binding_identity,
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
            if !binding_paired(
                &e.context.binding,
                &e.context.configuration,
                e.context.binding_identity,
            ) {
                e.fence(stale());
            }
            return Ok(e.state);
        }
        if e.has_rejected_item() {
            Self::retire_buffered(e, e.staged.is_some(), cx, budget)?;
            return Ok(e.state);
        }
        Self::authorize(e, paired, NotificationPermission::Maintain, budget)?;
        // A reciprocal response is independent of data delivery and native ACKs.
        if e.queue
            .front()
            .is_some_and(|v| v.response_state == EventResponseState::Pending)
        {
            if Self::authorize(e, paired, NotificationPermission::Respond, budget)?
                == NotificationDecision::RejectOccurrence
            {
                return Ok(e.state);
            }
            e.check_live(binding_paired(
                &e.context.binding,
                &e.context.configuration,
                e.context.binding_identity,
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
            if !binding_paired(
                &e.context.binding,
                &e.context.configuration,
                e.context.binding_identity,
            ) {
                e.fence(stale());
            }
            return Ok(e.state);
        }
        if e.queue.front().is_some_and(|v| {
            matches!(
                v.response_state,
                EventResponseState::Delivered | EventResponseState::Declining
            ) && v.native.response.is_some()
        }) {
            debit(budget, WorkClass::BindingPolls, 1)?;
            let item = e.queue.front_mut().unwrap();
            let result = item.native.response.as_mut().unwrap().poll_retire(cx);
            e.sync_stop();
            match result {
                Poll::Ready(Ok(delivery)) => {
                    let item = e.queue.front_mut().unwrap();
                    if item.response_state == EventResponseState::Declining {
                        // Preserve the native outcome independently of the
                        // decline intent; retirement must not invent a send.
                        match delivery {
                            EventResponseDelivery::Delivered => {
                                e.responses_delivered = e.responses_delivered.saturating_add(1)
                            }
                            EventResponseDelivery::Unknown => {
                                e.responses_uncertain = e.responses_uncertain.saturating_add(1)
                            }
                            EventResponseDelivery::NotAttempted => {}
                        }
                        item.response_state = EventResponseState::Declined;
                    }
                    item.native.response.take().unwrap().abort_local();
                }
                Poll::Ready(Err(error)) => {
                    e.cleanup_error(error.clone());
                    e.fence(error);
                }
                Poll::Pending => {}
            }
            e.sync_stop();
            return Ok(e.state);
        }
        e.check_live(binding_paired(
            &e.context.binding,
            &e.context.configuration,
            e.context.binding_identity,
        ))?;
        // Prepay the independent post-callback policy probe before asking the
        // Binding for an owned value. A short work quantum never strands a
        // staged value or accidentally terminates a healthy conversation.
        debit(
            budget,
            WorkClass::ProviderProbes,
            e.context
                .authorizer
                .work()
                .checked_mul(2)
                .ok_or_else(capacity)?,
        )?;
        debit(budget, WorkClass::QueueOperations, 1)?;
        debit(budget, WorkClass::BindingPolls, 1)?;
        if let Some(validation) = &e.context.validation {
            validation.prepay_data(budget)?;
        }
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
                let bytes = notification_item_backing(&native.target, native.data.as_ref());
                let sequence = e.next_sequence;
                let has_response = native.response.is_some();
                e.staged = Some(Buffered {
                    native,
                    bytes: bytes.unwrap_or(usize::MAX),
                    sequence,
                    exposed: false,
                    rejected: false,
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
                let item = e.staged.as_ref().unwrap();
                if item.native.recipient != e.identity {
                    e.fence(stale());
                    return Ok(e.state);
                }
                if !e
                    .request
                    .kind
                    .accepts_occurrence(&e.request.target, &item.native.target)
                    || item
                        .native
                        .target
                        .name()
                        .is_some_and(|n| n.len() > self.limits.target_name_bytes)
                {
                    e.fence(invalid());
                    return Ok(e.state);
                }
                if bytes.is_none_or(|n| n > self.limits.payload_backing_bytes)
                    || (!e.request.kind.is_event() && (item.native.data.is_none() || has_response))
                    || (has_response && !e.support.event_responses)
                    || (e.request.responses == ResponseRequirement::Required && !has_response)
                {
                    e.fence(CoreError::Payload(context(ErrorPhase::Delivery)));
                    return Ok(e.state);
                }
                // Fresh policy AFTER native progress, independently for each
                // original recipient, precedes acceptance into its logical queue.
                let still_paired = binding_paired(
                    &e.context.binding,
                    &e.context.configuration,
                    e.context.binding_identity,
                );
                let mut result = if still_paired {
                    e.context.authorizer.authorize(NotificationAuthorization {
                        recipient: &e.identity,
                        target: &e.request.target,
                        kind: e.request.kind,
                        occurrence: Some(&e.staged.as_ref().unwrap().native.target),
                        permission: NotificationPermission::Deliver,
                    })
                } else {
                    Err(stale())
                };
                e.sync_stop();
                if e.state == NotificationState::Closing {
                    return Ok(e.state);
                }
                if !e.context.paired() {
                    e.fence(stale());
                    return Ok(e.state);
                }
                if result.is_ok() {
                    if let Err(error) = require_subscription(e.context.authorizer.authorize(
                        NotificationAuthorization {
                            recipient: &e.identity,
                            target: &e.request.target,
                            kind: e.request.kind,
                            occurrence: None,
                            permission: NotificationPermission::Maintain,
                        },
                    )) {
                        result = Err(error);
                    }
                    e.sync_stop();
                    if e.state == NotificationState::Closing {
                        return Ok(e.state);
                    }
                    if !e.context.paired() {
                        e.fence(stale());
                        return Ok(e.state);
                    }
                }
                match result {
                    Ok(NotificationDecision::Allow) => {}
                    Ok(NotificationDecision::RejectOccurrence) => {
                        e.staged.as_mut().unwrap().rejected = true;
                        return Ok(e.state);
                    }
                    Err(error) => {
                        e.fence(error);
                        return Ok(e.state);
                    }
                }
                if !binding_paired(
                    &e.context.binding,
                    &e.context.configuration,
                    e.context.binding_identity,
                ) {
                    e.fence(stale());
                    return Ok(e.state);
                }
                let item = e.staged.as_ref().unwrap();
                let kind = e.request.kind;
                let validation = e.context.validation.as_ref().map_or(Ok(()), |v| {
                    v.data(&item.native.target, item.native.data.as_ref())
                });
                e.sync_stop();
                if !binding_paired(
                    &e.context.binding,
                    &e.context.configuration,
                    e.context.binding_identity,
                ) {
                    e.fence(stale());
                }
                if e.state == NotificationState::Closing {
                    return Ok(e.state);
                }
                let item = e.staged.as_ref().unwrap();
                if item.native.recipient != e.identity {
                    e.fence(stale());
                } else if !kind.accepts_occurrence(&e.request.target, &item.native.target)
                    || item
                        .native
                        .target
                        .name()
                        .is_some_and(|n| n.len() > self.limits.target_name_bytes)
                {
                    e.fence(invalid());
                } else if let Err(error) = validation {
                    e.fence(error);
                } else if bytes.is_none_or(|n| n > self.limits.payload_backing_bytes)
                    || (!kind.is_event() && (item.native.data.is_none() || has_response))
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
                // A linked validator may revoke the original Consumer too.
                e.sync_stop();
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

    fn retire_buffered(
        e: &mut Entry<B, A>,
        staged: bool,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        debit(budget, WorkClass::CleanupItems, 1)?;
        if let Some(item) = if staged {
            e.staged.as_mut()
        } else {
            e.queue.front_mut()
        } {
            if let Some(response) = item.native.response.as_mut() {
                debit(budget, WorkClass::BindingPolls, 1)?;
                let result = response.poll_retire(cx);
                e.sync_stop();
                if !e.context.paired() {
                    e.fence(stale());
                }
                match result {
                    Poll::Ready(Ok(delivery)) => {
                        let item = if staged {
                            e.staged.as_mut()
                        } else {
                            e.queue.front_mut()
                        }
                        .unwrap();
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
                    Poll::Ready(Err(error)) => {
                        e.cleanup_error(error.clone());
                        if e.state == NotificationState::Active {
                            e.fence(error);
                        }
                    }
                    Poll::Pending => {}
                }
                return Ok(());
            }
            debit(budget, WorkClass::QueueOperations, 1)?;
            if staged {
                e.staged.take();
            } else {
                e.queue_bytes -= e.queue.pop_front().unwrap().bytes;
            }
            return Ok(());
        }
        Ok(())
    }

    fn cleanup(
        e: &mut Entry<B, A>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<NotificationState> {
        let staged = e.staged.is_some();
        if staged || !e.queue.is_empty() {
            Self::retire_buffered(e, staged, cx, budget)?;
            return Ok(e.state);
        }
        debit(budget, WorkClass::CleanupItems, 1)?;
        if !e.native_started {
            e.participant.take().unwrap().abort_local();
            e.state = NotificationState::Closed;
            return Ok(e.state);
        }
        debit(budget, WorkClass::BindingPolls, 1)?;
        e.close_started = true;
        match e.participant.as_mut().unwrap().poll_close(
            cx,
            if e.cancel_admitted && e.identity.consumer.is_current() {
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
            binding: e.context.binding_identity,
            first_cause: e.first_cause.take(),
            cleanup_error: e.cleanup_error.take(),
            cancellation_error: e.cancellation_error.take(),
            native_started: e.native_started,
            remote_cancellation: e.remote,
            loss: e.loss,
            delivered_items: e.delivered,
            responses_delivered: e.responses_delivered,
            responses_uncertain: e.responses_uncertain,
            _validation: e.context.validation.take(),
        })
    }
}
