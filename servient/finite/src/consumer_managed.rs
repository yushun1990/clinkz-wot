//! Managed finite Consumer registration. No execution storage or native driver.
use super::*;

/// A non-revoking identity for one original managed instance.
/// Cloning/dropping a handle does not remove it. Numeric slots alone are not
/// identity: membership also checks this registration's retained issuer.
/// ```compile_fail
/// use clinkz_wot_servient_finite::consumer::ConsumerInstanceHandle;
/// fn revoke(handle: &ConsumerInstanceHandle) { handle.revoke(); }
/// ```
#[derive(Clone)]
pub struct ConsumerInstanceHandle {
    identity: Rc<InstanceIdentity>,
}
struct InstanceIdentity {
    consumer: ConsumerInstanceRef,
    _funding: HostCapacityReservation,
}
impl ConsumerInstanceHandle {
    pub fn consumer(&self) -> ConsumerInstanceRef {
        self.identity.consumer.clone()
    }
    pub fn is_current(&self) -> bool {
        self.identity.consumer.is_current()
    }
}
impl core::fmt::Debug for ConsumerInstanceHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ConsumerInstanceHandle")
            .field("consumer", &self.identity.consumer)
            .finish_non_exhaustive()
    }
}

/// Lifecycle owner for registered finite Consumer instances and associations.
///
/// Registration consumes move-only authority. Returned instance handles and
/// `ConsumerAssociation` views cannot revoke it. Capacities below bound registry
/// metadata, not calls: supply any compatible `ConsumerCalls` independently.
///
/// Successful removal/close fences immediately, then releases unselected mappings
/// outside borrows. Budgeted operations can refuse before mutation. Success
/// means removal from acceptance, **not** native drain. The
/// application must schedule original call/work-owner progress after removal,
/// including with silent I/O, and provide actual timers for logical deadlines.
/// Owner Drop passively fences all registrations; accepted work retains its
/// original cleanup and qualified local-disposal path.
pub struct ConsumerOwner {
    bank: Arc<HostResourceAccounts>,
    max_instances: usize,
    max_associations: usize,
    active: Cell<bool>,
    registry: RefCell<ManagedRegistry>,
    backing: RefCell<Option<HostCapacityReservation>>,
}
struct ManagedRegistry {
    instances: Vec<RegisteredInstance>,
    associations: usize,
}
struct RegisteredInstance {
    // Revokes before association/service destruction, including on unwind.
    owner: ConsumerInstance,
    identity: Rc<InstanceIdentity>,
    associations: Vec<StandaloneConsumerAssociation>,
    // Last: actual vector backing retires before its original reservation.
    _backing: Option<HostCapacityReservation>,
}

impl ConsumerOwner {
    /// Caller-funded startup. Zero association capacity permits identity-only
    /// registrations. No vacant call slots, executor or notification port exists.
    pub fn try_new(
        bank: Arc<HostResourceAccounts>,
        max_instances: usize,
        max_associations: usize,
    ) -> CoreResult<Self> {
        if max_instances == 0 {
            return Err(invalid());
        }
        let bytes = max_instances
            .checked_mul(core::mem::size_of::<RegisteredInstance>())
            .and_then(|n| u64::try_from(n).ok())
            .ok_or_else(invalid)?;
        let backing = bank
            .reserve(K::EngineLiveBytesGlobalMax, bytes)
            .ok_or_else(busy)?;
        let mut instances = Vec::new();
        instances
            .try_reserve_exact(max_instances)
            .map_err(|_| busy())?;
        Ok(Self {
            bank,
            max_instances,
            max_associations,
            active: Cell::new(true),
            registry: RefCell::new(ManagedRegistry {
                instances,
                associations: 0,
            }),
            backing: RefCell::new(Some(backing)),
        })
    }

    /// Adopt the unique original instance authority. Refusal returns that owner.
    /// References issued before adoption remain passive and cannot revoke it.
    pub fn register_instance(
        &self,
        instance: ConsumerInstance,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerInstanceHandle, BindingInputRejection<ConsumerInstance>> {
        let consumer = instance.reference();
        let checked = (|| {
            if !self.active.get() || !consumer.is_current() {
                return Err(stale());
            }
            let registry = self.registry.borrow();
            if !pay(
                budget,
                W::QueueOperations,
                registry.instances.len() as u64 + 1,
            ) {
                return Err(busy());
            }
            if registry.instances.len() == self.max_instances {
                return Err(busy());
            }
            if registry
                .instances
                .iter()
                .any(|e| e.identity.consumer.slot().slot() == consumer.slot().slot())
            {
                return Err(invalid());
            }
            Ok(())
        })();
        if let Err(error) = checked {
            return Err(reject(instance, error));
        }
        let identity = match self.identity(consumer, budget) {
            Ok(identity) => identity,
            Err(error) => return Err(reject(instance, error)),
        };
        self.registry
            .borrow_mut()
            .instances
            .push(RegisteredInstance {
                owner: instance,
                identity: identity.clone(),
                associations: Vec::new(),
                _backing: None,
            });
        Ok(ConsumerInstanceHandle { identity })
    }

    /// Adopt a standalone association into this instance. The caller gives up
    /// its unique revoking owner; only the non-revoking view is returned.
    pub fn register_association(
        &self,
        instance: &ConsumerInstanceHandle,
        association: StandaloneConsumerAssociation,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerAssociation, BindingInputRejection<StandaloneConsumerAssociation>> {
        let checked = (|| {
            let registry = self.registry.borrow();
            let index = self.find(&registry, instance, budget)?;
            if !association.is_current() || association.consumer() != instance.identity.consumer {
                return Err(invalid());
            }
            if registry.associations == self.max_associations {
                return Err(busy());
            }
            let entry = &registry.instances[index];
            let wanted = if entry.associations.len() == entry.associations.capacity() {
                Some(
                    entry
                        .associations
                        .capacity()
                        .max(1)
                        .checked_mul(2)
                        .ok_or_else(invalid)?
                        .min(self.max_associations),
                )
            } else {
                None
            };
            let moves = if wanted.is_some() {
                entry.associations.len()
            } else {
                0
            };
            if !pay(budget, W::QueueOperations, moves as u64 + 1) {
                return Err(busy());
            }
            Ok((index, wanted))
        })();
        let (index, wanted) = match checked {
            Ok(checked) => checked,
            Err(error) => return Err(reject(association, error)),
        };
        if !self.prepay_association(&association, budget) {
            return Err(reject(association, busy()));
        }
        // Allocate/fund growth outside registry borrows. No linked code runs
        // between validation and installation; installation only moves owners.
        let mut replacement = Vec::new();
        let replacement_backing = if let Some(capacity) = wanted {
            let bytes =
                match capacity.checked_mul(core::mem::size_of::<StandaloneConsumerAssociation>()) {
                    Some(bytes) => bytes as u64,
                    None => return Err(reject(association, invalid())),
                };
            let backing = match self.bank.reserve(K::EngineLiveBytesGlobalMax, bytes) {
                Some(backing) => backing,
                None => return Err(reject(association, busy())),
            };
            if replacement.try_reserve_exact(capacity).is_err() {
                return Err(reject(association, busy()));
            }
            Some(backing)
        } else {
            None
        };
        let handle = association.handle();
        let (old, old_backing) = {
            let mut registry = self.registry.borrow_mut();
            let entry = &mut registry.instances[index];
            let old = if wanted.is_some() {
                replacement.append(&mut entry.associations);
                core::mem::replace(&mut entry.associations, replacement)
            } else {
                replacement
            };
            let old_backing = if wanted.is_some() {
                core::mem::replace(&mut entry._backing, replacement_backing)
            } else {
                None
            };
            entry.associations.push(association);
            registry.associations += 1;
            (old, old_backing)
        };
        drop(old);
        drop(old_backing);
        Ok(handle)
    }

    /// Fence/remove one association without revoking its parent or siblings.
    /// Native work remains in its original calls/stores, not in this return value.
    pub fn remove_association(
        &self,
        association: &ConsumerAssociation,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        let removed = {
            let mut registry = self.registry.borrow_mut();
            let (instance, index) = self.find_association(&registry, association, budget)?;
            association.fence();
            registry.associations -= 1;
            registry.instances[instance].associations.swap_remove(index)
        };
        drop(removed);
        Ok(())
    }

    /// Replace a facet with a fresh association under the same original parent.
    /// Old views remain fenced even when target/operation names are identical.
    pub fn replace_association(
        &self,
        expected: &ConsumerAssociation,
        replacement: StandaloneConsumerAssociation,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerAssociation, BindingInputRejection<StandaloneConsumerAssociation>> {
        let checked = (|| {
            let registry = self.registry.borrow();
            let indices = self.find_association(&registry, expected, budget)?;
            if !replacement.is_current() || replacement.consumer() != expected.consumer() {
                return Err(invalid());
            }
            Ok(indices)
        })();
        let (instance, index) = match checked {
            Ok(indices) => indices,
            Err(error) => return Err(reject(replacement, error)),
        };
        if !self.prepay_association(&replacement, budget) {
            return Err(reject(replacement, busy()));
        }
        let handle = replacement.handle();
        let removed = {
            let mut registry = self.registry.borrow_mut();
            expected.fence();
            core::mem::replace(
                &mut registry.instances[instance].associations[index],
                replacement,
            )
        };
        drop(removed);
        Ok(handle)
    }

    /// Remove the original parent and all of its registered facets. Fencing is
    /// immediate; completion does not mean accepted native calls have drained.
    pub fn remove_instance(
        &self,
        instance: &ConsumerInstanceHandle,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        let removed = {
            let mut registry = self.registry.borrow_mut();
            let index = self.find(&registry, instance, budget)?;
            registry.instances[index].owner.revoke();
            registry.associations -= registry.instances[index].associations.len();
            registry.instances.swap_remove(index)
        };
        drop(removed);
        Ok(())
    }

    /// Republish the same numeric slot with its checked next generation.
    /// Refusal leaves the original registration intact and returns replacement.
    pub fn replace_instance(
        &self,
        expected: &ConsumerInstanceHandle,
        replacement: ConsumerInstance,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerInstanceHandle, BindingInputRejection<ConsumerInstance>> {
        let consumer = replacement.reference();
        let checked = (|| {
            let registry = self.registry.borrow();
            let index = self.find(&registry, expected, budget)?;
            let before = expected.identity.consumer.slot();
            let after = consumer.slot();
            if !consumer.is_current()
                || after.slot() != before.slot()
                || before.generation().checked_next() != Some(after.generation())
            {
                return Err(invalid());
            }
            Ok(index)
        })();
        let index = match checked {
            Ok(index) => index,
            Err(error) => return Err(reject(replacement, error)),
        };
        let identity = match self.identity(consumer, budget) {
            Ok(identity) => identity,
            Err(error) => return Err(reject(replacement, error)),
        };
        let removed = {
            let mut registry = self.registry.borrow_mut();
            registry.instances[index].owner.revoke();
            registry.associations -= registry.instances[index].associations.len();
            core::mem::replace(
                &mut registry.instances[index],
                RegisteredInstance {
                    owner: replacement,
                    identity: identity.clone(),
                    associations: Vec::new(),
                    _backing: None,
                },
            )
        };
        drop(removed);
        Ok(ConsumerInstanceHandle { identity })
    }

    /// Close all registrations. Their prepaid retirement runs outside borrows;
    /// original work owners still require caller-driven native cleanup.
    pub fn begin_close(&self) {
        self.active.set(false);
        let backing = self.backing.borrow_mut().take();
        let removed = {
            let mut registry = self.registry.borrow_mut();
            // Fence every parent before any linked destructor can reenter.
            for entry in &registry.instances {
                entry.owner.revoke();
            }
            registry.associations = 0;
            core::mem::take(&mut registry.instances)
        };
        drop(removed);
        drop(backing);
    }
    pub fn is_closed(&self) -> bool {
        !self.active.get()
    }

    fn find(
        &self,
        registry: &ManagedRegistry,
        handle: &ConsumerInstanceHandle,
        budget: &mut WorkBudget,
    ) -> CoreResult<usize> {
        if !self.active.get() || !handle.is_current() {
            return Err(stale());
        }
        if !pay(
            budget,
            W::QueueOperations,
            registry.instances.len() as u64 + 1,
        ) {
            return Err(busy());
        }
        registry
            .instances
            .iter()
            .position(|e| Rc::ptr_eq(&e.identity, &handle.identity))
            .ok_or_else(stale)
    }
    fn find_association(
        &self,
        registry: &ManagedRegistry,
        handle: &ConsumerAssociation,
        budget: &mut WorkBudget,
    ) -> CoreResult<(usize, usize)> {
        if !self.active.get() || !handle.is_current() {
            return Err(stale());
        }
        if !pay(
            budget,
            W::QueueOperations,
            registry.instances.len() as u64 + 1,
        ) {
            return Err(busy());
        }
        let instance = registry
            .instances
            .iter()
            .position(|e| e.identity.consumer == handle.consumer())
            .ok_or_else(stale)?;
        let associations = &registry.instances[instance].associations;
        if !pay(budget, W::QueueOperations, associations.len() as u64 + 1) {
            return Err(busy());
        }
        let index = associations
            .iter()
            .position(|a| Rc::ptr_eq(&a.association.state, &handle.state))
            .ok_or_else(stale)?;
        Ok((instance, index))
    }
    fn identity(
        &self,
        consumer: ConsumerInstanceRef,
        budget: &mut WorkBudget,
    ) -> CoreResult<Rc<InstanceIdentity>> {
        if !pay(budget, W::CleanupItems, 1) {
            return Err(busy());
        }
        let funding = self
            .bank
            .reserve(
                K::EngineLiveBytesGlobalMax,
                (core::mem::size_of::<InstanceIdentity>() + 2 * core::mem::size_of::<usize>())
                    as u64,
            )
            .ok_or_else(busy)?;
        Ok(Rc::new(InstanceIdentity {
            consumer,
            _funding: funding,
        }))
    }
    fn prepay_association(
        &self,
        association: &StandaloneConsumerAssociation,
        budget: &mut WorkBudget,
    ) -> bool {
        let mappings = association.state.mappings.borrow();
        let count = mappings.as_ref().map_or(0, |m| m.definitions.len());
        pay(budget, W::CleanupItems, count as u64 + 1)
    }
}
impl Drop for ConsumerOwner {
    fn drop(&mut self) {
        self.begin_close();
    }
}
