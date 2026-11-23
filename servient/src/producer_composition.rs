//! The qualified handoff to the shared logical Servient. Preparation produces
//! independent definitions; native setup is owned by each participating
//! Binding; this boundary commits one final TD and behavior owner only after
//! all participants are installed closed and whole-description coverage is current.
#![allow(clippy::result_large_err)]
pub use crate::producer_execution::{OwnedProducerParticipation, ProducerWork};
use crate::{
    host_accounts::{HostCapacityReservation, HostResourceAccounts},
    producer::{Funding, QualificationFunding},
};
use alloc::{rc::Rc, sync::Arc, vec::Vec};
use clinkz_wot_core::{
    AffordanceTarget, BindingInputRejection, BindingOperationalError, producer::ProducerInstanceKey,
};
use clinkz_wot_core::{CoreError, CoreResult, ErrorContext, ErrorPhase, Operation, RetryClass};
use clinkz_wot_foundation::{ResourceKind as K, WorkClass};
use clinkz_wot_foundation::{ResourceLimits, WorkBudget};
use clinkz_wot_planning::finite_producer::PRODUCER_DESCRIPTION_BYTES;
use clinkz_wot_planning::producer_composition::{
    FormSite, PreparedOperation, PreparedThing, ProducerCapability, ProducerQualification,
    ProducerRegistrations, PublicationFacts,
};
use clinkz_wot_servient_finite::{
    CapturedActionRoute, CapturedNotificationRoute, CapturedProducerRoutes,
    ClosedProducerParticipation, ProducerPublication, ProducerRetirement, RawProducerServient,
};
use clinkz_wot_servient_finite::{MAX_PRODUCER_PARTICIPANTS, RawProducerInstance};
use clinkz_wot_td::ValidatedProducerScope;
use core::{any::Any, mem::size_of};

// The existing bounded eligible-provider walk is independent of native membership.
const MAX_PRODUCER_REGISTRATIONS: usize = 32;

/// Final qualification is fallible, publication is not. Retain the prepared
/// definitions separately from this logical publication and all physical
/// resources. The borrowed registration slice fixes membership for this call.
/// Reloadable membership in managed staging uses `ProducerRegistrations`.
/// No native poll, handler, allocation or fallible callback is made
/// inside the local commit; native gate acquisition is checked beforehand.
pub fn publish_prepared_thing(
    owner: &mut RawProducerServient,
    publication: &mut ProducerPublication,
    prepared: &PreparedThing,
    providers: &[&dyn ProducerCapability],
    participants: &mut [&mut dyn ClosedProducerParticipation],
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
) -> CoreResult<()> {
    check_publication_limits(
        limits,
        prepared.notification_mapping_count(),
        participants.len(),
        publication.key(),
    )?;
    let qualification = ProducerQualification::begin(providers, PublicationFacts::default())?;
    pay_coverage(
        prepared,
        participants.len(),
        publication.behavior_count(),
        budget,
    )?;
    let description = publication
        .staged_description()
        .ok_or_else(|| CoreError::Validation(context()))?;
    let coverage = qualification.check_final(
        description,
        publication.key(),
        limits,
        budget,
        prepared,
        |scope, op| publication_supports(publication, scope, op),
    )?;
    if !publication.behaviors_described_by(|site, op| {
        prepared
            .operations()
            .iter()
            .any(|p| p.site.site() == site && p.operation == op)
    }) {
        return Err(CoreError::UnsupportedOperation(context()));
    }
    owner.publish_if_current_with_cleanup(
        publication,
        participants,
        &coverage,
        Some(budget),
        |key, participants| capture_mappings(key, prepared, participants).map(Some),
    )
}

fn capture_mappings(
    key: ProducerInstanceKey,
    prepared: &PreparedThing,
    participants: &[&mut dyn ClosedProducerParticipation],
) -> CoreResult<CapturedProducerRoutes> {
    let members = participants.iter().map(|p| &**p);
    check_participant_coverage(key, prepared, members.clone())?;
    Ok(CapturedProducerRoutes {
        actions: capture_action_routes(key, prepared, members.clone())?,
        notifications: capture_notification_routes(key, prepared, members)?,
    })
}

fn capture_notification_routes<'a>(
    key: ProducerInstanceKey,
    prepared: &PreparedThing,
    participants: impl ExactSizeIterator<Item = &'a dyn ClosedProducerParticipation> + Clone,
) -> CoreResult<Vec<CapturedNotificationRoute>> {
    let mut routes = Vec::new();
    routes
        .try_reserve_exact(prepared.notification_mapping_count())
        .map_err(|_| busy())?;
    for operation in prepared.operations() {
        let Some(kind) = operation.notification_start() else {
            continue;
        };
        let target = match &operation.site {
            FormSite::Thing => AffordanceTarget::Thing,
            FormSite::Property(n) => AffordanceTarget::Property(Arc::from(n.as_str())),
            FormSite::Event(n) => AffordanceTarget::Event(Arc::from(n.as_str())),
            FormSite::Action(_) => return Err(CoreError::UnsupportedOperation(context())),
        };
        let participant = participants
            .clone()
            .find(|p| {
                p.covers(
                    key,
                    operation.binding,
                    operation.site.site(),
                    operation.form,
                    operation.operation,
                    operation.definition_any(),
                )
            })
            .ok_or_else(|| CoreError::UnsupportedOperation(context()))?;
        if participant.notification_publication_slots() == 0 {
            return Err(CoreError::UnsupportedOperation(context()));
        }
        let (route, services, data_bytes) = participant
            .notification_publication(
                key,
                operation.binding,
                operation.site.site(),
                operation.form,
                operation.definition_any(),
            )
            .ok_or_else(|| CoreError::UnsupportedOperation(context()))?;
        if route.binding() != operation.binding || data_bytes == 0 {
            return Err(CoreError::Validation(context()));
        }
        routes.push(CapturedNotificationRoute {
            target,
            kind,
            route,
            services,
            data_bytes,
        });
    }
    Ok(routes)
}

fn action_operations(prepared: &PreparedThing) -> impl Iterator<Item = &PreparedOperation> {
    prepared.operations().iter().filter(|o| {
        matches!(o.site, FormSite::Action(_)) || o.operation == Operation::QueryAllActions
    })
}

fn capture_action_routes<'a>(
    key: ProducerInstanceKey,
    prepared: &PreparedThing,
    participants: impl ExactSizeIterator<Item = &'a dyn ClosedProducerParticipation> + Clone,
) -> CoreResult<Vec<CapturedActionRoute>> {
    let mut routes = Vec::new();
    routes
        .try_reserve_exact(action_operations(prepared).count())
        .map_err(|_| busy())?;
    for operation in action_operations(prepared) {
        let target = match &operation.site {
            FormSite::Action(name) => AffordanceTarget::Action(Arc::from(name.as_str())),
            FormSite::Thing if operation.operation == Operation::QueryAllActions => {
                AffordanceTarget::Thing
            }
            _ => continue,
        };
        let participant = participants
            .clone()
            .find(|p| {
                p.covers(
                    key,
                    operation.binding,
                    operation.site.site(),
                    operation.form,
                    operation.operation,
                    operation.definition_any(),
                )
            })
            .ok_or_else(|| CoreError::UnsupportedOperation(context()))?;
        let (route, validator) = match operation.operation {
            Operation::InvokeAction => {
                let (route, validator) = participant
                    .action_validation(
                        key,
                        operation.binding,
                        operation.site.site(),
                        operation.form,
                        operation.definition_any(),
                    )
                    .ok_or_else(|| CoreError::UnsupportedOperation(context()))?;
                (route, Some(validator))
            }
            Operation::QueryAction | Operation::CancelAction | Operation::QueryAllActions => (
                participant
                    .action_control_route(
                        key,
                        operation.binding,
                        operation.site.site(),
                        operation.form,
                        operation.operation,
                        operation.definition_any(),
                    )
                    .ok_or_else(|| CoreError::UnsupportedOperation(context()))?,
                None,
            ),
            _ => return Err(CoreError::UnsupportedOperation(context())),
        };
        if route.binding != operation.binding {
            return Err(CoreError::Validation(context()));
        }
        routes.push(CapturedActionRoute {
            target,
            operation: operation.operation,
            route,
            validation_work: validator
                .as_ref()
                .map_or(0, |v| v.additional_validation_work()),
            validator,
        });
    }
    Ok(routes)
}

fn pay_coverage(
    prepared: &PreparedThing,
    participants: usize,
    behaviors: usize,
    budget: &mut WorkBudget,
) -> CoreResult<()> {
    let operations = prepared.operations().len();
    let mappings = prepared.notification_mapping_count();
    if behaviors > operations {
        return Err(CoreError::UnsupportedOperation(context()));
    }
    budget
        .consume(WorkClass::HandlerSteps, operations as u64)
        .map_err(|_| busy())?;
    let actions = action_operations(prepared).count();
    // Count record comparisons in the complete-set walks. An Action behavior
    // can advertise invoke, query and cancel in the reverse behavior check.
    let comparisons = [
        (operations, 3 * participants), // coverage both ways, then capture
        (operations, 4 * behaviors),    // behavior lookup and reverse coverage
        (mappings + actions, 2 * behaviors), // routes and behaviors both ways
        (actions, actions),             // duplicate Action routes
        (mappings, mappings),           // duplicate captured mappings
        (behaviors, behaviors),         // duplicate registrations
        (participants, participants),   // duplicate member identities
    ];
    let units = comparisons
        .into_iter()
        .try_fold(0u64, |total, (rows, width)| {
            total.checked_add((rows as u64).checked_mul(width as u64)?)
        })
        .ok_or_else(busy)?;
    budget
        .consume(WorkClass::HandlerSteps, units)
        .map_err(|_| busy())?;
    Ok(())
}

fn slot_limit(resource: K, limit: u64, count: usize, key: ProducerInstanceKey) -> CoreError {
    CoreError::LimitExceeded {
        resource,
        limit,
        requested: Some(count as u64),
        observed: None,
        context: context().with_thing(key.slot()),
    }
}

fn check_publication_limits(
    limits: &ResourceLimits,
    mappings: usize,
    participants: usize,
    key: ProducerInstanceKey,
) -> CoreResult<()> {
    for (kind, count) in [
        (K::BindingRoutesPerThingMax, mappings),
        (K::BindingRoutesGlobalMax, mappings),
        (K::RouteReadinessTokensPerThingMax, participants),
        (K::RouteReadinessTokensGlobalMax, participants),
    ] {
        let mut limit = limits.get(kind).unwrap_or(0);
        if kind == K::RouteReadinessTokensPerThingMax {
            limit = limit.min(MAX_PRODUCER_PARTICIPANTS as u64);
        }
        if count as u64 > limit {
            return Err(slot_limit(kind, limit, count, key));
        }
    }
    Ok(())
}

fn reserve_slots(
    accounts: &Arc<HostResourceAccounts>,
    limits: &ResourceLimits,
    kind: K,
    count: usize,
    key: ProducerInstanceKey,
) -> CoreResult<HostCapacityReservation> {
    let ceiling = limits.get(kind).unwrap_or(0);
    let mut slots = accounts.reserve(kind, 0).expect("empty Host reservation");
    if count != 0 && !slots.try_grow_up_to(count as u64, ceiling) {
        return Err(slot_limit(
            kind,
            ceiling.min(accounts.account(kind).limit()),
            count,
            key,
        ));
    }
    Ok(slots)
}

fn check_participant_coverage<'a>(
    key: ProducerInstanceKey,
    prepared: &PreparedThing,
    participants: impl ExactSizeIterator<Item = &'a dyn ClosedProducerParticipation> + Clone,
) -> CoreResult<()> {
    if participants.len() == 0 {
        return Err(CoreError::Validation(context()));
    }
    for operation in prepared.operations() {
        let owners = participants
            .clone()
            .filter(|p| {
                p.covers(
                    key,
                    operation.binding,
                    operation.site.site(),
                    operation.form,
                    operation.operation,
                    operation.definition_any(),
                )
            })
            .count();
        if owners != 1 {
            return Err(CoreError::UnsupportedOperation(context()));
        }
    }
    // A ready but unselected participant must never be opened: it could make
    // native ingress for a different, already-live logical Thing dispatch.
    if participants.clone().any(|participant| {
        let covered = prepared
            .operations()
            .iter()
            .filter(|operation| {
                participant.covers(
                    key,
                    operation.binding,
                    operation.site.site(),
                    operation.form,
                    operation.operation,
                    operation.definition_any(),
                )
            })
            .count();
        covered == 0
            || participant
                .qualified_operation_count()
                .is_some_and(|count| count != covered)
    }) {
        return Err(CoreError::UnsupportedOperation(context()));
    }
    Ok(())
}

/// Check actual staged behavior, not merely a nominal operation family. A
/// Thing-level collection Form needs its own registered collection behavior;
/// it is not lowered into individual Property reads.
pub fn publication_supports(
    publication: &ProducerPublication,
    scope: ValidatedProducerScope<'_>,
    op: Operation,
) -> bool {
    match scope {
        ValidatedProducerScope::Thing => {
            publication.supports(&clinkz_wot_core::AffordanceTarget::Thing, op)
        }
        ValidatedProducerScope::Property(n) => publication.supports_property(n, op),
        ValidatedProducerScope::Action(n) => publication.supports_action(n, op),
        ValidatedProducerScope::Event(n) => publication.supports_event(n, op),
    }
}
fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}

/// Pure caller-provisioned owners. Construction/compilation precede this
/// ownership handoff and acquire no native resources. The deployment's startup
/// registration backing and the raw logical root/Action-store baseline remain
/// caller-provisioned; their references are retained here. Native participants
/// declare their additional Engine backing, separately from SDK/OS policy.
pub struct ComposedProducerInput {
    pub instance: RawProducerInstance,
    pub prepared: PreparedThing,
    pub providers: ProducerRegistrations,
    pub participants: Vec<OwnedProducerParticipation>,
    pub expected: Option<ProducerInstanceKey>,
    /// Trusted complete handler indirect backing, additional to actual handler
    /// layouts. Output/native allowances come from the actual participants.
    pub handler_backing_bytes: usize,
}

/// Owns the fixed complete native participation set and its original aggregate
/// bank reservations. Typed loans cannot silently substitute a member: exact
/// opaque identities are rechecked at publication and retirement, and each
/// actual participant independently retains the same funding owner.
pub struct ComposedProducerExposure {
    participants: Vec<OwnedProducerParticipation>,
    publication: ProducerPublication,
    stage: Option<ProducerStage>,
    publication_failure: Option<CoreError>,
    _funding: Rc<ComposedFunding>,
}

struct ProducerStage {
    prepared: PreparedThing,
    _registrations: ProducerRegistrations,
    coverage: PublicationFacts,
    _qualification: QualificationFunding,
}

struct ComposedAdmission {
    funding: Rc<ComposedFunding>,
    qualification: QualificationFunding,
    coverage: PublicationFacts,
    routes: CapturedProducerRoutes,
}

struct ComposedFunding {
    _backing: Funding,
    _notification_mappings: HostCapacityReservation,
    _members: HostCapacityReservation,
}

impl ComposedProducerExposure {
    /// Admission either returns all original input owners or takes the complete
    /// handoff after reserving persistent/result/cleanup and final scratch.
    pub fn try_new(
        owner: &RawProducerServient,
        input: ComposedProducerInput,
        limits: Arc<ResourceLimits>,
        accounts: Arc<HostResourceAccounts>,
        budget: &mut WorkBudget,
    ) -> Result<Self, BindingInputRejection<ComposedProducerInput>> {
        let admission = (|| -> CoreResult<ComposedAdmission> {
            let mut coverage = PublicationFacts::default();
            let registrations = input.providers.snapshot(&mut coverage)?;
            if input.participants.is_empty()
                || registrations.is_empty()
                || registrations.len() > MAX_PRODUCER_REGISTRATIONS
                || input.prepared.operations().is_empty()
            {
                return Err(CoreError::Validation(context()));
            }
            let mappings = input.prepared.notification_mapping_count();
            let members = input.participants.len();
            let key = input.instance.key();
            check_publication_limits(&limits, mappings, members, key)?;
            let notification_mappings =
                reserve_slots(&accounts, &limits, K::BindingRoutesGlobalMax, mappings, key)?;
            let member_slots = reserve_slots(
                &accounts,
                &limits,
                K::RouteReadinessTokensGlobalMax,
                members,
                key,
            )?;
            pay_coverage(
                &input.prepared,
                members,
                input.instance.behavior_count(),
                budget,
            )?;
            // Validate the raw owner's bounded registration envelope before
            // traversing or accounting caller-supplied handler collections.
            owner.check_publication_input(&input.instance, input.expected)?;
            if input.participants.iter().enumerate().any(|(i, p)| {
                !p.has_original_native()
                    || input.participants[..i]
                        .iter()
                        .any(|prior| p.participation_id().same_owner(prior.participation_id()))
            }) {
                return Err(CoreError::Validation(context()));
            }
            let providers: Vec<_> = registrations.iter().map(|p| p.as_ref()).collect();
            let qualification = ProducerQualification::begin(&providers, coverage)?;
            let routes = capture_action_routes(
                input.instance.key(),
                &input.prepared,
                input.participants.iter().map(|p| p.as_participation()),
            )?;
            let notification_routes = capture_notification_routes(
                input.instance.key(),
                &input.prepared,
                input.participants.iter().map(|p| p.as_participation()),
            )?;
            let mut bytes = size_of::<Self>()
                .checked_add(input.instance.backing_bytes().ok_or_else(busy)?)
                .and_then(|n| n.checked_add(input.prepared.retained_bytes()))
                .and_then(|n| n.checked_add(ProducerPublication::staging_backing_for(members)))
                .and_then(|n| n.checked_add(input.handler_backing_bytes))
                .and_then(|n| {
                    n.checked_add(CapturedActionRoute::backing_bytes(
                        &routes,
                        routes.capacity(),
                    )?)
                })
                .and_then(|n| {
                    n.checked_add(CapturedNotificationRoute::backing_bytes(
                        &notification_routes,
                        notification_routes.capacity(),
                    )?)
                })
                .and_then(|n| {
                    n.checked_add(
                        input.participants.capacity() * size_of::<OwnedProducerParticipation>(),
                    )
                })
                .and_then(|n| {
                    n.checked_add(registrations.len() * size_of::<Rc<dyn ProducerCapability>>())
                })
                // Original Funding, retention-pair Rc allocations and bounded
                // temporary references used for qualification and gate capture.
                .and_then(|n| {
                    n.checked_add(
                        size_of::<ComposedFunding>()
                            + (input.participants.len() + 3) * 8 * size_of::<usize>()
                            + (registrations.len() + 2 * input.participants.len())
                                * size_of::<&dyn ClosedProducerParticipation>(),
                    )
                })
                .ok_or_else(busy)?;
            for p in &input.participants {
                bytes = bytes
                    .checked_add(
                        p.admission_backing_bytes(
                            &owner.participation_token(),
                            input.instance.key(),
                        )
                        .and_then(|native| {
                            native.checked_add(p.overhead())?.checked_add(
                                p.notification_publication_slots().checked_mul(
                                    CapturedNotificationRoute::publication_overhead_bytes(),
                                )?,
                            )
                        })
                        .ok_or_else(|| CoreError::UnsupportedOperation(context()))?,
                    )
                    .ok_or_else(busy)?;
            }
            let backing = Funding::reserve(
                &accounts,
                &[
                    (K::EngineLiveBytesGlobalMax, bytes),
                    (K::CleanupBytesMax, bytes),
                    (K::CleanupItemsMax, input.participants.len() + 1),
                    (K::ThingsGlobalMax, 1),
                ],
            )?;
            let funding = Rc::new(ComposedFunding {
                _backing: backing,
                _notification_mappings: notification_mappings,
                _members: member_slots,
            });
            let provider_bytes = final_check_bytes(&registrations)?;
            let temporary = usize::try_from(
                limits
                    .get(K::AdmissionTemporaryBytesPerOperationMax)
                    .unwrap_or(0),
            )
            .ok()
            .and_then(|n| n.checked_add(PRODUCER_DESCRIPTION_BYTES * 512))
            .and_then(|n| n.checked_add(provider_bytes))
            .and_then(|n| n.checked_add(qualification.backing_bytes()?))
            .ok_or_else(busy)?;
            let scratch = QualificationFunding::reserve(&accounts, temporary)?;
            let description = input
                .instance
                .description()
                .ok_or_else(|| CoreError::Validation(context()))?;
            let coverage = qualification.check_final(
                description,
                input.instance.key(),
                &limits,
                budget,
                &input.prepared,
                |scope, op| match scope {
                    ValidatedProducerScope::Thing => input
                        .instance
                        .supports(&clinkz_wot_core::AffordanceTarget::Thing, op),
                    ValidatedProducerScope::Property(n) => input.instance.supports_property(n, op),
                    ValidatedProducerScope::Action(n) => input.instance.supports_action(n, op),
                    ValidatedProducerScope::Event(n) => input.instance.supports_event(n, op),
                },
            )?;
            if !input.instance.behaviors_described_by(|site, op| {
                input
                    .prepared
                    .operations()
                    .iter()
                    .any(|p| p.site.site() == site && p.operation == op)
            }) {
                return Err(CoreError::UnsupportedOperation(context()));
            }
            check_participant_coverage(
                input.instance.key(),
                &input.prepared,
                input.participants.iter().map(|p| p.as_participation()),
            )?;
            drop(PublicationFacts::lock_current(&[&coverage])?);
            Ok(ComposedAdmission {
                funding,
                qualification: scratch,
                coverage,
                routes: CapturedProducerRoutes {
                    actions: routes,
                    notifications: notification_routes,
                },
            })
        })();
        let ComposedAdmission {
            funding,
            qualification,
            coverage,
            routes,
        } = match admission {
            Ok(funding) => funding,
            Err(error) => return Err(reject(input, error)),
        };
        let ComposedProducerInput {
            instance,
            prepared,
            providers,
            mut participants,
            expected,
            handler_backing_bytes,
        } = input;
        let refs: Vec<_> = participants.iter().map(|p| p.as_participation()).collect();
        let mut publication =
            match owner.prepare_composed_publication(instance, expected, &refs, routes) {
                Ok(publication) => publication,
                Err(rejection) => {
                    let (instance, error) = rejection.into_parts();
                    return Err(BindingInputRejection::new(
                        ComposedProducerInput {
                            instance,
                            prepared,
                            providers,
                            participants,
                            expected,
                            handler_backing_bytes,
                        },
                        error,
                    ));
                }
            };
        publication
            .retain_owner(funding.clone())
            .expect("new unpublished instance is exclusive");
        publication
            .bind_action_result_accounts(accounts.clone())
            .expect("new unpublished instance has no previous result bank");
        for participant in &mut participants {
            participant.retain_owner(funding.clone());
        }
        Ok(Self {
            participants,
            publication,
            stage: Some(ProducerStage {
                prepared,
                _registrations: providers,
                coverage,
                _qualification: qualification,
            }),
            publication_failure: None,
            _funding: funding,
        })
    }

    pub fn key(&self) -> ProducerInstanceKey {
        self.publication.key()
    }
    pub fn is_committed(&self) -> bool {
        self.publication.is_committed()
    }
    /// First final-qualification/local-publication failure. Native failures
    /// remain at their concrete progress/settlement owners.
    pub fn publication_failure(&self) -> Option<&CoreError> {
        self.publication_failure.as_ref()
    }

    /// A concrete native loan grants neither a new participant slot nor logical
    /// dispatch authority. A moved/replaced driver keeps its own original charge
    /// and causes the complete-set checks to refuse the substituted member.
    pub fn participant_mut<T: Any>(&mut self, index: usize) -> Option<&mut T> {
        self.participants
            .get_mut(index)?
            .as_any_mut()
            .downcast_mut()
    }

    /// One bounded turn of every retained member. Supply independent credit per
    /// participant and lane; an error or short turn never skips a sibling.
    /// Native readiness and maintenance turns are deployment responsibilities,
    /// as for Consumer execution. This does not publish, emit, query, cancel a
    /// remote operation, acknowledge results or claim completed retirement.
    pub fn step(
        &mut self,
        owner: &mut RawProducerServient,
        cx: &mut core::task::Context<'_>,
        now: clinkz_wot_foundation::MonotonicInstant,
        mut credit: impl FnMut(usize, ProducerWork) -> WorkBudget,
    ) -> CoreResult<()> {
        let key = self.key();
        let mut failure = None;
        for (index, participant) in self.participants.iter_mut().enumerate() {
            if let Err(error) =
                participant.step(owner, key, cx, now, &mut |lane| credit(index, lane))
                && failure
                    .as_ref()
                    .is_none_or(|e| matches!(e, CoreError::Backpressure(_)))
            {
                failure = Some(error);
            }
        }
        failure.map_or(Ok(()), Err)
    }

    /// First returned operational turn error, or the member's native failure
    /// evidence. Full settlement and cleanup remain at their original owners.
    pub fn progress_failure(&self, index: usize) -> Option<&CoreError> {
        self.participants.get(index)?.failure()
    }

    pub fn publish(
        &mut self,
        owner: &mut RawProducerServient,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        // A completed or retired exposure no longer owns a candidate.
        let stage = self
            .stage
            .as_ref()
            .ok_or_else(|| CoreError::Validation(context()))?;
        let result = (|| {
            pay_coverage(
                &stage.prepared,
                self.participants.len(),
                self.publication.behavior_count(),
                budget,
            )?;
            for participant in &self.participants {
                if !participant.has_original_native() {
                    return Err(CoreError::StaleHandle(context()));
                }
                if let Some(error) = participant.native_failure() {
                    return Err(error.clone());
                }
                if participant.retired() {
                    return Err(CoreError::Cancelled(context()));
                }
            }
            // A stale candidate can roll back even while native setup is pending.
            drop(PublicationFacts::lock_current(&[&stage.coverage])?);
            if !self
                .participants
                .iter()
                .all(|p| p.ready_for(&owner.participation_token(), self.publication.key()))
            {
                if self
                    .participants
                    .iter()
                    .any(|p| p.ingress_gate().is_withdrawn())
                {
                    return Err(CoreError::Cancelled(context()));
                }
                return Err(busy());
            }
            let mut participants: Vec<_> = self
                .participants
                .iter_mut()
                .map(|p| p.as_participation_mut())
                .collect();
            owner.publish_if_current_with_cleanup(
                &mut self.publication,
                &mut participants,
                &stage.coverage,
                Some(budget),
                |key, members| capture_mappings(key, &stage.prepared, members).map(Some),
            )
        })();
        match result {
            Ok(()) => {
                self.stage = None;
                self.publication.release_retired();
                Ok(())
            }
            Err(error) => {
                // A terminal native cause can itself be Backpressure, including
                // when a member failed during the last installation callback.
                let recorded = self
                    .participants
                    .iter()
                    .filter(|p| p.has_original_native())
                    .find_map(|p| p.native_failure());
                let cause = match recorded {
                    Some(native) => native.clone(),
                    None if matches!(error, CoreError::Backpressure(_)) => return Err(error),
                    None => error,
                };
                self.publication_failure = Some(cause.clone());
                self.begin_close();
                Err(cause)
            }
        }
    }

    pub fn begin_close(&mut self) {
        self.publication.revoke();
        self.stage = None;
        for participant in &mut self.participants {
            if participant.has_original_native() {
                participant.begin_retire(ProducerRetirement::Close);
            }
        }
    }

    /// Retirement always uses the retained original membership. It cannot take
    /// a caller-selected subset, and pending native/Action results keep funding.
    pub fn retire(
        &mut self,
        owner: &mut RawProducerServient,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        owner.retire_composed_with_budget(
            &mut self.publication,
            self.participants.iter().map(|p| p.as_participation()),
            budget,
        )?;
        // Typed native close may have ended setup without begin_close() on
        // this facade. Acknowledged retirement permanently ends qualification.
        self.stage = None;
        Ok(())
    }
}
impl Drop for ComposedProducerExposure {
    fn drop(&mut self) {
        self.publication.revoke();
        self.stage = None;
        // Actual member Drop supplies bounded local disposal. No I/O polling,
        // application cancellation callback or completed drain is claimed here.
    }
}

fn busy() -> CoreError {
    CoreError::Backpressure(context())
}
fn final_check_bytes(providers: &[Rc<dyn ProducerCapability>]) -> CoreResult<usize> {
    providers.iter().try_fold(0, |maximum, provider| {
        provider
            .final_check_temporary_bytes()
            .map(|bytes| maximum.max(bytes))
            .ok_or_else(|| CoreError::UnsupportedOperation(context()))
    })
}
fn reject(
    input: ComposedProducerInput,
    error: CoreError,
) -> BindingInputRejection<ComposedProducerInput> {
    BindingInputRejection::new(input, BindingOperationalError::new(error))
}
