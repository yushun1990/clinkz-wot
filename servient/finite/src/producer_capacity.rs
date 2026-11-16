//! Optional finite work capacity, independent of persistent native participation.
use super::*;
use clinkz_wot_core::host_accounts::{HostCapacityReservation, HostResourceAccounts};
use clinkz_wot_foundation::ResourceKind as K;

pub(super) const MAX_WORK: usize = 32;

#[derive(Default)]
pub(super) struct ProducerFiniteQueue {
    head: Option<Box<ProducerFiniteSettlement>>,
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
    retained: Cell<usize>,
    sequence: Cell<u64>,
    accounts: Arc<HostResourceAccounts>,
    _funding: [HostCapacityReservation; 2],
}

impl ProducerFiniteCapacity {
    /// Provision bounded accounting, without reserving an exchange per Thing.
    /// Each receive opportunity reserves its complete input/result/cleanup
    /// allowance at this original bank before calling native receive.
    pub fn try_new(
        owner: &RawProducerServient,
        limits: RawFiniteLimits,
        active: usize,
        retained: usize,
        accounts: Arc<HostResourceAccounts>,
    ) -> CoreResult<Self> {
        if !(1..=MAX_WORK).contains(&active)
            || !(active..=MAX_WORK).contains(&retained)
            || !(1..=MAX_WORK).contains(&limits.candidates)
            || limits.output_retained_bytes < size_of::<InteractionOutput>()
            || work_bytes(limits).is_none()
        {
            return Err(CoreError::Validation(context()));
        }
        let bytes = (size_of::<CapacityState>() + 2 * size_of::<usize>()) as u64;
        let reserve = |kind| accounts.reserve(kind, bytes).ok_or_else(busy);
        let funding = [
            reserve(K::EngineLiveBytesGlobalMax)?,
            reserve(K::CleanupBytesMax)?,
        ];
        Ok(Self(Rc::new(CapacityState {
            issuer: Rc::downgrade(&owner.issuer),
            limits,
            active_limit: active,
            retained_limit: retained,
            active: Cell::new(0),
            retained: Cell::new(0),
            sequence: Cell::new(0),
            accounts,
            _funding: funding,
        })))
    }

    pub fn active_work(&self) -> usize {
        self.0.active.get()
    }
    pub fn retained_work(&self) -> usize {
        self.0.retained.get()
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

    pub(super) fn acquire(&self) -> Option<Box<ProducerFiniteSettlement>> {
        if self.active_work() == self.0.active_limit
            || self.retained_work() == self.0.retained_limit
        {
            return None;
        }
        let sequence = self.0.sequence.get().checked_add(1)?;
        let bytes = work_bytes(self.0.limits)? as u64;
        let reserve = |kind, amount| self.0.accounts.reserve(kind, amount);
        // All fallible reservations precede the allocation and native callback.
        // Partial reservation failure unwinds only these original leases.
        let funding = [
            Some(reserve(K::EngineLiveBytesGlobalMax, bytes)?),
            Some(reserve(K::CleanupBytesMax, bytes)?),
            Some(reserve(K::CleanupItemsMax, 1)?),
            Some(reserve(K::InFlightResponsesGlobalMax, 1)?),
            Some(reserve(K::PendingHandlerCallsGlobalMax, 1)?),
            Some(reserve(K::BindingIngressItemsGlobalMax, 1)?),
        ];
        self.0.sequence.set(sequence);
        self.0.active.set(self.active_work() + 1);
        self.0.retained.set(self.retained_work() + 1);
        Some(Box::new(ProducerFiniteSettlement {
            outcome: None,
            accepted: None,
            owner: None,
            next: None,
            sequence,
            lease: WorkLease {
                capacity: self.clone(),
                active: true,
                funding,
            },
        }))
    }
}

fn work_bytes(limits: RawFiniteLimits) -> Option<usize> {
    size_of::<ProducerFiniteSettlement>()
        .checked_add(
            limits
                .candidates
                .checked_mul(size_of::<clinkz_wot_core::finite::FiniteCandidate>())?,
        )?
        .checked_add(limits.input_body_bytes)?
        .checked_add(limits.output_retained_bytes)
}

fn busy() -> CoreError {
    CoreError::Backpressure(context())
}

struct WorkLease {
    capacity: ProducerFiniteCapacity,
    active: bool,
    funding: [Option<HostCapacityReservation>; 6],
}
impl WorkLease {
    fn finish(&mut self) {
        if self.active {
            self.active = false;
            self.capacity.0.active.set(self.capacity.active_work() - 1);
            // Native ingress has ended. The outstanding-obligation quota,
            // original result bytes, response and cleanup remain until disposal.
            self.funding[5] = None;
        }
    }
}
impl Drop for WorkLease {
    fn drop(&mut self) {
        self.finish();
        self.capacity
            .0
            .retained
            .set(self.capacity.retained_work() - 1);
    }
}

/// An original finite outcome and its funding. Data and target names are lent,
/// so retaining original backing requires retaining this owner (or an Rc of it).
/// Acknowledgement frees the participant's unread record; only disposal refunds
/// the retained-work allowance. Dropping it never changes another exchange.
pub struct ProducerFiniteSettlement {
    outcome: Option<RawFiniteSettlement>,
    accepted: Option<AcceptedBehavior>,
    owner: Option<Rc<dyn core::any::Any>>,
    next: Option<Box<Self>>,
    sequence: u64,
    // Last: destroy original result/behavior/owner before releasing its bank.
    lease: WorkLease,
}
impl ProducerFiniteSettlement {
    fn outcome(&self) -> &RawFiniteSettlement {
        self.outcome.as_ref().expect("terminal finite outcome")
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
    pub fn delivery(&self) -> FiniteDelivery {
        self.outcome().delivery
    }
    pub fn result(&self) -> Result<clinkz_wot_core::InteractionOutputView<'_>, &CoreError> {
        self.outcome().result.as_ref().map(InteractionOutput::view)
    }

    pub(super) fn complete(
        mut self: Box<Self>,
        outcome: RawFiniteSettlement,
        accepted: Option<AcceptedBehavior>,
        owner: Option<Rc<dyn core::any::Any>>,
        queue: &mut ProducerFiniteQueue,
    ) {
        self.outcome = Some(outcome);
        self.accepted = accepted;
        self.owner = owner;
        self.lease.finish();
        // At most MAX_WORK original records, no allocation or external callback.
        let mut tail = &mut queue.head;
        while let Some(record) = tail {
            tail = &mut record.next;
        }
        *tail = Some(self);
        queue.len += 1;
    }

    pub(super) fn take(queue: &mut ProducerFiniteQueue) -> Option<Box<Self>> {
        let mut record = queue.head.take()?;
        queue.head = record.next.take();
        queue.len -= 1;
        Some(record)
    }
}

impl core::fmt::Debug for ProducerFiniteSettlement {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ProducerFiniteSettlement")
            .field("sequence", &self.sequence)
            .field("outcome", &self.outcome)
            .finish()
    }
}
