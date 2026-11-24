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
use clinkz_wot_core::{producer::*, *};
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
    any::Any,
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

struct InstalledBase {
    description: Arc<str>,
    services: Vec<ServiceRef>,
    lease: Box<dyn Any>,
}
type AcquireBase = dyn Fn(&str, &mut WorkBudget) -> CoreResult<InstalledBase>;
struct BaseSetup {
    acquire: Box<AcquireBase>,
    backing_bytes: usize,
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
        let setup = move |description: &str, budget: &mut WorkBudget| {
            let lease = acquire()?;
            let description = description_at_base(description, lease.as_ref(), budget)?.into();
            let services = registrations
                .iter()
                .map(|r| match &r.at_base {
                    Some(specialize) => specialize(&lease),
                    None => Ok(r.draft.clone()),
                })
                .collect::<CoreResult<_>>()?;
            Ok(InstalledBase {
                description,
                services,
                lease: Box::new(lease),
            })
        };
        Self::build(
            draft,
            Some(BaseSetup {
                acquire: Box::new(setup),
                backing_bytes: base_backing_bytes,
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
    /// `poll_publish` completes installation under caller-supplied time and credit.
    pub fn expose(
        &mut self,
        description: Arc<str>,
        behaviors: Vec<RawFiniteBehavior>,
        policy: ExposurePolicy,
        budget: &mut WorkBudget,
    ) -> CoreResult<ComposedProducerExposure> {
        // This owner outlives all provisional inputs, including their destructors
        // on a refusal after native acquisition.
        let mut prepaid;
        let slot = match policy.replaces {
            Some(old) => ThingSlotId::new(
                old.slot().slot(),
                old.slot().generation().checked_next().ok_or_else(busy)?,
            ),
            None => ThingSlotId::new(SlotIndex::new(self.next_slot), Generation::INITIAL),
        };
        let next_slot = if policy.replaces.is_none() {
            self.next_slot.checked_add(1).ok_or_else(busy)?
        } else {
            self.next_slot
        };
        let authority = ProducerInstanceAuthority::try_new(slot)?;
        let key = authority.key();
        let mut instance =
            RawProducerInstance::new(authority, behaviors).with_description(description.clone());
        self.owner
            .check_publication_input(&instance, policy.replaces)?;
        let bytes = self.config.candidate_bytes;
        let storage = ProducerStorage::reserve(&self.accounts, bytes)?;
        let mut facts = PublicationFacts::default();
        let mut providers = capabilities(&self.services, &mut facts)?;
        let scratch = QualificationFunding::reserve(
            &self.accounts,
            setup_scratch(&self.limits, &providers, &self.services, bytes)?,
        )?;
        let base_bytes = self.acquire.as_ref().map_or(0, |base| base.backing_bytes);
        let preparation_limit = |instance: &RawProducerInstance| -> CoreResult<ResourceLimits> {
            let known = instance
                .backing_bytes()
                .and_then(|n| n.checked_add(policy.handler_backing_bytes))
                .and_then(|n| n.checked_add(base_bytes))
                .ok_or_else(busy)?;
            let available = bytes.checked_sub(known).ok_or_else(busy)? as u64;
            let live = ResourceKind::EngineLiveBytesGlobalMax;
            Ok((*self.limits).clone().with_limit(
                live,
                Some(available.min(self.limits.get(live).unwrap_or(0))),
            ))
        };
        let mut preparation_limits = preparation_limit(&instance)?;
        let mut prepared = prepare(&instance, &providers, &preparation_limits, budget)?;
        let members = selected_members(&prepared);
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
        prepaid = Some(FiniteSetupFunding {
            stage: ProducerStageFunding {
                base: None,
                base_cleanup: None,
                base_bytes,
                storage,
                scratch,
            },
            counts,
            facts,
        });
        let funding = prepaid.as_mut().unwrap();
        let installed;
        let services = if let Some(acquire) = &self.acquire {
            // No draft output can certify a mapping at the address about to be assigned.
            drop(prepared);
            drop(PublicationFacts::lock_current(&[&funding.facts])?);
            funding.stage.base_cleanup = Some(
                self.accounts
                    .reserve(ResourceKind::CleanupItemsMax, 1)
                    .ok_or_else(busy)?,
            );
            budget
                .consume(WorkClass::BindingPolls, 1 + self.services.len() as u64)
                .map_err(|_| busy())?;
            budget
                .consume(WorkClass::CleanupItems, 1)
                .map_err(|_| busy())?;
            let base = (acquire.acquire)(&description, budget)?;
            funding.stage.base = Some(base.lease);
            instance = instance.with_description(base.description);
            installed = base.services;
            providers = capabilities(&installed, &mut funding.facts)?;
            if setup_scratch(
                &self.limits,
                &providers,
                &installed,
                funding.facts.backing_bytes().ok_or_else(busy)?,
            )? > funding.stage.scratch.bytes() as usize
            {
                return Err(busy());
            }
            preparation_limits = preparation_limit(&instance)?;
            prepared = prepare(&instance, &providers, &preparation_limits, budget)?;
            &installed
        } else {
            &self.services
        };
        let participants = self.assemble(services, &prepared, key, &preparation_limits, budget)?;
        let exposure = ComposedProducerExposure::admit(
            &self.owner,
            ComposedProducerInput {
                instance,
                prepared,
                providers: ProducerRegistrations::new(providers),
                participants,
                expected: policy.replaces,
                handler_backing_bytes: policy.handler_backing_bytes,
            },
            self.limits.clone(),
            self.accounts.clone(),
            budget,
            &mut prepaid,
        )
        .map_err(cause)?;
        self.next_slot = next_slot;
        Ok(exposure)
    }

    fn assemble(
        &self,
        services: &[ServiceRef],
        prepared: &PreparedThing,
        key: ProducerInstanceKey,
        preparation_limits: &ResourceLimits,
        budget: &mut WorkBudget,
    ) -> CoreResult<Vec<OwnedProducerParticipation>> {
        let mut remaining = (preparation_limits
            .get(ResourceKind::EngineLiveBytesGlobalMax)
            .unwrap_or(0) as usize)
            .checked_sub(prepared.retained_bytes())
            .ok_or_else(busy)?;
        let mut participants = Vec::with_capacity(selected_members(prepared));
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
                    .checked_add(size_of::<OwnedProducerParticipation>() + member.overhead())
                    .ok_or_else(busy)?;
                remaining = remaining.checked_sub(backing).ok_or_else(busy)?;
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
