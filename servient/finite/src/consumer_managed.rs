//! Managed finite Consumer registration. No execution storage or native driver.
use super::*;
use alloc::boxed::Box;

const MAX_THING_ID_BYTES: usize = 2048;
const MAX_CONFIGURATION_NAME_BYTES: usize = 256;

struct RegisteredName {
    value: Box<str>,
    _funding: HostCapacityReservation,
}

// Private contiguous storage, funded at actual capacity. Growth is staged
// outside registry borrows, including simultaneous old/new backing. Installing
// growth only moves records; the empty old allocation is dropped after unlock.
struct Records<T> {
    entries: Vec<T>,
    _funding: Option<HostCapacityReservation>,
}
impl<T> Default for Records<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            _funding: None,
        }
    }
}
impl<T> core::ops::Deref for Records<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.entries
    }
}
impl<T> core::ops::DerefMut for Records<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.entries
    }
}
impl<T> Records<T> {
    fn prepare_growth(
        bank: &Arc<HostResourceAccounts>,
        len: usize,
        capacity: usize,
        limit: usize,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<Self>> {
        if len < capacity {
            return Ok(None);
        }
        if len >= limit {
            return Err(busy());
        }
        let wanted = capacity.max(1).saturating_mul(2).min(limit);
        let bytes = wanted
            .checked_mul(core::mem::size_of::<T>())
            .and_then(|n| u64::try_from(n).ok())
            .ok_or_else(invalid)?;
        if !pay(budget, W::QueueOperations, len as u64) {
            return Err(busy());
        }
        let funding = bank
            .reserve(K::EngineLiveBytesGlobalMax, bytes)
            .ok_or_else(busy)?;
        let mut entries = Vec::new();
        entries.try_reserve_exact(wanted).map_err(|_| busy())?;
        Ok(Some(Self {
            entries,
            _funding: Some(funding),
        }))
    }
    fn install_growth(&mut self, growth: Option<Self>) -> Option<Self> {
        growth.map(|mut next| {
            debug_assert!(next.entries.capacity() > self.len());
            next.entries.append(&mut self.entries);
            core::mem::replace(self, next)
        })
    }
    fn insert(&mut self, index: usize, value: T) {
        // A registry commit must never implicitly grow an allocation.
        assert!(self.len() < self.entries.capacity());
        self.entries.insert(index, value);
    }
}

// Our binary search has at most floor(log2(n)) + 1 comparisons. Pay the
// complete bound before inspection; width includes the bounded key comparison.
fn search_work(budget: &mut WorkBudget, len: usize, width: usize) -> CoreResult<()> {
    let probes = usize::BITS - len.leading_zeros();
    let work = (probes as u64)
        .checked_mul(width as u64)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(invalid)?;
    if !pay(budget, W::QueueOperations, work) {
        return Err(busy());
    }
    Ok(())
}
fn search<T>(
    entries: &[T],
    width: usize,
    budget: &mut WorkBudget,
    mut compare: impl FnMut(&T) -> Ordering,
) -> CoreResult<Result<usize, usize>> {
    search_work(budget, entries.len(), width)?;
    let (mut low, mut high) = (0, entries.len());
    while low < high {
        let mid = low + (high - low) / 2;
        match compare(&entries[mid]) {
            Ordering::Less => low = mid + 1,
            Ordering::Greater => high = mid,
            Ordering::Equal => return Ok(Ok(mid)),
        }
    }
    Ok(Err(low))
}
fn pay_moves(budget: &mut WorkBudget, a: usize, b: usize) -> CoreResult<()> {
    let work = a
        .checked_add(b)
        .and_then(|n| n.checked_add(2))
        .and_then(|n| u64::try_from(n).ok())
        .ok_or_else(invalid)?;
    if !pay(budget, W::QueueOperations, work) {
        return Err(busy());
    }
    Ok(())
}

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
    thing_id: Option<Rc<RegisteredName>>,
    _funding: HostCapacityReservation,
}
impl ConsumerInstanceHandle {
    pub fn consumer(&self) -> ConsumerInstanceRef {
        self.identity.consumer.clone()
    }
    pub fn is_current(&self) -> bool {
        self.identity.consumer.is_current()
    }
    /// Original public ID, when this instance was registered by ID.
    pub fn thing_id(&self) -> Option<&str> {
        self.identity.thing_id.as_ref().map(|n| n.value.as_ref())
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
/// ID registration constructs original authority; low-level adoption consumes
/// move-only authority. Returned instance handles and `ConsumerAssociation` views
/// cannot revoke it. ID/configuration lookup recovers management after view loss.
/// Capacities bound registry metadata, not calls: supply any compatible
/// `ConsumerCalls` independently.
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
}
#[derive(Default)]
struct ManagedRegistry {
    // Named entries in lexical ID order, followed by unnamed entries in slot
    // order. Records themselves form the ID index: no row-number aliases.
    instances: Records<RegisteredInstance>,
    // The same passive identities in numeric slot order. This is only a local
    // index, not another authority or public Thing identity.
    slots: Records<Rc<InstanceIdentity>>,
    associations: usize,
}
struct RegisteredInstance {
    // Revokes before association/service destruction, including on unwind.
    owner: ConsumerInstance,
    identity: Rc<InstanceIdentity>,
    // Named configurations in lexical order, followed by unnamed associations.
    associations: Records<RegisteredAssociation>,
}
struct RegisteredAssociation {
    owner: StandaloneConsumerAssociation,
    configuration: Option<Rc<RegisteredName>>,
}

impl ConsumerOwner {
    /// Register a public Thing ID in this owner's scope, with no call capacity
    /// or configured mappings yet. Use the TD's already-qualified URI verbatim.
    /// Empty/over-2048-byte IDs and duplicate live IDs refuse; no ID is generated
    /// for an anonymous TD. Equality is exact UTF-8 equality, without URI aliases.
    /// Dropping the returned snapshot is safe: `instance` can recover it by ID.
    /// Lookup/free-slot selection is logarithmic; insertion prepays actual table
    /// shifts and any lazy growth. Arbitrary caller-chosen high slots are valid.
    pub fn register_thing(
        &self,
        thing_id: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerInstanceHandle> {
        let slot = {
            let registry = self.registry.borrow();
            if self.find_thing(&registry, thing_id, budget)?.is_some() {
                return Err(invalid());
            }
            if registry.instances.len() == self.max_instances {
                return Err(busy());
            }
            self.free_slot(&registry, budget)?
        };
        let name = self.name(thing_id, MAX_THING_ID_BYTES, budget)?;
        self.adopt_instance(
            ConsumerInstance::try_new_funded(
                ThingSlotId::new(SlotIndex::new(slot), Generation::INITIAL),
                &self.bank,
            )?,
            Some(name),
            budget,
        )
        .map_err(|e| e.error().error().clone())
    }

    /// Resolve the current registration, not a delayed command's intended old
    /// revision. Use snapshot-based remove/replace APIs for that condition.
    /// A public ID is addressing information, never an authorization capability.
    pub fn instance(
        &self,
        thing_id: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerInstanceHandle> {
        let registry = self.registry.borrow();
        let index = self
            .find_thing(&registry, thing_id, budget)?
            .ok_or_else(|| CoreError::NotFound(context()))?;
        Ok(ConsumerInstanceHandle {
            identity: registry.instances[index].identity.clone(),
        })
    }

    /// Install or replace one explicitly named access/mapping configuration.
    /// The standalone input must belong to the resolved original instance;
    /// refusal returns it without closing any existing configuration. Other
    /// configurations and accepted work retain their original policy/accounts.
    /// There is no implicit default, merge or fallback to another configuration.
    pub fn set_configuration(
        &self,
        thing_id: &str,
        configuration: &str,
        association: StandaloneConsumerAssociation,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerAssociation, BindingInputRejection<StandaloneConsumerAssociation>> {
        let checked = (|| {
            let registry = self.registry.borrow();
            let index = self
                .find_thing(&registry, thing_id, budget)?
                .ok_or_else(|| CoreError::NotFound(context()))?;
            let entry = &registry.instances[index];
            let current = self
                .find_configuration(entry, configuration, budget)?
                .map(|i| (index, i, entry.associations[i].owner.handle()));
            Ok((
                ConsumerInstanceHandle {
                    identity: entry.identity.clone(),
                },
                current,
            ))
        })();
        let (instance, current) = match checked {
            Ok(checked) => checked,
            Err(error) => return Err(reject(association, error)),
        };
        if let Some((instance, index, current)) = current {
            return self.replace_association_at((instance, index), &current, association, budget);
        }
        let name = match self.name(configuration, MAX_CONFIGURATION_NAME_BYTES, budget) {
            Ok(name) => name,
            Err(error) => return Err(reject(association, error)),
        };
        self.adopt_association(&instance, association, Some(name), budget)
    }

    /// Recover a non-revoking snapshot of exactly this configuration. All
    /// existing finite operations are available on the transient returned view.
    /// Dropping every view does not make this lookup or targeted removal fail.
    pub fn association(
        &self,
        thing_id: &str,
        configuration: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerAssociation> {
        let registry = self.registry.borrow();
        let instance = self
            .find_thing(&registry, thing_id, budget)?
            .ok_or_else(|| CoreError::NotFound(context()))?;
        let entry = &registry.instances[instance];
        let index = self
            .find_configuration(entry, configuration, budget)?
            .ok_or_else(|| CoreError::NotFound(context()))?;
        Ok(entry.associations[index].owner.handle())
    }

    /// Remove the configuration current at this ID/name lookup. To require an
    /// expected original configuration, use `remove_association` with its view.
    pub fn remove_configuration(
        &self,
        thing_id: &str,
        configuration: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        let association = self.association(thing_id, configuration, budget)?;
        self.remove_association(&association, budget)
    }

    /// Remove the Thing current at this ID lookup, including all configurations.
    /// Unknown IDs return NotFound. Removal is a fence, not native drain. A later
    /// registration may reuse the public ID; original work never resolves it again.
    pub fn remove_thing(&self, thing_id: &str, budget: &mut WorkBudget) -> CoreResult<()> {
        let instance = self.instance(thing_id, budget)?;
        self.remove_instance(&instance, budget)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn read_property(
        &self,
        thing_id: &str,
        configuration: &str,
        calls: &ConsumerCalls,
        property: &str,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerCall> {
        self.read_property_with_variables(
            thing_id,
            configuration,
            calls,
            property,
            &[],
            deadline,
            now,
            budget,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn read_property_with_variables(
        &self,
        thing_id: &str,
        configuration: &str,
        calls: &ConsumerCalls,
        property: &str,
        variables: &[(&str, &str)],
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerCall> {
        // Snapshot and end the registry borrow before policy/credential/native
        // callbacks. If it is removed in a callback, admission must refuse;
        // it cannot retry lookup and accidentally use a successor.
        self.association(thing_id, configuration, budget)?
            .read_property_with_variables(calls, property, variables, deadline, now, budget)
    }

    /// Establish limits without population-sized allocation. Index backing grows
    /// from actual registrations. Zero association capacity permits identity-only
    /// registrations. No vacant call slots, executor or notification port exists.
    pub fn try_new(
        bank: Arc<HostResourceAccounts>,
        max_instances: usize,
        max_associations: usize,
    ) -> CoreResult<Self> {
        if max_instances == 0 {
            return Err(invalid());
        }
        Ok(Self {
            bank,
            max_instances,
            max_associations,
            active: Cell::new(true),
            registry: RefCell::new(ManagedRegistry::default()),
        })
    }

    /// Adopt the unique original instance authority. Refusal returns that owner.
    /// References issued before adoption remain passive and cannot revoke it.
    pub fn register_instance(
        &self,
        instance: ConsumerInstance,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerInstanceHandle, BindingInputRejection<ConsumerInstance>> {
        self.adopt_instance(instance, None, budget)
    }

    fn adopt_instance(
        &self,
        instance: ConsumerInstance,
        thing_id: Option<Rc<RegisteredName>>,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerInstanceHandle, BindingInputRejection<ConsumerInstance>> {
        let consumer = instance.reference();
        let checked = (|| {
            if !self.active.get() || !consumer.is_current() {
                return Err(stale());
            }
            let registry = self.registry.borrow();
            if registry.instances.len() == self.max_instances {
                return Err(busy());
            }
            let slot = self
                .slot_position(&registry, consumer.slot().slot(), budget)?
                .err()
                .ok_or_else(invalid)?;
            let index = self
                .instance_position(
                    &registry,
                    thing_id.as_ref().map(|n| n.value.as_ref()),
                    consumer.slot().slot(),
                    budget,
                )?
                .err()
                .ok_or_else(invalid)?;
            pay_moves(
                budget,
                registry.instances.len() - index,
                registry.slots.len() - slot,
            )?;
            Ok((
                index,
                slot,
                registry.instances.entries.capacity(),
                registry.slots.entries.capacity(),
                registry.instances.len(),
            ))
        })();
        let (index, slot, capacity, slot_capacity, len) = match checked {
            Ok(checked) => checked,
            Err(error) => return Err(reject(instance, error)),
        };
        let growth =
            match Records::prepare_growth(&self.bank, len, capacity, self.max_instances, budget) {
                Ok(growth) => growth,
                Err(error) => return Err(reject(instance, error)),
            };
        let slot_growth = match Records::prepare_growth(
            &self.bank,
            len,
            slot_capacity,
            self.max_instances,
            budget,
        ) {
            Ok(growth) => growth,
            Err(error) => return Err(reject(instance, error)),
        };
        let identity = match self.identity(consumer, thing_id, budget) {
            Ok(identity) => identity,
            Err(error) => return Err(reject(instance, error)),
        };
        let retired = {
            let mut registry = self.registry.borrow_mut();
            let old = registry.instances.install_growth(growth);
            let old_slots = registry.slots.install_growth(slot_growth);
            registry.instances.insert(
                index,
                RegisteredInstance {
                    owner: instance,
                    identity: identity.clone(),
                    associations: Records::default(),
                },
            );
            registry.slots.insert(slot, identity.clone());
            (old, old_slots)
        };
        drop(retired);
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
        self.adopt_association(instance, association, None, budget)
    }

    fn adopt_association(
        &self,
        instance: &ConsumerInstanceHandle,
        association: StandaloneConsumerAssociation,
        configuration: Option<Rc<RegisteredName>>,
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
            let at = if let Some(name) = &configuration {
                self.configuration_position(entry, &name.value, budget)?
                    .err()
                    .ok_or_else(invalid)?
            } else {
                entry.associations.len()
            };
            pay_moves(budget, entry.associations.len() - at, 0)?;
            Ok((
                index,
                at,
                entry.associations.len(),
                entry.associations.entries.capacity(),
            ))
        })();
        let (index, at, len, capacity) = match checked {
            Ok(checked) => checked,
            Err(error) => return Err(reject(association, error)),
        };
        if !self.prepay_association(&association, budget) {
            return Err(reject(association, busy()));
        }
        let growth =
            match Records::prepare_growth(&self.bank, len, capacity, self.max_associations, budget)
            {
                Ok(growth) => growth,
                Err(error) => return Err(reject(association, error)),
            };
        let handle = association.handle();
        let old = {
            let mut registry = self.registry.borrow_mut();
            let entry = &mut registry.instances[index];
            let old = entry.associations.install_growth(growth);
            entry.associations.insert(
                at,
                RegisteredAssociation {
                    owner: association,
                    configuration,
                },
            );
            registry.associations += 1;
            old
        };
        drop(old);
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
            pay_moves(
                budget,
                registry.instances[instance].associations.len() - index - 1,
                0,
            )?;
            association.fence();
            registry.associations -= 1;
            registry.instances[instance]
                .associations
                .entries
                .remove(index)
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
            self.find_association(&registry, expected, budget)
        })();
        let indices = match checked {
            Ok(indices) => indices,
            Err(error) => return Err(reject(replacement, error)),
        };
        self.replace_association_at(indices, expected, replacement, budget)
    }

    fn replace_association_at(
        &self,
        (instance, index): (usize, usize),
        expected: &ConsumerAssociation,
        replacement: StandaloneConsumerAssociation,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerAssociation, BindingInputRejection<StandaloneConsumerAssociation>> {
        if !replacement.is_current() || replacement.consumer() != expected.consumer() {
            return Err(reject(replacement, invalid()));
        }
        if !self.prepay_association(&replacement, budget) {
            return Err(reject(replacement, busy()));
        }
        let handle = replacement.handle();
        let removed = {
            let mut registry = self.registry.borrow_mut();
            expected.fence();
            core::mem::replace(
                &mut registry.instances[instance].associations[index].owner,
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
            let slot = self
                .slot_position(&registry, instance.identity.consumer.slot().slot(), budget)?
                .map_err(|_| stale())?;
            pay_moves(
                budget,
                registry.instances.len() - index - 1,
                registry.slots.len() - slot - 1,
            )?;
            registry.instances[index].owner.revoke();
            registry.associations -= registry.instances[index].associations.len();
            let removed = registry.instances.entries.remove(index);
            let old_index = registry.slots.entries.remove(slot);
            (removed, old_index)
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
            let slot = self
                .slot_position(&registry, before.slot(), budget)?
                .map_err(|_| stale())?;
            pay_moves(budget, 0, 0)?;
            Ok((index, slot))
        })();
        let (index, slot) = match checked {
            Ok(indices) => indices,
            Err(error) => return Err(reject(replacement, error)),
        };
        let thing_id = self.registry.borrow().instances[index]
            .identity
            .thing_id
            .clone();
        let identity = match self.identity(consumer, thing_id, budget) {
            Ok(identity) => identity,
            Err(error) => return Err(reject(replacement, error)),
        };
        let removed = {
            let mut registry = self.registry.borrow_mut();
            registry.instances[index].owner.revoke();
            registry.associations -= registry.instances[index].associations.len();
            let removed = core::mem::replace(
                &mut registry.instances[index],
                RegisteredInstance {
                    owner: replacement,
                    identity: identity.clone(),
                    associations: Records::default(),
                },
            );
            let old_index = core::mem::replace(&mut registry.slots[slot], identity.clone());
            (removed, old_index)
        };
        drop(removed);
        Ok(ConsumerInstanceHandle { identity })
    }

    /// Close all registrations. Their prepaid retirement runs outside borrows;
    /// original work owners still require caller-driven native cleanup.
    pub fn begin_close(&self) {
        self.active.set(false);
        let removed = {
            let mut registry = self.registry.borrow_mut();
            // Fence every parent before any linked destructor can reenter.
            for entry in registry.instances.iter() {
                entry.owner.revoke();
            }
            core::mem::take(&mut *registry)
        };
        drop(removed);
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
        let index = self
            .instance_position(
                registry,
                handle.thing_id(),
                handle.identity.consumer.slot().slot(),
                budget,
            )?
            .map_err(|_| stale())?;
        if !Rc::ptr_eq(&registry.instances[index].identity, &handle.identity) {
            return Err(stale());
        }
        Ok(index)
    }
    fn find_thing(
        &self,
        registry: &ManagedRegistry,
        thing_id: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<usize>> {
        if !self.active.get() {
            return Err(stale());
        }
        Self::check_name(thing_id, MAX_THING_ID_BYTES)?;
        Ok(self
            .instance_position(registry, Some(thing_id), SlotIndex::new(0), budget)?
            .ok())
    }
    fn instance_position(
        &self,
        registry: &ManagedRegistry,
        name: Option<&str>,
        slot: SlotIndex,
        budget: &mut WorkBudget,
    ) -> CoreResult<Result<usize, usize>> {
        search(
            &registry.instances,
            name.map_or(1, |n| n.len() + 1),
            budget,
            |entry| match (&entry.identity.thing_id, name) {
                (Some(a), Some(b)) => a.value.as_ref().cmp(b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => entry.identity.consumer.slot().slot().cmp(&slot),
            },
        )
    }
    fn slot_position(
        &self,
        registry: &ManagedRegistry,
        slot: SlotIndex,
        budget: &mut WorkBudget,
    ) -> CoreResult<Result<usize, usize>> {
        search(&registry.slots, 1, budget, |e| {
            e.consumer.slot().slot().cmp(&slot)
        })
    }
    fn free_slot(&self, registry: &ManagedRegistry, budget: &mut WorkBudget) -> CoreResult<u32> {
        search_work(budget, registry.slots.len(), 1)?;
        // Unique nonnegative sorted slots satisfy slot[i] >= i. Equality holds
        // precisely before the first hole. Binary search that monotone predicate;
        // arbitrary high caller slots cannot prevent reuse of a lower free one.
        let (mut low, mut high) = (0, registry.slots.len());
        while low < high {
            let mid = low + (high - low) / 2;
            if u64::from(registry.slots[mid].consumer.slot().slot().get()) == mid as u64 {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        u32::try_from(low).map_err(|_| invalid())
    }
    fn configuration_position(
        &self,
        instance: &RegisteredInstance,
        configuration: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<Result<usize, usize>> {
        Self::check_name(configuration, MAX_CONFIGURATION_NAME_BYTES)?;
        search(
            &instance.associations,
            configuration.len() + 1,
            budget,
            |entry| {
                entry
                    .configuration
                    .as_ref()
                    .map_or(Ordering::Greater, |n| n.value.as_ref().cmp(configuration))
            },
        )
    }
    fn find_configuration(
        &self,
        instance: &RegisteredInstance,
        configuration: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<usize>> {
        Ok(self
            .configuration_position(instance, configuration, budget)?
            .ok())
    }
    fn check_name(value: &str, max: usize) -> CoreResult<()> {
        if value.is_empty() || value.len() > max {
            return Err(invalid());
        }
        Ok(())
    }
    fn name(
        &self,
        value: &str,
        max: usize,
        budget: &mut WorkBudget,
    ) -> CoreResult<Rc<RegisteredName>> {
        Self::check_name(value, max)?;
        if !pay(budget, W::QueueOperations, value.len() as u64 + 1)
            || !pay(budget, W::CleanupItems, 1)
        {
            return Err(busy());
        }
        let bytes = core::mem::size_of::<RegisteredName>()
            + 2 * core::mem::size_of::<usize>()
            + value.len();
        let funding = self
            .bank
            .reserve(K::EngineLiveBytesGlobalMax, bytes as u64)
            .ok_or_else(busy)?;
        Ok(Rc::new(RegisteredName {
            value: Box::from(value),
            _funding: funding,
        }))
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
        let consumer = handle.consumer();
        let slot = self
            .slot_position(registry, consumer.slot().slot(), budget)?
            .map_err(|_| stale())?;
        let identity = &registry.slots[slot];
        if identity.consumer != consumer {
            return Err(stale());
        }
        let instance = self.find(
            registry,
            &ConsumerInstanceHandle {
                identity: identity.clone(),
            },
            budget,
        )?;
        let associations = &registry.instances[instance].associations;
        if !pay(budget, W::QueueOperations, associations.len() as u64 + 1) {
            return Err(busy());
        }
        let index = associations
            .iter()
            .position(|a| Rc::ptr_eq(&a.owner.association.state, &handle.state))
            .ok_or_else(stale)?;
        Ok((instance, index))
    }
    fn identity(
        &self,
        consumer: ConsumerInstanceRef,
        thing_id: Option<Rc<RegisteredName>>,
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
            thing_id,
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
