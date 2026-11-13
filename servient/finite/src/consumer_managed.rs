//! Managed finite Consumer lifecycle. IDs address AVL nodes; slots are private.
use super::*;
use alloc::boxed::Box;
#[path = "consumer_tree.rs"]
mod tree;
use tree::Tree;

const MAX_THING_ID_BYTES: usize = 2048;
const MAX_CONFIGURATION_NAME_BYTES: usize = 256;
struct RegisteredName {
    value: Box<str>,
    _funding: HostCapacityReservation,
}
fn charge(budget: &mut WorkBudget, amount: u64) -> CoreResult<()> {
    if pay(budget, W::QueueOperations, amount) {
        Ok(())
    } else {
        Err(busy())
    }
}

// One funded ticket per allocated internal coordinate. Free tickets are reused
// without searching or allocating. They carry no logical permission or native
// work, and old Core references retain their own immutable generation/issuer.
struct SlotTicket {
    id: ThingSlotId,
    next: Option<Box<SlotTicket>>,
    _funding: HostCapacityReservation,
}
#[derive(Default)]
struct Slots {
    next: u64,
    free: Option<Box<SlotTicket>>,
}
#[derive(Clone, Copy)]
struct SlotPlan {
    id: ThingSlotId,
    fresh: bool,
}
impl Slots {
    fn plan(&self) -> CoreResult<SlotPlan> {
        if let Some(ticket) = &self.free {
            return Ok(SlotPlan {
                id: ticket.id,
                fresh: false,
            });
        }
        let slot = u32::try_from(self.next).map_err(|_| invalid())?;
        Ok(SlotPlan {
            id: ThingSlotId::new(SlotIndex::new(slot), Generation::INITIAL),
            fresh: true,
        })
    }
    fn take(&mut self, plan: SlotPlan, fresh: Option<Box<SlotTicket>>) -> Box<SlotTicket> {
        if plan.fresh {
            assert!(self.free.is_none() && self.next == u64::from(plan.id.slot().get()));
            self.next += 1;
            fresh.unwrap()
        } else {
            let mut ticket = self.free.take().unwrap();
            self.free = ticket.next.take();
            assert_eq!(ticket.id, plan.id);
            ticket
        }
    }
    fn release(&mut self, mut ticket: Box<SlotTicket>) -> Option<Box<SlotTicket>> {
        let Some(next) = ticket.id.generation().checked_next() else {
            return Some(ticket);
        };
        ticket.id = ThingSlotId::new(ticket.id.slot(), next);
        ticket.next = self.free.take();
        self.free = Some(ticket);
        None
    }
}
impl Drop for Slots {
    fn drop(&mut self) {
        // Free lists can be long. Shutdown is linear but never recursively drops
        // a population-sized chain on the stack.
        while let Some(mut ticket) = self.free.take() {
            self.free = ticket.next.take();
        }
    }
}

/// Non-revoking snapshot of one original named registration. It is optional for
/// ordinary ID operations and useful for conditional management/composition.
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
    thing_id: Rc<RegisteredName>,
    _funding: HostCapacityReservation,
}
impl ConsumerInstanceHandle {
    pub fn consumer(&self) -> ConsumerInstanceRef {
        self.identity.consumer.clone()
    }
    pub fn is_current(&self) -> bool {
        self.identity.consumer.is_current()
    }
    pub fn thing_id(&self) -> &str {
        &self.identity.thing_id.value
    }
}
impl core::fmt::Debug for ConsumerInstanceHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ConsumerInstanceHandle")
            .field("consumer", &self.identity.consumer)
            .finish_non_exhaustive()
    }
}

/// Actual logical owner of registered finite Consumers and named configurations.
/// All ordinary management addresses TD IDs. Internal slots are owner-assigned;
/// standalone caller-owned authority remains a separate low-level representation.
/// Registration, lookup and removal touch logarithmic tree paths, never a global
/// array or resize pass. No call slots, scheduler or notification source is added.
/// Removal fences immediately on success, not native drain: applications supply
/// original work progress, clock observations, credit and actual deadline timers.
/// Returned snapshots/views never revoke on Drop. Owner Drop passively fences all
/// parents before disposal; native/service disposal retains its finite qualification.
/// Caller-owned roots cannot dictate managed coordinates:
/// ```compile_fail
/// use clinkz_wot_servient_finite::consumer::ConsumerOwner;
/// use clinkz_wot_core::consumer::ConsumerInstance;
/// use clinkz_wot_foundation::WorkBudget;
/// fn adopt(owner: &ConsumerOwner, root: ConsumerInstance, budget: &mut WorkBudget) {
///     owner.register_instance(root, budget);
/// }
/// ```
pub struct ConsumerOwner {
    bank: Arc<HostResourceAccounts>,
    max_instances: usize,
    max_associations: usize,
    active: Cell<bool>,
    registry: RefCell<ManagedRegistry>,
}
#[derive(Default)]
struct ManagedRegistry {
    instances: Tree<RegisteredInstance>,
    slots: Slots,
    associations: usize,
}
struct RegisteredInstance {
    // First: revoke before linked service destruction, including unwind.
    owner: ConsumerInstance,
    identity: Rc<InstanceIdentity>,
    configurations: Tree<StandaloneConsumerAssociation>,
    slot: Option<Box<SlotTicket>>,
}
impl ConsumerOwner {
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
    /// Install logical identity only. The URI is already qualified by TD ingress;
    /// IDs are exact, nonempty, at most 2048 bytes and unique within this owner.
    /// Prepared configurations are installed separately before operations.
    pub fn register_thing(
        &self,
        thing_id: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerInstanceHandle> {
        Self::check_name(thing_id, MAX_THING_ID_BYTES)?;
        let plan = {
            let registry = self.registry.borrow();
            if !self.active.get() {
                return Err(stale());
            }
            registry.instances.lookup_work(thing_id, budget)?;
            if registry.instances.get(thing_id).is_some() {
                return Err(invalid());
            }
            if registry.instances.len() == self.max_instances {
                return Err(busy());
            }
            registry.instances.mutation_work(thing_id, budget)?;
            charge(budget, 8)?;
            // Covers the generation's Core/identity/node retirement, including
            // rollback. Pay before constructing any of those owners.
            if !pay(budget, W::CleanupItems, 1) {
                return Err(busy());
            }
            registry.slots.plan()?
        };
        // All fallible acquisition is outside the registry and before ticket
        // consumption/publication. Failure leaves both tree and free list intact.
        let name = self.name(thing_id, MAX_THING_ID_BYTES, budget)?;
        let fresh = if plan.fresh {
            if !pay(budget, W::CleanupItems, 1) {
                return Err(busy());
            }
            let funding = self
                .bank
                .reserve(
                    K::EngineLiveBytesGlobalMax,
                    core::mem::size_of::<SlotTicket>() as u64,
                )
                .ok_or_else(busy)?;
            Some(Box::new(SlotTicket {
                id: plan.id,
                next: None,
                _funding: funding,
            }))
        } else {
            None
        };
        let owner = ConsumerInstance::try_new_funded(plan.id, &self.bank)?;
        let identity = self.identity(owner.reference(), name.clone())?;
        let entry = RegisteredInstance {
            owner,
            identity: identity.clone(),
            configurations: Tree::default(),
            slot: None,
        };
        let mut node = Tree::prepare(name, entry, &self.bank).map_err(|_| busy())?;
        {
            let mut registry = self.registry.borrow_mut();
            node.value.slot = Some(registry.slots.take(plan, fresh));
            registry.instances.insert_absent(node);
        }
        Ok(ConsumerInstanceHandle { identity })
    }
    pub fn instance(
        &self,
        thing_id: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerInstanceHandle> {
        let registry = self.registry.borrow();
        Ok(ConsumerInstanceHandle {
            identity: self.find(&registry, thing_id, budget)?.identity.clone(),
        })
    }
    pub fn association(
        &self,
        thing_id: &str,
        configuration: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerAssociation> {
        Self::check_name(configuration, MAX_CONFIGURATION_NAME_BYTES)?;
        let registry = self.registry.borrow();
        let entry = self.find(&registry, thing_id, budget)?;
        entry.configurations.lookup_work(configuration, budget)?;
        entry
            .configurations
            .get(configuration)
            .map(|a| a.handle())
            .ok_or_else(|| CoreError::NotFound(context()))
    }
    /// Install/update exactly this configuration; no default, merge or fallback.
    /// Input must belong to the current original parent. Refusal returns it.
    pub fn set_configuration(
        &self,
        thing_id: &str,
        configuration: &str,
        association: StandaloneConsumerAssociation,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerAssociation, BindingInputRejection<StandaloneConsumerAssociation>> {
        self.set(thing_id, configuration, None, association, budget)
    }
    /// Optional conditional update, expressed with public names and an original
    /// non-revoking view, without caller-managed internal coordinates.
    pub fn replace_configuration(
        &self,
        thing_id: &str,
        configuration: &str,
        expected: &ConsumerAssociation,
        association: StandaloneConsumerAssociation,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerAssociation, BindingInputRejection<StandaloneConsumerAssociation>> {
        self.set(thing_id, configuration, Some(expected), association, budget)
    }
    fn set(
        &self,
        thing_id: &str,
        configuration: &str,
        expected: Option<&ConsumerAssociation>,
        association: StandaloneConsumerAssociation,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerAssociation, BindingInputRejection<StandaloneConsumerAssociation>> {
        let checked = (|| {
            Self::check_name(configuration, MAX_CONFIGURATION_NAME_BYTES)?;
            let registry = self.registry.borrow();
            let entry = self.find(&registry, thing_id, budget)?;
            if !association.is_current() || association.consumer() != entry.identity.consumer {
                return Err(invalid());
            }
            entry.configurations.lookup_work(configuration, budget)?;
            let current = entry.configurations.get(configuration).map(|a| a.handle());
            if expected.is_some_and(|e| {
                current
                    .as_ref()
                    .is_none_or(|c| !Rc::ptr_eq(&c.state, &e.state))
            }) {
                return Err(stale());
            }
            registry.instances.lookup_work(thing_id, budget)?; // commit lookup
            if current.is_some() {
                entry.configurations.lookup_work(configuration, budget)?;
            } else {
                if registry.associations == self.max_associations {
                    return Err(busy());
                }
                entry.configurations.mutation_work(configuration, budget)?;
            }
            charge(budget, 2)?;
            if !self.prepay_association(&association, budget) {
                return Err(busy());
            }
            Ok(current)
        })();
        let current = match checked {
            Ok(current) => current,
            Err(e) => return Err(reject(association, e)),
        };
        let handle = association.handle();
        if let Some(current) = current {
            let removed = {
                let mut registry = self.registry.borrow_mut();
                let entry = registry.instances.get_mut(thing_id).unwrap();
                current.fence();
                core::mem::replace(
                    entry.configurations.get_mut(configuration).unwrap(),
                    association,
                )
            };
            drop(removed);
        } else {
            let name = match self.name(configuration, MAX_CONFIGURATION_NAME_BYTES, budget) {
                Ok(n) => n,
                Err(e) => return Err(reject(association, e)),
            };
            let node =
                Tree::prepare(name, association, &self.bank).map_err(|a| reject(a, busy()))?;
            let mut registry = self.registry.borrow_mut();
            registry
                .instances
                .get_mut(thing_id)
                .unwrap()
                .configurations
                .insert_absent(node);
            registry.associations += 1;
        }
        Ok(handle)
    }
    pub fn remove_configuration(
        &self,
        thing_id: &str,
        configuration: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        self.remove_configuration_checked(thing_id, configuration, None, budget)
    }
    pub fn remove_configuration_if_current(
        &self,
        thing_id: &str,
        configuration: &str,
        expected: &ConsumerAssociation,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        self.remove_configuration_checked(thing_id, configuration, Some(expected), budget)
    }
    fn remove_configuration_checked(
        &self,
        thing_id: &str,
        configuration: &str,
        expected: Option<&ConsumerAssociation>,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        let current = {
            Self::check_name(configuration, MAX_CONFIGURATION_NAME_BYTES)?;
            let registry = self.registry.borrow();
            let entry = self.find(&registry, thing_id, budget)?;
            entry.configurations.lookup_work(configuration, budget)?;
            let current = entry
                .configurations
                .get(configuration)
                .ok_or_else(|| CoreError::NotFound(context()))?
                .handle();
            if expected.is_some_and(|e| !Rc::ptr_eq(&e.state, &current.state)) {
                return Err(stale());
            }
            entry.configurations.mutation_work(configuration, budget)?;
            registry.instances.lookup_work(thing_id, budget)?;
            charge(budget, 2)?;
            current
        };
        let removed = {
            let mut registry = self.registry.borrow_mut();
            current.fence();
            let node = registry
                .instances
                .get_mut(thing_id)
                .unwrap()
                .configurations
                .remove(configuration)
                .unwrap();
            registry.associations -= 1;
            node
        };
        drop(removed);
        Ok(())
    }
    /// ID commands address the current registration, not an external delayed
    /// command's intended revision. Removal completion is not native drain.
    pub fn remove_thing(&self, thing_id: &str, budget: &mut WorkBudget) -> CoreResult<()> {
        self.remove(thing_id, None, budget)
    }
    /// Optional removal conditioned on an original opaque snapshot.
    pub fn remove_instance(
        &self,
        expected: &ConsumerInstanceHandle,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        self.remove(expected.thing_id(), Some(expected), budget)
    }
    fn remove(
        &self,
        thing_id: &str,
        expected: Option<&ConsumerInstanceHandle>,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        {
            let registry = self.registry.borrow();
            let entry = self.find_expected(&registry, thing_id, expected, budget)?;
            registry.instances.mutation_work(thing_id, budget)?;
            registry.instances.lookup_work(thing_id, budget)?;
            charge(budget, 8)?;
            // Merely inspect here: no fence or ticket mutation before all credit.
            assert!(entry.slot.is_some());
        }
        let (removed, retired_slot) = {
            let mut registry = self.registry.borrow_mut();
            registry.instances.get(thing_id).unwrap().owner.revoke();
            let mut node = registry.instances.remove(thing_id).unwrap();
            registry.associations -= node.value.configurations.len();
            let retired = registry.slots.release(node.value.slot.take().unwrap());
            (node, retired)
        };
        drop((removed, retired_slot));
        Ok(())
    }
    pub fn replace_thing(
        &self,
        thing_id: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerInstanceHandle> {
        let expected = self.instance(thing_id, budget)?;
        self.replace_instance(&expected, budget)
    }
    /// Owner-generated replacement, optionally conditioned on an original snapshot.
    /// The internal generation advances without asking the caller for coordinates.
    pub fn replace_instance(
        &self,
        expected: &ConsumerInstanceHandle,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerInstanceHandle> {
        let thing_id = expected.thing_id();
        let (name, next) = {
            let registry = self.registry.borrow();
            let entry = self.find_expected(&registry, thing_id, Some(expected), budget)?;
            let old = entry.identity.consumer.slot();
            let next = old.generation().checked_next().ok_or_else(invalid)?;
            registry.instances.lookup_work(thing_id, budget)?;
            charge(budget, 8)?;
            if !pay(budget, W::CleanupItems, 1) {
                return Err(busy());
            }
            (
                entry.identity.thing_id.clone(),
                ThingSlotId::new(old.slot(), next),
            )
        };
        let owner = ConsumerInstance::try_new_funded(next, &self.bank)?;
        let identity = self.identity(owner.reference(), name)?;
        let new = RegisteredInstance {
            owner,
            identity: identity.clone(),
            configurations: Tree::default(),
            slot: None,
        };
        let removed = {
            let mut registry = self.registry.borrow_mut();
            let entry = registry.instances.get_mut(thing_id).unwrap();
            entry.owner.revoke();
            let mut old = core::mem::replace(entry, new);
            let mut slot = old.slot.take().unwrap();
            slot.id = next;
            entry.slot = Some(slot);
            registry.associations -= old.configurations.len();
            old
        };
        drop(removed);
        Ok(ConsumerInstanceHandle { identity })
    }
    /// Whole-owner shutdown necessarily visits its population, and fences every
    /// parent before any linked disposal. Ordinary single-ID operations do not.
    pub fn begin_close(&self) {
        self.active.set(false);
        let removed = {
            let mut registry = self.registry.borrow_mut();
            registry.instances.visit(|entry| entry.owner.revoke());
            core::mem::take(&mut *registry)
        };
        drop(removed);
    }
    pub fn is_closed(&self) -> bool {
        !self.active.get()
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

    fn find<'a>(
        &self,
        registry: &'a ManagedRegistry,
        id: &str,
        budget: &mut WorkBudget,
    ) -> CoreResult<&'a RegisteredInstance> {
        if !self.active.get() {
            return Err(stale());
        }
        Self::check_name(id, MAX_THING_ID_BYTES)?;
        registry.instances.lookup_work(id, budget)?;
        registry
            .instances
            .get(id)
            .ok_or_else(|| CoreError::NotFound(context()))
    }
    fn find_expected<'a>(
        &self,
        registry: &'a ManagedRegistry,
        id: &str,
        expected: Option<&ConsumerInstanceHandle>,
        budget: &mut WorkBudget,
    ) -> CoreResult<&'a RegisteredInstance> {
        if expected.is_some_and(|e| !e.is_current()) {
            return Err(stale());
        }
        let entry = self.find(registry, id, budget).map_err(|e| {
            if expected.is_some() && matches!(e, CoreError::NotFound(_)) {
                stale()
            } else {
                e
            }
        })?;
        if expected.is_some_and(|e| !Rc::ptr_eq(&e.identity, &entry.identity)) {
            return Err(stale());
        }
        Ok(entry)
    }
    fn check_name(value: &str, max: usize) -> CoreResult<()> {
        if value.is_empty() || value.len() > max {
            Err(invalid())
        } else {
            Ok(())
        }
    }
    fn name(
        &self,
        value: &str,
        max: usize,
        budget: &mut WorkBudget,
    ) -> CoreResult<Rc<RegisteredName>> {
        Self::check_name(value, max)?;
        charge(budget, value.len() as u64 + 1)?;
        if !pay(budget, W::CleanupItems, 1) {
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
    fn identity(
        &self,
        consumer: ConsumerInstanceRef,
        thing_id: Rc<RegisteredName>,
    ) -> CoreResult<Rc<InstanceIdentity>> {
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

#[cfg(test)]
mod slot_tests {
    use super::*;
    use clinkz_wot_foundation::{GatewayDefaultV1, StaticResourceProfile};
    #[test]
    fn exhausted_ticket_retires_without_wrap_and_free_list_disposal_is_iterative() {
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let mut slots = Slots {
            next: 1,
            free: None,
        };
        let ticket = Box::new(SlotTicket {
            id: ThingSlotId::new(SlotIndex::new(0), Generation::new(u32::MAX).unwrap()),
            next: None,
            _funding: bank
                .reserve(
                    K::EngineLiveBytesGlobalMax,
                    core::mem::size_of::<SlotTicket>() as u64,
                )
                .unwrap(),
        });
        drop(
            slots
                .release(ticket)
                .expect("do not wrap an exhausted generation"),
        );
        assert!(slots.free.is_none());
        assert_eq!(slots.plan().unwrap().id.slot().get(), 1);
        for i in 1..20_000 {
            let ticket = Box::new(SlotTicket {
                id: ThingSlotId::new(SlotIndex::new(i), Generation::INITIAL),
                next: None,
                _funding: bank
                    .reserve(
                        K::EngineLiveBytesGlobalMax,
                        core::mem::size_of::<SlotTicket>() as u64,
                    )
                    .unwrap(),
            });
            assert!(slots.release(ticket).is_none());
        }
        drop(slots);
        assert_eq!(bank.account(K::EngineLiveBytesGlobalMax).used(), 0);
    }
}
