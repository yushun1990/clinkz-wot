//! Source-free Servient-local publication. Qualification belongs to Planning;
//! this module owns the actual description/authority/behavior linearization.
use super::*;

/// Bounded original-membership storage for this manually driven Host slice.
pub const MAX_PRODUCER_PARTICIPANTS: usize = 32;

/// Move-only identity of one native participation owner. It is neither a
/// logical permission nor infrastructure shutdown authority. Implementations
/// retain it until that participation's complete local disposal.
pub struct ProducerParticipationId(Rc<()>);
impl ProducerParticipationId {
    pub fn new() -> Self {
        Self(Rc::new(()))
    }
    pub fn same_owner(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Default for ProducerParticipationId {
    fn default() -> Self {
        Self::new()
    }
}

/// Preallocated unpublished instance and its one logical permission cell.
/// After commit, Drop revokes only this incarnation. Retained work still pins
/// its original instance and requests cancellation on its next funded step;
/// it cannot inherit fresh acceptance or shut down I/O.
pub struct ProducerPublication {
    current: Rc<Cell<bool>>,
    issuer: Weak<()>,
    expected: Option<ProducerInstanceKey>,
    key: ProducerInstanceKey,
    committed: bool,
    cancelled: Cell<bool>,
    participants: Vec<Weak<()>>,
    registry_participants: Vec<Weak<()>>,
    // Both original instances retain capacity until publication metadata ends.
    retired: Option<InstanceEntry>,
    instance: Option<Rc<RawProducerInstance>>,
}

impl ProducerPublication {
    pub fn staging_backing_bytes() -> usize {
        size_of::<Self>()
            + 2 * MAX_PRODUCER_PARTICIPANTS * size_of::<Weak<()>>()
            + size_of::<Cell<bool>>()
            + 2 * size_of::<usize>()
    }

    /// Attach the actual capacity owner while the staged instance is still
    /// exclusive. Accepted finite/Action work will retain this same owner.
    pub fn retain_owner(&mut self, owner: Rc<dyn core::any::Any>) -> CoreResult<()> {
        if self.committed {
            return Err(CoreError::Validation(context()));
        }
        let instance = self
            .instance
            .as_mut()
            .and_then(Rc::get_mut)
            .ok_or_else(|| CoreError::Validation(context()))?;
        // Preserve a caller-provisioned backing owner as well as the new bank.
        instance.retained_owner = Some(Rc::new((instance.retained_owner.take(), owner)));
        Ok(())
    }
    pub fn key(&self) -> ProducerInstanceKey {
        self.key
    }
    pub fn is_committed(&self) -> bool {
        self.committed
    }
    /// Bind the complete original native set before setup. Rebinding to a
    /// different/duplicate set is refused, including after partial setup fails.
    /// Storage was prepaid at publication staging, not at the local commit.
    pub fn bind_participants(
        &mut self,
        participants: &[&dyn ClosedProducerParticipation],
    ) -> CoreResult<()> {
        if self.committed {
            return Err(CoreError::Validation(context()));
        }
        self.record_participants(participants.iter().map(|p| p.participation_id()))
    }

    fn same_participants<'a>(
        &self,
        ids: impl ExactSizeIterator<Item = &'a ProducerParticipationId> + Clone,
    ) -> bool {
        ids.len() == self.participants.len()
            && ids.clone().enumerate().all(|(i, id)| {
                !ids.clone().take(i).any(|p| Rc::ptr_eq(&p.0, &id.0))
                    && self
                        .participants
                        .iter()
                        .any(|p| p.as_ptr() == Rc::as_ptr(&id.0))
            })
    }

    fn record_participants<'a>(
        &mut self,
        ids: impl ExactSizeIterator<Item = &'a ProducerParticipationId> + Clone,
    ) -> CoreResult<()> {
        if ids.len() == 0 || ids.len() > MAX_PRODUCER_PARTICIPANTS {
            return Err(CoreError::Validation(context()));
        }
        if !self.participants.is_empty() {
            return if self.same_participants(ids) {
                Ok(())
            } else {
                Err(CoreError::Validation(context()))
            };
        }
        if ids
            .clone()
            .enumerate()
            .any(|(i, id)| ids.clone().take(i).any(|prior| Rc::ptr_eq(&prior.0, &id.0)))
        {
            return Err(CoreError::Validation(context()));
        }
        for id in ids {
            self.participants.push(Rc::downgrade(&id.0));
            self.registry_participants.push(Rc::downgrade(&id.0));
        }
        Ok(())
    }
    /// Fence fresh acceptance immediately. Already accepted Actions observe
    /// this same cell and request cancellation during funded progress; no
    /// application callback or native shutdown occurs here.
    pub fn revoke(&self) {
        self.current.set(false);
        self.cancelled.set(true);
    }
    pub fn staged_description(&self) -> Option<&str> {
        self.instance.as_ref()?.description.as_deref()
    }
    pub fn supports(&self, target: &AffordanceTarget, operation: Operation) -> bool {
        self.instance
            .as_ref()
            .is_some_and(|i| i.supports(target, operation))
    }
    pub fn supports_property(&self, name: &str, operation: Operation) -> bool {
        self.instance
            .as_ref()
            .is_some_and(|i| i.supports_property(name, operation))
    }
    pub fn supports_action(&self, name: &str, operation: Operation) -> bool {
        self.instance
            .as_ref()
            .is_some_and(|i| i.supports_action(name, operation))
    }
    /// Every staged behavior must be described by at least one prepared Form.
    /// Checking Forms against behavior alone would leave extra registered
    /// handlers reachable through a native ingress without an advertised Form.
    pub fn behaviors_described_by(
        &self,
        advertised: impl Fn(clinkz_wot_core::thing::AffordanceSite<'_>, Operation) -> bool,
    ) -> bool {
        self.instance
            .as_ref()
            .is_some_and(|i| i.behaviors_described_by(advertised))
    }
    /// Dispose a displaced registry reference outside publication's commit.
    /// Accepted work keeps its independent original-instance reference.
    pub fn release_retired(&mut self) {
        self.retired = None;
    }
}

impl Drop for ProducerPublication {
    fn drop(&mut self) {
        self.revoke();
    }
}

/// Closed participation in one logical publication. This is a readiness and
/// local ingress-gate lease, NOT an Engine-wide native session/resource API.
/// Bindings may implement it over a shared listener, broker or other native
/// owner with many Things. Every participant must already have been qualified
/// against the complete final TD; no fallible I/O or callback runs in `open`.
pub trait ClosedProducerParticipation {
    /// Immutable for this owner's entire lifetime, including retirement.
    fn participation_id(&self) -> &ProducerParticipationId;
    /// Retain the actual transferred capacity owner until native work, captured
    /// backing and unread results are disposed. This cannot dispatch or do I/O.
    fn retain_owner(&mut self, owner: Rc<dyn core::any::Any>);
    /// Managed ownership handoff: the complete additional controlled backing
    /// plus promised input/result capacity, only before this participation has
    /// acquired unreserved native work. SDK/shared infrastructure stays native.
    /// None means this participant has no managed Host admission contract.
    fn admission_backing_bytes(
        &self,
        _owner: &RawProducerServient,
        _key: ProducerInstanceKey,
    ) -> Option<usize> {
        None
    }
    /// Bounded local retirement intent, without polling I/O or calling an
    /// application. Progress remains specific to the native participant.
    fn begin_retire(&mut self);
    /// Passive readiness for this exact logical instance and Servient owner.
    /// Readiness alone does not establish qualified Form/operation coverage.
    fn ready_for(&self, owner: &RawProducerServient, key: ProducerInstanceKey) -> bool;
    /// Exact native mapping/definition support. One participant may cover many
    /// Forms and Things, but each selected operation has exactly one owner at
    /// publication. This is not an Engine protocol matcher.
    fn covers(
        &self,
        _key: ProducerInstanceKey,
        _binding: (BindingId, BindingGeneration),
        _site: clinkz_wot_core::thing::AffordanceSite<'_>,
        _form: u32,
        _operation: Operation,
        _definition: &dyn core::any::Any,
    ) -> bool {
        false
    }
    fn open(&mut self);
    /// True only after this participation's native obligations and unread
    /// settlements have retired. The physical infrastructure may remain shared.
    fn retired(&self) -> bool;
}

impl<B: FiniteIngressBinding> ClosedProducerParticipation for RawFiniteRuntime<B> {
    fn participation_id(&self) -> &ProducerParticipationId {
        &self.participation
    }
    fn retain_owner(&mut self, owner: Rc<dyn core::any::Any>) {
        self.retained_owner = Some(Rc::new((self.retained_owner.take(), owner)));
    }
    fn admission_backing_bytes(
        &self,
        owner: &RawProducerServient,
        key: ProducerInstanceKey,
    ) -> Option<usize> {
        if self.state != RawFiniteState::Starting
            || self.native_started
            || self.retained_owner.is_some()
            || self.validation != FiniteOutputValidation::Required
            || self.issuer.as_ptr() != Rc::as_ptr(&owner.issuer)
            || !self.pair_is_current()
            || !self.binding.prepared_for_instance(key)
        {
            return None;
        }
        size_of::<Self>()
            .checked_add(2 * size_of::<usize>())?
            .checked_add(self.binding.engine_backing_bytes()?)?
            .checked_add(
                self.limits
                    .candidates
                    .checked_mul(size_of::<clinkz_wot_core::finite::FiniteCandidate>())?,
            )?
            .checked_add(self.limits.input_body_bytes)?
            .checked_add(self.limits.output_retained_bytes)
    }
    fn begin_retire(&mut self) {
        self.begin_close();
    }
    fn ready_for(&self, owner: &RawProducerServient, key: ProducerInstanceKey) -> bool {
        // A captured service cannot qualify dispatch that skips validation.
        // Explicitly unvalidated execution remains at activate_raw().
        self.issuer.as_ptr() == Rc::as_ptr(&owner.issuer)
            && self.state == RawFiniteState::InstalledClosed
            && self.validation == FiniteOutputValidation::Required
            && self.pair_is_current()
            && self.binding.prepared_for_instance(key)
    }
    fn covers(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: clinkz_wot_core::thing::AffordanceSite<'_>,
        form: u32,
        operation: Operation,
        definition: &dyn core::any::Any,
    ) -> bool {
        binding == self.identity
            && self
                .binding
                .covers_definition(key, site, form, operation, definition)
    }
    fn open(&mut self) {
        self.state = RawFiniteState::Serving;
    }
    fn retired(&self) -> bool {
        self.state == RawFiniteState::Closed && self.settlement.is_none()
    }
}

impl RawProducerServient {
    /// Preback the logical registry before any native setup. Empty does not
    /// confer acceptance, unlike constructing an already active raw registry.
    pub fn try_empty(limits: RawProducerRegistryLimits) -> CoreResult<Self> {
        if !(1..=MAX_INSTANCES).contains(&limits.instances)
            || !(1..=MAX_BEHAVIORS_PER_INSTANCE).contains(&limits.behaviors_per_instance)
            || limits.target_name_bytes > MAX_TARGET_NAME_BYTES
        {
            return Err(CoreError::Validation(context()));
        }
        Ok(Self {
            instances: Vec::with_capacity(limits.instances),
            issuer: Rc::new(()),
            limits,
            actions: None,
        })
    }

    /// Funding estimate for controlled registry/instance layouts, including Rc
    /// counters. Caller adds its description, behavior and indirect backing.
    pub fn publication_backing(limits: RawProducerRegistryLimits) -> Option<usize> {
        if limits.instances > MAX_INSTANCES {
            return None;
        }
        Some(
            size_of::<Self>()
                + limits.instances * size_of::<InstanceEntry>()
                + size_of::<RawProducerInstance>()
                + size_of::<RawFiniteBehavior>()
                + size_of::<ProducerPublication>()
                + size_of::<Cell<bool>>()
                + size_of::<Cell<bool>>()
                + size_of::<[usize; 8]>()
                + 2 * MAX_PRODUCER_PARTICIPANTS * size_of::<Weak<()>>(),
        )
    }

    pub fn description(&self, key: ProducerInstanceKey) -> Option<&str> {
        self.instances
            .iter()
            .find(|e| e.current.get() && e.instance.key() == key)
            .and_then(|e| e.instance.description.as_deref())
    }

    fn publication_index(
        &self,
        key: ProducerInstanceKey,
        expected: Option<ProducerInstanceKey>,
    ) -> CoreResult<usize> {
        let existing = self
            .instances
            .iter()
            .position(|e| e.instance.key().slot().slot() == key.slot().slot());
        match (existing, expected) {
            (None, None) if self.instances.len() < self.limits.instances => {
                Ok(self.instances.len())
            }
            (Some(i), Some(old)) if self.instances[i].instance.key() == old && old != key => Ok(i),
            _ => Err(CoreError::StaleHandle(context())),
        }
    }

    /// Passive bounded validation against this logical owner's actual registry
    /// and behavior envelope, before traversing caller-provisioned collections.
    pub fn check_publication_input(
        &self,
        instance: &RawProducerInstance,
        expected: Option<ProducerInstanceKey>,
    ) -> CoreResult<()> {
        if !Self::valid_instance(instance, self.limits)
            || (!instance.actions.is_empty() && self.actions.is_none())
            || instance.description.is_none()
        {
            return Err(CoreError::Validation(context()));
        }
        self.publication_index(instance.key(), expected)?;
        Ok(())
    }

    /// Pure staging, with all allocations and behavior validation before native
    /// setup. Refusal returns the original authority/behavior/description owner.
    pub fn prepare_publication(
        &self,
        instance: RawProducerInstance,
        expected: Option<ProducerInstanceKey>,
    ) -> Result<ProducerPublication, BindingInputRejection<RawProducerInstance>> {
        if self.check_publication_input(&instance, expected).is_err() {
            return Err(rejection(instance));
        }
        let key = instance.key();
        Ok(ProducerPublication {
            instance: Some(Rc::new(instance)),
            current: Rc::new(Cell::new(false)),
            issuer: Rc::downgrade(&self.issuer),
            expected,
            retired: None,
            key,
            committed: false,
            cancelled: Cell::new(false),
            participants: Vec::with_capacity(MAX_PRODUCER_PARTICIPANTS),
            registry_participants: Vec::with_capacity(MAX_PRODUCER_PARTICIPANTS),
        })
    }

    /// Final passive checks precede the non-fallible local commit. The caller
    /// must already have qualified the final description against this exact
    /// closed participant. No Binding/service/handler callback, allocation or
    /// old-instance destruction occurs inside the commit below.
    pub fn publish<B: FiniteIngressBinding>(
        &mut self,
        publication: &mut ProducerPublication,
        runtime: &mut RawFiniteRuntime<B>,
    ) -> CoreResult<()> {
        self.publish_composed(publication, &mut [runtime])
    }

    /// All independently installed native participants must be ready at the
    /// same boundary. Neither a Binding nor an individual Form owns a second
    /// logical registry. The caller checks complete final support, collision
    /// and endpoint facts before this non-fallible local commit.
    pub fn publish_composed(
        &mut self,
        publication: &mut ProducerPublication,
        participants: &mut [&mut dyn ClosedProducerParticipation],
    ) -> CoreResult<()> {
        if publication.committed
            || publication.cancelled.get()
            || publication.instance.is_none()
            || publication.issuer.as_ptr() != Rc::as_ptr(&self.issuer)
            || participants.is_empty()
            || participants.len() > MAX_PRODUCER_PARTICIPANTS
            || !participants
                .iter()
                .all(|p| p.ready_for(self, publication.key))
        {
            return Err(CoreError::Validation(context()));
        }
        let index = self.publication_index(publication.key, publication.expected)?;
        if index == self.instances.len() && self.instances.len() == self.instances.capacity() {
            return Err(CoreError::Backpressure(context()));
        }
        publication.record_participants(participants.iter().map(|p| p.participation_id()))?;
        let entry = InstanceEntry {
            instance: Rc::clone(publication.instance.as_ref().expect("uncommitted owner")),
            current: Rc::clone(&publication.current),
            participations: core::mem::take(&mut publication.registry_participants),
        };
        // COMMIT: exclusive &mut access orders description, behavior, current
        // logical permission and native participation opening. Capacity was preallocated.
        if index == self.instances.len() {
            self.instances.push(entry);
        } else {
            self.instances[index].current.set(false);
            publication.retired = Some(core::mem::replace(&mut self.instances[index], entry));
            // Fixed-size bounded Action scan, no application callback. The
            // original invocation still owns its work and late settlement.
            if let Some(actions) = &mut self.actions {
                actions.revoke(publication.expected.expect("replacement checked"));
            }
        }
        publication.current.set(true);
        publication.committed = true;
        for participant in participants {
            participant.open();
        }
        Ok(())
    }

    /// Explicit acknowledged local retirement, after native cleanup and result
    /// acknowledgement. Removing an old incarnation cannot remove a successor.
    pub fn retire_publication<B: FiniteIngressBinding>(
        &mut self,
        publication: &mut ProducerPublication,
        runtime: &RawFiniteRuntime<B>,
    ) -> CoreResult<()> {
        if runtime.issuer.as_ptr() != Rc::as_ptr(&self.issuer) {
            return Err(CoreError::StaleHandle(context()));
        }
        if publication.participants.is_empty() {
            publication.bind_participants(&[runtime])?;
        }
        self.retire_composed(publication, &[runtime])
    }

    /// Require exactly the complete original set, with no duplicates, foreign
    /// leases or substitute objects. Native disposal and Action acknowledgement
    /// are independent obligations; neither alone permits logical retirement.
    pub fn retire_composed(
        &mut self,
        publication: &mut ProducerPublication,
        participants: &[&dyn ClosedProducerParticipation],
    ) -> CoreResult<()> {
        if publication.issuer.as_ptr() != Rc::as_ptr(&self.issuer)
            || participants.is_empty()
            || !publication.same_participants(participants.iter().map(|p| p.participation_id()))
            || !participants.iter().all(|p| p.retired())
            || self
                .actions
                .as_ref()
                .is_some_and(|a| a.retains_instance(publication.key))
        {
            return Err(CoreError::Validation(context()));
        }
        publication.revoke();
        if let Some(i) = self
            .instances
            .iter()
            .position(|e| e.instance.key() == publication.key)
        {
            self.instances.remove(i);
        }
        publication.instance = None;
        publication.release_retired();
        Ok(())
    }

    /// Reclaim a revoked composed record after its publication/exposure guard
    /// was lost. Every original native owner must have actually disappeared,
    /// and continuing Actions/results must have been explicitly acknowledged.
    /// A diagnostic key alone can never retire a surviving participation.
    pub fn retire_revoked_publication(
        &mut self,
        key: ProducerInstanceKey,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        budget
            .consume(
                WorkClass::CleanupItems,
                (MAX_INSTANCES + MAX_PRODUCER_PARTICIPANTS + 128) as u64,
            )
            .map_err(|_| CoreError::Backpressure(context()))?;
        let index = self
            .instances
            .iter()
            .position(|e| e.instance.key() == key)
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        let entry = &self.instances[index];
        if entry.current.get()
            || entry.participations.is_empty()
            || entry.participations.iter().any(|p| p.strong_count() != 0)
            || self
                .actions
                .as_ref()
                .is_some_and(|a| a.retains_instance(key))
        {
            return Err(CoreError::Backpressure(context()));
        }
        self.instances.remove(index);
        Ok(())
    }
}
