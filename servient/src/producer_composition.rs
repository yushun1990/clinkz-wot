//! The qualified handoff to the shared logical Servient. Preparation produces
//! independent definitions; native setup is owned by each participating
//! Binding; this boundary commits one final TD and behavior owner only after
//! all participants are closed and the *whole* description is requalified.
#![allow(clippy::result_large_err)]
use crate::{
    host_accounts::HostResourceAccounts,
    producer::{Funding, QualificationFunding},
};
use alloc::{boxed::Box, rc::Rc, sync::Arc, vec::Vec};
use clinkz_wot_core::{
    BindingInputRejection, BindingOperationalError, producer::ProducerInstanceKey,
};
use clinkz_wot_core::{CoreError, CoreResult, ErrorContext, ErrorPhase, Operation, RetryClass};
use clinkz_wot_foundation::{ResourceKind as K, WorkClass};
use clinkz_wot_foundation::{ResourceLimits, WorkBudget};
use clinkz_wot_planning::finite_producer::PRODUCER_DESCRIPTION_BYTES;
use clinkz_wot_planning::producer_composition::{
    PreparedThing, ProducerCapability, check_final_thing,
};
use clinkz_wot_servient_finite::{
    CapturedActionRoute, CapturedNotificationRoute, ClosedProducerParticipation,
    ProducerPublication, RawProducerServient,
};
use clinkz_wot_servient_finite::{MAX_PRODUCER_PARTICIPANTS, RawProducerInstance};
use clinkz_wot_td::ValidatedProducerScope;
use core::{any::Any, mem::size_of};

/// Final qualification is fallible, publication is not. Retain the prepared
/// definitions separately from this logical publication and all physical
/// resources. No native poll, handler, allocation or fallible callback is made
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
    check_prepared_thing(
        owner,
        publication,
        prepared,
        providers,
        participants,
        limits,
        budget,
    )?;
    if prepared.operations().iter().any(|o| {
        matches!(
            o.site,
            clinkz_wot_planning::producer_composition::FormSite::Action(_)
        )
    }) {
        let routes = capture_action_routes(
            publication.key(),
            prepared,
            participants.iter().map(|p| &**p),
        )?;
        publication.bind_action_routes(routes)?;
    }
    publication.bind_notification_routes(capture_notification_routes(
        publication.key(),
        prepared,
        participants.iter().map(|p| &**p),
    )?)?;
    owner.publish_composed(publication, participants)
}

fn capture_notification_routes<'a>(
    key: ProducerInstanceKey,
    prepared: &PreparedThing,
    participants: impl ExactSizeIterator<Item = &'a dyn ClosedProducerParticipation> + Clone,
) -> CoreResult<Vec<CapturedNotificationRoute>> {
    let mut routes = Vec::new();
    let count = prepared
        .operations()
        .iter()
        .filter(|o| {
            matches!(
                o.operation,
                Operation::ObserveProperty | Operation::SubscribeEvent
            )
        })
        .count();
    // This manually driven Host publication has a fixed comparison envelope.
    // Larger sets require another indexed/resumable representation.
    if count > MAX_PRODUCER_PARTICIPANTS {
        return Err(CoreError::UnsupportedOperation(context()));
    }
    routes.try_reserve_exact(count).map_err(|_| busy())?;
    for operation in prepared.operations() {
        use clinkz_wot_planning::producer_composition::FormSite;
        let target = match (&operation.site, operation.operation) {
            (FormSite::Property(n), Operation::ObserveProperty) => {
                clinkz_wot_core::AffordanceTarget::Property(Arc::from(n.as_str()))
            }
            (FormSite::Event(n), Operation::SubscribeEvent) => {
                clinkz_wot_core::AffordanceTarget::Event(Arc::from(n.as_str()))
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
        if routes
            .iter()
            .any(|r: &CapturedNotificationRoute| r.target == target && r.route.same_mapping(&route))
        {
            return Err(CoreError::UnsupportedOperation(context()));
        }
        routes.push(CapturedNotificationRoute {
            target,
            route,
            services,
            data_bytes,
        });
    }
    Ok(routes)
}

fn capture_action_routes<'a>(
    key: ProducerInstanceKey,
    prepared: &PreparedThing,
    participants: impl ExactSizeIterator<Item = &'a dyn ClosedProducerParticipation> + Clone,
) -> CoreResult<Vec<CapturedActionRoute>> {
    let mut routes = Vec::new();
    routes
        .try_reserve_exact(
            prepared
                .operations()
                .iter()
                .filter(|o| {
                    matches!(
                        o.site,
                        clinkz_wot_planning::producer_composition::FormSite::Action(_)
                    )
                })
                .count(),
        )
        .map_err(|_| busy())?;
    for operation in prepared.operations() {
        let clinkz_wot_planning::producer_composition::FormSite::Action(name) = &operation.site
        else {
            continue;
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
            Operation::QueryAction | Operation::CancelAction => (
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
            action: Arc::from(name.as_str()),
            operation: operation.operation,
            route,
            validator,
        });
    }
    Ok(routes)
}

fn check_prepared_thing(
    _owner: &RawProducerServient,
    publication: &ProducerPublication,
    prepared: &PreparedThing,
    providers: &[&dyn ProducerCapability],
    participants: &[&mut dyn ClosedProducerParticipation],
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
) -> CoreResult<()> {
    if participants.len() > MAX_PRODUCER_PARTICIPANTS {
        return Err(CoreError::Validation(context()));
    }
    let units = prepared
        .operations()
        .len()
        .checked_mul(2 * participants.len() + 32)
        .and_then(|n| u64::try_from(n).ok())
        .ok_or_else(busy)?;
    budget
        .consume(WorkClass::HandlerSteps, units)
        .map_err(|_| busy())?;
    let description = publication
        .staged_description()
        .ok_or_else(|| CoreError::Validation(context()))?;
    check_final_thing(
        description,
        publication.key(),
        limits,
        budget,
        providers,
        prepared,
        |scope, op| publication_supports(publication, scope, op),
    )?;
    if !publication.behaviors_described_by(|site, op| {
        prepared
            .operations()
            .iter()
            .any(|prepared| prepared.site.site() == site && prepared.operation == op)
    }) {
        return Err(CoreError::UnsupportedOperation(context()));
    }
    check_participant_coverage(
        publication.key(),
        prepared,
        participants.iter().map(|p| &**p),
    )
}

fn check_participant_coverage<'a>(
    key: ProducerInstanceKey,
    prepared: &PreparedThing,
    participants: impl ExactSizeIterator<Item = &'a dyn ClosedProducerParticipation> + Clone,
) -> CoreResult<()> {
    if participants.len() == 0 || participants.len() > MAX_PRODUCER_PARTICIPANTS {
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
        !prepared.operations().iter().any(|operation| {
            participant.covers(
                key,
                operation.binding,
                operation.site.site(),
                operation.form,
                operation.operation,
                operation.definition_any(),
            )
        })
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

/// Host storage erasure only. Native polling, Action progress and controls
/// remain their concrete interfaces; this introduces no universal driver.
pub trait OwnedProducerParticipation: ClosedProducerParticipation + Any {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn as_participation(&self) -> &dyn ClosedProducerParticipation;
    fn as_participation_mut(&mut self) -> &mut dyn ClosedProducerParticipation;
}
impl<T: ClosedProducerParticipation + Any> OwnedProducerParticipation for T {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn as_participation(&self) -> &dyn ClosedProducerParticipation {
        self
    }
    fn as_participation_mut(&mut self) -> &mut dyn ClosedProducerParticipation {
        self
    }
}

/// Pure caller-provisioned owners. Construction/compilation precede this
/// ownership handoff and acquire no native resources. Actual immutable startup
/// registrations and the raw logical root/Action-store baseline remain
/// caller-provisioned; their references are retained here. Native participants
/// declare their additional Engine backing, separately from SDK/OS policy.
pub struct ComposedProducerInput {
    pub instance: RawProducerInstance,
    pub prepared: PreparedThing,
    pub providers: Vec<Rc<dyn ProducerCapability>>,
    pub participants: Vec<Box<dyn OwnedProducerParticipation>>,
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
    participants: Vec<Box<dyn OwnedProducerParticipation>>,
    prepared: PreparedThing,
    providers: Vec<Rc<dyn ProducerCapability>>,
    publication: ProducerPublication,
    limits: Arc<ResourceLimits>,
    qualification: Option<QualificationFunding>,
    qualification_provider_bytes: usize,
    publication_failure: Option<CoreError>,
    _funding: Rc<Funding>,
}

struct ComposedAdmission {
    funding: Rc<Funding>,
    qualification: QualificationFunding,
    provider_bytes: usize,
    routes: Vec<CapturedActionRoute>,
    notification_routes: Vec<CapturedNotificationRoute>,
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
            if input.participants.is_empty()
                || input.participants.len() > MAX_PRODUCER_PARTICIPANTS
                || input.providers.is_empty()
                || input.providers.len() > MAX_PRODUCER_PARTICIPANTS
                || input.prepared.operations().is_empty()
            {
                return Err(CoreError::Validation(context()));
            }
            let units = input
                .prepared
                .operations()
                .len()
                .checked_mul(2 * input.participants.len() + 32)
                .and_then(|n| n.checked_add(MAX_PRODUCER_PARTICIPANTS * MAX_PRODUCER_PARTICIPANTS))
                .and_then(|n| u64::try_from(n).ok())
                .ok_or_else(busy)?;
            budget
                .consume(WorkClass::HandlerSteps, units)
                .map_err(|_| busy())?;
            // Validate the raw owner's bounded registration envelope before
            // traversing or accounting caller-supplied handler collections.
            owner.check_publication_input(&input.instance, input.expected)?;
            if input.participants.iter().enumerate().any(|(i, p)| {
                input.participants[..i]
                    .iter()
                    .any(|prior| p.participation_id().same_owner(prior.participation_id()))
            }) {
                return Err(CoreError::Validation(context()));
            }
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
                .and_then(|n| n.checked_add(ProducerPublication::staging_backing_bytes()))
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
                        input.participants.capacity()
                            * size_of::<Box<dyn OwnedProducerParticipation>>(),
                    )
                })
                .and_then(|n| {
                    n.checked_add(
                        input.providers.capacity() * size_of::<Rc<dyn ProducerCapability>>(),
                    )
                })
                // Original Funding, retention-pair Rc allocations and bounded
                // temporary reference arrays used by qualification/retirement.
                .and_then(|n| {
                    n.checked_add(
                        size_of::<Funding>()
                            + (input.participants.len() + 3) * 8 * size_of::<usize>()
                            + (input.providers.len() + 2 * input.participants.len())
                                * size_of::<&dyn ClosedProducerParticipation>(),
                    )
                })
                .ok_or_else(busy)?;
            for p in &input.participants {
                bytes = bytes
                    .checked_add(
                        p.admission_backing_bytes(owner, input.instance.key())
                            .ok_or_else(|| CoreError::UnsupportedOperation(context()))?,
                    )
                    .ok_or_else(busy)?;
            }
            let funding = Rc::new(Funding::reserve(
                &accounts,
                &[
                    (K::EngineLiveBytesGlobalMax, bytes),
                    (K::CleanupBytesMax, bytes),
                    (K::CleanupItemsMax, input.participants.len() + 1),
                    (K::ThingsGlobalMax, 1),
                ],
            )?);
            let provider_bytes = final_check_bytes(&input.providers)?;
            let temporary = usize::try_from(
                limits
                    .get(K::AdmissionTemporaryBytesPerOperationMax)
                    .unwrap_or(0),
            )
            .ok()
            .and_then(|n| n.checked_add(PRODUCER_DESCRIPTION_BYTES * 512))
            .and_then(|n| n.checked_add(provider_bytes))
            .ok_or_else(busy)?;
            let qualification = QualificationFunding::reserve(&accounts, temporary)?;
            let providers: Vec<_> = input.providers.iter().map(|p| p.as_ref()).collect();
            let description = input
                .instance
                .description()
                .ok_or_else(|| CoreError::Validation(context()))?;
            check_final_thing(
                description,
                input.instance.key(),
                &limits,
                budget,
                &providers,
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
            Ok(ComposedAdmission {
                funding,
                qualification,
                provider_bytes,
                routes,
                notification_routes,
            })
        })();
        let ComposedAdmission {
            funding,
            qualification,
            provider_bytes: qualification_provider_bytes,
            routes,
            notification_routes,
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
        let mut publication = match owner.prepare_publication(instance, expected) {
            Ok(publication) => publication,
            Err(rejection) => {
                return Err(reject(
                    ComposedProducerInput {
                        instance: rejection.into_input(),
                        prepared,
                        providers,
                        participants,
                        expected,
                        handler_backing_bytes,
                    },
                    CoreError::StaleHandle(context()),
                ));
            }
        };
        publication
            .retain_owner(funding.clone())
            .expect("new unpublished instance is exclusive");
        let refs: Vec<_> = participants.iter().map(|p| p.as_participation()).collect();
        publication
            .bind_participants(&refs)
            .expect("bounded unique original set checked before handoff");
        publication
            .bind_action_routes(routes)
            .expect("selected services and exact Action behaviors checked before handoff");
        publication
            .bind_notification_routes(notification_routes)
            .expect("selected services and exact notification sources checked before handoff");
        for participant in &mut participants {
            participant.retain_owner(funding.clone());
        }
        Ok(Self {
            participants,
            prepared,
            providers,
            publication,
            limits,
            qualification: Some(qualification),
            qualification_provider_bytes,
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
            .as_mut()
            .as_any_mut()
            .downcast_mut()
    }

    pub fn publish(
        &mut self,
        owner: &mut RawProducerServient,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        if self.qualification.is_none() || self.publication.is_committed() {
            return Err(CoreError::Validation(context()));
        }
        let units = self
            .prepared
            .operations()
            .len()
            .checked_mul(2 * self.participants.len() + 32)
            .and_then(|n| u64::try_from(n).ok())
            .ok_or_else(busy)?;
        budget
            .consume(WorkClass::HandlerSteps, units)
            .map_err(|_| busy())?;
        // The captured allowance must still cover every actual callback before
        // any support/check code can allocate. Growing/unknown bounds invalidate
        // this staged qualification instead of borrowing an unfunded callback.
        if let Err(error) = final_check_bytes(&self.providers).and_then(|bytes| {
            if bytes <= self.qualification_provider_bytes {
                Ok(())
            } else {
                Err(CoreError::Validation(context()))
            }
        }) {
            self.publication_failure = Some(error.clone());
            self.begin_close();
            return Err(error);
        }
        let providers: Vec<_> = self.providers.iter().map(|p| p.as_ref()).collect();
        let mut participants: Vec<_> = self
            .participants
            .iter_mut()
            .map(|p| p.as_participation_mut())
            .collect();
        if !participants
            .iter()
            .all(|p| p.ready_for(owner, self.publication.key()))
        {
            return Err(busy());
        }
        if let Err(error) = check_prepared_thing(
            owner,
            &self.publication,
            &self.prepared,
            &providers,
            &participants,
            &self.limits,
            budget,
        ) {
            if !matches!(error, CoreError::Backpressure(_)) {
                self.publication_failure = Some(error.clone());
                self.publication.revoke();
                self.qualification = None;
                for p in &mut participants {
                    p.begin_retire();
                }
            }
            return Err(error);
        }
        // Recheck actual mapping/service selections before the local commit.
        // A participant that changes routes while still claiming coverage must
        // not make the staged qualified controls reachable under stale keys.
        if let Err(error) = capture_action_routes(
            self.publication.key(),
            &self.prepared,
            participants.iter().map(|p| &**p),
        )
        .and_then(|routes| self.publication.bind_action_routes(routes))
        .and_then(|()| {
            capture_notification_routes(
                self.publication.key(),
                &self.prepared,
                participants.iter().map(|p| &**p),
            )
        })
        .and_then(|routes| self.publication.bind_notification_routes(routes))
        {
            self.publication_failure = Some(error.clone());
            self.publication.revoke();
            self.qualification = None;
            for participant in &mut participants {
                participant.begin_retire();
            }
            return Err(error);
        }
        if let Err(error) = owner.publish_composed(&mut self.publication, &mut participants) {
            self.publication_failure = Some(error.clone());
            self.publication.revoke();
            self.qualification = None;
            for participant in &mut participants {
                participant.begin_retire();
            }
            return Err(error);
        }
        // No phase scratch is retained by accepted work or unread results.
        self.qualification = None;
        self.publication.release_retired();
        Ok(())
    }

    pub fn begin_close(&mut self) {
        self.publication.revoke();
        self.qualification = None;
        for participant in &mut self.participants {
            participant.begin_retire();
        }
    }

    /// Retirement always uses the retained original membership. It cannot take
    /// a caller-selected subset, and pending native/Action results keep funding.
    pub fn retire(
        &mut self,
        owner: &mut RawProducerServient,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        budget
            .consume(
                WorkClass::CleanupItems,
                (MAX_PRODUCER_PARTICIPANTS + 128) as u64,
            )
            .map_err(|_| busy())?;
        let participants: Vec<_> = self
            .participants
            .iter()
            .map(|p| p.as_participation())
            .collect();
        owner.retire_composed(&mut self.publication, &participants)?;
        // Typed native close may have ended setup without begin_close() on
        // this facade. Acknowledged retirement permanently ends qualification.
        self.qualification = None;
        Ok(())
    }
}
impl Drop for ComposedProducerExposure {
    fn drop(&mut self) {
        self.publication.revoke();
        self.qualification = None;
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
