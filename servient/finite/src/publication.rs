//! Source-free Servient-local publication. Qualification belongs to Planning;
//! this module owns the actual description/authority/behavior linearization.
use super::*;
use clinkz_wot_core::action::{ActionRoute, ActionValidator};
pub use clinkz_wot_core::publication::{
    ClosedProducerParticipation, ProducerIngressGate, ProducerOwnerToken, ProducerParticipationId,
    ProducerRetirement,
};
use clinkz_wot_core::publication::{PublicationFacts, PublicationObserver};

/// Bounded original-membership storage for this manually driven Host slice.
pub const MAX_PRODUCER_PARTICIPANTS: usize = 32;

fn check_member_count(count: usize) -> CoreResult<()> {
    if count == 0 {
        return Err(CoreError::Validation(context()));
    }
    if count > MAX_PRODUCER_PARTICIPANTS {
        return Err(CoreError::LimitExceeded {
            resource: clinkz_wot_foundation::ResourceKind::RouteReadinessTokensPerThingMax,
            limit: MAX_PRODUCER_PARTICIPANTS as u64,
            requested: Some(count as u64),
            observed: None,
            context: context(),
        });
    }
    Ok(())
}

/// Qualified, source-free Action mapping. Invoke routes carry their actual
/// validation service; query/cancel routes establish the permitted controls.
/// Native participants supply these during setup, not individual requests.
pub struct CapturedActionRoute {
    pub target: AffordanceTarget,
    pub operation: Operation,
    pub route: ActionRoute,
    pub validator: Option<Rc<dyn ActionValidator>>,
    /// Original extra work per validation callback; zero for control routes.
    pub validation_work: u64,
}

/// Source-free route and service owners supplied by the installed Bindings.
pub struct CapturedProducerRoutes {
    pub actions: Vec<CapturedActionRoute>,
    pub notifications: Vec<CapturedNotificationRoute>,
}

impl CapturedActionRoute {
    pub fn backing_bytes(routes: &[Self], capacity: usize) -> Option<usize> {
        routes
            .iter()
            .try_fold(capacity.checked_mul(size_of::<Self>())?, |sum, route| {
                sum.checked_add(route.target.name().map_or(0, str::len) + 2 * size_of::<usize>())
            })
    }
}

// Shared by every authority decision for one local incarnation. Weak references
// preserve independent native lifetimes, including when an owner leaves its facade.
pub(super) struct PublicationPermission {
    active: Cell<bool>,
    members: Vec<Weak<()>>,
    gates: Vec<Weak<ProducerIngressGate>>,
}

impl PublicationPermission {
    pub(super) fn raw() -> Self {
        Self {
            active: Cell::new(true),
            members: Vec::new(),
            gates: Vec::new(),
        }
    }

    fn staged(members: usize) -> Self {
        Self {
            active: Cell::new(false),
            members: Vec::with_capacity(members),
            gates: Vec::with_capacity(members),
        }
    }

    pub(super) fn is_current(&self) -> bool {
        if self.active.get()
            && (self.members.iter().any(|id| id.strong_count() == 0)
                || self
                    .gates
                    .iter()
                    .any(|gate| gate.upgrade().is_none_or(|g| !g.is_open())))
        {
            self.revoke();
        }
        self.active.get()
    }

    pub(super) fn check_work(&self) -> usize {
        // Validate the original identities/gates and allow for withdrawal.
        1 + self.members.len() + 2 * self.gates.len()
    }

    pub(super) fn revoke(&self) {
        if !self.active.replace(false) {
            return;
        }
        for gate in self.gates.iter().filter_map(Weak::upgrade) {
            gate.withdraw();
        }
    }
}

/// Preallocated unpublished instance and its shared logical permission.
/// After commit, Drop revokes only this incarnation. Retained work still pins
/// its original instance and requests cancellation on its next funded step;
/// it cannot inherit fresh acceptance or shut down I/O.
pub struct ProducerPublication {
    permission: Rc<PublicationPermission>,
    issuer: Weak<()>,
    expected: Option<ProducerInstanceKey>,
    key: ProducerInstanceKey,
    committed: bool,
    cancelled: Cell<bool>,
    action_routes: Option<Vec<CapturedActionRoute>>,
    notification_routes: Option<Vec<CapturedNotificationRoute>>,
    // Both original instances retain capacity until publication metadata ends.
    retired: Option<InstanceEntry>,
    instance: Option<Rc<RawProducerInstance>>,
}

impl ProducerPublication {
    pub fn staging_backing_bytes() -> usize {
        Self::staging_backing_for(MAX_PRODUCER_PARTICIPANTS)
    }

    /// Controlled staging layout for a known membership. A raw publication
    /// with deferred membership provisions the full manual Host envelope.
    pub fn staging_backing_for(members: usize) -> usize {
        let members = members.min(MAX_PRODUCER_PARTICIPANTS);
        size_of::<Self>()
            + members * (size_of::<Weak<()>>() + size_of::<Weak<ProducerIngressGate>>())
            + size_of::<PublicationPermission>()
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
    pub fn expected_incumbent(&self) -> Option<ProducerInstanceKey> {
        self.expected
    }
    /// Bind the actual managed result bank before publication. Every admitted
    /// Action must acquire its own result/cleanup lease before behavior entry.
    /// A retry cannot substitute another account for the original instance.
    pub fn bind_action_result_accounts(
        &mut self,
        accounts: Arc<clinkz_wot_core::host_accounts::HostResourceAccounts>,
    ) -> CoreResult<()> {
        if self.committed || self.cancelled.get() {
            return Err(CoreError::Validation(context()));
        }
        let instance = self
            .instance
            .as_mut()
            .and_then(Rc::get_mut)
            .ok_or_else(|| CoreError::Validation(context()))?;
        if instance
            .action_result_accounts
            .as_ref()
            .is_some_and(|old| !Arc::ptr_eq(old, &accounts))
        {
            return Err(CoreError::Validation(context()));
        }
        instance.action_result_accounts = Some(accounts);
        Ok(())
    }
    /// Prepay destruction when the publication first takes each table. Commit
    /// moves these same rows to the registry, which may drop before the facade.
    /// Requalification compares against the originals without adopting again.
    fn bind_routes(
        &mut self,
        routes: CapturedProducerRoutes,
        mut lifetime: Option<&mut WorkBudget>,
    ) -> CoreResult<()> {
        let mut rows = 0;
        if self.action_routes.is_none() {
            rows += routes.actions.len();
        }
        if self.notification_routes.is_none() {
            rows += routes.notifications.len();
        }
        pay_cleanup(&mut lifetime, rows)?;
        self.bind_action_routes(routes.actions)?;
        self.bind_notification_routes(routes.notifications)
    }

    /// Capture the original notification publication mapping, validation and security
    /// services. Requalification cannot replace any one of these independently.
    /// Raw callers transfer their provisioned vector. Managed admission reserves
    /// its slots and backing before binding the complete set.
    pub fn bind_notification_routes(
        &mut self,
        routes: Vec<CapturedNotificationRoute>,
    ) -> CoreResult<()> {
        if self.committed || self.cancelled.get() {
            return Err(CoreError::Validation(context()));
        }
        if let Some(old) = &self.notification_routes {
            // The originally bound table already covers the immutable source
            // set. A retry must reproduce every route and service in order.
            if old.len() != routes.len()
                || old.iter().zip(&routes).any(|(a, b)| {
                    a.target != b.target
                        || a.kind != b.kind
                        || !a.route.same_mapping(&b.route)
                        || !Rc::ptr_eq(&a.services, &b.services)
                        || a.data_bytes != b.data_bytes
                })
            {
                return Err(CoreError::Validation(context()));
            }
            return Ok(());
        }
        let instance = self
            .instance
            .as_ref()
            .ok_or_else(|| CoreError::Validation(context()))?;
        if routes.iter().enumerate().any(|(i, r)| {
            !r.kind.accepts_target(&r.target)
                || !instance.supports_notification(&r.target, r.kind.start_operation())
                || r.data_bytes == 0
                || routes[..i]
                    .iter()
                    .any(|p| p.target == r.target && p.route.same_mapping(&r.route))
        }) || instance.notification_sources.iter().any(|e| {
            !routes
                .iter()
                .any(|r| r.target == e.target() && r.kind == e.kind())
        }) {
            return Err(CoreError::Validation(context()));
        }
        self.notification_routes = Some(routes);
        Ok(())
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

    /// Install actual mappings and invoke services selected by the complete
    /// preparation. A retry may only provide the same routes and services.
    pub fn bind_action_routes(&mut self, routes: Vec<CapturedActionRoute>) -> CoreResult<()> {
        if self.committed || self.cancelled.get() {
            return Err(CoreError::Validation(context()));
        }
        let instance = self
            .instance
            .as_ref()
            .ok_or_else(|| CoreError::Validation(context()))?;
        if routes.iter().enumerate().any(|(i, route)| {
            !instance.supports(&route.target, route.operation)
                || (route.operation == Operation::InvokeAction) != route.validator.is_some()
                || route
                    .validator
                    .as_ref()
                    .map_or(0, |v| v.additional_validation_work())
                    != route.validation_work
                || !matches!(
                    route.operation,
                    Operation::InvokeAction
                        | Operation::QueryAction
                        | Operation::CancelAction
                        | Operation::QueryAllActions
                )
                || routes[..i].iter().any(|prior| {
                    prior.target == route.target
                        && prior.operation == route.operation
                        && prior.route == route.route
                })
        }) || (instance.action_query.is_some()
            && !routes.iter().any(|r| {
                r.target == AffordanceTarget::Thing && r.operation == Operation::QueryAllActions
            }))
            || instance.actions.iter().any(|action| {
                !routes.iter().any(|route| {
                    route.target.name() == Some(action.name())
                        && route.operation == Operation::InvokeAction
                })
            })
        {
            return Err(CoreError::Validation(context()));
        }
        if let Some(old) = &self.action_routes {
            if old.len() != routes.len()
                || old.iter().zip(&routes).any(|(a, b)| {
                    a.target != b.target
                        || a.operation != b.operation
                        || a.route != b.route
                        || a.validation_work != b.validation_work
                        || !match (&a.validator, &b.validator) {
                            (None, None) => true,
                            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                            _ => false,
                        }
                })
            {
                return Err(CoreError::Validation(context()));
            }
        } else {
            self.action_routes = Some(routes);
        }
        Ok(())
    }

    fn same_participants<'a>(
        &self,
        ids: impl ExactSizeIterator<Item = &'a ProducerParticipationId> + Clone,
    ) -> bool {
        ids.len() == self.permission.members.len()
            && ids.clone().enumerate().all(|(i, id)| {
                !ids.clone().take(i).any(|p| p.same_owner(id))
                    && self.permission.members.iter().any(|p| id.matches(p))
            })
    }

    fn record_participants<'a>(
        &mut self,
        ids: impl ExactSizeIterator<Item = &'a ProducerParticipationId> + Clone,
    ) -> CoreResult<()> {
        check_member_count(ids.len())?;
        if !self.permission.members.is_empty() {
            return if self.same_participants(ids) {
                Ok(())
            } else {
                Err(CoreError::Validation(context()))
            };
        }
        if ids
            .clone()
            .enumerate()
            .any(|(i, id)| ids.clone().take(i).any(|prior| prior.same_owner(id)))
        {
            return Err(CoreError::Validation(context()));
        }
        Rc::get_mut(&mut self.permission)
            .expect("unpublished permission")
            .members
            .extend(ids.map(ProducerParticipationId::downgrade));
        Ok(())
    }

    /// Fence fresh acceptance immediately. Already accepted Actions observe
    /// this permission and request cancellation during funded progress; no
    /// application callback or native shutdown occurs here.
    pub fn revoke(&self) {
        self.cancelled.set(true);
        self.permission.revoke();
    }
    pub fn staged_description(&self) -> Option<&str> {
        self.instance.as_ref()?.description.as_deref()
    }
    pub fn behavior_count(&self) -> usize {
        self.instance.as_ref().map_or(0, |i| i.behavior_count())
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
    pub fn supports_event(&self, name: &str, operation: Operation) -> bool {
        self.instance
            .as_ref()
            .is_some_and(|i| i.supports_event(name, operation))
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

impl InstanceEntry {
    fn displacement_items(&self, last_instance_owner: bool) -> usize {
        let routes = self.notification_routes.as_ref().map_or(0, Vec::len)
            + self.action_routes.as_ref().map_or(0, Vec::len);
        // The instance already owns lifetime disposal of its notification sources.
        let handlers = if last_instance_owner {
            self.instance.behavior_count() - self.instance.notification_sources.len()
        } else {
            0
        };
        routes + handlers
    }
}

// A finite runtime contributes the passive Core publication facet.
impl<B: FiniteIngressBinding> ClosedProducerParticipation for RawFiniteRuntime<B> {
    fn qualified_operation_count(&self) -> Option<usize> {
        self.binding.qualified_operation_count()
    }
    fn observe_installation(
        &self,
        facts: &mut PublicationObserver<'_>,
    ) -> clinkz_wot_core::CoreResult<()> {
        self.binding.observe_installation(facts)?;
        self.binding.check_ingress()
    }

    fn associated_with(&self, owner: &ProducerOwnerToken, key: ProducerInstanceKey) -> bool {
        owner.matches_issuer(&self.issuer) && self.binding.prepared_for_instance(key)
    }
    fn native_failure(&self) -> Option<&CoreError> {
        self.first_cause.as_ref()
    }
    fn participation_id(&self) -> &ProducerParticipationId {
        &self.participation
    }
    fn retain_owner(&mut self, owner: Rc<dyn core::any::Any>) {
        self.retained_owner = Some(Rc::new((self.retained_owner.take(), owner)));
    }
    fn admission_backing_bytes(
        &self,
        owner: &ProducerOwnerToken,
        key: ProducerInstanceKey,
    ) -> Option<usize> {
        if self.state != RawFiniteState::Starting
            || self.native_started
            || self.retained_owner.is_some()
            || self.validation != FiniteValidation::Required
            || !owner.matches_issuer(&self.issuer)
            || !self.pair_is_current()
            || !self.binding.prepared_for_instance(key)
        {
            return None;
        }
        let persistent = size_of::<Self>()
            .checked_add(2 * size_of::<usize>())?
            .checked_add(self.binding.engine_backing_bytes()?)?;
        if self.capacity.is_some() {
            // Actual shared work/result/cleanup backing is reserved at its
            // original work bank before receive, not once per publication.
            return Some(persistent);
        }
        persistent
            .checked_add(
                self.limits
                    .candidates
                    .checked_mul(size_of::<clinkz_wot_core::finite::FiniteCandidate>())?,
            )?
            .checked_add(self.limits.input_body_bytes)?
            .checked_add(self.limits.output_retained_bytes)
    }
    fn begin_retire(&mut self, intent: ProducerRetirement) {
        if intent == ProducerRetirement::Drain && self.state == RawFiniteState::Delivering {
            self.ingress_gate.withdraw();
        } else {
            self.begin_close();
        }
    }
    fn ready_for(&self, owner: &ProducerOwnerToken, key: ProducerInstanceKey) -> bool {
        // A captured service cannot qualify dispatch that skips validation.
        // Explicitly unvalidated execution remains at activate_raw().
        owner.matches_issuer(&self.issuer)
            && self.state == RawFiniteState::InstalledClosed
            && self.validation == FiniteValidation::Required
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
    fn ingress_gate(&self) -> &Rc<ProducerIngressGate> {
        &self.ingress_gate
    }
    fn retired(&self) -> bool {
        self.state == RawFiniteState::Closed
            && self.settlement.is_none()
            && self.completed.is_empty()
    }
}

impl RawProducerServient {
    /// Reconcile native evidence at the logical owner. Only a committed member's
    /// failure fences this incarnation; uncommitted owners may still retire
    /// their own native work without affecting a publication or accepted Actions.
    pub fn reconcile_participation(
        &mut self,
        key: ProducerInstanceKey,
        participant: &mut dyn super::ClosedProducerParticipation,
    ) -> CoreResult<()> {
        if !participant.associated_with(&self.participation_token(), key) {
            return Err(CoreError::StaleHandle(context()));
        }
        let failed = participant.native_failure().is_some();
        let gate = participant.ingress_gate();
        if failed || gate.is_withdrawn() {
            let id = participant.participation_id();
            let committed_member = self.instances.iter().any(|entry| {
                entry.instance.key() == key
                    && entry
                        .permission
                        .members
                        .iter()
                        .any(|member| id.matches(member))
            });
            if committed_member {
                self.revoke(key);
            }
            // The original gate carries withdrawal even after another
            // incarnation has taken this slot. Retirement is local to this member.
            participant.begin_retire(ProducerRetirement::Drain);
        } else if gate.is_open() && self.description(key).is_none() {
            participant.begin_retire(ProducerRetirement::Drain);
        }
        Ok(())
    }

    pub fn participation_token(&self) -> ProducerOwnerToken {
        ProducerOwnerToken::new(&self.issuer)
    }
    /// Preback the logical registry before any native setup. Empty does not
    /// confer acceptance, unlike constructing an already active raw registry.
    pub fn try_empty(limits: RawProducerRegistryLimits) -> CoreResult<Self> {
        if !(1..=MAX_INSTANCES).contains(&limits.instances)
            || limits.behaviors_per_instance == 0
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
                + size_of::<PublicationPermission>()
                + size_of::<ProducerIngressGate>()
                + size_of::<[usize; 8]>()
                + MAX_PRODUCER_PARTICIPANTS * size_of::<Weak<()>>()
                + MAX_PRODUCER_PARTICIPANTS * size_of::<Weak<ProducerIngressGate>>(),
        )
    }

    pub fn description(&self, key: ProducerInstanceKey) -> Option<&str> {
        self.instances
            .iter()
            .find(|e| e.instance.key() == key && e.permission.is_current())
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
            || ((!instance.actions.is_empty() || instance.action_query.is_some())
                && self.actions.is_none())
            || instance.description.is_none()
        {
            return Err(CoreError::Validation(context()));
        }
        self.publication_index(instance.key(), expected)?;
        Ok(())
    }

    /// Check the largest indivisible cleanup pass before closed native setup.
    /// The registry can fill or an incumbent can lose its last external owner
    /// between staging and cutover. The incumbent's route table can only shrink.
    pub fn check_cleanup_quantum(
        &self,
        key: ProducerInstanceKey,
        expected: Option<ProducerInstanceKey>,
        members: usize,
        quantum: u64,
    ) -> CoreResult<()> {
        check_member_count(members)?;
        let index = self.publication_index(key, expected)?;
        let displaced = self
            .instances
            .get(index)
            .map_or(0, |e| e.displacement_items(true));
        // Find the entry, inspect its original members and, once, withdraw
        // their gates. Later turns can always drain a row or remove the entry.
        let retirement = self.limits.instances + 2 * members;
        let required = retirement.max(displaced) as u64;
        if required > quantum {
            return Err(CoreError::LimitExceeded {
                resource: clinkz_wot_foundation::ResourceKind::CleanupWorkItemsPerStepMax,
                limit: quantum,
                requested: Some(required),
                observed: None,
                context: context().with_thing(key.slot()),
            });
        }
        Ok(())
    }

    /// Pure staging, with all allocations and behavior validation before native
    /// setup. Refusal returns the original authority/behavior/description owner.
    pub fn prepare_publication(
        &self,
        instance: RawProducerInstance,
        expected: Option<ProducerInstanceKey>,
    ) -> Result<ProducerPublication, BindingInputRejection<RawProducerInstance>> {
        self.stage_publication(instance, expected, MAX_PRODUCER_PARTICIPANTS)
    }

    fn stage_publication(
        &self,
        instance: RawProducerInstance,
        expected: Option<ProducerInstanceKey>,
        members: usize,
    ) -> Result<ProducerPublication, BindingInputRejection<RawProducerInstance>> {
        if let Err(error) = check_member_count(members)
            .and_then(|()| self.check_publication_input(&instance, expected))
        {
            return Err(BindingInputRejection::new(
                instance,
                BindingOperationalError::new(error),
            ));
        }
        let key = instance.key();
        Ok(ProducerPublication {
            instance: Some(Rc::new(instance)),
            permission: Rc::new(PublicationPermission::staged(members)),
            issuer: Rc::downgrade(&self.issuer),
            expected,
            retired: None,
            key,
            committed: false,
            cancelled: Cell::new(false),
            action_routes: None,
            notification_routes: None,
        })
    }

    /// Bind the caller's complete closed membership and prepay the staged
    /// routes' lifetime disposal before ownership transfer. A linked validator
    /// may change its work declaration during capture. Return the original
    /// instance on refusal.
    pub fn prepare_composed_publication(
        &self,
        instance: RawProducerInstance,
        expected: Option<ProducerInstanceKey>,
        participants: &[&dyn ClosedProducerParticipation],
        routes: CapturedProducerRoutes,
        lifetime: &mut WorkBudget,
    ) -> Result<ProducerPublication, BindingInputRejection<RawProducerInstance>> {
        let mut publication = self.stage_publication(instance, expected, participants.len())?;
        if let Err(error) = publication
            .bind_participants(participants)
            .and_then(|()| publication.bind_routes(routes, Some(lifetime)))
        {
            let instance = publication
                .instance
                .take()
                .expect("new unpublished instance");
            let instance =
                Rc::into_inner(instance).expect("staging owns the only instance reference");
            return Err(BindingInputRejection::new(
                instance,
                BindingOperationalError::new(error),
            ));
        }
        Ok(publication)
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
        self.publish_if_current(
            publication,
            participants,
            &PublicationFacts::default(),
            |_, _| Ok(None),
        )
    }

    /// Source-free installation of caller-qualified coverage. All observations
    /// supplied by preparation and the closed native owners must still hold
    /// together after the final linked callback and through the local commit.
    pub fn publish_if_current(
        &mut self,
        publication: &mut ProducerPublication,
        participants: &mut [&mut dyn ClosedProducerParticipation],
        coverage: &PublicationFacts,
        capture: impl FnOnce(
            ProducerInstanceKey,
            &[&mut dyn ClosedProducerParticipation],
        ) -> CoreResult<Option<CapturedProducerRoutes>>,
    ) -> CoreResult<()> {
        self.publish_if_current_with_cleanup(publication, participants, coverage, None, capture)
    }

    /// A budgeted caller funds newly adopted route rows and disposal of the
    /// displaced records before changing either incarnation's gates. Unmetered
    /// raw callers provide their own cleanup.
    pub fn publish_if_current_with_cleanup(
        &mut self,
        publication: &mut ProducerPublication,
        participants: &mut [&mut dyn ClosedProducerParticipation],
        coverage: &PublicationFacts,
        mut cleanup: Option<&mut WorkBudget>,
        capture: impl FnOnce(
            ProducerInstanceKey,
            &[&mut dyn ClosedProducerParticipation],
        ) -> CoreResult<Option<CapturedProducerRoutes>>,
    ) -> CoreResult<()> {
        if publication.committed
            || publication.cancelled.get()
            || publication.instance.is_none()
            || publication.issuer.as_ptr() != Rc::as_ptr(&self.issuer)
        {
            return Err(CoreError::Validation(context()));
        }
        check_member_count(participants.len())?;
        let mut installation = PublicationFacts::default();
        for participant in participants.iter() {
            participant.observe_installation(&mut installation.observer())?;
        }
        if let Some(routes) = capture(publication.key, participants)? {
            publication.bind_routes(routes, cleanup.as_deref_mut())?;
        }
        if !participants
            .iter()
            .all(|p| p.ready_for(&self.participation_token(), publication.key))
        {
            return Err(CoreError::Validation(context()));
        }
        let index = self.publication_index(publication.key, publication.expected)?;
        if index == self.instances.len() && self.instances.len() == self.instances.capacity() {
            return Err(CoreError::Backpressure(context()));
        }
        publication.record_participants(participants.iter().map(|p| p.participation_id()))?;
        if publication
            .instance
            .as_ref()
            .is_some_and(|i| !i.actions.is_empty() || i.action_query.is_some())
            && publication.action_routes.is_none()
        {
            return Err(CoreError::Validation(context()));
        }
        if publication
            .instance
            .as_ref()
            .is_some_and(|i| !i.notification_sources.is_empty())
            && publication.notification_routes.is_none()
        {
            return Err(CoreError::Validation(context()));
        }
        // Gather gates before the local linearization boundary. Every linked
        // callback and fallible allocation precedes the commit.
        let mut gates = Vec::new();
        gates
            .try_reserve_exact(participants.len())
            .map_err(|_| CoreError::Backpressure(context()))?;
        gates.extend(participants.iter().map(|p| Rc::clone(p.ingress_gate())));
        if gates.iter().enumerate().any(|(i, gate)| {
            !gate.is_staged() || gates[..i].iter().any(|prior| Rc::ptr_eq(prior, gate))
        }) {
            return Err(CoreError::Validation(context()));
        }
        let _current = PublicationFacts::lock_current(&[coverage, &installation])?;
        if publication.cancelled.get() {
            return Err(CoreError::Cancelled(context()));
        }
        let incumbent = self.instances.get(index);
        let action_store = self
            .actions
            .as_mut()
            .filter(|_| incumbent.is_some_and(|e| e.instance.action_entries.get() != 0));
        if let (Some(entry), Some(budget)) = (incumbent, cleanup) {
            let records = entry.displacement_items(Rc::strong_count(&entry.instance) == 1);
            let cutover = entry.permission.gates.len()
                + action_store
                    .as_deref()
                    .map_or(0, ActionStore::retained_count);
            budget
                .consume(WorkClass::HandlerSteps, cutover as u64)
                .map_err(|_| CoreError::Backpressure(context()))?;
            budget
                .consume(WorkClass::CleanupItems, records as u64)
                .map_err(|_| CoreError::Backpressure(context()))?;
        }
        Rc::get_mut(&mut publication.permission)
            .expect("unpublished permission")
            .gates
            .extend(gates.iter().map(Rc::downgrade));
        let entry = InstanceEntry {
            instance: Rc::clone(publication.instance.as_ref().expect("uncommitted owner")),
            permission: Rc::clone(&publication.permission),
            action_routes: publication.action_routes.take(),
            notification_routes: publication.notification_routes.take(),
        };
        // COMMIT: exclusive &mut access orders description, behavior, current
        // logical permission and native participation opening. Capacity was preallocated.
        if index == self.instances.len() {
            self.instances.push(entry);
        } else {
            self.instances[index].permission.revoke();
            publication.retired = Some(core::mem::replace(&mut self.instances[index], entry));
            // Fixed-size bounded Action scan, no application callback. The
            // original invocation still owns its work and late settlement.
            if let Some(actions) = action_store {
                actions.revoke(publication.expected.expect("replacement checked"));
            }
        }
        publication.permission.active.set(true);
        publication.committed = true;
        for gate in gates {
            gate.open();
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
        if publication.permission.members.is_empty() {
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
        self.retire_members(publication, participants.iter().copied(), None)
    }

    /// Managed retirement checks each member in binding order. A manual caller
    /// supplying another order also funds verification of the complete set.
    pub fn retire_composed_with_budget<'a>(
        &mut self,
        publication: &mut ProducerPublication,
        participants: impl ExactSizeIterator<Item = &'a dyn ClosedProducerParticipation> + Clone,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        self.retire_members(publication, participants, Some(budget))
    }

    fn retire_members<'a>(
        &mut self,
        publication: &mut ProducerPublication,
        participants: impl ExactSizeIterator<Item = &'a dyn ClosedProducerParticipation> + Clone,
        mut budget: Option<&mut WorkBudget>,
    ) -> CoreResult<()> {
        let members = &publication.permission.members;
        if publication.issuer.as_ptr() != Rc::as_ptr(&self.issuer)
            || members.is_empty()
            || participants.len() != members.len()
        {
            return Err(CoreError::Validation(context()));
        }
        // One visit per bound member in the managed order. A manual caller
        // supplying the same set in another order also pays for that set check.
        pay_cleanup(&mut budget, members.len())?;
        let mut ordered = true;
        for (participant, id) in participants.clone().zip(members) {
            if !participant.retired() {
                return Err(CoreError::Validation(context()));
            }
            ordered &= participant.participation_id().matches(id);
        }
        if !ordered {
            pay_cleanup(&mut budget, 2 * members.len() * members.len())?;
            if !publication.same_participants(participants.map(|p| p.participation_id())) {
                return Err(CoreError::Validation(context()));
            }
        }
        if publication
            .instance
            .as_ref()
            .is_some_and(|i| i.retains_publication_work())
        {
            return Err(CoreError::Validation(context()));
        }
        if publication.permission.active.get() {
            pay_cleanup(&mut budget, publication.permission.gates.len())?;
        }
        publication.revoke();
        pay_cleanup(&mut budget, self.instances.len())?;
        if let Some(i) = self
            .instances
            .iter()
            .position(|e| e.instance.key() == publication.key)
        {
            // Leave unfinished rows with the fenced registry entry. If the
            // facade disappears between turns the original Servient can drain it.
            let entry = &mut self.instances[i];
            drain_routes(&mut entry.notification_routes, &mut budget)?;
            drain_routes(&mut entry.action_routes, &mut budget)?;
            pay_cleanup(&mut budget, 1)?;
            self.instances.swap_remove(i);
        }
        drain_routes(&mut publication.notification_routes, &mut budget)?;
        drain_routes(&mut publication.action_routes, &mut budget)?;
        if let Some(entry) = &mut publication.retired {
            drain_routes(&mut entry.notification_routes, &mut budget)?;
            drain_routes(&mut entry.action_routes, &mut budget)?;
        }
        publication.release_retired();
        publication.instance = None;
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
        let mut budget = Some(budget);
        pay_cleanup(&mut budget, self.instances.len())?;
        let index = self
            .instances
            .iter()
            .position(|e| e.instance.key() == key)
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        let entry = &mut self.instances[index];
        let permission = &entry.permission;
        if permission.members.is_empty() {
            return Err(CoreError::Backpressure(context()));
        }
        pay_cleanup(&mut budget, permission.members.len())?;
        let active = permission.active.get();
        let mut gone = 0;
        let mut lost = false;
        for (id, gate) in permission.members.iter().zip(&permission.gates) {
            let vanished = id.strong_count() == 0;
            gone += usize::from(vanished);
            lost = lost || vanished || (active && gate.upgrade().is_none_or(|g| !g.is_open()));
        }
        if lost && active {
            pay_cleanup(&mut budget, permission.gates.len())?;
            permission.revoke();
        }
        if gone != permission.members.len() || entry.instance.retains_publication_work() {
            return Err(CoreError::Backpressure(context()));
        }
        drain_routes(&mut entry.notification_routes, &mut budget)?;
        drain_routes(&mut entry.action_routes, &mut budget)?;
        pay_cleanup(&mut budget, 1)?;
        self.instances.swap_remove(index);
        Ok(())
    }
}

fn pay_cleanup(budget: &mut Option<&mut WorkBudget>, items: usize) -> CoreResult<()> {
    if let Some(budget) = budget.as_deref_mut() {
        budget
            .consume(WorkClass::CleanupItems, items as u64)
            .map_err(|_| CoreError::Backpressure(context()))?;
    }
    Ok(())
}

/// Remove only records covered by this turn. The vector itself is the cursor.
fn drain_routes<T>(
    routes: &mut Option<Vec<T>>,
    budget: &mut Option<&mut WorkBudget>,
) -> CoreResult<()> {
    if let (Some(records), Some(budget)) = (routes.as_mut(), budget.as_deref_mut()) {
        let count = (records.len() as u64).min(budget.remaining(WorkClass::CleanupItems));
        budget
            .consume(WorkClass::CleanupItems, count)
            .expect("available cleanup credit");
        records.truncate(records.len() - count as usize);
        if !records.is_empty() {
            return Err(CoreError::Backpressure(context()));
        }
    }
    *routes = None;
    Ok(())
}
