//! Caller-owned publication and independent reply slots for one trusted final
//! NoSec Property Read mapping. The deployment supplies the only eligible
//! Binding, the exact definition, buffers and a periodically driven turn domain.
use clinkz_wot_core::{
    AffordanceSite, CoreError, CoreResult, Deadline, ErrorContext, ErrorPhase, Operation,
    PayloadView, RetryClass,
    execution::producer::{
        FiniteDelivery, FiniteEffect, ProducerIngressGate, ReplyRetirement, ReplyRetirementError,
    },
    security::SecurityRequirement,
};
use clinkz_wot_foundation::MonotonicInstant;
use core::{
    cell::{Cell, RefCell, RefMut},
    task::{Context, Poll},
};

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
}
fn stale() -> CoreError {
    CoreError::StaleHandle(context())
}
fn invalid() -> CoreError {
    CoreError::Validation(context())
}
fn busy() -> CoreError {
    CoreError::Backpressure(context())
}
fn check_time(deadline: Deadline, now: MonotonicInstant) -> CoreResult<()> {
    match deadline.checked_is_elapsed_at(now) {
        Some(false) => Ok(()),
        Some(true) => Err(CoreError::TimedOut(context())),
        None => Err(invalid()),
    }
}

/// One local drive domain. All sibling slots in a manual deployment borrow it.
/// Nested calls defer progress. Revocation needs no turn.
#[derive(Default)]
pub struct ProducerTurns(Cell<bool>);
struct Turn<'a>(&'a Cell<bool>);
impl ProducerTurns {
    fn enter(&self) -> Option<Turn<'_>> {
        if self.0.replace(true) {
            None
        } else {
            Some(Turn(&self.0))
        }
    }
}
impl Drop for Turn<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// Storage of the current incarnation. Keys borrow this exact slot, even after
/// replacement; a recycled index cannot authorize an earlier caller.
pub struct PublicationSlot<'a> {
    turns: &'a ProducerTurns,
    current: Cell<Option<u64>>,
    sequence: Cell<u64>,
}
impl<'a> PublicationSlot<'a> {
    pub fn new(turns: &'a ProducerTurns) -> Self {
        Self {
            turns,
            current: Cell::new(None),
            sequence: Cell::new(0),
        }
    }
}
#[derive(Clone, Copy)]
pub struct PublicationKey<'a> {
    slot: &'a PublicationSlot<'a>,
    incarnation: u64,
}
impl PartialEq for PublicationKey<'_> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.slot, other.slot) && self.incarnation == other.incarnation
    }
}
impl Eq for PublicationKey<'_> {}
impl PublicationKey<'_> {
    pub fn is_current(self) -> bool {
        self.slot.current.get() == Some(self.incarnation)
    }
    fn revoke(self) {
        if self.is_current() {
            self.slot.current.set(None);
        }
    }
}

#[derive(Clone, Copy)]
pub struct ReadRevocation<'a>(PublicationKey<'a>);
impl ReadRevocation<'_> {
    pub fn revoke(self) {
        self.0.revoke();
    }
}

pub use clinkz_wot_core::execution::producer::{PreparedRead, ReadMember};
pub trait ReadBehavior {
    /// Passive, nonrepeating revision of application permission.
    fn revision(&self) -> u64 {
        0
    }
    fn authorize(&self, site: AffordanceSite<'_>) -> CoreResult<()>;
    fn read(&self, site: AffordanceSite<'_>, output: &mut [u8]) -> CoreResult<usize>;
}

pub struct ReadOffer<'a, R> {
    original: PublicationKey<'a>,
    reply: R,
    deadline: Deadline,
}
impl<R> ReadOffer<'_, R> {
    pub fn deadline(&self) -> Deadline {
        self.deadline
    }
    pub fn into_reply(self) -> R {
        self.reply
    }
}
#[derive(Clone, Copy)]
pub struct ExchangeKey<'a> {
    original: PublicationKey<'a>,
    index: usize,
    sequence: u64,
}

pub struct ReadSettlement<'a> {
    original: PublicationKey<'a>,
    output: RefMut<'a, [u8]>,
    length: CoreResult<usize>,
    retirement: ReplyRetirement,
    delivery: FiniteDelivery,
}
impl ReadSettlement<'_> {
    pub fn recipient(&self) -> PublicationKey<'_> {
        self.original
    }
    pub fn result(&self) -> Result<&[u8], &CoreError> {
        self.length.as_ref().map(|&n| &self.output[..n])
    }
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.retirement.first_cause.as_ref()
    }
    pub fn cleanup_error(&self) -> Option<&CoreError> {
        self.retirement.cleanup_error.as_ref()
    }
    /// Each settlement follows a handler entry. Its side effects remain unknown.
    pub fn effect(&self) -> FiniteEffect {
        FiniteEffect::Unknown
    }
    pub fn delivery(&self) -> FiniteDelivery {
        self.delivery
    }
}

pub struct ReadSlot<'a, B: ReadMember> {
    scratch: B::Scratch,
    active: Option<ReadExchange<'a, B>>,
}
impl<B: ReadMember> ReadSlot<'_, B> {
    pub fn new(scratch: B::Scratch) -> Self {
        Self {
            scratch,
            active: None,
        }
    }
}
struct ReadExchange<'a, B: ReadMember> {
    reply: B::Reply,
    deadline: Deadline,
    sequence: u64,
    outcome: ReadSettlement<'a>,
}

/// The deployment lends actual active records. A returned result only borrows
/// the original output backing and logical key, independently of this owner.
pub struct ReadPublication<'a, 'slots, B: ReadMember> {
    member: Option<B>,
    definition: &'a PreparedRead<'a, B::Program>,
    behavior: &'a dyn ReadBehavior,
    key: PublicationKey<'a>,
    expected: Option<u64>,
    gate: ProducerIngressGate,
    revision: u64,
    failure: Option<CoreError>,
    active: &'slots mut [ReadSlot<'a, B>],
    next: u64,
    deadline: Deadline,
    last_now: Option<MonotonicInstant>,
}

impl<'a, 'slots, B: ReadMember> ReadPublication<'a, 'slots, B> {
    pub fn stage(
        slot: &'a PublicationSlot<'a>,
        replaces: Option<PublicationKey<'a>>,
        definition: &'a PreparedRead<'a, B::Program>,
        member: B,
        behavior: &'a dyn ReadBehavior,
        active: &'slots mut [ReadSlot<'a, B>],
        deadline: Deadline,
    ) -> Result<Self, (B, CoreError)> {
        let meaning = definition.meaning;
        if active.is_empty()
            || active.iter().any(|s| s.active.is_some())
            || definition.description.is_empty()
            || definition.media.is_empty()
            || meaning.operation() != Operation::ReadProperty
            || definition.output_bytes == 0
            || *meaning.security() != SecurityRequirement::NOSEC
            || meaning.uri_variables().is_some()
            || meaning.responses().is_some()
        {
            return Err((member, invalid()));
        }
        if replaces.is_some_and(|k| !core::ptr::eq(k.slot, slot))
            || replaces.map(|k| k.incarnation) != slot.current.get()
        {
            return Err((member, stale()));
        }
        let Some(incarnation) = slot.sequence.get().checked_add(1) else {
            return Err((member, busy()));
        };
        slot.sequence.set(incarnation);
        let revision = member.revision();
        Ok(Self {
            member: Some(member),
            definition,
            behavior,
            key: PublicationKey { slot, incarnation },
            expected: slot.current.get(),
            gate: ProducerIngressGate::default(),
            revision,
            failure: None,
            active,
            next: 0,
            deadline,
            last_now: None,
        })
    }
    pub fn key(&self) -> PublicationKey<'a> {
        self.key
    }
    pub fn revocation(&self) -> Option<ReadRevocation<'a>> {
        self.key.is_current().then_some(ReadRevocation(self.key))
    }
    pub fn description(&self) -> Option<&'a str> {
        (self.key.is_current() && self.gate.is_open()).then_some(self.definition.description)
    }
    pub fn failure(&self) -> Option<&CoreError> {
        self.failure.as_ref()
    }
    fn fence(&mut self, cause: CoreError) {
        if self.failure.is_none() {
            for exchange in self
                .active
                .iter_mut()
                .filter_map(|slot| slot.active.as_mut())
            {
                exchange.outcome.retirement.interrupt(cause.clone());
            }
            self.failure = Some(cause);
        }
        self.gate.withdraw();
        self.key.revoke();
    }
    pub fn close(&mut self) {
        let staged = self.gate.is_staged();
        self.fence(CoreError::Cancelled(context()));
        if staged {
            self.member.take();
        }
    }
    fn observe_time(&mut self, now: MonotonicInstant) -> CoreResult<()> {
        if self
            .last_now
            .is_some_and(|last| now.checked_duration_since(last).is_none())
        {
            return Err(invalid());
        }
        self.last_now = Some(now);
        Ok(())
    }
    pub fn poll_publish(
        &mut self,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> Poll<CoreResult<()>> {
        if let Some(error) = &self.failure {
            return Poll::Ready(Err(error.clone()));
        }
        if !self.gate.is_staged() {
            return Poll::Ready(Err(stale()));
        }
        let Some(_turn) = self.key.slot.turns.enter() else {
            return Poll::Pending;
        };
        let result = (|| {
            check_time(self.deadline, now)?;
            self.observe_time(now)?;
            let member = self.member.as_mut().unwrap();
            if member.revision() != self.revision || self.key.slot.current.get() != self.expected {
                return Poll::Ready(Err(stale()));
            }
            if member.poll_ready(cx)?.is_pending() {
                return Poll::Pending;
            }
            if !member.covers(self.definition) {
                return Poll::Ready(Err(invalid()));
            }
            member.check_ingress()?;
            if member.revision() != self.revision || self.key.slot.current.get() != self.expected {
                return Poll::Ready(Err(stale()));
            }
            crate::commit_producer_gates(core::iter::once(&self.gate), || {
                self.key.slot.current.set(Some(self.key.incarnation))
            })?;
            Poll::Ready(Ok(()))
        })();
        if let Poll::Ready(Err(error)) = &result {
            self.fence(error.clone());
            self.member.take();
        }
        result
    }
    fn health(&mut self) -> CoreResult<()> {
        let member = self.member.as_ref().ok_or_else(stale)?;
        let result = member.check_ingress().and_then(|_| {
            if member.revision() == self.revision {
                Ok(())
            } else {
                Err(stale())
            }
        });
        if let Err(error) = &result {
            self.fence(error.clone());
        }
        result
    }
    fn current(&self) -> CoreResult<()> {
        if self.key.is_current() && self.gate.is_open() {
            Ok(())
        } else {
            Err(stale())
        }
    }
    pub fn poll_offer(
        &mut self,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> Poll<CoreResult<ReadOffer<'a, B::Reply>>> {
        let Some(_turn) = self.key.slot.turns.enter() else {
            return Poll::Pending;
        };
        self.observe_time(now)?;
        self.current()?;
        self.health()?;
        self.current()?;
        match self.member.as_mut().unwrap().poll_receive(cx, now) {
            Poll::Ready(Ok((reply, deadline))) => Poll::Ready(Ok(ReadOffer {
                original: self.key,
                reply,
                deadline,
            })),
            Poll::Ready(Err(error)) => {
                self.fence(error.clone());
                Poll::Ready(Err(error))
            }
            Poll::Pending => Poll::Pending,
        }
    }
    /// Refusal returns the complete original offer. The output cell is lent
    /// only for accepted work; dropping the settlement releases that exact cell.
    #[allow(clippy::type_complexity)]
    pub fn accept(
        &mut self,
        offer: ReadOffer<'a, B::Reply>,
        output: &'a RefCell<[u8]>,
        now: MonotonicInstant,
    ) -> Result<ExchangeKey<'a>, (ReadOffer<'a, B::Reply>, CoreError)> {
        let Some(_turn) = self.key.slot.turns.enter() else {
            return Err((offer, busy()));
        };
        let admission = (|| {
            if offer.original != self.key {
                return Err(stale());
            }
            self.observe_time(now)?;
            self.current()?;
            check_time(offer.deadline, now)?;
            self.health()?;
            let index = self
                .active
                .iter()
                .position(|s| s.active.is_none())
                .ok_or_else(busy)?;
            let sequence = self.next.checked_add(1).ok_or_else(busy)?;
            let buffer = output.try_borrow_mut().map_err(|_| busy())?;
            if buffer.len() < self.definition.output_bytes {
                return Err(invalid());
            }
            self.permission()?;
            Ok((index, sequence, buffer))
        })();
        let (index, sequence, mut output) = match admission {
            Ok(v) => v,
            Err(error) => return Err((offer, error)),
        };
        self.next = sequence;
        let bytes = &mut output[..self.definition.output_bytes];
        let length = self
            .behavior
            .read(self.definition.meaning.site(), bytes)
            .and_then(|n| {
                let value = bytes.get(..n).ok_or_else(invalid)?;
                self.member.as_ref().unwrap().validate(
                    self.definition,
                    payload(self.definition.media, value),
                    &mut self.active[index].scratch,
                )?;
                Ok(n)
            });
        self.active[index].active = Some(ReadExchange {
            reply: offer.reply,
            deadline: offer.deadline,
            sequence,
            outcome: ReadSettlement {
                original: self.key,
                output,
                length,
                retirement: ReplyRetirement::default(),
                delivery: FiniteDelivery::Unknown,
            },
        });
        Ok(ExchangeKey {
            original: self.key,
            index,
            sequence,
        })
    }
    fn permission(&mut self) -> CoreResult<()> {
        let revision = self.behavior.revision();
        self.behavior.authorize(self.definition.meaning.site())?;
        self.health()?;
        if self.behavior.revision() != revision {
            return Err(stale());
        }
        self.current()
    }
    fn index(&self, key: ExchangeKey<'_>) -> CoreResult<usize> {
        if key.original != self.key
            || self
                .active
                .get(key.index)
                .and_then(|s| s.active.as_ref())
                .is_none_or(|e| e.sequence != key.sequence)
        {
            return Err(stale());
        }
        Ok(key.index)
    }
    /// Recover the original keys for manual drain if an application drops a handle.
    pub fn pending_exchanges(&self) -> impl Iterator<Item = ExchangeKey<'a>> + '_ {
        self.active.iter().enumerate().filter_map(|(index, slot)| {
            slot.active.as_ref().map(|exchange| ExchangeKey {
                original: self.key,
                index,
                sequence: exchange.sequence,
            })
        })
    }
    /// Fence ingress immediately and release the member once its last original
    /// reply has retired. Outstanding settlements hold only their output loans.
    pub fn retire(&mut self) -> CoreResult<()> {
        self.close();
        if self.active.iter().any(|s| s.active.is_some()) {
            return Err(busy());
        }
        self.member.take();
        Ok(())
    }
    pub fn retirement(&self, key: ExchangeKey<'_>) -> CoreResult<&ReplyRetirement> {
        Ok(&self.active[self.index(key)?]
            .active
            .as_ref()
            .unwrap()
            .outcome
            .retirement)
    }
    pub fn poll_exchange(
        &mut self,
        key: ExchangeKey<'_>,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> Poll<CoreResult<ReadSettlement<'a>>> {
        let index = self.index(key)?;
        let Some(_turn) = self.key.slot.turns.enter() else {
            return Poll::Pending;
        };
        let clock = self.observe_time(now);
        let slot = &mut self.active[index];
        let exchange = slot.active.as_mut().unwrap();
        if let Err(error) = clock {
            exchange.outcome.retirement.interrupt(error);
        }
        if let Err(error) = check_time(exchange.deadline, now) {
            exchange.outcome.retirement.interrupt(error);
        }
        let health = self.health().and_then(|_| self.current());
        let slot = &mut self.active[index];
        let exchange = slot.active.as_mut().unwrap();
        if let Err(error) = health {
            exchange.outcome.retirement.interrupt(error);
        }
        if exchange.outcome.retirement.first_cause.is_none() {
            let verified = match exchange.outcome.result() {
                Ok(bytes) => self.member.as_ref().unwrap().validate(
                    self.definition,
                    payload(self.definition.media, bytes),
                    &mut slot.scratch,
                ),
                Err(_) => Ok(()),
            }
            .and_then(|_| self.permission());
            if let Err(error) = verified {
                self.active[index]
                    .active
                    .as_mut()
                    .unwrap()
                    .outcome
                    .retirement
                    .interrupt(error);
            }
        }
        let slot = &mut self.active[index];
        let exchange = slot.active.as_mut().unwrap();
        let member = self.member.as_mut().unwrap();
        let value = exchange
            .outcome
            .length
            .as_ref()
            .map(|&n| payload(self.definition.media, &exchange.outcome.output[..n]));
        let retired = exchange.outcome.retirement.poll(|cause| match cause {
            Some(cause) => {
                member.poll_retire(&mut exchange.reply, cause, !self.key.is_current(), cx)
            }
            None => member
                .poll_send(&mut exchange.reply, value, cx)
                .map(|result| {
                    result
                        .map(|()| FiniteDelivery::Delivered)
                        .map_err(ReplyRetirementError::Isolated)
                }),
        });
        match retired {
            Poll::Ready(Ok(delivery)) => {
                let mut exchange = self.active[index].active.take().unwrap();
                exchange.outcome.delivery = delivery;
                drop(exchange.reply);
                Poll::Ready(Ok(exchange.outcome))
            }
            Poll::Ready(Err(error)) => {
                self.fence(error);
                Poll::Pending
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
impl<B: ReadMember> Drop for ReadPublication<'_, '_, B> {
    fn drop(&mut self) {
        self.key.revoke();
        self.gate.withdraw();
        for slot in self.active.iter_mut() {
            drop(slot.active.take());
        }
    }
}
fn payload<'a>(media: &'a str, body: &'a [u8]) -> PayloadView<'a> {
    PayloadView {
        body,
        content_type: media,
        content_coding: None,
    }
}
