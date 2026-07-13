//! Bounded progress, cleanup, and terminal status values shared by portable drivers.

use core::fmt;
use core::num::NonZeroU16;

use clinkz_wot_foundation::{ClockId, MonotonicInstant};

use crate::CoreError;
use crate::error::RetryClass;
use crate::identity::{
    BindingGeneration, BindingId, BindingSlotId, CleanupSlotId, PlanId, ThingSlotId,
};

/// One maintained class of work already known to be pending.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum PendingWorkClass {
    /// Input already queued by a binding.
    BindingInput = 1 << 0,
    /// A response awaiting delivery progress.
    ResponseDelivery = 1 << 1,
    /// An outbound request awaiting binding progress.
    OutboundRequest = 1 << 2,
    /// Subscription data already queued or ready.
    SubscriptionData = 1 << 3,
    /// A maintained timer known to be due.
    Timer = 1 << 4,
    /// General cleanup work already queued.
    Cleanup = 1 << 5,
    /// Producer emission fan-out with a retained cursor.
    EmissionFanOut = 1 << 6,
    /// Binding publication with a retained target cursor.
    BindingPublication = 1 << 7,
    /// Subscription cancellation awaiting progress.
    SubscriptionCancellation = 1 << 8,
    /// Prepared-route readiness awaiting progress.
    RouteReadiness = 1 << 9,
    /// Prepared or active route cleanup awaiting progress.
    RouteCleanup = 1 << 10,
}

/// A bounded, nonempty summary of maintained ready work.
///
/// Construction requires at least one [`PendingWorkClass`]. The summary is
/// populated from queues, cursors, and slot state; callers must not scan all
/// runtime tables merely to construct it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PendingWork(NonZeroU16);

impl PendingWork {
    /// Creates a nonempty summary containing one class.
    pub const fn new(class: PendingWorkClass) -> Self {
        match NonZeroU16::new(class as u16) {
            Some(bits) => Self(bits),
            None => unreachable!(),
        }
    }

    /// Returns a summary that also contains `class`.
    #[must_use]
    pub const fn with(mut self, class: PendingWorkClass) -> Self {
        self.0 = match NonZeroU16::new(self.0.get() | class as u16) {
            Some(bits) => bits,
            None => unreachable!(),
        };
        self
    }

    /// Returns whether this summary contains `class`.
    pub const fn contains(self, class: PendingWorkClass) -> bool {
        self.0.get() & class as u16 != 0
    }

    /// Returns the stable nonzero bit representation.
    pub const fn bits(self) -> NonZeroU16 {
        self.0
    }
}

impl From<PendingWorkClass> for PendingWork {
    fn from(class: PendingWorkClass) -> Self {
        Self::new(class)
    }
}

/// Result of starting a bounded operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartStatus<T> {
    /// The operation completed synchronously.
    Ready(T),
    /// The initialized operation slot owns pending work.
    Pending,
}

/// One item or the retained terminal result of a process.
///
/// The terminal value remains inline because the frozen portable schema cannot
/// require allocation or indirection in `no_std` builds.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProcessEvent<T, D = ()> {
    /// One process item.
    Item(T),
    /// The process terminal result, emitted at most once.
    Terminal(ProcessTerminal<D>),
}

/// Retained terminal result shared by subscriptions and discovery processes.
///
/// [`CoreError`] is intentionally inline in the exact frozen schema, so this
/// enum may be larger than its non-failure variants.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProcessTerminal<D = ()> {
    /// The process completed normally.
    Completed,
    /// The process was cancelled and cleanup reached a terminal state.
    Cancelled,
    /// The process deadline expired.
    TimedOut,
    /// A bounded buffer overflow terminated the process.
    Overflowed {
        /// Number of lost items when it is safely known.
        lost: Option<u64>,
    },
    /// A process-specific terminal result.
    Domain(D),
    /// A structured engine failure.
    Failed(CoreError),
}

/// Outcome of one manually driven runtime step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepStatus<T> {
    /// No transition, value, or maintained pending work was observed.
    Idle,
    /// At least one transition occurred, a value was produced, or known work remains.
    Progress {
        /// At most one value produced by this step.
        value: Option<T>,
        /// Nonempty maintained work summary when known work remains.
        pending: Option<PendingWork>,
    },
    /// The driven facade itself reached its terminal state.
    Terminal(T),
}

impl<T> StepStatus<T> {
    /// Creates progress without requiring either optional field to be present.
    pub const fn progress(value: Option<T>, pending: Option<PendingWork>) -> Self {
        Self::Progress { value, pending }
    }
}

/// Bounded cleanup action retained by a cleanup owner.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
#[repr(u8)]
pub enum CleanupOperation {
    /// Cancel a prepared-route readiness operation.
    CancelRouteReadiness,
    /// Abort a prepared route before publication.
    AbortPreparedRoute,
    /// Shut down an active, committed, serving, or draining route.
    ShutdownRoute,
    /// Cancel an outbound request that has not reached a terminal state.
    CancelRequest,
    /// Cancel a subscription before its guard becomes public.
    CancelSubscriptionStart,
    /// Stop an active subscription and release its guard.
    StopSubscription,
    /// Cancel an in-progress response delivery.
    CancelResponseDelivery,
    /// Cancel an in-progress producer emission.
    CancelEmission,
    /// Cancel another retained process, such as Discovery or Directory work.
    CancelProcess,
}

/// Generation-bearing identity of one retained cleanup record.
///
/// The handle contains no cleanup plan or guard. A runtime resolves it against
/// its bounded cleanup table and rejects it after the slot generation changes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CleanupHandle(CleanupSlotId);

impl CleanupHandle {
    /// Creates a cleanup handle for a reserved cleanup record slot.
    pub const fn new(record_id: CleanupSlotId) -> Self {
        Self(record_id)
    }

    /// Returns the cleanup record slot identity.
    pub const fn record_id(self) -> CleanupSlotId {
        self.0
    }
}

impl From<CleanupSlotId> for CleanupHandle {
    fn from(record_id: CleanupSlotId) -> Self {
        Self::new(record_id)
    }
}

impl From<CleanupHandle> for CleanupSlotId {
    fn from(handle: CleanupHandle) -> Self {
        handle.record_id()
    }
}

impl fmt::Display for CleanupHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Bounded, redacted record for cleanup retained by a runtime owner.
///
/// The record contains only numeric or generation-bearing references. Route
/// guards, teardown plans, payloads, credentials, Thing descriptions, URIs,
/// security expressions, diagnostic names, and error chains remain in their
/// owning arenas. `status_code` is an application- or binding-defined redacted
/// code and must not require retention of its textual cause.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CleanupRecord {
    record_identity: CleanupHandle,
    subject: CleanupSlotId,
    owner: CleanupSlotId,
    thing: Option<ThingSlotId>,
    binding: Option<BindingSlotId>,
    binding_registration: Option<(BindingId, BindingGeneration)>,
    plan: Option<PlanId>,
    operation: CleanupOperation,
    deadline: Option<MonotonicInstant>,
    retry_not_before: Option<MonotonicInstant>,
    retry_attempts: u16,
    retry_class: RetryClass,
    status_code: u16,
}

const _: () = assert!(core::mem::size_of::<CleanupRecord>() <= 128);

impl CleanupRecord {
    /// Creates a cleanup record when its retry count is within the explicit limit.
    ///
    /// `cleanup_retry_attempts_max` is the applicable resource-profile value.
    /// Zero permits the initial cleanup attempt but rejects a retained retry
    /// count greater than zero.
    #[allow(clippy::too_many_arguments)]
    pub const fn try_new(
        record_identity: CleanupHandle,
        subject: CleanupSlotId,
        owner: CleanupSlotId,
        operation: CleanupOperation,
        retry_attempts: u16,
        retry_class: RetryClass,
        status_code: u16,
        cleanup_retry_attempts_max: u64,
    ) -> Option<Self> {
        if retry_attempts as u64 > cleanup_retry_attempts_max {
            return None;
        }

        Some(Self {
            record_identity,
            subject,
            owner,
            thing: None,
            binding: None,
            binding_registration: None,
            plan: None,
            operation,
            deadline: None,
            retry_not_before: None,
            retry_attempts,
            retry_class,
            status_code,
        })
    }

    /// Returns a record with optional bounded diagnostic identities attached.
    #[must_use]
    pub const fn with_diagnostic_identities(
        mut self,
        thing: Option<ThingSlotId>,
        binding: Option<BindingSlotId>,
        binding_registration: Option<(BindingId, BindingGeneration)>,
        plan: Option<PlanId>,
    ) -> Self {
        self.thing = thing;
        self.binding = binding;
        self.binding_registration = binding_registration;
        self.plan = plan;
        self
    }

    /// Sets deadline and retry timing after validating the runtime clock.
    ///
    /// Both instants, when present, must use `runtime_clock_id`. A retry instant
    /// later than a present terminal deadline is rejected.
    pub fn try_with_timing(
        mut self,
        runtime_clock_id: ClockId,
        deadline: Option<MonotonicInstant>,
        retry_not_before: Option<MonotonicInstant>,
    ) -> Option<Self> {
        if deadline.is_some_and(|instant| instant.clock_id() != runtime_clock_id)
            || retry_not_before.is_some_and(|instant| instant.clock_id() != runtime_clock_id)
        {
            return None;
        }

        if let (Some(deadline), Some(retry_not_before)) = (deadline, retry_not_before)
            && retry_not_before.ticks() > deadline.ticks()
        {
            return None;
        }

        self.deadline = deadline;
        self.retry_not_before = retry_not_before;
        Some(self)
    }

    /// Updates the retry count when it remains within the explicit limit.
    pub const fn try_with_retry_attempts(
        mut self,
        retry_attempts: u16,
        cleanup_retry_attempts_max: u64,
    ) -> Option<Self> {
        if retry_attempts as u64 > cleanup_retry_attempts_max {
            return None;
        }
        self.retry_attempts = retry_attempts;
        Some(self)
    }

    /// Returns the cleanup record identity.
    pub const fn record_identity(self) -> CleanupHandle {
        self.record_identity
    }

    /// Returns the generation-bearing subject identity.
    pub const fn subject(self) -> CleanupSlotId {
        self.subject
    }

    /// Returns the generation-bearing cleanup owner identity.
    pub const fn owner(self) -> CleanupSlotId {
        self.owner
    }

    /// Returns the optional generation-bearing Thing diagnostic identity.
    pub const fn thing(self) -> Option<ThingSlotId> {
        self.thing
    }

    /// Returns the optional generation-bearing binding diagnostic identity.
    pub const fn binding(self) -> Option<BindingSlotId> {
        self.binding
    }

    /// Returns the optional binding registration identity and generation.
    pub const fn binding_registration(self) -> Option<(BindingId, BindingGeneration)> {
        self.binding_registration
    }

    /// Returns the optional immutable plan diagnostic identity.
    pub const fn plan(self) -> Option<PlanId> {
        self.plan
    }

    /// Returns the retained cleanup action.
    pub const fn operation(self) -> CleanupOperation {
        self.operation
    }

    /// Returns the terminal cleanup deadline when configured.
    pub const fn deadline(self) -> Option<MonotonicInstant> {
        self.deadline
    }

    /// Returns the earliest instant at which the next retry may run.
    pub const fn retry_not_before(self) -> Option<MonotonicInstant> {
        self.retry_not_before
    }

    /// Returns the number of cleanup retries already attempted.
    pub const fn retry_attempts(self) -> u16 {
        self.retry_attempts
    }

    /// Returns the retry advice retained for this cleanup item.
    pub const fn retry_class(self) -> RetryClass {
        self.retry_class
    }

    /// Returns the bounded, redacted status code.
    pub const fn status_code(self) -> u16 {
        self.status_code
    }
}

/// Result of one bounded cleanup-progress operation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CleanupOutcome {
    /// Engine-local and external cleanup completed.
    Complete,
    /// Cleanup ownership transferred to the owner named by the record.
    PendingCleanup(CleanupRecord),
    /// Engine-local cleanup completed but externally observable residue remains.
    ResidualExternalState(CleanupRecord),
}

impl CleanupOutcome {
    /// Returns the retained cleanup record for pending or residual outcomes.
    pub const fn record(&self) -> Option<&CleanupRecord> {
        match self {
            Self::Complete => None,
            Self::PendingCleanup(record) | Self::ResidualExternalState(record) => Some(record),
        }
    }

    /// Returns whether all engine-local and external cleanup completed.
    pub const fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }

    /// Returns whether cleanup remains owned by a runtime queue.
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::PendingCleanup(_))
    }

    /// Returns whether engine-local cleanup terminated with external residue.
    pub const fn is_residual(&self) -> bool {
        matches!(self, Self::ResidualExternalState(_))
    }
}

#[cfg(test)]
mod tests {
    use core::mem::size_of;

    use clinkz_wot_foundation::{ClockId, Generation, MonotonicInstant, SlotIndex};

    use super::{
        CleanupHandle, CleanupOperation, CleanupOutcome, CleanupRecord, PendingWork,
        PendingWorkClass, ProcessEvent, ProcessTerminal, StepStatus,
    };
    use crate::error::RetryClass;
    use crate::identity::{
        BindingGeneration, BindingId, BindingSlotId, CleanupSlotId, PlanId, ThingSlotId,
    };

    const fn generation(value: u32) -> Generation {
        match Generation::new(value) {
            Some(generation) => generation,
            None => panic!("the test generation must be nonzero"),
        }
    }

    const fn slot(value: u32) -> SlotIndex {
        SlotIndex::new(value)
    }

    fn cleanup_record() -> CleanupRecord {
        CleanupRecord::try_new(
            CleanupHandle::new(CleanupSlotId::new(slot(1), generation(2))),
            CleanupSlotId::new(slot(3), generation(4)),
            CleanupSlotId::new(slot(5), generation(6)),
            CleanupOperation::ShutdownRoute,
            2,
            RetryClass::Safe,
            41,
            16,
        )
        .expect("retry attempts are within the explicit limit")
        .with_diagnostic_identities(
            Some(ThingSlotId::new(slot(7), generation(8))),
            Some(BindingSlotId::new(slot(9), generation(10))),
            Some((BindingId::new(11), BindingGeneration::new(generation(12)))),
            Some(PlanId::new(slot(13), generation(14))),
        )
        .try_with_timing(
            ClockId::new(15),
            Some(MonotonicInstant::new(ClockId::new(15), 20)),
            Some(MonotonicInstant::new(ClockId::new(15), 18)),
        )
        .expect("timing uses one clock and retry precedes the deadline")
    }

    #[test]
    fn pending_work_is_nonempty_and_composable() {
        let pending = PendingWork::new(PendingWorkClass::ResponseDelivery)
            .with(PendingWorkClass::RouteCleanup);
        assert!(pending.contains(PendingWorkClass::ResponseDelivery));
        assert!(pending.contains(PendingWorkClass::RouteCleanup));
        assert!(!pending.contains(PendingWorkClass::Timer));
        assert_ne!(pending.bits().get(), 0);
    }

    #[test]
    fn progress_can_report_value_and_pending_together() {
        let status = StepStatus::progress(
            Some(7),
            Some(PendingWork::new(PendingWorkClass::EmissionFanOut)),
        );
        assert!(matches!(
            status,
            StepStatus::Progress {
                value: Some(7),
                pending: Some(_)
            }
        ));
        assert_eq!(
            StepStatus::<u8>::progress(None, None),
            StepStatus::Progress {
                value: None,
                pending: None,
            }
        );
    }

    #[test]
    fn process_terminal_is_carried_inside_the_event() {
        let event = ProcessEvent::<(), u8>::Terminal(ProcessTerminal::Overflowed { lost: Some(3) });
        assert_eq!(
            event,
            ProcessEvent::Terminal(ProcessTerminal::Overflowed { lost: Some(3) })
        );
    }

    #[test]
    fn cleanup_record_preserves_the_exact_bounded_schema() {
        let record = cleanup_record();

        assert_eq!(record.record_identity().record_id().slot(), slot(1));
        assert_eq!(record.subject().generation(), generation(4));
        assert_eq!(record.owner().generation(), generation(6));
        assert_eq!(record.thing().map(ThingSlotId::slot), Some(slot(7)));
        assert_eq!(record.binding().map(BindingSlotId::slot), Some(slot(9)));
        assert_eq!(
            record.binding_registration(),
            Some((BindingId::new(11), BindingGeneration::new(generation(12))))
        );
        assert_eq!(record.plan().map(PlanId::slot), Some(slot(13)));
        assert_eq!(record.operation(), CleanupOperation::ShutdownRoute);
        assert_eq!(record.deadline().map(MonotonicInstant::ticks), Some(20));
        assert_eq!(
            record.retry_not_before().map(MonotonicInstant::ticks),
            Some(18)
        );
        assert_eq!(record.retry_attempts(), 2);
        assert_eq!(record.retry_class(), RetryClass::Safe);
        assert_eq!(record.status_code(), 41);
    }

    #[test]
    fn cleanup_record_rejects_attempts_above_the_explicit_limit() {
        let record_identity = CleanupHandle::new(CleanupSlotId::new(slot(1), generation(1)));
        let subject = CleanupSlotId::new(slot(2), generation(1));
        let owner = CleanupSlotId::new(slot(3), generation(1));

        assert_eq!(
            CleanupRecord::try_new(
                record_identity,
                subject,
                owner,
                CleanupOperation::CancelProcess,
                1,
                RetryClass::CallerDecision,
                0,
                0,
            ),
            None
        );
        let initial = CleanupRecord::try_new(
            record_identity,
            subject,
            owner,
            CleanupOperation::CancelProcess,
            0,
            RetryClass::CallerDecision,
            0,
            0,
        )
        .expect("zero disables retry but permits the initial cleanup attempt");
        assert_eq!(initial.try_with_retry_attempts(5, 4), None);
        assert!(initial.try_with_retry_attempts(4, 4).is_some());
    }

    #[test]
    fn cleanup_record_rejects_invalid_clock_or_retry_order() {
        let record = CleanupRecord::try_new(
            CleanupHandle::new(CleanupSlotId::new(slot(1), generation(1))),
            CleanupSlotId::new(slot(2), generation(1)),
            CleanupSlotId::new(slot(3), generation(1)),
            CleanupOperation::AbortPreparedRoute,
            0,
            RetryClass::Never,
            0,
            16,
        )
        .expect("the retry count is valid");
        let runtime_clock = ClockId::new(7);

        assert_eq!(
            record.try_with_timing(
                runtime_clock,
                Some(MonotonicInstant::new(ClockId::new(8), 20)),
                None,
            ),
            None
        );
        assert_eq!(
            record.try_with_timing(
                runtime_clock,
                Some(MonotonicInstant::new(runtime_clock, 20)),
                Some(MonotonicInstant::new(ClockId::new(8), 18)),
            ),
            None
        );
        assert_eq!(
            record.try_with_timing(
                runtime_clock,
                Some(MonotonicInstant::new(runtime_clock, 20)),
                Some(MonotonicInstant::new(runtime_clock, 21)),
            ),
            None
        );
    }

    #[test]
    fn cleanup_values_are_copyable_bounded_and_terminal_bearing() {
        const fn assert_copy<T: Copy>() {}

        assert_copy::<CleanupHandle>();
        assert_copy::<CleanupRecord>();
        assert_copy::<CleanupOutcome>();
        assert!(size_of::<CleanupRecord>() <= 128);

        let record = cleanup_record();
        let pending = CleanupOutcome::PendingCleanup(record);
        let residual = CleanupOutcome::ResidualExternalState(record);
        assert!(CleanupOutcome::Complete.is_complete());
        assert_eq!(CleanupOutcome::Complete.record(), None);
        assert!(pending.is_pending());
        assert_eq!(pending.record(), Some(&record));
        assert!(residual.is_residual());
        assert_eq!(residual.record(), Some(&record));
    }
}
