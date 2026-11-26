//! Optional finite work capacity, independent of persistent native participation.
use super::*;
use clinkz_wot_core::host_accounts::{HostCapacityReservation, HostResourceAccounts};
use clinkz_wot_foundation::ResourceKind as K;
use core::cell::RefCell;

pub(super) const MAX_WORK: usize = 32;

#[derive(Default)]
pub(super) struct ProducerFiniteQueue {
    head: Option<ProducerFiniteSettlement>,
    len: usize,
}
impl ProducerFiniteQueue {
    pub(super) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(super) fn insertion_work(&self) -> u64 {
        // One visit per unread link, then one insertion. Active work already
        // holds retained credit, so len + 1 is within the capacity's limit.
        self.len as u64 + 1
    }
}

/// A caller-provisioned work bank shared by finite Producer participants of one
/// logical owner. Clones grant capacity access, never publication or dispatch.
/// Active work and retained outcomes have separate limits; retained includes
/// active, unread and acknowledged-but-still-owned outcomes.
#[derive(Clone)]
pub struct ProducerFiniteCapacity(Rc<CapacityState>);

struct CapacityState {
    issuer: Weak<()>,
    limits: RawFiniteLimits,
    active_limit: usize,
    retained_limit: usize,
    active: Cell<usize>,
    // Boxes have stable addresses and move between this list, a native turn,
    // the unread queue and the application. The free list holds no pool references.
    #[allow(clippy::vec_box)]
    free: RefCell<Vec<Box<ResultRecord>>>,
    sequence: Cell<u64>,
    shared_results: Option<Arc<HostResourceAccounts>>,
}

impl ProducerFiniteCapacity {
    /// Limit active exchanges and retained results independently. A deployment
    /// sharing its outstanding-handler quota with other callers supplies that
    /// same bank. Its single permit follows the result through disposal.
    pub fn try_new(
        owner: &RawProducerServient,
        limits: RawFiniteLimits,
        active: usize,
        retained: usize,
        shared_results: Option<Arc<HostResourceAccounts>>,
    ) -> CoreResult<Self> {
        if !(1..=MAX_WORK).contains(&active)
            || !(active..=MAX_WORK).contains(&retained)
            || !(1..=MAX_WORK).contains(&limits.candidates)
            || limits.output_retained_bytes < size_of::<InteractionOutput>()
        {
            return Err(CoreError::Validation(context()));
        }
        Ok(Self(Rc::new(CapacityState {
            issuer: Rc::downgrade(&owner.issuer),
            limits,
            active_limit: active,
            retained_limit: retained,
            active: Cell::new(0),
            free: RefCell::new((0..retained).map(|_| Box::default()).collect()),
            sequence: Cell::new(0),
            shared_results,
        })))
    }

    pub fn active_work(&self) -> usize {
        self.0.active.get()
    }
    pub fn retained_work(&self) -> usize {
        self.0.retained_limit - self.0.free.borrow().len()
    }

    pub(super) fn supports(&self, issuer: &Weak<()>, limits: RawFiniteLimits) -> bool {
        self.0.issuer.ptr_eq(issuer)
            && self.0.issuer.strong_count() != 0
            && limits.candidates <= self.0.limits.candidates
            && limits.input_body_bytes <= self.0.limits.input_body_bytes
            && limits.output_body_bytes <= self.0.limits.output_body_bytes
            && limits.output_media_capacity <= self.0.limits.output_media_capacity
            && limits.output_retained_bytes <= self.0.limits.output_retained_bytes
    }

    pub(super) fn acquire(&self) -> Option<ProducerFiniteSettlement> {
        if self.active_work() == self.0.active_limit
            || self.retained_work() == self.0.retained_limit
        {
            return None;
        }
        let sequence = self.0.sequence.get().checked_add(1)?;
        let permit = match &self.0.shared_results {
            Some(bank) => Some(bank.reserve(K::PendingHandlerCallsGlobalMax, 1)?),
            None => None,
        };
        let record = self.0.free.borrow_mut().pop()?;
        self.0.sequence.set(sequence);
        self.0.active.set(self.active_work() + 1);
        Some(ProducerFiniteSettlement {
            record: Some(record),
            sequence,
            capacity: self.clone(),
            active: true,
            _permit: permit,
        })
    }
}

#[derive(Default)]
struct ResultRecord {
    outcome: Option<RawFiniteSettlement>,
    accepted: Option<AcceptedBehavior>,
    owner: Option<Rc<dyn core::any::Any>>,
    next: Option<ProducerFiniteSettlement>,
}

/// An original finite outcome and its funding. Data and target names are lent,
/// so retaining original backing requires retaining this owner (or an Rc of it).
/// Acknowledgement frees the participant's unread record; only disposal refunds
/// the retained-work allowance. Dropping it never changes another exchange.
pub struct ProducerFiniteSettlement {
    record: Option<Box<ResultRecord>>,
    sequence: u64,
    capacity: ProducerFiniteCapacity,
    active: bool,
    _permit: Option<HostCapacityReservation>,
}
impl ProducerFiniteSettlement {
    fn release_active(&mut self) {
        if self.active {
            self.active = false;
            self.capacity.0.active.set(self.capacity.active_work() - 1);
        }
    }
    pub fn cleanup_error(&self) -> Option<&CoreError> {
        self.outcome().cleanup_error.as_ref()
    }
    fn outcome(&self) -> &RawFiniteSettlement {
        self.record
            .as_ref()
            .unwrap()
            .outcome
            .as_ref()
            .expect("terminal finite outcome")
    }
    /// Unique within the original capacity, never reused. Idle receive attempts
    /// may leave gaps; this is local identity, not a native protocol identifier.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn binding(&self) -> (BindingId, BindingGeneration) {
        self.outcome().binding
    }
    pub fn recipient(&self) -> Option<ProducerInstanceKey> {
        self.outcome().recipient
    }
    pub fn operation(&self) -> Option<Operation> {
        self.outcome().operation
    }
    pub fn target(&self) -> Option<clinkz_wot_core::thing::AffordanceSite<'_>> {
        use clinkz_wot_core::thing::AffordanceSite;
        self.outcome().target.as_ref().map(|target| match target {
            AffordanceTarget::Thing => AffordanceSite::Thing,
            AffordanceTarget::Property(name) => AffordanceSite::Property(name),
            _ => unreachable!("finite Producer target"),
        })
    }
    pub fn effect(&self) -> FiniteEffect {
        self.outcome().effect
    }
    /// First local interruption of this exchange, retained alongside its result.
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.outcome().first_cause.as_ref()
    }
    pub fn delivery(&self) -> FiniteDelivery {
        self.outcome().delivery
    }
    pub fn result(&self) -> Result<clinkz_wot_core::InteractionOutputView<'_>, &CoreError> {
        self.outcome().result.as_ref().map(InteractionOutput::view)
    }

    pub(super) fn complete(
        mut self,
        outcome: RawFiniteSettlement,
        accepted: Option<AcceptedBehavior>,
        owner: Option<Rc<dyn core::any::Any>>,
        queue: &mut ProducerFiniteQueue,
    ) {
        let record = self.record.as_mut().unwrap();
        record.outcome = Some(outcome);
        record.accepted = accepted;
        record.owner = owner;
        self.release_active();
        // At most MAX_WORK original records, no allocation or external callback.
        let mut tail = &mut queue.head;
        while let Some(record) = tail {
            tail = &mut record.record.as_mut().unwrap().next;
        }
        *tail = Some(self);
        queue.len += 1;
    }

    pub(super) fn take(queue: &mut ProducerFiniteQueue) -> Option<Self> {
        let mut record = queue.head.take()?;
        queue.head = record.record.as_mut().unwrap().next.take();
        queue.len -= 1;
        Some(record)
    }
}

impl Drop for ProducerFiniteSettlement {
    fn drop(&mut self) {
        let mut record = self.record.take().unwrap();
        // Drop linked owners before publishing capacity or borrowing the free
        // list. Their destructors may make a reentrant admission attempt.
        *record = ResultRecord::default();
        self.release_active();
        self.capacity.0.free.borrow_mut().push(record);
    }
}

impl core::fmt::Debug for ProducerFiniteSettlement {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ProducerFiniteSettlement")
            .field("sequence", &self.sequence)
            .field("outcome", &self.record.as_ref().unwrap().outcome)
            .finish()
    }
}
