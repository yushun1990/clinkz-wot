//! Caller-driven raw Host notification capacity. Each admitted conversation
//! retains its original logical association. Native infrastructure, persistent
//! association count and execution capacity have independent lifetimes.
use super::ConsumerTurn;
use alloc::{
    collections::VecDeque,
    rc::{Rc, Weak},
    vec::Vec,
};
use clinkz_wot_core::consumer::ConsumerInstanceRef;
use clinkz_wot_core::thing::AffordanceSite;
use clinkz_wot_core::{notification::*, *};
use clinkz_wot_foundation::{Generation, SlotIndex};
use core::{
    cell::Cell,
    task::{Context, Poll},
};

const MAX_SUBSCRIPTIONS: usize = 32;
const MAX_QUEUE_ITEMS: usize = 32;

/// Local conversation, delivery and report capacity. No native availability is
/// promised. Zero subscriptions, queue items or reports disables admission.
#[derive(Clone, Copy, Debug)]
pub struct RawNotificationLimits {
    pub subscriptions: usize,
    /// Reports in progress, unread or held by the caller.
    pub reports: usize,
    pub queue_items: usize,
    /// Cumulative attributable backing of the logical delivery queue.
    pub queue_payload_bytes: usize,
    /// One native handoff (including its attribution) or one response input.
    pub payload_backing_bytes: usize,
    /// Each subscription or cancellation input independently.
    pub control_backing_bytes: usize,
    pub target_name_bytes: usize,
}
impl Default for RawNotificationLimits {
    fn default() -> Self {
        Self {
            subscriptions: 4,
            reports: 8,
            queue_items: 4,
            queue_payload_bytes: 16_384,
            payload_backing_bytes: 4608,
            control_backing_bytes: 4608,
            target_name_bytes: 256,
        }
    }
}

/// A unique receive cursor. It cannot be cloned. Drop fences delivery immediately
/// and leaves the complete owner in its root slot for manual progress.
///
/// ```compile_fail,E0599
/// fn duplicate(cursor: clinkz_wot_servient_finite::notification::NotificationCursor) {
///     let _other_receiver = cursor.clone();
/// }
/// ```
pub struct NotificationCursor {
    identity: NotificationIdentity,
    control: Rc<NotificationControl>,
}
struct NotificationControl {
    consumer: ConsumerInstanceRef,
    stopped: Cell<bool>,
    abandoned: Cell<bool>,
}
impl NotificationControl {
    fn request_stop(&self) {
        // Once revoked, the original instance stays revoked. Record a stop
        // only while it can precede that loss of authority.
        if self.consumer.is_current() {
            self.stopped.set(true);
        }
    }
    fn passive_cause(&self) -> Option<CoreError> {
        if self.stopped.get() {
            Some(cancelled())
        } else if !self.consumer.is_current() {
            Some(stale())
        } else {
            None
        }
    }
}
impl NotificationCursor {
    pub fn identity(&self) -> &NotificationIdentity {
        &self.identity
    }
    pub fn stop_handle(&self) -> NotificationStop {
        NotificationStop(Rc::downgrade(&self.control))
    }
}
impl Drop for NotificationCursor {
    fn drop(&mut self) {
        self.control.abandoned.set(true);
        self.control.request_stop();
    }
}
/// Cloneable cancellation intent only; it confers no receive/response or native
/// infrastructure-shutdown authority. No user/native callback runs here.
#[derive(Clone)]
pub struct NotificationStop(Weak<NotificationControl>);
impl NotificationStop {
    pub fn request_stop(&self) {
        if let Some(control) = self.0.upgrade() {
            control.request_stop();
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationState {
    Establishing,
    Active,
    Closing,
    Closed,
}
/// Disposition of the conversation visited by one bounded drive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationTurn {
    Idle,
    Advanced,
    /// The visited native operation registered its readiness interest.
    Waiting,
    /// A retirement poll failed and still owns the original work.
    RetirementBlocked,
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
/// use clinkz_wot_servient_finite::notification::{NotificationCursor, RawNotificationServient};
/// fn advance_while_borrowed<B: ConsumerNotificationBinding, A: NotificationAuthorizer>(
///     root: &mut RawNotificationServient<B, A>, cursor: &mut NotificationCursor,
/// ) {
///     let bytes = root.front(cursor).unwrap().unwrap().data.unwrap().body;
///     root.advance(cursor).unwrap();
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
    pub native_error: Option<&'a CoreError>,
    pub cleanup_error: Option<&'a CoreError>,
    pub cancellation_error: Option<&'a CoreError>,
}
/// Available only after all local participants, reply jobs and retained data
/// are disposed. A remote Unknown/Unsupported is not a local cleanup failure.
pub struct NotificationSettlement {
    pub identity: NotificationIdentity,
    target: AffordanceTarget,
    pub binding: (BindingId, BindingGeneration),
    pub first_cause: Option<CoreError>,
    pub native_error: Option<CoreError>,
    pub cleanup_error: Option<CoreError>,
    pub cancellation_error: Option<CoreError>,
    pub native_started: bool,
    pub remote_cancellation: RemoteCancellation,
    pub loss: NotificationLoss,
    pub delivered_items: u64,
    pub responses_delivered: u64,
    pub responses_uncertain: u64,
    _report: ReportSlot,
}
impl NotificationSettlement {
    pub fn target(&self) -> AffordanceSite<'_> {
        site(&self.target)
    }
}

// Terminal evidence is stored inline in the original entry, then moved by
// value. This slot counts that evidence through caller disposal, even if the
// conversation slot has been reused or its capacity owner dropped.
struct ReportSlot(Rc<Cell<usize>>);
impl ReportSlot {
    fn claim(used: &Rc<Cell<usize>>, limit: usize) -> CoreResult<Self> {
        if used.get() == limit {
            return Err(capacity());
        }
        used.set(used.get() + 1);
        Ok(Self(used.clone()))
    }
}
impl Drop for ReportSlot {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
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
type Item<B> = Buffered<
    <<B as ConsumerNotificationBinding>::Participation as NotificationParticipation>::Response,
>;

#[derive(Clone, Copy)]
enum Position {
    Staged,
    Front,
}

struct Entry<B: ConsumerNotificationBinding, A: NotificationAuthorizer> {
    context: RawNotificationContext<B, A>,
    identity: NotificationIdentity,
    request: RawNotificationRequest,
    control: Rc<NotificationControl>,
    state: NotificationState,
    support: NotificationSupport,
    participant: Option<B::Participation>,
    native_started: bool,
    receive_next: bool,
    close_started: bool,
    close_next: bool,
    queued_next: bool,
    queue: VecDeque<Item<B>>,
    staged: Option<Item<B>>,
    queue_bytes: usize,
    report: Option<ReportSlot>,
    cancellation: Option<CancellationInput>,
    cancellation_withdrawn: bool,
    first_cause: Option<CoreError>,
    native_error: Option<CoreError>,
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
        self.sync_stop(Some(cause));
    }
    fn sync_stop(&mut self, cause: Option<CoreError>) {
        if self.is_live()
            && let Some(cause) = self.control.passive_cause().or(cause)
        {
            self.first_cause = Some(cause);
            self.state = NotificationState::Closing;
        }
    }
    fn observe_live(&mut self) {
        self.sync_stop(None);
        if self.is_live() && !self.context.paired() {
            self.fence(stale());
        }
        self.sync_stop(None);
    }
    fn is_live(&self) -> bool {
        matches!(
            self.state,
            NotificationState::Establishing | NotificationState::Active
        )
    }
    fn check_live(&mut self) -> CoreResult<()> {
        self.observe_live();
        if self.is_live() {
            Ok(())
        } else {
            Err(self.first_cause.clone().unwrap_or_else(cancelled))
        }
    }
    fn has_rejected_item(&self) -> bool {
        self.staged.as_ref().is_some_and(|item| item.rejected)
            || self.queue.front().is_some_and(|item| item.rejected)
    }
    fn item(&self, position: Position) -> &Item<B> {
        match position {
            Position::Staged => self.staged.as_ref().unwrap(),
            Position::Front => self.queue.front().unwrap(),
        }
    }
    fn item_mut(&mut self, position: Position) -> &mut Item<B> {
        match position {
            Position::Staged => self.staged.as_mut().unwrap(),
            Position::Front => self.queue.front_mut().unwrap(),
        }
    }
    fn pop_front(&mut self) {
        self.queue_bytes -= self.queue.pop_front().unwrap().bytes;
    }
    fn native_failure(&mut self, error: CoreError) {
        self.native_error.get_or_insert_with(|| error.clone());
        self.fence(error);
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
    next_generation: Option<Generation>,
    entry: Option<Entry<B, A>>,
}

/// Exclusive manual access linearizes progress, delivery, stop and settlement.
/// Policy and native callbacks execute without locks. Linked callbacks and all
/// local destructors must be finite/cooperative; this is no preemption claim.
pub struct RawNotificationServient<B: ConsumerNotificationBinding, A: NotificationAuthorizer> {
    issuer: NotificationIssuer,
    slots: Vec<Slot<B, A>>,
    limits: RawNotificationLimits,
    reports: Rc<Cell<usize>>,
    current: bool,
    progress_cursor: usize,
    cleanup_cursor: usize,
    retained: usize,
}
impl<B: ConsumerNotificationBinding, A: NotificationAuthorizer> Drop
    for RawNotificationServient<B, A>
{
    fn drop(&mut self) {
        let _turn = ConsumerTurn::scope();
        self.slots.clear();
    }
}

type ContextRejection<B, A> = BindingInputRejection<(
    B,
    <B as ConsumerNotificationBinding>::Configuration,
    A,
    Option<Rc<dyn NotificationValidation>>,
)>;

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
    // Last so all caller-supplied fields are released inside the turn, even
    // for a context returned unused from admission or during unwinding.
    drop_turn: Option<ConsumerTurn>,
}
impl<B: ConsumerNotificationBinding, A: NotificationAuthorizer> Drop
    for RawNotificationContext<B, A>
{
    fn drop(&mut self) {
        self.drop_turn = ConsumerTurn::scope();
    }
}
impl<B: ConsumerNotificationBinding, A: NotificationAuthorizer> RawNotificationContext<B, A> {
    pub fn try_new(
        binding: B,
        configuration: B::Configuration,
        authorizer: A,
        consumer: ConsumerInstanceRef,
        validation: Option<Rc<dyn NotificationValidation>>,
    ) -> Result<Self, ContextRejection<B, A>> {
        let mut turn = match ConsumerTurn::enter() {
            Ok(turn) => turn,
            Err(error) => {
                return Err(reject(
                    (binding, configuration, authorizer, validation),
                    error,
                ));
            }
        };
        Self::new_in_turn(
            &mut turn,
            binding,
            configuration,
            authorizer,
            consumer,
            validation,
        )
    }
    pub(super) fn new_in_turn(
        _turn: &mut ConsumerTurn,
        binding: B,
        configuration: B::Configuration,
        authorizer: A,
        consumer: ConsumerInstanceRef,
        validation: Option<Rc<dyn NotificationValidation>>,
    ) -> Result<Self, ContextRejection<B, A>> {
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
            drop_turn: None,
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
pub(super) fn reject<T>(input: T, error: CoreError) -> BindingInputRejection<T> {
    BindingInputRejection::new(input, BindingOperationalError::new(error))
}
fn site(target: &AffordanceTarget) -> AffordanceSite<'_> {
    match target {
        AffordanceTarget::Thing => AffordanceSite::Thing,
        AffordanceTarget::Property(name) => AffordanceSite::Property(name),
        AffordanceTarget::Action(name) => AffordanceSite::Action(name),
        AffordanceTarget::Event(name) => AffordanceSite::Event(name),
    }
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
        if limits.subscriptions > MAX_SUBSCRIPTIONS || limits.queue_items > MAX_QUEUE_ITEMS {
            return Err(capacity());
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(limits.subscriptions)
            .map_err(|_| capacity())?;
        slots.resize_with(limits.subscriptions, || Slot {
            next_generation: Some(Generation::INITIAL),
            entry: None,
        });
        Ok(Self {
            issuer: NotificationIssuer::default(),
            slots,
            limits,
            reports: Rc::default(),
            current: true,
            progress_cursor: 0,
            cleanup_cursor: 0,
            retained: 0,
        })
    }

    /// Conversations including locally closed, unread reports.
    pub fn retained_subscriptions(&self) -> usize {
        self.retained
    }

    fn index(&self, identity: &NotificationIdentity) -> CoreResult<usize> {
        let i = identity.subscription().slot().get() as usize;
        self.slots
            .get(i)
            .and_then(|s| s.entry.as_ref())
            .filter(|e| e.identity == *identity)
            .map(|_| i)
            .ok_or_else(stale)
    }

    /// Before effect, refusal returns the exact request, program and context.
    #[allow(clippy::type_complexity)]
    pub fn subscribe(
        &mut self,
        request: RawNotificationRequest,
        program: B::Program,
        context: RawNotificationContext<B, A>,
    ) -> Result<
        NotificationCursor,
        BindingInputRejection<(
            RawNotificationRequest,
            B::Program,
            RawNotificationContext<B, A>,
        )>,
    > {
        let mut turn = match ConsumerTurn::enter() {
            Ok(turn) => turn,
            Err(error) => return Err(reject((request, program, context), error)),
        };
        self.subscribe_in_turn(&mut turn, request, program, context)
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn subscribe_in_turn(
        &mut self,
        _turn: &mut ConsumerTurn,
        request: RawNotificationRequest,
        program: B::Program,
        mut context: RawNotificationContext<B, A>,
    ) -> Result<
        NotificationCursor,
        BindingInputRejection<(
            RawNotificationRequest,
            B::Program,
            RawNotificationContext<B, A>,
        )>,
    > {
        let checked = (|| {
            if !self.current || !context.consumer.is_current() {
                return Err(stale());
            }
            let kind = request.kind;
            if !kind.accepts_target(&request.target) {
                return Err(unsupported());
            }
            let limits = self.limits;
            if limits.queue_items == 0
                || request
                    .target
                    .name()
                    .is_some_and(|s| s.len() > limits.target_name_bytes)
                || notification_payload_backing(request.subscription.0.as_ref())
                    .is_none_or(|n| n > limits.control_backing_bytes)
                || !kind.is_event()
                    && (limits.queue_payload_bytes == 0 || limits.payload_backing_bytes == 0)
            {
                return Err(capacity());
            }
            if !kind.is_event()
                && (request.responses != ResponseRequirement::None
                    || request.subscription.0.is_some())
            {
                return Err(invalid());
            }
            let i = self
                .slots
                .iter()
                .position(|s| s.entry.is_none() && s.next_generation.is_some())
                .ok_or_else(capacity)?;
            let report = ReportSlot::claim(&self.reports, limits.reports)?;
            if !context.paired() {
                return Err(stale());
            }
            // Policy and Binding may retain an admission identity. Spend it
            // before the first such callback, including on refusal or unwind.
            let slot = &mut self.slots[i];
            let generation = slot.next_generation.unwrap();
            slot.next_generation = generation.checked_next();
            let identity = self.issuer.identity(
                context.consumer.clone(),
                SubscriptionId::new(SlotIndex::new(i as u32), generation),
            );
            require_subscription(context.authorizer.authorize(NotificationAuthorization {
                recipient: &identity,
                target: &request.target,
                kind,
                occurrence: None,
                permission: NotificationPermission::Establish,
            }))?;
            let support = context.binding.support(&program, kind)?;
            if !context.paired() {
                return Err(stale());
            }
            if request.responses == ResponseRequirement::Required && !support.event_responses {
                return Err(unsupported());
            }
            if let Some(validation) = &context.validation {
                validation
                    .subscription(&request.subscription)
                    .map_err(NotificationValidationError::into_error)?;
            }
            if !context.paired() {
                return Err(stale());
            }
            // Reserve real queue storage before obtaining a participation.
            // The in-place handoff and terminal fields need no later allocation.
            let mut queue = VecDeque::new();
            queue
                .try_reserve_exact(limits.queue_items)
                .map_err(|_| capacity())?;
            Ok((i, identity, support, report, queue))
        })();
        let (i, identity, support, report, queue) = match checked {
            Ok(v) => v,
            Err(error) => return Err(reject((request, program, context), error)),
        };

        let control = Rc::new(NotificationControl {
            consumer: identity.consumer().clone(),
            stopped: Cell::new(false),
            abandoned: Cell::new(false),
        });
        let participant = match context.binding.prepare(program, &identity) {
            Ok(participant) => participant,
            Err(e) => {
                let (program, error) = e.into_parts();
                return Err(BindingInputRejection::new(
                    (request, program, context),
                    error,
                ));
            }
        };
        let cursor = NotificationCursor {
            identity: identity.clone(),
            control: control.clone(),
        };
        self.retained += 1;
        self.slots[i].entry = Some(Entry {
            context,
            identity,
            request,
            control,
            support,
            state: NotificationState::Establishing,
            participant: Some(participant),
            native_started: false,
            receive_next: false,
            close_started: false,
            close_next: false,
            queued_next: false,
            queue,
            staged: None,
            queue_bytes: 0,
            report: Some(report),
            cancellation: None,
            cancellation_withdrawn: false,
            first_cause: None,
            native_error: None,
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

    /// Fence this capacity's original work, leaving independent Consumers and
    /// Binding services in their existing ownership domains.
    pub fn revoke(&mut self) {
        self.fence(stale());
    }

    pub fn begin_close(&mut self) {
        self.fence(cancelled());
    }

    fn fence(&mut self, cause: CoreError) {
        self.current = false;
        for entry in self.slots.iter_mut().filter_map(|slot| slot.entry.as_mut()) {
            entry.fence(cause.clone());
        }
    }

    /// Visit one slot in rotation. Keep scheduling turns for other slots
    /// and locally runnable work even when the visited native poll waits.
    /// Closed reports with abandoned cursors are disposed in place.
    pub fn step(&mut self, cx: &mut Context<'_>) -> CoreResult<NotificationTurn> {
        let _turn = ConsumerTurn::enter()?;
        let count = self.slots.len();
        if count == 0 {
            return Ok(NotificationTurn::Idle);
        }
        let i = self.progress_cursor;
        self.progress_cursor = (i + 1) % count;
        let Some(entry) = &mut self.slots[i].entry else {
            return Ok(NotificationTurn::Idle);
        };
        // A policy error already fences this entry. Aggregate driving leaves
        // the original error with the work and continues rotation.
        let turn = entry
            .progress(Some(self.limits), self.current, cx)
            .unwrap_or(NotificationTurn::Advanced);
        if entry.state == NotificationState::Closed && entry.control.abandoned.get() {
            drop(self.take_closed(i)?);
        }
        Ok(turn)
    }

    /// One rotating cleanup visit, including after every cursor is lost.
    /// Returns a report only when the original local owners are retired.
    pub fn step_cleanup(
        &mut self,
        cx: &mut Context<'_>,
    ) -> CoreResult<Option<NotificationSettlement>> {
        let _turn = ConsumerTurn::enter()?;
        if self.slots.is_empty() {
            return Ok(None);
        }
        let i = self.cleanup_cursor;
        self.cleanup_cursor = (i + 1) % self.slots.len();
        let Some(entry) = &mut self.slots[i].entry else {
            return Ok(None);
        };
        entry.sync_stop(None);
        if entry.is_live() && !entry.has_rejected_item() {
            return Ok(None);
        }
        entry.progress(None, self.current, cx)?;
        if entry.state == NotificationState::Closed {
            self.take_closed(i).map(Some)
        } else {
            Ok(None)
        }
    }

    /// A local stop only fences delivery. A mapped stop takes the application
    /// turn, then fences local use before validating or authorizing its input.
    pub fn stop(
        &mut self,
        cursor: &NotificationCursor,
        cancellation: Option<CancellationInput>,
    ) -> Result<(), BindingInputRejection<Option<CancellationInput>>> {
        let _turn = if cancellation.is_some() {
            match ConsumerTurn::enter() {
                Ok(turn) => Some(turn),
                Err(error) => return Err(reject(cancellation, error)),
            }
        } else {
            None
        };
        let i = match self.index(cursor.identity()) {
            Ok(i) => i,
            Err(error) => return Err(reject(cancellation, error)),
        };
        let entry = self.slots[i].entry.as_mut().unwrap();
        entry.fence(cancelled());
        let Some(input) = cancellation.as_ref() else {
            return Ok(());
        };
        let checked = (|| {
            if entry.state == NotificationState::Closed
                || entry.close_started
                || entry.cancellation.is_some()
            {
                return Err(invalid());
            }
            if !self.current || !entry.context.paired() {
                return Err(stale());
            }
            if !entry.support.cancellation {
                return Err(unsupported());
            }
            if entry.request.kind != NotificationKind::Event && input.0.is_some() {
                return Err(invalid());
            }
            if notification_payload_backing(input.0.as_ref())
                .is_none_or(|n| n > self.limits.control_backing_bytes)
            {
                return Err(capacity());
            }
            if let Some(validation) = &entry.context.validation {
                validation
                    .cancellation(input)
                    .map_err(NotificationValidationError::into_error)?;
            }
            entry.authorize_cancel(self.current)
        })();
        if let Err(error) = checked {
            entry
                .cancellation_error
                .get_or_insert_with(|| error.clone());
            return Err(reject(cancellation, error));
        }
        entry.cancellation = cancellation;
        Ok(())
    }

    pub fn status(&self, identity: &NotificationIdentity) -> CoreResult<NotificationStatus<'_>> {
        let e = self.slots[self.index(identity)?].entry.as_ref().unwrap();
        let state = if e.is_live() && e.control.passive_cause().is_some() {
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
            native_error: e.native_error.as_ref(),
            cleanup_error: e.cleanup_error.as_ref(),
            cancellation_error: e.cancellation_error.as_ref(),
        })
    }

    pub fn front(
        &mut self,
        cursor: &mut NotificationCursor,
    ) -> CoreResult<Option<RawNotificationView<'_>>> {
        let _turn = ConsumerTurn::enter()?;
        let i = self.index(cursor.identity())?;
        let e = self.slots[i].entry.as_mut().unwrap();
        e.authorize(NotificationPermission::Maintain, None)?;
        if e.state != NotificationState::Active || e.queue.is_empty() {
            return Ok(None);
        }
        if e.authorize(NotificationPermission::Deliver, Some(Position::Front))?
            == NotificationDecision::RejectOccurrence
        {
            return Ok(None);
        }
        let item = e.queue.front_mut().unwrap();
        if !item.exposed {
            item.exposed = true;
            e.delivered = e.delivered.saturating_add(1);
        }
        Ok(Some(RawNotificationView {
            target: site(&item.native.target),
            sequence: item.sequence,
            data: item.native.data.as_ref().map(Payload::view),
            response: item.response_state,
        }))
    }

    pub fn respond(
        &mut self,
        cursor: &mut NotificationCursor,
        input: EventDataResponse,
    ) -> Result<(), BindingInputRejection<EventDataResponse>> {
        let _turn = match ConsumerTurn::enter() {
            Ok(turn) => turn,
            Err(error) => return Err(reject(input, error)),
        };
        let result = (|| {
            let i = self.index(cursor.identity())?;
            let e = self.slots[i].entry.as_mut().unwrap();
            e.authorize(NotificationPermission::Maintain, None)?;
            if notification_payload_backing(input.0.as_ref())
                .is_none_or(|n| n > self.limits.payload_backing_bytes)
            {
                return Err(capacity());
            }
            let item = e.queue.front().ok_or_else(invalid)?;
            if item.rejected {
                return Err(occurrence_denied());
            }
            if !item.exposed || item.response_state != EventResponseState::Awaiting {
                return Err(unsupported());
            }
            if e.authorize(NotificationPermission::Respond, Some(Position::Front))?
                == NotificationDecision::RejectOccurrence
            {
                return Err(occurrence_denied());
            }
            if let Some(validation) = &e.context.validation {
                let result = validation.response(&e.queue.front().unwrap().native.target, &input);
                e.check_live()?;
                match result {
                    Ok(()) => {}
                    Err(NotificationValidationError::Rejected(error)) => return Err(error),
                    Err(NotificationValidationError::Dependency(error)) => {
                        e.fence(error.clone());
                        return Err(error);
                    }
                }
            }
            Ok(i)
        })();
        match result {
            Ok(i) => {
                let item = self.slots[i]
                    .entry
                    .as_mut()
                    .unwrap()
                    .queue
                    .front_mut()
                    .unwrap();
                item.response_state = EventResponseState::Pending;
                item.response_input = Some(input);
                Ok(())
            }
            Err(error) => Err(reject(input, error)),
        }
    }

    pub fn advance(&mut self, cursor: &mut NotificationCursor) -> CoreResult<()> {
        let _turn = ConsumerTurn::enter()?;
        let i = self.index(cursor.identity())?;
        let e = self.slots[i].entry.as_mut().unwrap();
        e.sync_stop(None);
        if e.state != NotificationState::Active {
            return Err(cancelled());
        }
        let item = e.queue.front().ok_or_else(invalid)?;
        if item.rejected || !item.exposed || item.native.response.is_some() {
            return Err(CoreError::Lifecycle(context(ErrorPhase::Delivery)));
        }
        e.pop_front();
        Ok(())
    }

    /// Give up an exposed, unused optional reply. Its original owner still
    /// needs to retire before the application can advance the cursor.
    pub fn decline_response(&mut self, cursor: &mut NotificationCursor) -> CoreResult<()> {
        let _turn = ConsumerTurn::enter()?;
        let i = self.index(cursor.identity())?;
        let e = self.slots[i].entry.as_mut().unwrap();
        e.check_live()?;
        let item = e.queue.front_mut().ok_or_else(invalid)?;
        if e.request.responses != ResponseRequirement::None
            || item.rejected
            || !item.exposed
            || item.response_state != EventResponseState::Awaiting
        {
            return Err(unsupported());
        }
        item.response_state = EventResponseState::Declining;
        Ok(())
    }

    /// Advance one original conversation by at most one native poll or one
    /// item release. Inspect its logical state separately with status.
    pub fn progress(
        &mut self,
        identity: &NotificationIdentity,
        cx: &mut Context<'_>,
    ) -> CoreResult<NotificationTurn> {
        let _turn = ConsumerTurn::enter()?;
        let i = self.index(identity)?;
        self.slots[i]
            .entry
            .as_mut()
            .unwrap()
            .progress(Some(self.limits), self.current, cx)
    }

    /// Acknowledgement frees the conversation and its execution context.
    /// The report retains the original evidence and coordinate in its own slot.
    pub fn take_settlement(
        &mut self,
        identity: &NotificationIdentity,
    ) -> CoreResult<NotificationSettlement> {
        let _turn = ConsumerTurn::enter()?;
        self.take_closed(self.index(identity)?)
    }

    fn take_closed(&mut self, i: usize) -> CoreResult<NotificationSettlement> {
        let slot = &mut self.slots[i];
        if slot.entry.as_ref().unwrap().state != NotificationState::Closed {
            return Err(CoreError::Lifecycle(context(ErrorPhase::Cleanup)));
        }
        let mut e = slot.entry.take().unwrap();
        self.retained -= 1;
        Ok(NotificationSettlement {
            identity: e.identity.clone(),
            target: core::mem::replace(&mut e.request.target, AffordanceTarget::Thing),
            binding: e.context.binding_identity,
            first_cause: e.first_cause.take(),
            native_error: e.native_error.take(),
            cleanup_error: e.cleanup_error.take(),
            cancellation_error: e.cancellation_error.take(),
            native_started: e.native_started,
            remote_cancellation: e.remote,
            loss: e.loss,
            delivered_items: e.delivered,
            responses_delivered: e.responses_delivered,
            responses_uncertain: e.responses_uncertain,
            _report: e.report.take().unwrap(),
        })
    }
}

impl<B: ConsumerNotificationBinding, A: NotificationAuthorizer> Entry<B, A> {
    fn decision(
        &self,
        permission: NotificationPermission,
        item: Option<Position>,
    ) -> CoreResult<NotificationDecision> {
        self.context
            .authorizer
            .authorize(NotificationAuthorization {
                recipient: &self.identity,
                target: &self.request.target,
                kind: self.request.kind,
                occurrence: item.map(|p| &self.item(p).native.target),
                permission,
            })
    }

    fn authorize(
        &mut self,
        permission: NotificationPermission,
        item: Option<Position>,
    ) -> CoreResult<NotificationDecision> {
        self.check_live()?;
        if item.is_some_and(|p| self.item(p).rejected) {
            return Ok(NotificationDecision::RejectOccurrence);
        }
        let mut decision = self.decision(permission, item);
        self.check_live()?;
        if item.is_some() && decision.is_ok() {
            // The occurrence decision may synchronously withdraw the
            // independent continuing grant, even when refusing the item.
            let grant = require_subscription(self.decision(NotificationPermission::Maintain, None));
            self.check_live()?;
            if let Err(error) = grant {
                decision = Err(error);
            }
        }
        match decision {
            Ok(NotificationDecision::Allow) => Ok(NotificationDecision::Allow),
            Ok(NotificationDecision::RejectOccurrence) if item.is_some() => {
                self.item_mut(item.unwrap()).rejected = true;
                Ok(NotificationDecision::RejectOccurrence)
            }
            result => {
                let error = result.err().unwrap_or_else(invalid);
                self.fence(error.clone());
                Err(error)
            }
        }
    }

    fn authorize_cancel(&self, current: bool) -> CoreResult<()> {
        if !current || !self.context.paired() {
            return Err(stale());
        }
        let result = require_subscription(self.decision(NotificationPermission::Cancel, None));
        if !self.context.paired() {
            return Err(stale());
        }
        result
    }

    fn progress(
        &mut self,
        delivery: Option<RawNotificationLimits>,
        current: bool,
        cx: &mut Context<'_>,
    ) -> CoreResult<NotificationTurn> {
        self.observe_live();
        match self.state {
            NotificationState::Closed => return Ok(NotificationTurn::Closed),
            NotificationState::Closing => return Ok(self.cleanup(current, cx)),
            NotificationState::Establishing => {
                self.authorize(NotificationPermission::Establish, None)?;
                return Ok(self.poll_start(cx));
            }
            NotificationState::Active => {}
        }
        // Observe the continuing grant even while a reply waits. After a
        // fence, cleanup can advance both owners without another Maintain check.
        self.authorize(NotificationPermission::Maintain, None)?;
        // A pending front reply gives the ingress lane the next turn. A
        // retained staged item occupies that lane until it retires. The queue
        // itself stays in delivery order until the conversation is fenced.
        let ingress = self.staged.is_some() || delivery.is_some();
        if (!self.receive_next || !ingress)
            && let Some(item) = self.queue.front().filter(|item| {
                item.rejected
                    || delivery.is_some()
                        && item.native.response.is_some()
                        && matches!(
                            item.response_state,
                            EventResponseState::Pending
                                | EventResponseState::Delivered
                                | EventResponseState::Declining
                        )
            })
        {
            let turn = if !item.rejected && item.response_state == EventResponseState::Pending {
                if self.authorize(NotificationPermission::Respond, Some(Position::Front))?
                    == NotificationDecision::RejectOccurrence
                {
                    NotificationTurn::Advanced
                } else {
                    self.poll_response(cx)
                }
            } else {
                self.retire_item(Position::Front, cx)
            };
            self.receive_next = turn == NotificationTurn::Waiting;
            return Ok(turn);
        }
        self.receive_next = false;
        Ok(if self.staged.is_some() {
            self.retire_item(Position::Staged, cx)
        } else if let Some(limits) = delivery {
            self.poll_receive(limits, cx)
        } else {
            NotificationTurn::Idle
        })
    }

    fn poll_start(&mut self, cx: &mut Context<'_>) -> NotificationTurn {
        self.native_started = true;
        let result = self
            .participant
            .as_mut()
            .unwrap()
            .poll_start(cx, &self.request.subscription);
        self.observe_live();
        match result {
            Poll::Pending if self.is_live() => NotificationTurn::Waiting,
            Poll::Pending => NotificationTurn::Advanced,
            Poll::Ready(result) => {
                self.request.subscription.0.take();
                match result {
                    Ok(()) if self.is_live() => self.state = NotificationState::Active,
                    Err(error) => self.native_failure(error),
                    _ => {}
                }
                NotificationTurn::Advanced
            }
        }
    }

    fn poll_response(&mut self, cx: &mut Context<'_>) -> NotificationTurn {
        let item = self.queue.front_mut().unwrap();
        let result = item
            .native
            .response
            .as_mut()
            .unwrap()
            .poll_deliver(cx, item.response_input.as_ref().unwrap());
        self.observe_live();
        match result {
            Poll::Ready(Ok(())) => {
                self.queue.front_mut().unwrap().response_state = EventResponseState::Delivered;
                self.responses_delivered = self.responses_delivered.saturating_add(1);
            }
            Poll::Ready(Err(error)) => self.native_failure(error),
            Poll::Pending if self.is_live() => return NotificationTurn::Waiting,
            Poll::Pending => {}
        }
        NotificationTurn::Advanced
    }

    fn poll_receive(
        &mut self,
        limits: RawNotificationLimits,
        cx: &mut Context<'_>,
    ) -> NotificationTurn {
        let signal = self
            .participant
            .as_mut()
            .unwrap()
            .poll_next(cx, limits.payload_backing_bytes);
        match signal {
            Poll::Ready(Ok(NotificationSignal::Data(native))) => {
                // Own the handoff before any linked callback, including a
                // refused or late candidate with its original reply.
                let bytes = notification_item_backing(&native.target, native.data.as_ref());
                let has_response = native.response.is_some();
                self.staged = Some(Buffered {
                    native,
                    bytes: bytes.unwrap_or(usize::MAX),
                    sequence: self.next_sequence,
                    exposed: false,
                    rejected: false,
                    response_input: None,
                    response_state: if has_response {
                        EventResponseState::Awaiting
                    } else {
                        EventResponseState::Unavailable
                    },
                });
                self.accept_staged(limits, bytes);
            }
            Poll::Ready(Ok(NotificationSignal::Loss(count))) => {
                self.observe_live();
                if count == 0 {
                    self.fence(invalid());
                } else {
                    self.loss.add(count);
                    if self.request.overflow == NotificationOverflow::Terminate {
                        self.fence(CoreError::Backpressure(context(ErrorPhase::Delivery)));
                    }
                }
            }
            Poll::Ready(Err(error)) => {
                self.observe_live();
                self.native_failure(error);
            }
            Poll::Pending => {
                self.observe_live();
                if self.is_live() {
                    return NotificationTurn::Waiting;
                }
            }
        }
        NotificationTurn::Advanced
    }

    fn accept_staged(&mut self, limits: RawNotificationLimits, bytes: Option<usize>) {
        self.observe_live();
        if !self.is_live() {
            return;
        }
        let item = self.staged.as_ref().unwrap();
        let kind = self.request.kind;
        let has_response = item.native.response.is_some();
        let shape_error = if item.native.recipient != self.identity {
            Some(stale())
        } else if !kind.accepts_occurrence(&self.request.target, &item.native.target)
            || item
                .native
                .target
                .name()
                .is_some_and(|n| n.len() > limits.target_name_bytes)
        {
            Some(invalid())
        } else if bytes.is_none_or(|n| n > limits.payload_backing_bytes)
            || !kind.is_event() && (item.native.data.is_none() || has_response)
            || has_response && !self.support.event_responses
            || self.request.responses == ResponseRequirement::Required && !has_response
        {
            Some(CoreError::Payload(context(ErrorPhase::Delivery)))
        } else {
            None
        };
        if let Some(error) = shape_error {
            self.fence(error);
            return;
        }
        if !matches!(
            self.authorize(NotificationPermission::Deliver, Some(Position::Staged)),
            Ok(NotificationDecision::Allow)
        ) {
            return;
        }
        let item = self.staged.as_ref().unwrap();
        if let Some(validation) = &self.context.validation {
            let result = validation.data(&item.native.target, item.native.data.as_ref());
            self.observe_live();
            if let Err(error) = result {
                self.fence(error.into_error());
            }
            if !self.is_live() {
                return;
            }
        }
        let bytes = bytes.expect("checked handoff backing");
        let queued = self
            .queue_bytes
            .checked_add(bytes)
            .filter(|total| *total <= limits.queue_payload_bytes);
        if let Some(total) = queued.filter(|_| self.queue.len() < limits.queue_items) {
            if let Some(next) = self.next_sequence.checked_add(1) {
                self.next_sequence = next;
                self.queue_bytes = total;
                self.queue.push_back(self.staged.take().unwrap());
                return;
            }
        } else {
            self.loss.add(1);
            if self.request.overflow == NotificationOverflow::DropNewest && !has_response {
                self.staged.take();
                return;
            }
        }
        self.fence(CoreError::Backpressure(context(ErrorPhase::Delivery)));
    }

    fn retire_item(&mut self, position: Position, cx: &mut Context<'_>) -> NotificationTurn {
        if let Some(response) = &mut self.item_mut(position).native.response {
            let result = response.poll_retire(cx);
            self.observe_live();
            match result {
                Poll::Ready(Ok(delivery)) => {
                    let item = self.item_mut(position);
                    let counted = item.response_state == EventResponseState::Delivered;
                    if item.response_state == EventResponseState::Declining {
                        item.response_state = EventResponseState::Declined;
                    }
                    drop(item.native.response.take());
                    item.response_input.take();
                    if !counted {
                        match delivery {
                            EventResponseDelivery::Delivered => {
                                self.responses_delivered =
                                    self.responses_delivered.saturating_add(1);
                            }
                            EventResponseDelivery::Unknown => {
                                self.responses_uncertain =
                                    self.responses_uncertain.saturating_add(1);
                            }
                            EventResponseDelivery::NotAttempted => {}
                        }
                    }
                    self.observe_live();
                    return NotificationTurn::Advanced;
                }
                Poll::Ready(Err(error)) => {
                    self.cleanup_error.get_or_insert_with(|| error.clone());
                    self.fence(error);
                    return NotificationTurn::RetirementBlocked;
                }
                Poll::Pending => return NotificationTurn::Waiting,
            }
        }

        match position {
            Position::Staged => {
                self.staged.take();
            }
            Position::Front => self.pop_front(),
        }
        NotificationTurn::Advanced
    }

    fn cleanup(&mut self, current: bool, cx: &mut Context<'_>) -> NotificationTurn {
        self.request.subscription.0.take();
        let item = if self.staged.is_some() && (!self.queued_next || self.queue.is_empty()) {
            Some(Position::Staged)
        } else {
            self.queue.front().map(|_| Position::Front)
        };
        let turn = match item {
            Some(position) if self.participant.is_none() || !self.close_next => {
                self.close_next = true;
                self.queued_next = matches!(position, Position::Staged);
                let queued = self.queue.len();
                let turn = self.retire_item(position, cx);
                // Fenced data can no longer be exposed. Even a blocked front
                // must give every other queued owner its own retirement turn.
                if matches!(position, Position::Front) && self.queue.len() == queued {
                    self.queue.rotate_left(1);
                }
                turn
            }
            _ if self.participant.is_some() => {
                self.close_next = false;
                self.retire_participation(current, cx)
            }
            _ => NotificationTurn::Advanced,
        };
        if self.participant.is_none() && self.staged.is_none() && self.queue.is_empty() {
            self.queue = VecDeque::new();
            self.state = NotificationState::Closed;
            NotificationTurn::Closed
        } else {
            turn
        }
    }

    fn retire_participation(&mut self, current: bool, cx: &mut Context<'_>) -> NotificationTurn {
        if !self.native_started {
            self.participant.take().unwrap().abort_local();
            self.cancellation.take();
            return NotificationTurn::Advanced;
        }
        if self.cancellation.is_some()
            && !self.cancellation_withdrawn
            && let Err(error) = self.authorize_cancel(current)
        {
            self.cancellation_error.get_or_insert(error);
            // Revoke the loan, keeping its backing with the original close.
            self.cancellation_withdrawn = true;
        }
        let input = if self.cancellation_withdrawn {
            None
        } else {
            self.cancellation.as_ref()
        };
        self.close_started = true;
        match self.participant.as_mut().unwrap().poll_close(cx, input) {
            Poll::Ready(Ok(remote)) => {
                self.remote = remote;
                drop(self.participant.take());
                self.cancellation.take();
                NotificationTurn::Advanced
            }
            Poll::Ready(Err(error)) => {
                self.cleanup_error.get_or_insert(error);
                NotificationTurn::RetirementBlocked
            }
            Poll::Pending => NotificationTurn::Waiting,
        }
    }
}
