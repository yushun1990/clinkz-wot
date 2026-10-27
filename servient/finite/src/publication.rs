//! Source-free Servient-local publication. Qualification belongs to Planning;
//! this module owns the actual description/authority/behavior linearization.
use super::*;

/// Preallocated unpublished instance and its one logical permission cell.
/// After commit, Drop revokes only this incarnation. Retained work still pins
/// its original instance; it cannot inherit fresh acceptance or shut down I/O.
pub struct ProducerPublication {
    instance: Option<Rc<RawProducerInstance>>,
    current: Rc<Cell<bool>>,
    issuer: Weak<()>,
    expected: Option<ProducerInstanceKey>,
    retired: Option<InstanceEntry>,
    key: ProducerInstanceKey,
    committed: bool,
    cancelled: Cell<bool>,
}

impl ProducerPublication {
    pub fn key(&self) -> ProducerInstanceKey {
        self.key
    }
    pub fn is_committed(&self) -> bool {
        self.committed
    }
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
        self.instance.as_ref().is_some_and(|i| {
            i.behaviors.iter().all(|b| match &b.target {
                AffordanceTarget::Property(name) => advertised(
                    clinkz_wot_core::thing::AffordanceSite::Property(name),
                    b.operation,
                ),
                _ => false,
            }) && i.actions.iter().all(|a| {
                [
                    Operation::InvokeAction,
                    Operation::QueryAction,
                    Operation::CancelAction,
                ]
                .into_iter()
                .filter(|op| a.supports(*op))
                .all(|op| advertised(clinkz_wot_core::thing::AffordanceSite::Action(a.name()), op))
            })
        })
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
    fn ready_for(&self, owner: &RawProducerServient, _key: ProducerInstanceKey) -> bool {
        self.issuer.as_ptr() == Rc::as_ptr(&owner.issuer)
            && self.state == RawFiniteState::InstalledClosed
            && self.pair_is_current()
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
                + size_of::<[usize; 8]>(),
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

    /// Pure staging, with all allocations and behavior validation before native
    /// setup. Refusal returns the original authority/behavior/description owner.
    pub fn prepare_publication(
        &self,
        instance: RawProducerInstance,
        expected: Option<ProducerInstanceKey>,
    ) -> Result<ProducerPublication, BindingInputRejection<RawProducerInstance>> {
        if !Self::valid_instance(&instance, self.limits)
            || (!instance.actions.is_empty() && self.actions.is_none())
            || instance.description.is_none()
            || self.publication_index(instance.key(), expected).is_err()
        {
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
        let entry = InstanceEntry {
            instance: Rc::clone(publication.instance.as_ref().expect("uncommitted owner")),
            current: Rc::clone(&publication.current),
        };
        // COMMIT: exclusive &mut access orders description, behavior, current
        // logical permission and native participation opening. Capacity was preallocated.
        if index == self.instances.len() {
            self.instances.push(entry);
        } else {
            self.instances[index].current.set(false);
            publication.retired = Some(core::mem::replace(&mut self.instances[index], entry));
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
        self.retire_composed(publication, &[runtime])
    }

    /// Caller must supply every original native participant, not an arbitrary
    /// subset of retired leases. This manual facet does not retain or audit the
    /// original set, and is not a funded aggregate native-owner account.
    pub fn retire_composed(
        &mut self,
        publication: &mut ProducerPublication,
        participants: &[&dyn ClosedProducerParticipation],
    ) -> CoreResult<()> {
        if publication.issuer.as_ptr() != Rc::as_ptr(&self.issuer)
            || participants.is_empty()
            || !participants.iter().all(|p| p.retired())
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
}
