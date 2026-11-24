//! Application entry for finite Host publication. Deployment supplies linked
//! native services and independent work capacity once. The caller drives time,
//! readiness, publication and retirement on the existing composed owner.
use crate::host_accounts::HostResourceAccounts;
use crate::producer::QualificationFunding;
use crate::producer_composition::{
    ComposedProducerExposure, ComposedProducerInput, FiniteSetupFunding,
    MAX_PRODUCER_REGISTRATIONS, MemberFunding, OwnedProducerParticipation, ProducerStageFunding,
    ProducerStorage, ProducerWork, check_publication_limits, instance_supports,
    qualification_bytes,
};
use alloc::{boxed::Box, rc::Rc, sync::Arc, vec::Vec};
use clinkz_wot_core::{producer::*, publication::ProducerOwnerToken, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::producer_composition::*;
use clinkz_wot_servient_finite::{
    FiniteValidation, ProducerFiniteCapacity, ProducerRevocation, RawFiniteRuntime,
    RawProducerInstance, RawProducerServient,
};
pub use clinkz_wot_servient_finite::{
    RawFiniteBehavior, RawFiniteLimits, RawProducerRegistryLimits,
};
use core::{
    fmt,
    task::{Context, Poll},
};

fn busy() -> CoreError {
    CoreError::Backpressure(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}
fn invalid() -> CoreError {
    CoreError::Validation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}
fn cause<T>(rejection: BindingInputRejection<T>) -> CoreError {
    rejection.into_parts().1.into_parts().1
}

/// One finite admission on the Host. The candidate allowance covers provisional
/// definitions, native assembly, endpoint owners and eventual serving backing.
/// The shared TD and linked callback scratch have a separate reservation.
pub struct HostProducerLimits {
    pub registry: RawProducerRegistryLimits,
    pub exchange: RawFiniteLimits,
    pub active_exchanges: usize,
    pub retained_results: usize,
    pub candidate_bytes: usize,
}

#[derive(Default)]
pub struct ExposurePolicy {
    pub replaces: Option<ProducerInstanceKey>,
    /// Trusted indirect backing of the supplied application handlers.
    pub handler_backing_bytes: usize,
}

trait Service {
    fn capability(self: Rc<Self>) -> Rc<dyn ProducerCapability>;
    fn setup_peak(&self) -> usize;
    fn assemble(
        &self,
        registration: usize,
        prepared: &PreparedThing,
        key: ProducerInstanceKey,
        host: &HostProducer,
    ) -> CoreResult<OwnedProducerParticipation>;
}
impl<S: FiniteProducerService + 'static> Service for S {
    fn setup_peak(&self) -> usize {
        self.setup_peak_bytes()
    }
    fn capability(self: Rc<Self>) -> Rc<dyn ProducerCapability> {
        self
    }
    fn assemble(
        &self,
        registration: usize,
        prepared: &PreparedThing,
        key: ProducerInstanceKey,
        host: &HostProducer,
    ) -> CoreResult<OwnedProducerParticipation> {
        let operations = prepared
            .operations()
            .iter()
            .filter(|o| o.registration() == registration);
        let (binding, config) = FiniteProducerService::assemble(self, key, operations)?;
        let native = RawFiniteRuntime::try_new(
            binding,
            config,
            &host.owner,
            host.config.exchange,
            FiniteValidation::Required,
            Deadline::NONE,
        )
        .map_err(cause)?
        .with_capacity(host.capacity.clone())
        .map_err(cause)?;
        Ok(OwnedProducerParticipation::finite(native))
    }
}

type ServiceRef = Rc<dyn Service>;
type AtBase<L> = Box<dyn Fn(&L) -> CoreResult<ServiceRef>>;

/// One actual startup registration, including its Binding-owned assembly.
/// A base-dependent service specializes its own view of the original closed
/// lease. Fixed competitors stay in the same eligible set for both walks.
pub struct FiniteProducerRegistration<L = ()> {
    draft: ServiceRef,
    at_base: Option<AtBase<L>>,
}
impl<L: 'static> FiniteProducerRegistration<L> {
    pub fn fixed(service: impl FiniteProducerService + 'static) -> Self {
        Self {
            draft: Rc::new(service),
            at_base: None,
        }
    }
    /// Specialization is pure. Its inputs come from the original lease and the
    /// service configuration observed before acquisition. The returned view
    /// reports any additional dependencies used for final qualification.
    /// Backpressure defers this registration on the same lease until the caller retries.
    pub fn at_base<S: FiniteProducerService + 'static>(
        service: S,
        specialize: fn(&S, &L) -> CoreResult<S>,
    ) -> Self {
        let service = Rc::new(service);
        Self {
            draft: service.clone(),
            at_base: Some(Box::new(move |lease| {
                let next = specialize(&service, lease)?;
                if next.identity() != service.identity() {
                    return Err(invalid());
                }
                Ok(Rc::new(next))
            })),
        }
    }
}

/// Keep the concrete lease and its matching registration functions together
/// until every service has had a turn on the acquired endpoint.
trait RegisteredBase: AsRef<str> {
    fn specialize(&self, index: usize) -> CoreResult<ServiceRef>;
}
struct DeploymentBase<L> {
    lease: L,
    registrations: Rc<[FiniteProducerRegistration<L>]>,
}
impl<L: AsRef<str>> AsRef<str> for DeploymentBase<L> {
    fn as_ref(&self) -> &str {
        self.lease.as_ref()
    }
}
impl<L: AsRef<str>> RegisteredBase for DeploymentBase<L> {
    fn specialize(&self, index: usize) -> CoreResult<ServiceRef> {
        let registration = &self.registrations[index];
        match &registration.at_base {
            Some(specialize) => specialize(&self.lease),
            None => Ok(registration.draft.clone()),
        }
    }
}
type AcquireBase = dyn Fn() -> CoreResult<Box<dyn RegisteredBase>>;
struct BaseSetup {
    acquire: Box<AcquireBase>,
    backing_bytes: usize,
}

/// The original application-owned arguments to an exposure.
pub struct ExposureInput {
    pub description: Arc<str>,
    pub behaviors: Vec<RawFiniteBehavior>,
    pub policy: ExposurePolicy,
}

/// The caller owns the original inputs and any closed endpoint awaiting finalization.
/// Dropping this value releases a staged endpoint and its reservations locally.
pub struct ExposureError {
    error: CoreError,
    state: ExposureState,
}
enum ExposureState {
    Refused(ExposureInput),
    Assigned(Box<ExposureCandidate>),
}
impl ExposureError {
    fn refused(
        error: CoreError,
        description: Arc<str>,
        behaviors: Vec<RawFiniteBehavior>,
        policy: ExposurePolicy,
    ) -> Self {
        Self {
            error,
            state: ExposureState::Refused(ExposureInput {
                description,
                behaviors,
                policy,
            }),
        }
    }
    pub fn error(&self) -> &CoreError {
        &self.error
    }
    /// Whether this retry currently owns a closed, funded base on its original Host.
    pub fn is_staged(&self) -> bool {
        matches!(self.state, ExposureState::Assigned(_))
    }
    /// Roll back any closed setup and recover the original TD, handlers and policy.
    pub fn into_input(self) -> ExposureInput {
        match self.state {
            ExposureState::Refused(input) => input,
            ExposureState::Assigned(candidate) => candidate.into_input(),
        }
    }
    /// Resume on the original Host when a closed endpoint has already been acquired.
    pub fn retry(
        self,
        host: &mut HostProducer,
        budget: &mut WorkBudget,
    ) -> Result<ComposedProducerExposure, Self> {
        match self.state {
            ExposureState::Refused(input) => {
                host.expose(input.description, input.behaviors, input.policy, budget)
            }
            ExposureState::Assigned(candidate) => {
                if !candidate
                    .issuer
                    .same_owner(&host.owner.participation_token())
                {
                    return Err(Self {
                        error: CoreError::StaleHandle(ErrorContext::new(
                            ErrorPhase::Admission,
                            RetryClass::Never,
                        )),
                        state: ExposureState::Assigned(candidate),
                    });
                }
                candidate.finish(host, budget)
            }
        }
    }
}
impl fmt::Debug for ExposureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for ExposureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl core::error::Error for ExposureError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&self.error)
    }
}

struct AssignedServices {
    views: Vec<ServiceRef>,
    observed: usize,
    // Move the original pin into publication funding once the TD is finalized.
    pending: Option<Box<dyn RegisteredBase>>,
}

impl AssignedServices {
    /// Keep each specialized view, including one whose observations are busy.
    /// The completed prefix keeps its original versions through commit.
    fn specialize(
        &mut self,
        count: usize,
        facts: &mut PublicationFacts,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<&str>> {
        let Some(base) = self.pending.as_deref() else {
            return Ok(None);
        };
        while self.observed < count {
            budget
                .consume(WorkClass::BindingPolls, 1)
                .map_err(|_| busy())?;
            if self.observed == self.views.len() {
                self.views.push(base.specialize(self.observed)?);
            }
            self.views[self.observed]
                .clone()
                .capability()
                .observe_configuration(&mut facts.observer())?;
            self.observed += 1;
        }
        Ok(Some(base.as_ref()))
    }
}

// Pure definitions and specialized service owners drop before their funding.
struct ExposureSetup {
    assigned: Option<AssignedServices>,
    prepared: Option<PreparedThing>,
    participants: Option<Vec<OwnedProducerParticipation>>,
    providers: ProducerRegistrations,
    funding: FiniteSetupFunding,
}
struct ExposureCandidate {
    issuer: ProducerOwnerToken,
    description: Arc<str>,
    instance: RawProducerInstance,
    policy: ExposurePolicy,
    setup: ExposureSetup,
}
impl ExposureCandidate {
    fn into_input(self) -> ExposureInput {
        ExposureInput {
            description: self.description,
            behaviors: self.instance.into_finite_behaviors(),
            policy: self.policy,
        }
    }
    fn fail(self, error: CoreError) -> ExposureError {
        let state = if self.setup.assigned.is_some() && matches!(error, CoreError::Backpressure(_))
        {
            ExposureState::Assigned(Box::new(self))
        } else {
            ExposureState::Refused(self.into_input())
        };
        ExposureError { error, state }
    }

    fn prepare_members(
        &mut self,
        host: &HostProducer,
        budget: &mut WorkBudget,
    ) -> CoreResult<Vec<OwnedProducerParticipation>> {
        let setup = &mut self.setup;
        host.owner
            .check_publication_input(&self.instance, self.policy.replaces)?;
        drop(PublicationFacts::lock_current(&[&setup.funding.facts])?);
        if let Some(assigned) = &mut setup.assigned
            && let Some(base) =
                assigned.specialize(host.services.len(), &mut setup.funding.facts, budget)?
        {
            let description = description_at_base(&self.description, base, budget)?;
            let providers: Vec<_> = assigned
                .views
                .iter()
                .cloned()
                .map(Service::capability)
                .collect();
            let required = setup_scratch(
                &host.limits,
                &providers,
                &assigned.views,
                setup.funding.facts.backing_bytes().ok_or_else(busy)?,
            )?;
            remaining_backing(
                ResourceKind::AdmissionTemporaryBytesGlobalMax,
                setup.funding.stage.scratch.bytes(),
                Some(required),
                self.instance.key(),
            )?;
            setup.providers.replace(providers);
            self.instance.set_description(description.into());
            setup.funding.stage.base = Some(assigned.pending.take().expect("finalized base"));
        }
        if let Some(participants) = setup.participants.take() {
            return Ok(participants);
        }
        let limits =
            host.preparation_limits(&self.instance, &self.policy, setup.funding.stage.base_bytes)?;
        if setup.prepared.is_none() {
            let providers = setup.providers.snapshot(&mut setup.funding.facts)?;
            setup.prepared = Some(prepare(&self.instance, &providers, &limits, budget)?);
        }
        let prepared = setup.prepared.as_ref().expect("prepared candidate");
        host.assemble(
            setup
                .assigned
                .as_ref()
                .map_or(host.services.as_slice(), |assigned| {
                    assigned.views.as_slice()
                }),
            prepared,
            self.instance.key(),
            &limits,
            &mut setup.funding.counts,
            budget,
        )
    }

    fn finish(
        mut self,
        host: &HostProducer,
        budget: &mut WorkBudget,
    ) -> Result<ComposedProducerExposure, ExposureError> {
        let participants = match self.prepare_members(host, budget) {
            Ok(participants) => participants,
            Err(error) => return Err(self.fail(error)),
        };
        let mut funding = Some(self.setup.funding);
        let result = ComposedProducerExposure::admit(
            &host.owner,
            ComposedProducerInput {
                instance: self.instance,
                prepared: self.setup.prepared.take().expect("prepared candidate"),
                providers: self.setup.providers,
                participants,
                expected: self.policy.replaces,
                handler_backing_bytes: self.policy.handler_backing_bytes,
            },
            host.limits.clone(),
            host.accounts.clone(),
            budget,
            &mut funding,
        );
        match result {
            Ok(exposure) => Ok(exposure),
            Err(rejection) => {
                let (input, failure) = rejection.into_parts();
                self.instance = input.instance;
                self.setup.funding = funding.expect("refused candidate keeps funding");
                self.setup.prepared = Some(input.prepared);
                self.setup.providers = input.providers;
                self.setup.participants = Some(input.participants);
                Err(self.fail(failure.into_parts().1))
            }
        }
    }
}

/// Deployment-owned finite services, logical publications and shared work capacity.
/// Each exposure gets fresh authority, independently of the TD's optional ID.
pub struct HostProducer {
    owner: RawProducerServient,
    capacity: ProducerFiniteCapacity,
    services: Vec<ServiceRef>,
    acquire: Option<BaseSetup>,
    config: HostProducerLimits,
    limits: Arc<ResourceLimits>,
    accounts: Arc<HostResourceAccounts>,
    next_slot: u32,
    _registry: crate::host_accounts::HostCapacityReservation,
}
impl HostProducer {
    pub fn new(
        registrations: Vec<FiniteProducerRegistration>,
        config: HostProducerLimits,
        limits: Arc<ResourceLimits>,
        accounts: Arc<HostResourceAccounts>,
    ) -> CoreResult<Self> {
        if !(1..=MAX_PRODUCER_REGISTRATIONS).contains(&registrations.len())
            || registrations.iter().any(|r| r.at_base.is_some())
        {
            return Err(invalid());
        }
        Self::build(
            registrations.into_iter().map(|r| r.draft).collect(),
            None,
            config,
            limits,
            accounts,
        )
    }

    /// The deployment selects one source of the TD base. A successful bounded,
    /// nonblocking acquisition returns the actual URI and its original closed
    /// lease. Dropping that lease must roll back locally, including on error.
    /// `base_backing_bytes` declares the peak controlled backing of acquisition,
    /// the returned lease and all per-candidate specialized registration views.
    /// It remains charged through commit or rollback. Startup services and OS
    /// socket memory are provisioned by the deployment.
    pub fn with_assigned_base<L: AsRef<str> + 'static>(
        registrations: Vec<FiniteProducerRegistration<L>>,
        acquire: impl Fn() -> CoreResult<L> + 'static,
        base_backing_bytes: usize,
        config: HostProducerLimits,
        limits: Arc<ResourceLimits>,
        accounts: Arc<HostResourceAccounts>,
    ) -> CoreResult<Self> {
        if !(1..=MAX_PRODUCER_REGISTRATIONS).contains(&registrations.len())
            || base_backing_bytes < size_of::<L>()
            || base_backing_bytes > config.candidate_bytes
        {
            return Err(invalid());
        }
        let draft = registrations.iter().map(|r| r.draft.clone()).collect();
        let registrations: Rc<[_]> = registrations.into();
        let setup = move || -> CoreResult<Box<dyn RegisteredBase>> {
            Ok(Box::new(DeploymentBase {
                lease: acquire()?,
                registrations: registrations.clone(),
            }))
        };
        let backing_bytes = base_backing_bytes
            .checked_add(size_of::<DeploymentBase<L>>() - size_of::<L>())
            .ok_or_else(invalid)?;
        Self::build(
            draft,
            Some(BaseSetup {
                acquire: Box::new(setup),
                backing_bytes,
            }),
            config,
            limits,
            accounts,
        )
    }

    fn build(
        services: Vec<ServiceRef>,
        acquire: Option<BaseSetup>,
        config: HostProducerLimits,
        limits: Arc<ResourceLimits>,
        accounts: Arc<HostResourceAccounts>,
    ) -> CoreResult<Self> {
        if config.candidate_bytes == 0 {
            return Err(invalid());
        }
        let bytes = RawProducerServient::publication_backing(config.registry)
            .and_then(|n| {
                n.checked_add(size_of::<Self>() + services.capacity() * size_of::<ServiceRef>())
            })
            .ok_or_else(invalid)?;
        let registry = accounts
            .reserve(ResourceKind::EngineLiveBytesGlobalMax, bytes as u64)
            .ok_or_else(busy)?;
        let owner = RawProducerServient::try_empty(config.registry)?;
        let capacity = ProducerFiniteCapacity::try_new(
            &owner,
            config.exchange,
            config.active_exchanges,
            config.retained_results,
            accounts.clone(),
        )?;
        Ok(Self {
            owner,
            capacity,
            services,
            acquire,
            config,
            limits,
            accounts,
            next_slot: 0,
            _registry: registry,
        })
    }

    /// Prepare the application's TD and behaviors and return its closed candidate.
    /// A refusal returns the application inputs. After base acquisition, a busy
    /// specializer or short finalization turn can resume on the same funded endpoint.
    pub fn expose(
        &mut self,
        description: Arc<str>,
        behaviors: Vec<RawFiniteBehavior>,
        policy: ExposurePolicy,
        budget: &mut WorkBudget,
    ) -> Result<ComposedProducerExposure, ExposureError> {
        let (authority, next_slot) = match self.new_authority(policy.replaces) {
            Ok(next) => next,
            Err(error) => {
                return Err(ExposureError::refused(
                    error,
                    description,
                    behaviors,
                    policy,
                ));
            }
        };
        let instance =
            RawProducerInstance::new(authority, behaviors).with_description(description.clone());
        let setup = match self.prepare_exposure(&instance, &policy, budget) {
            Ok(setup) => setup,
            Err(error) => {
                return Err(ExposureError::refused(
                    error,
                    description,
                    instance.into_finite_behaviors(),
                    policy,
                ));
            }
        };
        let candidate = ExposureCandidate {
            issuer: self.owner.participation_token(),
            description,
            instance,
            policy,
            setup,
        };
        let result = candidate.finish(self, budget);
        // A staged retry owns this logical slot even while other Things publish.
        if result
            .as_ref()
            .err()
            .is_none_or(|failure| failure.is_staged())
        {
            self.next_slot = next_slot;
        }
        result
    }

    fn new_authority(
        &self,
        replaces: Option<ProducerInstanceKey>,
    ) -> CoreResult<(ProducerInstanceAuthority, u32)> {
        let (slot, next) = match replaces {
            Some(old) => (
                ThingSlotId::new(
                    old.slot().slot(),
                    old.slot().generation().checked_next().ok_or_else(busy)?,
                ),
                self.next_slot,
            ),
            None => (
                ThingSlotId::new(SlotIndex::new(self.next_slot), Generation::INITIAL),
                self.next_slot.checked_add(1).ok_or_else(busy)?,
            ),
        };
        Ok((ProducerInstanceAuthority::try_new(slot)?, next))
    }

    fn preparation_limits(
        &self,
        instance: &RawProducerInstance,
        policy: &ExposurePolicy,
        base_bytes: usize,
    ) -> CoreResult<ResourceLimits> {
        let known = instance
            .backing_bytes()
            .and_then(|n| n.checked_add(policy.handler_backing_bytes))
            .and_then(|n| n.checked_add(base_bytes));
        let live = ResourceKind::EngineLiveBytesGlobalMax;
        let available = remaining_backing(
            live,
            self.config.candidate_bytes as u64,
            known,
            instance.key(),
        )?;
        Ok((*self.limits).clone().with_limit(
            live,
            Some(available.min(self.limits.get(live).unwrap_or(0))),
        ))
    }

    fn prepare_exposure(
        &self,
        instance: &RawProducerInstance,
        policy: &ExposurePolicy,
        budget: &mut WorkBudget,
    ) -> CoreResult<ExposureSetup> {
        self.owner
            .check_publication_input(instance, policy.replaces)?;
        let key = instance.key();
        let bytes = self.config.candidate_bytes;
        let storage = ProducerStorage::reserve(&self.accounts, bytes)?;
        let mut facts = PublicationFacts::default();
        let providers = capabilities(&self.services, &mut facts)?;
        let scratch = QualificationFunding::reserve(
            &self.accounts,
            setup_scratch(&self.limits, &providers, &self.services, bytes)?,
        )?;
        // A staged retry can retain both the application's original and finalized TD.
        let base_bytes = match &self.acquire {
            Some(base) => base
                .backing_bytes
                .checked_add(instance.description().ok_or_else(invalid)?.len())
                .and_then(|n| {
                    n.checked_add(2 * size_of::<usize>() + size_of::<ExposureCandidate>())
                })
                .ok_or_else(busy)?,
            None => 0,
        };
        let limits = self.preparation_limits(instance, policy, base_bytes)?;
        let draft = prepare(instance, &providers, &limits, budget)?;
        // Replacing the base preserves every Form/operation row. Each row has
        // one winner and a registration assembles at most one native member.
        let members = if self.acquire.is_some() {
            let global = ResourceKind::RouteReadinessTokensGlobalMax;
            let ceiling = self
                .limits
                .get(ResourceKind::RouteReadinessTokensPerThingMax)
                .unwrap_or(0)
                .min(self.limits.get(global).unwrap_or(0))
                .min(self.accounts.account(global).limit());
            let possible = draft.operations().len().min(self.services.len()) as u64;
            // A zero ceiling is reported by the normal one-member limit check.
            possible.min(ceiling).max(1) as usize
        } else {
            selected_members(&draft)
        };
        check_publication_limits(&self.limits, 0, members, key)?;
        self.owner.check_cleanup_quantum(
            key,
            policy.replaces,
            members,
            self.limits
                .get(ResourceKind::CleanupWorkItemsPerStepMax)
                .unwrap_or(0),
        )?;
        let counts = MemberFunding::reserve(&self.accounts, &self.limits, members, key)?;
        let mut funding = FiniteSetupFunding {
            stage: ProducerStageFunding {
                base: None,
                base_cleanup: None,
                base_bytes,
                storage,
                scratch,
            },
            counts,
            facts,
        };
        let (assigned, prepared) = if let Some(acquire) = &self.acquire {
            drop(draft);
            drop(PublicationFacts::lock_current(&[&funding.facts])?);
            funding.stage.base_cleanup = Some(
                self.accounts
                    .reserve(ResourceKind::CleanupItemsMax, 1)
                    .ok_or_else(busy)?,
            );
            // Admit the first pass before binding, then charge each actual
            // callback on the turn that invokes it, including a retried callback.
            if budget.remaining(WorkClass::BindingPolls) < 1 + self.services.len() as u64 {
                return Err(busy());
            }
            budget
                .consume(WorkClass::BindingPolls, 1)
                .map_err(|_| busy())?;
            budget
                .consume(WorkClass::CleanupItems, 1)
                .map_err(|_| busy())?;
            let base = AssignedServices {
                views: Vec::with_capacity(self.services.len()),
                observed: 0,
                pending: Some((acquire.acquire)()?),
            };
            (Some(base), None)
        } else {
            (None, Some(draft))
        };
        Ok(ExposureSetup {
            assigned,
            prepared,
            participants: None,
            providers: ProducerRegistrations::new(providers),
            funding,
        })
    }

    fn assemble(
        &self,
        services: &[ServiceRef],
        prepared: &PreparedThing,
        key: ProducerInstanceKey,
        preparation_limits: &ResourceLimits,
        counts: &mut MemberFunding,
        budget: &mut WorkBudget,
    ) -> CoreResult<Vec<OwnedProducerParticipation>> {
        let live = ResourceKind::EngineLiveBytesGlobalMax;
        let mut remaining = remaining_backing(
            live,
            preparation_limits.get(live).unwrap_or(0),
            Some(prepared.retained_bytes()),
            key,
        )?;
        let members = selected_members(prepared);
        check_publication_limits(&self.limits, 0, members, key)?;
        counts.retain_selected(members, key)?;
        let mut participants = Vec::with_capacity(members);
        for (index, service) in services.iter().enumerate() {
            budget
                .consume(
                    WorkClass::PlanningItems,
                    1 + 2 * prepared.operations().len() as u64,
                )
                .map_err(|_| busy())?;
            if prepared
                .operations()
                .iter()
                .any(|o| o.registration() == index)
            {
                let member = service.assemble(index, prepared, key, self)?;
                let backing = member
                    .admission_backing_bytes(&self.owner.participation_token(), key)
                    .ok_or_else(invalid)?
                    .checked_add(size_of::<OwnedProducerParticipation>())
                    .and_then(|n| n.checked_add(member.overhead()));
                remaining = remaining_backing(live, remaining, backing, key)?;
                participants.push(member);
            }
        }
        Ok(participants)
    }

    pub fn description(&self, key: ProducerInstanceKey) -> Option<&str> {
        self.owner.description(key)
    }
    pub fn revocation(&self, key: ProducerInstanceKey) -> Option<ProducerRevocation> {
        self.owner.revocation(key)
    }

    pub fn step(
        &mut self,
        exposure: &mut ComposedProducerExposure,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        credit: impl FnMut(usize, ProducerWork) -> WorkBudget,
    ) -> CoreResult<()> {
        exposure.step(&mut self.owner, cx, now, credit)
    }
    /// Drive closed members and commit once all are ready. Pending includes
    /// replenishable work credit. After success, regular steps serve requests.
    pub fn poll_publish(
        &mut self,
        exposure: &mut ComposedProducerExposure,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        publication_credit: &mut WorkBudget,
        credit: impl FnMut(usize, ProducerWork) -> WorkBudget,
    ) -> Poll<CoreResult<()>> {
        if !exposure.is_staging() {
            return Poll::Ready(Err(exposure
                .publication_failure()
                .cloned()
                .unwrap_or_else(invalid)));
        }
        if let Err(error) = self.step(exposure, cx, now, credit)
            && !matches!(error, CoreError::Backpressure(_))
        {
            return Poll::Ready(Err(error));
        }
        // A native start may itself have failed with Backpressure. Publication
        // distinguishes that recorded terminal cause from replenishable credit.
        match exposure.publish(&mut self.owner, publication_credit) {
            Err(CoreError::Backpressure(_)) if exposure.publication_failure().is_none() => {
                Poll::Pending
            }
            result => Poll::Ready(result),
        }
    }
    pub fn retire(
        &mut self,
        exposure: &mut ComposedProducerExposure,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        exposure.retire(&mut self.owner, budget)
    }
    pub fn retire_revoked(
        &mut self,
        key: ProducerInstanceKey,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        self.owner.retire_revoked_publication(key, budget)
    }
}

/// A candidate's prepaid storage has a fixed size for its entire lifetime.
fn remaining_backing(
    resource: ResourceKind,
    available: u64,
    required: Option<usize>,
    key: ProducerInstanceKey,
) -> CoreResult<u64> {
    let requested = required.and_then(|n| u64::try_from(n).ok());
    requested
        .and_then(|n| available.checked_sub(n))
        .ok_or_else(|| CoreError::LimitExceeded {
            resource,
            limit: available,
            requested,
            observed: None,
            context: ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
                .with_thing(key.slot()),
        })
}

fn capabilities(
    services: &[ServiceRef],
    facts: &mut PublicationFacts,
) -> CoreResult<Vec<Rc<dyn ProducerCapability>>> {
    services
        .iter()
        .map(|service| {
            let provider = service.clone().capability();
            provider.observe_configuration(&mut facts.observer())?;
            Ok(provider)
        })
        .collect()
}

fn setup_scratch(
    limits: &ResourceLimits,
    providers: &[Rc<dyn ProducerCapability>],
    services: &[ServiceRef],
    observations: usize,
) -> CoreResult<usize> {
    // Selection can hold one provisional winner while probing a competitor.
    let callback = services
        .iter()
        .map(|s| s.setup_peak())
        .max()
        .ok_or_else(invalid)?;
    qualification_bytes(limits, providers, observations)?
        .checked_add(callback.checked_mul(2).ok_or_else(busy)?)
        .ok_or_else(busy)
}

fn selected_members(prepared: &PreparedThing) -> usize {
    let mut selected = [false; MAX_PRODUCER_REGISTRATIONS];
    for op in prepared.operations() {
        selected[op.registration()] = true;
    }
    selected.into_iter().filter(|selected| *selected).count()
}

fn prepare(
    instance: &RawProducerInstance,
    providers: &[Rc<dyn ProducerCapability>],
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
) -> CoreResult<PreparedThing> {
    let refs: Vec<_> = providers.iter().map(|p| p.as_ref()).collect();
    let prepared = prepare_thing(
        instance.description().ok_or_else(invalid)?,
        instance.key(),
        limits,
        budget,
        &refs,
        |scope, op| instance_supports(instance, scope, op),
    )?;
    budget
        .consume(
            WorkClass::HandlerSteps,
            (instance.behavior_count() * prepared.operations().len()) as u64,
        )
        .map_err(|_| busy())?;
    if !instance.behaviors_described_by(|site, op| {
        prepared
            .operations()
            .iter()
            .any(|o| o.site.site() == site && o.operation == op)
    }) {
        return Err(CoreError::UnsupportedOperation(ErrorContext::new(
            ErrorPhase::Admission,
            RetryClass::Never,
        )));
    }
    Ok(prepared)
}
