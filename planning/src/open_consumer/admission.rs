//! Temporary storage and provisional mappings share the live-byte peak of one
//! Consumer admission. The Engine lease of a mapping belongs to its physical owner.
use super::*;
use ::core::cell::RefCell;

const GLOBAL: [K; 3] = [
    K::AdmissionPeakLiveBytesGlobalMax,
    K::AdmissionTemporaryBytesGlobalMax,
    K::EngineLiveBytesGlobalMax,
];

pub(super) struct ConsumerAdmission<'a> {
    pub(super) bank: &'a Arc<HostResourceAccounts>,
    pub(super) limits: &'a ResourceLimits,
    // Peak covers everything still in preparation. The last two leases cover
    // only temporary bytes. Durable owners acquire their own Engine leases.
    leases: RefCell<[HostCapacityReservation; 3]>,
}

impl<'a> ConsumerAdmission<'a> {
    pub(super) fn new(bank: &'a Arc<HostResourceAccounts>, limits: &'a ResourceLimits) -> Self {
        Self {
            bank,
            limits,
            leases: RefCell::new(GLOBAL.map(|k| bank.reserve(k, 0).expect("empty lease"))),
        }
    }

    /// The caller keeps the walk charge until the last cursor borrowing this
    /// admitted input has released its storage.
    pub(super) fn admit<'td>(
        &self,
        thing: &'td Thing,
        walk: &mut AdmissionBytes<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<ValidatedProducerCursor<'td>> {
        let td_error = crate::producer_property_read::td_error;
        let config =
            ValidatedThingAdmissionConfig::try_from_limits(self.limits).map_err(|_| invalid())?;
        let mut basic = ValidatedThingCursor::from_thing(
            thing,
            &config,
            AdmissionLedger::new(
                SlotIndex::new(0),
                Generation::INITIAL,
                0,
                config.temporary_allowance(),
                0,
                0,
                0,
                0,
            ),
        );
        loop {
            walk.resize(basic.next_step_temporary_bytes().map_err(td_error)?)?;
            let before = credits(budget);
            basic = match basic.step(budget, false) {
                ValidatedThingProgress::Complete(v) => return Ok(v.into_producer()),
                ValidatedThingProgress::Pending(next) => next,
                ValidatedThingProgress::Failed(e) => return Err(td_error(e)),
            };
            if before == credits(budget) {
                return Err(busy());
            }
        }
    }

    pub(super) fn temporary(&self, bytes: u64) -> CoreResult<AdmissionBytes<'_>> {
        self.fund(bytes, true)
    }

    pub(super) fn retained(
        &self,
        bytes: u64,
    ) -> CoreResult<(AdmissionBytes<'_>, HostCapacityReservation)> {
        let peak = self.fund(bytes, false)?;
        Ok((
            peak,
            self.reserve_global(K::EngineLiveBytesGlobalMax, bytes)?,
        ))
    }

    pub(super) fn reserve_global(
        &self,
        kind: K,
        amount: u64,
    ) -> CoreResult<HostCapacityReservation> {
        let mut lease = self.bank.reserve(kind, 0).expect("empty lease");
        self.grow_global(&mut lease, kind, amount)?;
        Ok(lease)
    }

    fn grow_global(
        &self,
        lease: &mut HostCapacityReservation,
        kind: K,
        bytes: u64,
    ) -> CoreResult<()> {
        let ceiling = self.limits.get(kind).unwrap_or(0);
        if lease.try_grow_up_to(bytes, ceiling) {
            Ok(())
        } else {
            let account = self.bank.account(kind);
            Err(refused(
                kind,
                ceiling.min(account.limit()).saturating_sub(account.used()),
                bytes,
            ))
        }
    }

    fn fund(&self, bytes: u64, temporary: bool) -> CoreResult<AdmissionBytes<'_>> {
        let mut charge = AdmissionBytes {
            owner: self,
            bytes: 0,
            temporary,
        };
        charge.grow(bytes)?;
        Ok(charge)
    }

    fn available(&self, kind: K, local_used: u64) -> u64 {
        let configured = self
            .limits
            .get(kind)
            .expect("checked admission configuration");
        if GLOBAL.contains(&kind) {
            let global = self.bank.account(kind);
            configured.min(global.limit()).saturating_sub(global.used())
        } else {
            configured.saturating_sub(local_used)
        }
    }
}

// Dropped with the allocation, including early returns and unwinding. A
// retained charge ends at definition handoff; its independent Engine lease moves.
pub(super) struct AdmissionBytes<'a> {
    owner: &'a ConsumerAdmission<'a>,
    bytes: u64,
    temporary: bool,
}

impl AdmissionBytes<'_> {
    pub(super) fn amount(&self) -> u64 {
        self.bytes
    }

    fn lease_count(&self) -> usize {
        if self.temporary { 3 } else { 1 }
    }

    pub(super) fn grow(&mut self, bytes: u64) -> CoreResult<()> {
        if bytes == 0 {
            return Ok(());
        }
        let mut leases = self.owner.leases.borrow_mut();
        let live = leases[0].amount();
        let scratch = leases[1].amount();
        let ceilings = [
            (K::AdmissionTemporaryBytesPerOperationMax, scratch),
            (K::AdmissionTemporaryBytesGlobalMax, scratch),
            (K::PeakLiveBytesPerAdmissionMax, live),
            (K::AdmissionPeakLiveBytesGlobalMax, live),
            (K::EngineLiveBytesGlobalMax, live),
        ];
        let relevant = if self.temporary {
            &ceilings[..]
        } else {
            &ceilings[2..]
        };
        let (kind, available) = relevant
            .iter()
            .map(|&(k, used)| (k, self.owner.available(k, used)))
            .min_by_key(|&(_, available)| available)
            .unwrap();
        if bytes > available {
            return Err(refused(kind, available, bytes));
        }
        for i in 0..self.lease_count() {
            if let Err(error) = self.owner.grow_global(&mut leases[i], GLOBAL[i], bytes) {
                // Another admission may have acquired capacity since observation.
                for acquired in &mut leases[..i] {
                    acquired.release_part(bytes);
                }
                return Err(error);
            }
        }
        self.bytes += bytes;
        Ok(())
    }

    pub(super) fn resize(&mut self, bytes: u64) -> CoreResult<()> {
        if bytes >= self.bytes {
            return self.grow(bytes - self.bytes);
        }
        self.release(self.bytes - bytes);
        Ok(())
    }

    pub(super) fn release(&mut self, bytes: u64) {
        self.bytes = self
            .bytes
            .checked_sub(bytes)
            .expect("original admission charge");
        for lease in &mut self.owner.leases.borrow_mut()[..self.lease_count()] {
            lease.release_part(bytes);
        }
    }
}

impl Drop for AdmissionBytes<'_> {
    fn drop(&mut self) {
        self.release(self.bytes);
    }
}

fn refused(resource: K, limit: u64, bytes: u64) -> CoreError {
    CoreError::LimitExceeded {
        resource,
        limit,
        requested: Some(bytes),
        observed: None,
        context: context(),
    }
}
