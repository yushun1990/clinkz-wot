//! Application-owned finite Things on a manually driven Host.
use crate::producer_composition::{check_participant_coverage, publication_supports};
use alloc::{
    boxed::Box,
    rc::{Rc, Weak},
    sync::Arc,
    vec::Vec,
};
use clinkz_wot_core::{
    finite::FiniteIngressBinding, host_accounts::HostResourceAccounts, producer::*, publication::*,
    *,
};
use clinkz_wot_foundation::{
    Generation, MonotonicInstant, ResourceLimits, SlotIndex, WorkBudget, WorkClass,
};
use clinkz_wot_planning::producer_composition::*;
use clinkz_wot_servient_finite::{
    FiniteValidation, MAX_PRODUCER_PARTICIPANTS, ProducerFiniteCapacity, ProducerFiniteSettlement,
    ProducerPublication, ProducerRevocation, RawFiniteRuntime, RawFiniteState, RawProducerInstance,
    RawProducerServient,
};
pub use clinkz_wot_servient_finite::{
    RawFiniteBehavior, RawFiniteLimits, RawProducerRegistryLimits,
};
use core::{
    cell::{Cell, RefCell},
    task::{Context, Poll},
};

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
fn busy() -> CoreError {
    CoreError::Backpressure(context())
}
fn invalid() -> CoreError {
    CoreError::Validation(context())
}
fn stale() -> CoreError {
    CoreError::StaleHandle(context())
}

/// Storage supplied at startup. Candidates occupy a position until commit or
/// rollback. A selected member occupies its position through native retirement.
/// A result position stays claimed until the application disposes of the record.
pub struct HostProducerLimits {
    pub registry: RawProducerRegistryLimits,
    pub candidates: usize,
    pub members: usize,
    pub exchange: RawFiniteLimits,
    pub active_exchanges: usize,
    pub retained_results: usize,
    /// Optional deployment-wide `PendingHandlerCallsGlobalMax` quota. Each finite
    /// exchange holds its permit from admission through disposal of its result.
    pub shared_handler_quota: Option<Arc<HostResourceAccounts>>,
}

#[derive(Default, Clone, Copy)]
pub struct ExposurePolicy {
    pub replaces: Option<ProducerInstanceKey>,
    pub deadline: Deadline,
}

pub struct ExposureInput {
    pub description: Arc<str>,
    pub behaviors: Vec<RawFiniteBehavior>,
    pub policy: ExposurePolicy,
}

trait HostedMember: ClosedProducerParticipation {
    fn fence_progress_clock(&mut self) -> CoreError;
    fn poll_one(
        &mut self,
        owner: &RawProducerServient,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> CoreResult<Poll<RawFiniteState>>;
    fn take_result(&mut self) -> Option<ProducerFiniteSettlement>;
}
impl<B: FiniteIngressBinding + 'static> HostedMember for RawFiniteRuntime<B> {
    fn fence_progress_clock(&mut self) -> CoreError {
        RawFiniteRuntime::fence_progress_clock(self)
    }
    fn poll_one(
        &mut self,
        owner: &RawProducerServient,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> CoreResult<Poll<RawFiniteState>> {
        owner.poll_finite(self, cx, now)
    }
    fn take_result(&mut self) -> Option<ProducerFiniteSettlement> {
        self.take_retained_settlement()
    }
}

// Type erasure stops at deployment registration. The Binding owns construction
// of the same concrete configuration that execution later verifies.
trait Service: ProducerCapability {
    fn member(
        &self,
        index: usize,
        prepared: &PreparedThing,
        key: ProducerInstanceKey,
        host: &HostProducer,
    ) -> CoreResult<Box<dyn HostedMember>>;
}
impl<S: FiniteProducerService + 'static> Service for S {
    fn member(
        &self,
        index: usize,
        prepared: &PreparedThing,
        key: ProducerInstanceKey,
        host: &HostProducer,
    ) -> CoreResult<Box<dyn HostedMember>> {
        let operations = prepared
            .operations()
            .iter()
            .filter(|op| op.registration() == index);
        let (binding, config) = self.assemble(key, operations)?;
        let member = RawFiniteRuntime::try_new(
            binding,
            config,
            &host.owner,
            host.exchange,
            FiniteValidation::Required,
            Deadline::NONE,
        )
        .map_err(|r| r.into_parts().1.into_parts().1)?;
        Ok(Box::new(
            member
                .with_capacity(host.work.clone())
                .map_err(|r| r.into_parts().1.into_parts().1)?,
        ))
    }
}
type ServiceRef = Rc<dyn Service>;
type Specialize<L> = Box<dyn Fn(&L) -> CoreResult<ServiceRef>>;

pub struct FiniteProducerRegistration<L = ()> {
    service: ServiceRef,
    specialize: Option<Specialize<L>>,
}
impl<L: 'static> FiniteProducerRegistration<L> {
    pub fn fixed(service: impl FiniteProducerService + 'static) -> Self {
        Self {
            service: Rc::new(service),
            specialize: None,
        }
    }
    pub fn at_base<S: FiniteProducerService + 'static>(
        service: S,
        specialize: fn(&S, &L) -> CoreResult<S>,
    ) -> Self {
        let service = Rc::new(service);
        Self {
            service: service.clone(),
            specialize: Some(Box::new(move |base| {
                let actual = specialize(&service, base)?;
                if actual.identity() != service.identity() {
                    return Err(invalid());
                }
                Ok(Rc::new(actual))
            })),
        }
    }
}

trait AssignedBase {
    fn uri(&self) -> &str;
    fn service(&self, index: usize) -> CoreResult<ServiceRef>;
}
struct Endpoint<L> {
    lease: L,
    registrations: Rc<[FiniteProducerRegistration<L>]>,
}
impl<L: AsRef<str>> AssignedBase for Endpoint<L> {
    fn uri(&self) -> &str {
        self.lease.as_ref()
    }
    fn service(&self, index: usize) -> CoreResult<ServiceRef> {
        let r = &self.registrations[index];
        match &r.specialize {
            Some(f) => f(&self.lease),
            None => Ok(r.service.clone()),
        }
    }
}
type Acquire = Box<dyn Fn() -> CoreResult<Box<dyn AssignedBase>>>;
type Storage = Rc<[RefCell<Option<Box<dyn HostedMember>>>]>;

/// Exclusive ownership of a concrete member position, including rollback.
struct Member {
    slots: Storage,
    index: usize,
    ready: bool,
}
impl Member {
    fn cell(&self) -> &RefCell<Option<Box<dyn HostedMember>>> {
        &self.slots[self.index]
    }
}
impl Drop for Member {
    fn drop(&mut self) {
        let native = self.cell().take();
        drop(native);
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Draft,
    Specialize,
    Select,
    Install,
    Commit,
}
// The existing TD walker and preparation adapters still use typed work units.
// A job may spend 64 Mi units per traversal class and probe registrations 4096
// times in total, including assigned-base retries. Execution has its own turns.
fn preparation_budget() -> WorkBudget {
    use WorkClass::*;
    [
        JsonSchemaNodes,
        CodecInputBytes,
        CodecOutputBytes,
        UriBytes,
        SecurityBranches,
        DocumentNodes,
        CleanupItems, // TD admission reserves local rollback for its allocated nodes.
        PlanningItems,
        BindingPolls,
    ]
    .into_iter()
    .fold(
        WorkBudget::new().with_remaining(ProviderProbes, 4096),
        |budget, class| budget.with_remaining(class, 64 * 1024 * 1024),
    )
}
struct Stage {
    phase: Phase,
    _slot: Rc<()>,
    endpoint: Option<Box<dyn AssignedBase>>,
    services: Vec<ServiceRef>,
    observed: usize,
    prepared: Option<PreparedThing>,
    facts: PublicationFacts,
    // The existing TD and Planning jobs still meter traversal. All passes and
    // retries of this candidate spend the same nonrenewable allowance.
    work: WorkBudget,
}

/// Sole owner of the original application, endpoint and native memberships.
/// The application drives this handle periodically, including under pool pressure.
pub struct HostExposure {
    issuer: ProducerOwnerToken,
    publication: ProducerPublication,
    original: Option<ExposureInputSource>,
    members: Vec<Member>,
    stage: Option<Stage>,
    last_now: Option<MonotonicInstant>,
    failure: Option<CoreError>,
    next: usize,
}
struct ExposureInputSource {
    description: Arc<str>,
    policy: ExposurePolicy,
}
impl HostExposure {
    pub fn key(&self) -> ProducerInstanceKey {
        self.publication.key()
    }
    fn observe_time(&mut self, now: MonotonicInstant) -> CoreResult<()> {
        if self
            .last_now
            .is_none_or(|last| now.checked_duration_since(last).is_some())
        {
            self.last_now = Some(now);
            return Ok(());
        }
        self.publication.revoke();
        // Start with the member whose turn observed the fault. During installation,
        // a sibling may already have started while this member is still unadmitted.
        let (before, from_next) = self.members.split_at(self.next);
        let mut cause = invalid();
        for member in from_next.iter().chain(before) {
            let error = member
                .cell()
                .borrow_mut()
                .as_mut()
                .unwrap()
                .fence_progress_clock();
            if matches!(cause, CoreError::Validation(_)) {
                cause = error;
            }
        }
        Err(cause)
    }
    /// First cause observed during publication or while driving a committed member.
    pub fn publication_failure(&self) -> Option<&CoreError> {
        self.failure.as_ref()
    }
    pub fn is_committed(&self) -> bool {
        self.publication.is_committed()
    }
    pub fn take_finite_settlement(&mut self) -> Option<ProducerFiniteSettlement> {
        self.members
            .iter()
            .find_map(|m| m.cell().borrow_mut().as_mut()?.take_result())
    }
    /// Roll back any unpublished candidate. Native Drop disposes of the closed
    /// leases locally before the original unique handlers can leave this call.
    pub fn into_input(mut self) -> Result<ExposureInput, Self> {
        if self.is_committed() {
            return Err(self);
        }
        self.begin_close();
        let source = self.original.take().expect("unpublished source");
        let instance = self
            .publication
            .take_unpublished_instance()
            .expect("unpublished instance");
        Ok(ExposureInput {
            description: source.description,
            behaviors: instance.into_finite_behaviors(),
            policy: source.policy,
        })
    }
    pub fn begin_close(&mut self) {
        self.publication.revoke();
        if !self.is_committed() {
            self.failure
                .get_or_insert_with(|| CoreError::Cancelled(context()));
            self.members.clear();
            self.stage = None;
        } else {
            for m in &self.members {
                m.cell()
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .begin_retire(ProducerRetirement::Close);
            }
        }
    }
    pub fn members_retired(&self) -> bool {
        self.members
            .iter()
            .all(|m| m.cell().borrow().as_ref().unwrap().retired())
    }
}
impl Drop for HostExposure {
    fn drop(&mut self) {
        self.publication.revoke();
    }
}

/// One logical publication owner with bounded candidate and installed member storage.
pub struct HostProducer {
    owner: RawProducerServient,
    services: Vec<ServiceRef>,
    acquire: Option<Acquire>,
    candidates: Vec<Weak<()>>,
    members: Storage,
    exchange: RawFiniteLimits,
    work: ProducerFiniteCapacity,
    planning: ResourceLimits,
    next_slot: u32,
}

std::thread_local! { static DRIVING: Cell<bool> = const { Cell::new(false) }; }
struct Driving;
impl Driving {
    fn enter() -> Option<Self> {
        if DRIVING.replace(true) {
            None
        } else {
            Some(Self)
        }
    }
}
impl Drop for Driving {
    fn drop(&mut self) {
        DRIVING.set(false);
    }
}

impl HostProducer {
    pub fn new(
        registrations: Vec<FiniteProducerRegistration>,
        config: HostProducerLimits,
        planning: ResourceLimits,
    ) -> CoreResult<Self> {
        if registrations.iter().any(|r| r.specialize.is_some()) {
            return Err(invalid());
        }
        Self::build(
            registrations.into_iter().map(|r| r.service).collect(),
            None,
            config,
            planning,
        )
    }
    /// A successful acquisition hands over an original closed endpoint. Its
    /// destructor must perform bounded local rollback even after partial setup.
    pub fn with_assigned_base<L: AsRef<str> + 'static>(
        registrations: Vec<FiniteProducerRegistration<L>>,
        acquire: impl Fn() -> CoreResult<L> + 'static,
        config: HostProducerLimits,
        planning: ResourceLimits,
    ) -> CoreResult<Self> {
        let services = registrations.iter().map(|r| r.service.clone()).collect();
        let registrations: Rc<[_]> = registrations.into();
        Self::build(
            services,
            Some(Box::new(move || {
                Ok(Box::new(Endpoint {
                    lease: acquire()?,
                    registrations: registrations.clone(),
                }))
            })),
            config,
            planning,
        )
    }
    fn build(
        services: Vec<ServiceRef>,
        acquire: Option<Acquire>,
        config: HostProducerLimits,
        planning: ResourceLimits,
    ) -> CoreResult<Self> {
        if [services.len(), config.members, config.candidates]
            .iter()
            .any(|n| !(1..=MAX_PRODUCER_PARTICIPANTS).contains(n))
        {
            return Err(invalid());
        }
        let owner = RawProducerServient::try_empty(config.registry)?;
        let work = ProducerFiniteCapacity::try_new(
            &owner,
            config.exchange,
            config.active_exchanges,
            config.retained_results,
            config.shared_handler_quota,
        )?;
        Ok(Self {
            owner,
            services,
            acquire,
            candidates: (0..config.candidates).map(|_| Weak::new()).collect(),
            members: (0..config.members).map(|_| RefCell::new(None)).collect(),
            exchange: config.exchange,
            work,
            planning,
            next_slot: 0,
        })
    }
    /// Stage application inputs without invoking a registration or acquiring an endpoint.
    pub fn expose(
        &mut self,
        description: Arc<str>,
        behaviors: Vec<RawFiniteBehavior>,
        policy: ExposurePolicy,
    ) -> Result<HostExposure, BindingInputRejection<ExposureInput>> {
        let reject = |behaviors, error| {
            BindingInputRejection::new(
                ExposureInput {
                    description: description.clone(),
                    behaviors,
                    policy,
                },
                BindingOperationalError::new(error),
            )
        };
        if description.len() > clinkz_wot_planning::finite_producer::PRODUCER_DESCRIPTION_BYTES {
            return Err(reject(behaviors, invalid()));
        }
        let Some(slot) = self.candidates.iter_mut().find(|s| s.strong_count() == 0) else {
            return Err(reject(behaviors, busy()));
        };
        let logical = match policy.replaces {
            Some(old) => old
                .slot()
                .generation()
                .checked_next()
                .map(|g| ThingSlotId::new(old.slot().slot(), g)),
            None => self
                .next_slot
                .checked_add(1)
                .map(|_| ThingSlotId::new(SlotIndex::new(self.next_slot), Generation::INITIAL)),
        };
        let authority = match logical
            .ok_or_else(busy)
            .and_then(ProducerInstanceAuthority::try_new)
        {
            Ok(a) => a,
            Err(error) => return Err(reject(behaviors, error)),
        };
        let instance =
            RawProducerInstance::new(authority, behaviors).with_description(description.clone());
        let publication = self
            .owner
            .prepare_publication(instance, policy.replaces)
            .map_err(|r| {
                let (instance, e) = r.into_parts();
                reject(instance.into_finite_behaviors(), e.into_parts().1)
            })?;
        let token = Rc::new(());
        *slot = Rc::downgrade(&token);
        if policy.replaces.is_none() {
            self.next_slot += 1;
        }
        Ok(HostExposure {
            issuer: self.owner.participation_token(),
            publication,
            members: Vec::new(),
            next: 0,
            stage: Some(Stage {
                phase: Phase::Draft,
                _slot: token,
                endpoint: None,
                services: Vec::new(),
                observed: 0,
                prepared: None,
                facts: PublicationFacts::default(),
                work: preparation_budget(),
            }),
            last_now: None,
            failure: None,
            original: Some(ExposureInputSource {
                description,
                policy,
            }),
        })
    }
    pub fn description(&self, key: ProducerInstanceKey) -> Option<&str> {
        self.owner.description(key)
    }
    pub fn revocation(&self, key: ProducerInstanceKey) -> Option<ProducerRevocation> {
        self.owner.revocation(key)
    }
    fn check_owner(&self, thing: &HostExposure) -> CoreResult<()> {
        if thing.issuer.same_owner(&self.owner.participation_token()) {
            Ok(())
        } else {
            Err(stale())
        }
    }
    pub fn poll_publish(
        &mut self,
        thing: &mut HostExposure,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> Poll<CoreResult<()>> {
        self.check_owner(thing)?;
        if let Some(error) = &thing.failure {
            return Poll::Ready(Err(error.clone()));
        }
        if thing.stage.is_none() {
            return Poll::Ready(Err(invalid()));
        }
        let Some(_turn) = Driving::enter() else {
            return Poll::Pending;
        };
        match self.install(thing, cx, now) {
            Ok(true) => {
                thing.stage = None;
                thing.original = None;
                thing.publication.release_retired();
                Poll::Ready(Ok(()))
            }
            Ok(false) => Poll::Pending,
            Err(error) => {
                thing.failure = Some(error.clone());
                thing.begin_close();
                Poll::Ready(Err(error))
            }
        }
    }
    fn install(
        &mut self,
        thing: &mut HostExposure,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> CoreResult<bool> {
        thing.observe_time(now)?;
        let key = thing.key();
        let stage = thing.stage.as_mut().unwrap();
        let source = thing.original.as_ref().unwrap();
        if source
            .policy
            .deadline
            .checked_is_elapsed_at(now)
            .ok_or_else(invalid)?
        {
            return Err(CoreError::TimedOut(context()));
        }
        drop(PublicationFacts::lock_current(&[&stage.facts])?);
        match stage.phase {
            Phase::Draft => {
                stage
                    .services
                    .try_reserve_exact(self.services.len())
                    .map_err(|_| busy())?;
                if let Some(acquire) = &self.acquire {
                    let providers: Vec<&dyn ProducerCapability> =
                        self.services.iter().map(|s| s.as_ref() as _).collect();
                    let advertised = check_finite_draft(
                        &source.description,
                        key,
                        &self.planning,
                        &mut stage.work,
                        &providers,
                        |scope, op| publication_supports(&thing.publication, scope, op),
                        &mut stage.facts,
                    )?;
                    if !thing.publication.behaviors_described_by(|site, op| {
                        advertised.iter().any(|(s, o)| s.site() == site && *o == op)
                    }) {
                        return Err(CoreError::UnsupportedOperation(context()));
                    }
                    stage.endpoint = Some(acquire()?);
                    stage.phase = Phase::Specialize;
                } else {
                    stage.prepared = Some(select(
                        &thing.publication,
                        &self.services,
                        &self.planning,
                        &mut stage.work,
                        &mut stage.facts,
                    )?);
                    stage.services.clone_from(&self.services);
                    stage.phase = Phase::Select;
                }
            }
            Phase::Specialize => {
                let endpoint = stage.endpoint.as_ref().unwrap();
                let index = stage.observed;
                if index == stage.services.len() {
                    stage
                        .work
                        .consume(WorkClass::ProviderProbes, 1)
                        .map_err(|_| busy())?;
                    match endpoint.service(index) {
                        Ok(service) => stage.services.push(service),
                        Err(CoreError::Backpressure(_)) => return Ok(false),
                        Err(error) => return Err(error),
                    }
                }
                stage
                    .work
                    .consume(WorkClass::ProviderProbes, 1)
                    .map_err(|_| busy())?;
                match stage.services[index].observe_configuration(&mut stage.facts.observer()) {
                    Ok(()) => stage.observed += 1,
                    Err(CoreError::Backpressure(_)) => return Ok(false),
                    Err(error) => return Err(error),
                }
                if stage.observed == self.services.len() {
                    let final_td =
                        description_at_base(&source.description, endpoint.uri(), &mut stage.work)?;
                    thing.publication.set_final_description(final_td.into())?;
                    stage.phase = Phase::Select;
                }
            }
            Phase::Select => {
                if stage.prepared.is_none() {
                    stage.prepared = Some(select(
                        &thing.publication,
                        &stage.services,
                        &self.planning,
                        &mut stage.work,
                        &mut stage.facts,
                    )?);
                }
                let prepared = stage.prepared.as_ref().unwrap();
                let providers: Vec<&dyn ProducerCapability> =
                    stage.services.iter().map(|p| p.as_ref() as _).collect();
                stage.facts = ProducerQualification::begin(
                    &providers,
                    core::mem::take(&mut stage.facts),
                    &mut stage.work,
                )?
                .check_final(
                    thing.publication.staged_description().unwrap(),
                    key,
                    &self.planning,
                    &mut stage.work,
                    prepared,
                    |scope, op| publication_supports(&thing.publication, scope, op),
                )?;
                let selected = stage.services.iter().enumerate().filter(|(i, _)| {
                    prepared
                        .operations()
                        .iter()
                        .any(|op| op.registration() == *i)
                });
                thing
                    .members
                    .try_reserve_exact(selected.clone().count())
                    .map_err(|_| busy())?;
                for (index, service) in selected {
                    let position = self
                        .members
                        .iter()
                        .position(|m| m.borrow().is_none())
                        .ok_or_else(busy)?;
                    let member = service.member(index, prepared, key, self)?;
                    self.members[position].replace(Some(member));
                    thing.members.push(Member {
                        slots: self.members.clone(),
                        index: position,
                        ready: false,
                    });
                }
                let loans: Vec<_> = thing.members.iter().map(|m| m.cell().borrow()).collect();
                let members: Vec<&dyn ClosedProducerParticipation> =
                    loans.iter().map(|m| m.as_deref().unwrap() as _).collect();
                thing.publication.bind_participants(&members)?;
                stage.phase = Phase::Install;
            }
            Phase::Install => {
                let count = thing.members.len();
                let member = &mut thing.members[thing.next];
                thing.next = (thing.next + 1) % count;
                if !member.ready {
                    let ready = {
                        let mut loan = member.cell().borrow_mut();
                        let native = loan.as_deref_mut().unwrap();
                        if !native.associated_with(&thing.issuer, key) {
                            return Err(stale());
                        }
                        let result = native.poll_one(&self.owner, cx, now)?;
                        if let Some(error) = native.native_failure() {
                            return Err(error.clone());
                        }
                        if result.is_pending() {
                            return Ok(false);
                        }
                        native.ready_for(&thing.issuer, key)
                    };
                    if !ready {
                        return Err(stale());
                    }
                    member.ready = true;
                }
                if thing.members.iter().all(|m| m.ready) {
                    thing.next = 0;
                    stage.phase = Phase::Commit;
                }
            }
            Phase::Commit => {
                let mut loans: Vec<_> = thing
                    .members
                    .iter()
                    .map(|m| m.cell().borrow_mut())
                    .collect();
                let mut members: Vec<&mut dyn ClosedProducerParticipation> = loans
                    .iter_mut()
                    .map(|m| m.as_deref_mut().unwrap() as _)
                    .collect();
                self.owner.publish_if_current(
                    &mut thing.publication,
                    &mut members,
                    &stage.facts,
                    |key, members| {
                        check_participant_coverage(
                            key,
                            stage.prepared.as_ref().unwrap(),
                            members.iter().map(|m| &**m),
                        )?;
                        Ok(None)
                    },
                )?;
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// One round-robin member opportunity. The caller promises periodic drives,
    /// including after releasing a retained outcome. No executor wake is required.
    pub fn step(
        &mut self,
        thing: &mut HostExposure,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> CoreResult<()> {
        self.check_owner(thing)?;
        if !thing.is_committed() || thing.members.is_empty() {
            return Err(invalid());
        }
        let Some(_turn) = Driving::enter() else {
            return Ok(());
        };
        if let Err(error) = thing.observe_time(now) {
            thing.failure.get_or_insert(error);
        }
        let mut loan = thing.members[thing.next].cell().borrow_mut();
        thing.next = (thing.next + 1) % thing.members.len();
        let member = loan.as_deref_mut().unwrap();
        let key = thing.key();
        if !member.associated_with(&thing.issuer, key) {
            thing.failure.get_or_insert_with(stale);
            thing.publication.revoke();
            member.begin_retire(ProducerRetirement::Close);
        }
        let _ = member.poll_one(&self.owner, cx, now)?;
        if let Some(cause) = member.native_failure() {
            thing.failure.get_or_insert_with(|| cause.clone());
        }
        if let Err(error) = self.owner.reconcile_participation(key, member) {
            thing.failure.get_or_insert_with(|| error.clone());
            thing.publication.revoke();
            member.begin_retire(ProducerRetirement::Close);
            return Err(error);
        }
        Ok(())
    }
    /// Reclaim a dropped exposure after every original native member has gone.
    pub fn retire_revoked(&mut self, key: ProducerInstanceKey) -> CoreResult<()> {
        self.owner.reclaim_revoked(key)
    }
    pub fn retire(&mut self, thing: &mut HostExposure) -> CoreResult<()> {
        self.check_owner(thing)?;
        if !thing.is_committed() {
            return Err(invalid());
        }
        {
            let loans: Vec<_> = thing.members.iter().map(|m| m.cell().borrow()).collect();
            let members: Vec<&dyn ClosedProducerParticipation> =
                loans.iter().map(|m| m.as_deref().unwrap() as _).collect();
            self.owner
                .retire_composed(&mut thing.publication, &members)?;
        }
        thing.members.clear();
        Ok(())
    }
}

fn select(
    publication: &ProducerPublication,
    services: &[ServiceRef],
    limits: &ResourceLimits,
    work: &mut WorkBudget,
    facts: &mut PublicationFacts,
) -> CoreResult<PreparedThing> {
    let providers: Vec<&dyn ProducerCapability> =
        services.iter().map(|s| s.as_ref() as _).collect();
    let prepared = select_finite_thing(
        publication.staged_description().unwrap(),
        publication.key(),
        limits,
        work,
        &providers,
        |scope, op| publication_supports(publication, scope, op),
        facts,
    )?;
    if !publication.behaviors_described_by(|site, op| {
        prepared
            .operations()
            .iter()
            .any(|p| p.site.site() == site && p.operation == op)
    }) {
        return Err(CoreError::UnsupportedOperation(context()));
    }
    drop(PublicationFacts::lock_current(&[facts])?);
    Ok(prepared)
}
