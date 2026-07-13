//! Bounded progress and terminal status values shared by portable drivers.

use core::num::NonZeroU16;

use crate::CoreError;

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProcessEvent<T, D = ()> {
    /// One process item.
    Item(T),
    /// The process terminal result, emitted at most once.
    Terminal(ProcessTerminal<D>),
}

/// Retained terminal result shared by subscriptions and discovery processes.
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

#[cfg(test)]
mod tests {
    use super::{PendingWork, PendingWorkClass, ProcessEvent, ProcessTerminal, StepStatus};

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
}
