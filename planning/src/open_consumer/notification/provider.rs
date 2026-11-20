//! Type erasure ends at selection. A winning definition
//! moves intact into the deployment's output after the revision decision.
use super::*;
use ::core::mem::size_of;
use clinkz_wot_core::security::SecurityRequirement;

/// A startup registration participating in one bounded Form search. Construct
/// this from the original service owners with `into_provider`.
pub struct ConsumerNotificationProvider<D> {
    pub(super) id: BindingId,
    pub(super) native: Box<dyn Candidate<D>>,
}

impl<B: ConsumerNotificationPreparation> RetainedConsumerNotificationRegistration<B>
where
    B::Configuration: Clone,
{
    /// Supply a constructor for the application's selected output. The search
    /// calls it exactly once on success, after qualification and uniqueness.
    /// Single-type registries use the identity constructor.
    pub fn into_provider<D: 'static>(
        self,
        selected: fn(ConsumerNotificationDefinition<B>) -> D,
    ) -> ConsumerNotificationProvider<D> {
        ConsumerNotificationProvider {
            id: self.binding.registration_identity().binding_id(),
            native: Box::new(TypedProvider {
                registration: self,
                selected,
            }),
        }
    }
}

struct TypedProvider<B: ConsumerNotificationBinding, D> {
    registration: RetainedConsumerNotificationRegistration<B>,
    selected: fn(ConsumerNotificationDefinition<B>) -> D,
}

pub(super) trait Candidate<D> {
    fn check(&self, id: BindingId) -> CoreResult<()>;
    fn probe<'a>(
        &self,
        id: BindingId,
        form: &Form<'_>,
        choice: FormChoice<'a, D>,
        admission: &'a ConsumerAdmission<'a>,
        budget: &mut WorkBudget,
        denied: &mut Option<CoreError>,
    ) -> CoreResult<FormChoice<'a, D>>;
}

// The first winner takes the only copy of the schemas. Competitors borrow
// those exact trees from the funded definition while determining uniqueness.
pub(super) enum FormChoice<'a, D> {
    Open {
        schemas: NotificationSchemas,
        funding: HostCapacityReservation,
        peak: AdmissionBytes<'a>,
    },
    Selected(Selected<'a, D>),
}

pub(super) struct Selected<'a, D> {
    definition: Box<dyn Definition<D>>,
    // Field order keeps both charges live during refusal and local disposal.
    _mapping_peak: AdmissionBytes<'a>,
    _schema_peak: AdmissionBytes<'a>,
}
impl<D> Selected<'_, D> {
    pub(super) fn handoff(self) -> D {
        self.definition.handoff()
    }
}

trait Definition<D> {
    fn schemas(&self) -> &NotificationSchemas;
    fn handoff(self: Box<Self>) -> D;
}

struct TypedDefinition<B: ConsumerNotificationBinding, D> {
    definition: ConsumerNotificationDefinition<B>,
    selected: fn(ConsumerNotificationDefinition<B>) -> D,
}
impl<B: ConsumerNotificationBinding, D> Definition<D> for TypedDefinition<B, D> {
    fn schemas(&self) -> &NotificationSchemas {
        &self.definition.input().schemas
    }
    fn handoff(self: Box<Self>) -> D {
        (self.selected)(self.definition)
    }
}

impl<B: ConsumerNotificationPreparation, D: 'static> Candidate<D> for TypedProvider<B, D>
where
    B::Configuration: Clone,
{
    fn check(&self, id: BindingId) -> CoreResult<()> {
        let r = &self.registration;
        let identity = r.binding.registration_identity();
        if identity.binding_id() != id
            || r.binding.identity() != (id, identity.binding_generation())
            || r.binding.preparation_bounds().work == 0
            || r.services.access.work() == 0
            || r.services.data.policy().validation_work == 0
        {
            return Err(invalid());
        }
        r.services.check_backing()
    }

    fn probe<'a>(
        &self,
        registered_id: BindingId,
        form: &Form<'_>,
        choice: FormChoice<'a, D>,
        admission: &'a ConsumerAdmission<'a>,
        budget: &mut WorkBudget,
        denied: &mut Option<CoreError>,
    ) -> CoreResult<FormChoice<'a, D>> {
        self.check(registered_id)?;
        let r = &self.registration;
        let id = r.binding.registration_identity();
        let bounds = r.binding.preparation_bounds();
        let native_config = r.binding.configuration();
        let access = &r.services.access;
        let data = &r.services.data;
        let policy = data.policy();
        let data_revision = data.qualification_revision();
        let access_work = access.work();
        let access_revision = access.qualification_revision();
        let schemas = match &choice {
            FormChoice::Open { schemas, .. } => schemas,
            FormChoice::Selected(winner) => winner.definition.schemas(),
        };
        let eligible = || {
            data.available()
                && data.content_type() == form.facts.content_type
                && schemas.supports(data.service())
                && access.supports_security(&SecurityRequirement::NOSEC)
        };
        if !eligible() {
            return Ok(choice);
        }
        let current = || {
            r.services.check_backing()?;
            if r.binding.registration_identity() != id
                || r.binding.identity() != (registered_id, id.binding_generation())
                || r.binding.configuration() != native_config
                || r.binding.preparation_bounds() != bounds
                || data.policy() != policy
                || data.qualification_revision() != data_revision
                || access.work() != access_work
                || access.qualification_revision() != access_revision
                || !eligible()
            {
                return Err(invalid());
            }
            Ok(())
        };
        debit(budget, W::ProviderProbes, access_work)?;
        let authorized = access.authorize(id, form.endpoint, schemas.kind().start_operation());
        current()?;
        match authorized {
            Ok(()) => {}
            Err(
                e @ CoreError::Security {
                    reason: SecurityFailureReason::AuthorizationDenied,
                    ..
                },
            ) => {
                denied.get_or_insert(e);
                return Ok(choice);
            }
            Err(
                CoreError::UnsupportedOperation(_)
                | CoreError::Validation(_)
                | CoreError::Binding(_)
                | CoreError::Security {
                    reason:
                        SecurityFailureReason::MissingCredentials
                        | SecurityFailureReason::InvalidCredentials
                        | SecurityFailureReason::UnsupportedScheme,
                    ..
                },
            ) => return Ok(choice),
            Err(e) => return Err(e),
        }
        check_limit(
            admission.limits,
            K::PlanCompileWorkUnitsPerStepMax,
            bounds.work,
        )?;
        check_limit(
            admission.limits,
            K::BindingArtifactBytesPerThingMax,
            bounds.retained_bytes,
        )?;
        debit(budget, W::BindingPolls, bounds.work)?;
        debit(budget, W::CleanupItems, 1)?;
        let definition_bytes = size_of::<TypedDefinition<B, D>>();
        check_limit(
            admission.limits,
            K::LargestContiguousAllocationBytesMax,
            definition_bytes as u64,
        )?;
        let fixed = definition_bytes
            + 3 * size_of::<HostCapacityReservation>()
            + form.endpoint.len()
            + form.facts.content_type.len()
            + form
                .target
                .name()
                .map_or(0, |n| n.len() + 2 * size_of::<usize>());
        let retained = bounds
            .retained_bytes
            .checked_add(fixed as u64)
            .ok_or_else(invalid)?;
        let (peak, engine) = admission.retained(retained)?;
        let cleanup = admission.reserve_global(K::CleanupItemsMax, 1)?;
        let mut funding = Vec::new();
        funding.try_reserve_exact(3).map_err(|_| busy())?;
        let scratch = admission.temporary(bounds.temporary_bytes)?;
        let configuration = native_config.clone();
        let compiled = r
            .binding
            .compile_notification_form(ConsumerNotificationInput {
                document: form.document,
                target: form.target,
                form: form.index,
                endpoint: form.endpoint,
                schemas,
                facts: &form.facts,
            });
        drop(scratch);
        let supported = compiled.and_then(|program| {
            let Some(program) = program else {
                return Ok(None);
            };
            let binding = r.binding.clone();
            let paired = || {
                binding.registration_identity() == id
                    && binding.configuration() == &configuration
                    && binding.identity() == (registered_id, id.binding_generation())
            };
            if !paired() {
                return Err(invalid());
            }
            let support = binding.support(&program, schemas.kind());
            if !paired() {
                return Err(invalid());
            }
            Ok(Some((binding, program, support?)))
        });
        current()?;
        if r.binding.configuration() != &configuration {
            return Err(invalid());
        }
        let (binding, program, support) = match supported {
            Ok(Some(p)) => p,
            Ok(None) | Err(CoreError::UnsupportedOperation(_)) => return Ok(choice),
            Err(e) => return Err(e),
        };
        let FormChoice::Open {
            schemas,
            funding: schema_engine,
            peak: schema_peak,
        } = choice
        else {
            return Err(CoreError::Selection {
                reason: SelectionFailureReason::AmbiguousBindingOwner,
                context: context(),
            });
        };
        funding.extend([engine, cleanup, schema_engine]);
        let definition =
            ConsumerNotificationDefinition::try_new(ConsumerNotificationDefinitionInput {
                binding,
                program,
                configuration,
                identity: id,
                target: form.target.clone(),
                endpoint: form.endpoint.into(),
                form: form.index,
                schemas,
                data: data.clone(),
                data_policy: policy,
                content_type: form.facts.content_type.into(),
                support,
                funding,
                bank: admission.bank.clone(),
            })
            .map_err(|e| match e.error().error() {
                // The same executor just qualified this exact program.
                CoreError::UnsupportedOperation(_) => invalid(),
                other => other.clone(),
            })?;
        Ok(FormChoice::Selected(Selected {
            definition: Box::new(TypedDefinition {
                definition,
                selected: self.selected,
            }),
            _mapping_peak: peak,
            _schema_peak: schema_peak,
        }))
    }
}
