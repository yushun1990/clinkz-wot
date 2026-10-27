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
            || !instance.actions.is_empty()
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
        if publication.committed
            || publication.instance.is_none()
            || publication.issuer.as_ptr() != Rc::as_ptr(&self.issuer)
            || runtime.issuer.as_ptr() != Rc::as_ptr(&self.issuer)
            || runtime.state != RawFiniteState::InstalledClosed
            || !runtime.pair_is_current()
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
        // logical permission and runtime opening. Capacity was preallocated.
        if index == self.instances.len() {
            self.instances.push(entry);
        } else {
            self.instances[index].current.set(false);
            publication.retired = Some(core::mem::replace(&mut self.instances[index], entry));
        }
        publication.current.set(true);
        publication.committed = true;
        runtime.state = RawFiniteState::Serving;
        Ok(())
    }

    /// Explicit acknowledged local retirement, after native cleanup and result
    /// acknowledgement. Removing an old incarnation cannot remove a successor.
    pub fn retire_publication<B: FiniteIngressBinding>(
        &mut self,
        publication: &mut ProducerPublication,
        runtime: &RawFiniteRuntime<B>,
    ) -> CoreResult<()> {
        if publication.issuer.as_ptr() != Rc::as_ptr(&self.issuer)
            || runtime.issuer.as_ptr() != Rc::as_ptr(&self.issuer)
            || runtime.state != RawFiniteState::Closed
            || runtime.settlement.is_some()
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
