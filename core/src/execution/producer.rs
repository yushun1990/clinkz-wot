//! Source-free finite Producer publication gates and original reply retirement.
use crate::{
    CoreError, CoreResult, Deadline, PayloadView,
    execution::{ValueExecution, read::ReadMapping},
};
use clinkz_wot_foundation::MonotonicInstant;
use core::{
    cell::Cell,
    task::{Context, Poll},
};

/// Semantic-effect certainty, independent of the native reply outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FiniteEffect {
    /// The handler was not entered; no application effect was attempted.
    NotInvoked,
    /// The handler was entered. Even an error or invalid/oversized result may
    /// follow a committed write; implicit replay is not safe.
    Unknown,
    /// The Property or root collection write reported complete success and its result
    /// passed validation. Does not imply atomic commit, rollback, per-member
    /// receipts or native reply delivery. Errors after entry remain Unknown.
    Completed,
    /// A validated, incomplete per-member report is retained in the result.
    Reported,
}

/// Whether the original result reached the Binding's declared delivery edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FiniteDelivery {
    Delivered,
    NotDelivered,
    Unknown,
}

/// Both failures retain the original reply. A member failure also withdraws ingress.
#[derive(Debug)]
pub enum ReplyRetirementError {
    Isolated(CoreError),
    Member(CoreError),
}

/// The first interruption and the first failed local disposal are independent evidence.
/// Host and caller-owned exchanges use the same retry decision.
#[derive(Default)]
pub struct ReplyRetirement {
    pub first_cause: Option<CoreError>,
    pub cleanup_error: Option<CoreError>,
}
impl ReplyRetirement {
    pub fn interrupt(&mut self, cause: CoreError) {
        self.first_cause.get_or_insert(cause);
    }
    /// Poll only the original delivery or its retirement. An isolated send
    /// failure starts retirement; a failed retirement keeps its own error.
    /// Only confirmed native disposal returns delivery evidence.
    pub fn poll(
        &mut self,
        native: impl FnOnce(Option<&CoreError>) -> Poll<Result<FiniteDelivery, ReplyRetirementError>>,
    ) -> Poll<CoreResult<FiniteDelivery>> {
        match native(self.first_cause.as_ref()) {
            Poll::Ready(Err(failure)) => {
                let (error, member) = match failure {
                    ReplyRetirementError::Isolated(e) => (e, false),
                    ReplyRetirementError::Member(e) => (e, true),
                };
                if self.first_cause.is_some() {
                    self.cleanup_error.get_or_insert_with(|| error.clone());
                } else {
                    self.interrupt(error.clone());
                }
                if member {
                    Poll::Ready(Err(error))
                } else {
                    Poll::Pending
                }
            }
            Poll::Ready(Ok(delivery)) => Poll::Ready(Ok(delivery)),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum IngressState {
    #[default]
    Staged,
    Open,
    Withdrawn,
}

/// One native participation's passive ingress and retirement signal. A fresh
/// gate waits for local commit. Withdrawal survives registry replacement and
/// is terminal for this native participation.
#[derive(Default)]
pub struct ProducerIngressGate(Cell<IngressState>);

impl ProducerIngressGate {
    pub fn is_staged(&self) -> bool {
        self.0.get() == IngressState::Staged
    }
    pub fn is_open(&self) -> bool {
        self.0.get() == IngressState::Open
    }
    pub fn is_withdrawn(&self) -> bool {
        self.0.get() == IngressState::Withdrawn
    }
    /// Servient opens each prechecked staged gate at its callback-free commit.
    pub fn open(&self) {
        if self.is_staged() {
            self.0.set(IngressState::Open);
        }
    }
    pub fn withdraw(&self) {
        self.0.set(IngressState::Withdrawn);
    }
}

/// A trusted artifact for the complete single-Form description. No source
/// parser is linked. The native program must cover this precise final mapping,
/// value meaning and media at the deployed endpoint.
pub struct PreparedRead<'a, P> {
    pub description: &'a str,
    pub meaning: ValueExecution<'a>,
    pub mapping: ReadMapping<'a>,
    pub program: &'a P,
    pub media: &'a str,
    pub output_bytes: usize,
}

/// The type of scratch is selected by the concrete validator. The native loan
/// remains with its exchange through an isolated cleanup error. All callbacks
/// are bounded, including local Drop of an unfinished reply or closed member.
pub trait ReadMember {
    type Program;
    type Reply;
    type Scratch;
    /// Passive, nonrepeating revision of every mutable coverage/service operand.
    fn revision(&self) -> u64;
    fn covers(&self, definition: &PreparedRead<'_, Self::Program>) -> bool;
    fn check_ingress(&self) -> CoreResult<()>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>>;
    /// Each offered right keeps its native correlation and deadline until consumed
    /// or dropped. The member bounds outstanding rights, including refused offers.
    fn poll_receive(
        &mut self,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> Poll<CoreResult<(Self::Reply, Deadline)>>;
    fn validate(
        &self,
        definition: &PreparedRead<'_, Self::Program>,
        value: PayloadView<'_>,
        scratch: &mut Self::Scratch,
    ) -> CoreResult<()>;
    fn poll_send(
        &mut self,
        reply: &mut Self::Reply,
        value: Result<PayloadView<'_>, &CoreError>,
        cx: &mut Context<'_>,
    ) -> Poll<CoreResult<()>>;
    /// With withdrawn ingress, finish the original local abort even if it
    /// previously required member withdrawal. Success consumes its native right.
    fn poll_retire(
        &mut self,
        reply: &mut Self::Reply,
        cause: &CoreError,
        withdrawn: bool,
        cx: &mut Context<'_>,
    ) -> Poll<Result<FiniteDelivery, ReplyRetirementError>>;
}
