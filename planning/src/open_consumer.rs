//! Open startup Consumer composition. Common TD admission is shared; native
//! compilation is portable, owned and incremental. Only completed source-free
//! definitions cross into execution. Finite and notification lifecycles remain
//! separate. Typed notification preparation is in the sibling facet below.
mod notification;
use crate::producer_composition::FormSite;
use crate::*;
use alloc::{sync::Arc, vec::Vec};
use clinkz_wot_core::{consumer::*, host_accounts::*, *};
use clinkz_wot_foundation::{
    AdmissionLedger, Generation, ResourceKind as K, ResourceLimits, SlotIndex, WorkClass as W,
};
use clinkz_wot_td::{
    semantic::{SecurityDetails, UriVariableScope},
    thing::Thing,
    *,
};
pub use notification::*;

const MAX_REGISTRATIONS: usize = 32;
const MAX_OPERATIONS: usize = 128;

pub struct ConsumerRegistrationInput {
    pub preparation: HostConsumerBindingPreparation,
    pub services: ConsumerServices,
}
struct Registration {
    compiler: HostBindingCompilerRegistration,
    execution: Arc<HostConsumerBindingRegistration>,
    services: ConsumerServices,
    metadata: ConsumerCompilerMetadata,
}
/// Immutable actual startup-linked registrations. Construction is caller-owned
/// startup work, with recoverable pairing; preparation reserves managed backing.
pub struct ConsumerRegistry {
    registrations: Vec<Registration>,
}
impl ConsumerRegistry {
    pub fn try_new(
        inputs: Vec<ConsumerRegistrationInput>,
    ) -> Result<Self, BindingInputRejection<Vec<ConsumerRegistrationInput>>> {
        let valid = !inputs.is_empty()
            && inputs.len() <= MAX_REGISTRATIONS
            && inputs.iter().enumerate().all(|(i, input)| {
                let p = &input.preparation;
                let Some(m) = p.compiler().consumer_metadata() else {
                    return false;
                };
                p.consumer_configuration() == Some(m.configuration)
                    && m.configuration.configuration() == p.identity().configuration()
                    && p.compiler().compatibility() == p.identity().artifact_compatibility()
                    && !inputs[..i].iter().any(|old| {
                        old.preparation.identity().binding_id() == p.identity().binding_id()
                    })
            });
        if !valid {
            return Err(BindingInputRejection::new(
                inputs,
                BindingOperationalError::new(invalid()),
            ));
        }
        let mut registrations = Vec::new();
        if registrations.try_reserve_exact(inputs.len()).is_err() {
            return Err(BindingInputRejection::new(
                inputs,
                BindingOperationalError::new(busy()),
            ));
        }
        for input in inputs {
            let metadata = input.preparation.compiler().consumer_metadata().unwrap();
            let (execution, compiler) = input.preparation.into_parts();
            registrations.push(Registration {
                compiler,
                execution: Arc::new(execution),
                metadata,
                services: input.services,
            });
        }
        Ok(Self { registrations })
    }

    /// Eager admission with explicitly resumable native jobs. The driver can
    /// supply small step budgets and cancellation while the one common semantic
    /// loan lives. Returning before a job finishes aborts it under prepaid work;
    /// no partially prepared output is installed. The registry/compiler can be
    /// destroyed immediately after successful return.
    pub fn prepare(
        &self,
        thing: &Thing,
        limits: &ResourceLimits,
        bank: Arc<HostResourceAccounts>,
        selector: ConsumerSelector,
        budget: &mut WorkBudget,
        mut drive: impl FnMut(&mut ConsumerCompilation<'_>),
    ) -> CoreResult<PreparedConsumer> {
        // Prepay common arena, normalized coordinate/output vectors and TD
        // continuation backing before Basic or semantic allocation.
        let common = limits
            .get(K::AdmissionTemporaryBytesPerOperationMax)
            .unwrap_or(0);
        let _scratch = reserve(
            &bank,
            &[
                (K::EngineLiveBytesGlobalMax, common),
                (K::AdmissionTemporaryBytesGlobalMax, common),
            ],
        )?;
        let vector_bytes = (MAX_OPERATIONS
            * (::core::mem::size_of::<ConsumerSupport>()
                + ::core::mem::size_of::<ConsumerDefinition>()
                + 512)) as u64;
        let mut output_funding = reserve(
            &bank,
            &[
                (K::EngineLiveBytesGlobalMax, vector_bytes),
                (K::CleanupItemsMax, MAX_OPERATIONS as u64),
            ],
        )?;
        let mut definitions = Vec::new();
        let mut support = Vec::new();
        definitions
            .try_reserve_exact(MAX_OPERATIONS)
            .map_err(|_| busy())?;
        support
            .try_reserve_exact(MAX_OPERATIONS)
            .map_err(|_| busy())?;
        let config =
            ValidatedThingAdmissionConfig::try_from_limits(limits).map_err(|_| invalid())?;
        let mut basic = ValidatedThingCursor::from_thing(
            thing,
            &config,
            AdmissionLedger::new(
                SlotIndex::new(0),
                Generation::INITIAL,
                0,
                common,
                0,
                0,
                0,
                0,
            ),
        );
        let validated = loop {
            let before = credits(budget);
            basic = match basic.step(budget, false) {
                ValidatedThingProgress::Complete(validated) => break validated,
                ValidatedThingProgress::Pending(next) => next,
                ValidatedThingProgress::Failed(cause) => {
                    return Err(crate::producer_property_read::td_error(cause));
                }
            };
            if before == credits(budget) {
                return Err(busy());
            }
        };
        // This is the shared whole-description walk, not Producer qualification:
        // unsupported operations/Forms below remain reported and may be skipped.
        let mut cursor = validated.into_producer();
        let mut ordinal = 0u32;
        let mut retained_cleanup_work = 4u64;
        let mut retained_artifact_items = 0u64;
        let mut retained_artifact_bytes = 0u64;
        loop {
            if !pay(budget, W::PlanningItems, 1) {
                return Err(busy());
            }
            match cursor
                .step(budget, false)
                .map_err(crate::producer_property_read::td_error)?
            {
                ValidatedProducerStep::Done => break,
                ValidatedProducerStep::Pending => {}
                ValidatedProducerStep::Ready(ValidatedProducerEvent::Scope { .. }) => {
                    cursor.acknowledge()
                }
                ValidatedProducerStep::Ready(ValidatedProducerEvent::Form(form)) => {
                    let site = match form.scope {
                        ValidatedProducerScope::Thing => FormSite::Thing,
                        ValidatedProducerScope::Property(n) => FormSite::Property(n.into()),
                        ValidatedProducerScope::Action(n) => FormSite::Action(n.into()),
                        ValidatedProducerScope::Event(n) => FormSite::Event(n.into()),
                    };
                    let index = form.index;
                    // The vocabulary's operation set is small and bounded by TD
                    // admission. Copy only enums; normalize common facts once/Form.
                    let ops = form.effective_operations.to_vec();
                    let failed_ops = ops.clone();
                    let resolved = cursor.with_form_semantics(
                        site.loan(),
                        index,
                        budget,
                        |document, target, budget| -> CoreResult<()> {
                            for operation in ops {
                                if support.len() == MAX_OPERATIONS || site_name(&site).len() > 256 {
                                    return Err(unsupported());
                                }
                                let mut chosen = None;
                                let mut chosen_budget = (0u64, 0u64, 0u64);
                                let finite = matches!(
                                    (&site, operation),
                                    (
                                        FormSite::Property(_),
                                        Operation::ReadProperty | Operation::WriteProperty
                                    ) | (FormSite::Action(_), Operation::InvokeAction)
                                );
                                let mut reason = if !finite {
                                    ConsumerUnsupported::Operation
                                } else if selector.form.is_some_and(|f| f != index) {
                                    ConsumerUnsupported::Selector
                                } else if target.len() > 256 {
                                    ConsumerUnsupported::Target
                                } else {
                                    ConsumerUnsupported::CommonMeaning
                                };
                                if finite
                                    && selector.form.is_none_or(|f| f == index)
                                    && target.len() <= 256
                                {
                                    let facts = document
                                        .form(site.owner(), index)
                                        .ok_or_else(unsupported)?;
                                    if !pay(budget, W::PlanningItems, 256) {
                                        return Err(busy());
                                    }
                                    let schemas =
                                        common_services(document, &site, index, operation);
                                    if let Ok(schemas) = schemas {
                                        reason = ConsumerUnsupported::NoCompleteProvider;
                                        for registration in &self.registrations {
                                            let id = registration.execution.identity();
                                            if selector
                                                .binding
                                                .is_some_and(|b| b != id.binding_id())
                                            {
                                                continue;
                                            }
                                            if !pay(budget, W::ProviderProbes, 1) {
                                                return Err(busy());
                                            }
                                            let data = &registration.services.data;
                                            if !registration.execution.supports_operation(operation)
                                                || data.content_type() != facts.content_type
                                                || !data.available()
                                                || !schemas
                                                    .input
                                                    .into_iter()
                                                    .chain(schemas.output)
                                                    .all(|s| data.supports_schema(s))
                                            {
                                                continue;
                                            }
                                            let m = registration.metadata;
                                            qualify_contract(m, limits)?;
                                            if !pay(
                                                budget,
                                                W::BindingPolls,
                                                m.contract.inspection_work + 2,
                                            ) {
                                                return Err(busy());
                                            }
                                            let _inspect = reserve(
                                                &bank,
                                                &[
                                                    (
                                                        K::EngineLiveBytesGlobalMax,
                                                        m.contract.inspection_temporary_bytes,
                                                    ),
                                                    (
                                                        K::AdmissionTemporaryBytesGlobalMax,
                                                        m.contract.inspection_temporary_bytes,
                                                    ),
                                                ],
                                            )?;
                                            if registration.execution.consumer_configuration()
                                                != Some(m.configuration)
                                            {
                                                return Err(invalid());
                                            }
                                            if !pay(
                                                budget,
                                                W::ProviderProbes,
                                                registration.services.access.work(),
                                            ) {
                                                return Err(busy());
                                            }
                                            if registration
                                                .services
                                                .access
                                                .authorize(id, target, operation)
                                                .is_err()
                                            {
                                                continue;
                                            }
                                            let candidate = BindingCandidate::new(
                                                id.binding_id(),
                                                id.binding_generation(),
                                                id.configuration(),
                                                id.artifact_compatibility(),
                                                id.diagnostic_ordinal(),
                                                0,
                                            );
                                            let input = BindingCompilerInput::operation_input(
                                                PlanId::new(
                                                    SlotIndex::new(ordinal),
                                                    Generation::INITIAL,
                                                ),
                                                candidate,
                                                BindingArtifactRole::ConsumerCall,
                                                ResolvedBindingInput {
                                                    operation,
                                                    owner: site.owner(),
                                                    property: site_name(&site),
                                                    form_index: index,
                                                    target,
                                                    content_type: Some(facts.content_type),
                                                    subprotocol: facts.subprotocol,
                                                    document,
                                                },
                                            );
                                            let bounds = match registration.compiler.bounds(&input)
                                            {
                                                Ok(b) => b,
                                                Err(CoreError::UnsupportedOperation(_)) => {
                                                    continue;
                                                }
                                                Err(e) => return Err(e),
                                            };
                                            // Bound complete rollback of all retained definitions,
                                            // including one provisional candidate, before acquisition.
                                            check_limit(
                                                limits,
                                                K::CleanupWorkItemsPerStepMax,
                                                retained_cleanup_work
                                                    .checked_add(m.contract.abort_work)
                                                    .and_then(|n| {
                                                        n.checked_add(
                                                            m.contract.artifact_drop_work + 4,
                                                        )
                                                    })
                                                    .ok_or_else(invalid)?,
                                            )?;
                                            let artifact_budget = (
                                                u64::from(bounds.artifact().retained_items()),
                                                bounds.artifact().retained_bytes(),
                                            );
                                            check_limit(
                                                limits,
                                                K::BindingArtifactsPerThingMax,
                                                retained_artifact_items
                                                    .checked_add(artifact_budget.0)
                                                    .ok_or_else(invalid)?,
                                            )?;
                                            check_limit(
                                                limits,
                                                K::BindingArtifactBytesPerThingMax,
                                                retained_artifact_bytes
                                                    .checked_add(artifact_budget.1)
                                                    .ok_or_else(invalid)?,
                                            )?;
                                            let mut job = ConsumerCompilation::new(
                                                &registration.compiler,
                                                input,
                                                bounds,
                                                m,
                                                bank.clone(),
                                                limits,
                                                budget,
                                            )?;
                                            drive(&mut job);
                                            let mut compiled = match job.finish() {
                                                Ok(a) => a,
                                                Err(CoreError::UnsupportedOperation(_)) => {
                                                    continue;
                                                }
                                                Err(e) => return Err(e),
                                            };
                                            if chosen.is_some() {
                                                return Err(ambiguous());
                                            }
                                            let service_bytes = data
                                                .policy()
                                                .retained_bytes
                                                .checked_add(
                                                    registration.services.access.retained_bytes(),
                                                )
                                                .ok_or_else(invalid)?;
                                            let retained_services = service_bytes
                                                .checked_add(
                                                    registration
                                                        .execution
                                                        .resources()
                                                        .admitted()
                                                        .retained_bytes(),
                                                )
                                                .and_then(|n| n.checked_add(target.len() as u64))
                                                .and_then(|n| {
                                                    n.checked_add(site_name(&site).len() as u64)
                                                })
                                                .and_then(|n| n.checked_add(256))
                                                .ok_or_else(invalid)?;
                                            let mut service_funding = reserve(
                                                &bank,
                                                &[(K::EngineLiveBytesGlobalMax, retained_services)],
                                            )?;
                                            compiled.funding.append(&mut service_funding);
                                            retained_artifact_items += artifact_budget.0;
                                            retained_artifact_bytes += artifact_budget.1;
                                            chosen_budget = (
                                                artifact_budget.0,
                                                artifact_budget.1,
                                                m.contract.artifact_drop_work + 2,
                                            );
                                            retained_cleanup_work = retained_cleanup_work
                                                .checked_add(m.contract.artifact_drop_work + 2)
                                                .ok_or_else(invalid)?;
                                            chosen = Some(
                                                ConsumerDefinition::try_new(
                                                    ConsumerDefinitionInput {
                                                        affordance: site_target(&site).unwrap(),
                                                        operation,
                                                        form: index,
                                                        target: target.into(),
                                                        artifact: compiled.artifact,
                                                        execution: registration.execution.clone(),
                                                        configuration: m.configuration,
                                                        services: registration.services.clone(),
                                                        schemas,
                                                        funding: compiled.funding,
                                                        bank: bank.clone(),
                                                    },
                                                )
                                                .map_err(|e| e.error().error().clone())?,
                                            );
                                        }
                                    }
                                }
                                let binding = chosen.as_ref().map(|d| d.binding().binding_id());
                                // TD Form order is the explicit default preference.
                                // Provider ambiguity within one Form always refuses.
                                if !pay(
                                    budget,
                                    W::PlanningItems,
                                    (definitions.len() * (site_name(&site).len() + 1)) as u64,
                                ) {
                                    return Err(busy());
                                }
                                let selected = chosen.as_ref().is_some_and(|candidate| {
                                    !definitions.iter().any(|old: &ConsumerDefinition| {
                                        old.affordance() == candidate.affordance()
                                            && old.operation() == operation
                                    })
                                });
                                if selected {
                                    definitions.push(chosen.unwrap());
                                } else {
                                    retained_artifact_items -= chosen_budget.0;
                                    retained_artifact_bytes -= chosen_budget.1;
                                    retained_cleanup_work -= chosen_budget.2;
                                }
                                support.push(ConsumerSupport {
                                    site: site.clone(),
                                    form: index,
                                    operation,
                                    binding,
                                    selected,
                                    unsupported: binding.is_none().then_some(reason),
                                });
                                ordinal = ordinal.checked_add(1).ok_or_else(unsupported)?;
                            }
                            Ok(())
                        },
                    );
                    match resolved {
                        Ok(Some(result)) => result?,
                        Ok(None) => return Err(busy()),
                        Err(ValidatedSemanticError::Admission(cause)) => {
                            return Err(crate::producer_property_read::td_error(cause));
                        }
                        Err(ValidatedSemanticError::Resolution(
                            clinkz_wot_td::semantic::SemanticError::Allocation,
                        )) => return Err(busy()),
                        Err(ValidatedSemanticError::Resolution(_)) => {
                            for operation in failed_ops {
                                if support.len() == MAX_OPERATIONS || site_name(&site).len() > 256 {
                                    return Err(unsupported());
                                }
                                support.push(ConsumerSupport {
                                    site: site.clone(),
                                    form: index,
                                    operation,
                                    binding: None,
                                    selected: false,
                                    unsupported: Some(ConsumerUnsupported::SemanticMeaning),
                                });
                                ordinal = ordinal.checked_add(1).ok_or_else(unsupported)?;
                            }
                        }
                    }
                    cursor.acknowledge();
                }
            }
        }
        if definitions.is_empty() {
            return Err(unsupported());
        }
        // Vector/layout funding moves to the execution owner, never compiler or
        // source retention. Failed selection drops all previous pure definitions.
        Ok(PreparedConsumer {
            definitions,
            support,
            funding: ::core::mem::take(&mut output_funding),
        })
    }
}
#[derive(Clone, Copy, Default)]
pub struct ConsumerSelector {
    pub binding: Option<BindingId>,
    pub form: Option<u32>,
}
/// Stable refusal categories; provider order is never an error tie breaker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsumerUnsupported {
    Operation,
    Selector,
    Target,
    CommonMeaning,
    NoCompleteProvider,
    SemanticMeaning,
}
#[derive(Debug)]
pub struct ConsumerSupport {
    pub site: FormSite,
    pub form: u32,
    pub operation: Operation,
    /// Complete prepared support, distinct from a coarse scheme/type index.
    pub binding: Option<BindingId>,
    pub selected: bool,
    pub unsupported: Option<ConsumerUnsupported>,
}
pub struct PreparedConsumer {
    definitions: Vec<ConsumerDefinition>,
    support: Vec<ConsumerSupport>,
    funding: Vec<HostCapacityReservation>,
}
impl PreparedConsumer {
    pub fn support(&self) -> &[ConsumerSupport] {
        &self.support
    }
    pub fn into_execution(self) -> (Vec<ConsumerDefinition>, Vec<HostCapacityReservation>) {
        (self.definitions, self.funding)
    }
}

/// One pure job. No source loan or compiler survives `finish`; zero/short work
/// invokes no callback, the lifetime step allowance never renews, and rollback
/// and variable artifact destruction are prepaid before start.
pub struct ConsumerCompilation<'a> {
    compiler: &'a HostBindingCompilerRegistration,
    input: BindingCompilerInput<'a>,
    metadata: ConsumerCompilerMetadata,
    bounds: BindingCompilerBounds,
    cursor: Option<HostBindingCompilerCursor>,
    output: Option<BindingArtifactEnvelope<HostBindingArtifact>>,
    error: Option<CoreError>,
    started: bool,
    remaining: u64,
    native_remaining: u64,
    funding: Vec<HostCapacityReservation>,
    _temporary: Vec<HostCapacityReservation>,
}
// Keep the physical artifact and its original leases in one drop-ordered owner
// until every fallible accounting step has completed. Destructuring into locals
// earlier would refund the later-bound leases before dropping the artifact.
struct CompletedCompilation {
    artifact: BindingArtifactEnvelope<HostBindingArtifact>,
    funding: Vec<HostCapacityReservation>,
}
impl<'a> ConsumerCompilation<'a> {
    fn new(
        compiler: &'a HostBindingCompilerRegistration,
        input: BindingCompilerInput<'a>,
        bounds: BindingCompilerBounds,
        metadata: ConsumerCompilerMetadata,
        bank: Arc<HostResourceAccounts>,
        limits: &ResourceLimits,
        budget: &mut WorkBudget,
    ) -> CoreResult<Self> {
        let c = metadata.contract;
        // One nonrenewable transaction allowance is shared by all candidates.
        // Per-step caller credit schedules this prepaid job; it cannot renew it.
        let total_work = c
            .step_work
            .checked_add(1)
            .and_then(|n| n.checked_mul(c.max_steps))
            .and_then(|n| n.checked_add(c.start_work + 1))
            .ok_or_else(invalid)?;
        if !pay(budget, W::BindingPolls, total_work) {
            return Err(busy());
        }
        if W::ALL
            .into_iter()
            .any(|w| w != W::BindingPolls && bounds.work().remaining(w) != 0)
        {
            return Err(unsupported());
        }
        if bounds.work().remaining(W::BindingPolls)
            > c.step_work.checked_mul(c.max_steps).ok_or_else(invalid)?
        {
            return Err(invalid());
        }
        let retained = bounds
            .artifact()
            .retained_bytes()
            .checked_add(metadata.slot.size() as u64)
            .ok_or_else(invalid)?;
        let temporary = bounds
            .cursor_bytes()
            .checked_add(bounds.temporary_bytes())
            .ok_or_else(invalid)?;
        check_limit(
            limits,
            K::LargestContiguousAllocationBytesMax,
            retained.max(temporary),
        )?;
        check_limit(
            limits,
            K::BindingArtifactBytesPerItemMax,
            bounds.artifact().retained_bytes(),
        )?;
        check_limit(limits, K::AdmissionTemporaryBytesPerOperationMax, temporary)?;
        // Host slot allocation/release and result-envelope transport are included.
        if !pay(
            budget,
            W::CleanupItems,
            c.abort_work + c.artifact_drop_work + 4,
        ) {
            return Err(busy());
        }
        let funding = reserve(
            &bank,
            &[
                (K::EngineLiveBytesGlobalMax, retained),
                (
                    K::BindingArtifactsGlobalMax,
                    bounds.artifact().retained_items() as u64,
                ),
                (K::BindingArtifactBytesGlobalMax, retained),
                (K::CleanupBytesMax, retained),
                (K::CleanupItemsMax, 1),
            ],
        )?;
        let temporary = reserve(
            &bank,
            &[
                (K::EngineLiveBytesGlobalMax, temporary),
                (K::AdmissionTemporaryBytesGlobalMax, temporary),
            ],
        )?;
        let native_remaining = bounds.work().remaining(W::BindingPolls);
        Ok(Self {
            compiler,
            input,
            metadata,
            bounds,
            cursor: None,
            output: None,
            error: None,
            started: false,
            remaining: c.max_steps,
            native_remaining,
            funding,
            _temporary: temporary,
        })
    }
    pub fn is_finished(&self) -> bool {
        self.output.is_some() || self.error.is_some()
    }
    pub fn step(&mut self, budget: &mut WorkBudget, cancel: bool) {
        if self.is_finished() {
            return;
        }
        if cancel {
            self.error = Some(CoreError::Cancelled(context()));
            return;
        }
        let c = self.metadata.contract;
        if !self.started {
            if !pay(budget, W::BindingPolls, c.start_work + 1) {
                return;
            }
            match self.compiler.start(&self.input) {
                Ok(cursor) => {
                    self.cursor = Some(cursor);
                    self.started = true;
                }
                Err(error) => self.error = Some(error),
            }
            return;
        }
        if self.remaining == 0 || self.native_remaining == 0 {
            self.error = Some(invalid());
            return;
        }
        if !pay(budget, W::BindingPolls, c.step_work + 1) {
            return;
        }
        self.remaining -= 1;
        // The callback cannot spend from sibling or future work allowances.
        let allowance = c.step_work.min(self.native_remaining);
        let mut native = WorkBudget::new().with_remaining(W::BindingPolls, allowance);
        match self
            .compiler
            .step(&self.input, self.cursor.take().unwrap(), &mut native)
        {
            BindingCompilerStep::Pending(cursor) => self.cursor = Some(cursor),
            BindingCompilerStep::Failed(failure) => {
                let (error, cursor) = failure.into_parts();
                self.cursor = Some(cursor);
                self.error = Some(error);
            }
            BindingCompilerStep::Complete(output) => {
                let candidate = self.input.candidate();
                let identity = BindingArtifactIdentity::new(
                    PlanSetGeneration::INITIAL,
                    self.input.plan_id(),
                    candidate.binding_id(),
                    candidate.binding_generation(),
                    candidate.configuration(),
                    candidate.compatibility(),
                    BindingArtifactRole::ConsumerCall,
                );
                match BindingArtifactEnvelope::try_new(
                    identity,
                    self.bounds.artifact(),
                    output.into_artifact(),
                ) {
                    Ok(artifact) => self.output = Some(artifact),
                    Err(_) => self.error = Some(invalid()),
                }
            }
        }
        let unused = native.remaining(W::BindingPolls);
        if unused > allowance {
            self.error = Some(invalid());
        }
        self.native_remaining = self
            .native_remaining
            .saturating_sub(allowance.saturating_sub(unused));
    }
    fn finish(mut self) -> CoreResult<CompletedCompilation> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        let output = self.output.take().ok_or_else(busy)?;
        Ok(CompletedCompilation {
            artifact: output,
            funding: ::core::mem::take(&mut self.funding),
        })
    }
}
impl Drop for ConsumerCompilation<'_> {
    fn drop(&mut self) {
        if let Some(cursor) = self.cursor.take() {
            assert!(
                self.compiler.abort(cursor).is_ok(),
                "original compiler cursor"
            );
        }
        // Owned output/cursor destruction precedes every original reservation.
        drop(self.output.take());
    }
}
fn common_services(
    doc: &clinkz_wot_td::semantic::ResolvedSemanticDocument<'_>,
    site: &FormSite,
    index: u32,
    operation: Operation,
) -> CoreResult<ConsumerSchemas> {
    let facts = doc.form(site.owner(), index).ok_or_else(unsupported)?;
    let [security] = facts.effective_security else {
        return Err(unsupported());
    };
    let security = doc.security(security).map_err(|_| unsupported())?;
    if !matches!(security.details, SecurityDetails::NoSec)
        || security.proxy.is_some()
        || security.semantics.children().next().is_some()
        || facts.scopes.is_some()
        || facts.content_coding.is_some()
        || facts.additional_responses
        || facts
            .response_content_type
            .is_some_and(|m| m != facts.content_type)
        || doc
            .uri_variables(UriVariableScope::Thing)
            .is_some_and(|mut vars| vars.next().is_some())
        || doc
            .uri_variables(UriVariableScope::Property)
            .is_some_and(|mut vars| vars.next().is_some())
        || doc
            .uri_variables(UriVariableScope::Action)
            .is_some_and(|mut vars| vars.next().is_some())
    {
        return Err(unsupported());
    }
    match (site, operation) {
        (FormSite::Property(_), Operation::ReadProperty)
            if !doc.property().schema.facts().write_only =>
        {
            Ok(ConsumerSchemas {
                input: None,
                output: Some(crate::type_schema::prepare_type_schema(
                    doc.property().schema,
                )?),
            })
        }
        (FormSite::Property(_), Operation::WriteProperty)
            if !doc.property().schema.facts().read_only =>
        {
            Ok(ConsumerSchemas {
                input: Some(crate::type_schema::prepare_type_schema(
                    doc.property().schema,
                )?),
                output: None,
            })
        }
        (FormSite::Action(_), Operation::InvokeAction) => {
            // Absence makes no synchronous claim. Continuing invocations need
            // their own retained association and controls, not a finite facade.
            if doc.action().is_none_or(|a| a.synchronous != Some(true)) {
                return Err(unsupported());
            }
            Ok(ConsumerSchemas {
                input: doc
                    .affordance_schema(site.owner(), "input")
                    .map(crate::type_schema::prepare_type_schema)
                    .transpose()?,
                output: doc
                    .affordance_schema(site.owner(), "output")
                    .map(crate::type_schema::prepare_type_schema)
                    .transpose()?,
            })
        }
        _ => Err(unsupported()),
    }
}
fn qualify_contract(m: ConsumerCompilerMetadata, limits: &ResourceLimits) -> CoreResult<()> {
    let c = m.contract;
    if c.max_steps == 0
        || c.step_work == 0
        || c.inspection_work == 0
        || c.start_work == 0
        || c.abort_work == 0
        || c.artifact_drop_work == 0
    {
        return Err(invalid());
    }
    check_limit(
        limits,
        K::AdmissionTemporaryBytesPerOperationMax,
        c.inspection_temporary_bytes,
    )?;
    for amount in [c.inspection_work, c.start_work, c.step_work] {
        check_limit(
            limits,
            K::PlanCompileWorkUnitsPerStepMax,
            amount.checked_add(2).ok_or_else(invalid)?,
        )?;
    }
    check_limit(
        limits,
        K::CleanupWorkItemsPerStepMax,
        c.abort_work
            .checked_add(c.artifact_drop_work)
            .and_then(|n| n.checked_add(4))
            .ok_or_else(invalid)?,
    )
}
fn check_limit(limits: &ResourceLimits, kind: K, amount: u64) -> CoreResult<()> {
    let limit = limits.get(kind).unwrap_or(0);
    if amount > limit {
        return Err(CoreError::LimitExceeded {
            resource: kind,
            limit,
            requested: Some(amount),
            observed: None,
            context: context(),
        });
    }
    Ok(())
}
fn site_name(site: &FormSite) -> &str {
    match site {
        FormSite::Thing => "",
        FormSite::Property(n) | FormSite::Action(n) | FormSite::Event(n) => n,
    }
}
fn site_target(site: &FormSite) -> Option<AffordanceTarget> {
    match site {
        FormSite::Property(n) => Some(AffordanceTarget::Property(Arc::from(n.as_str()))),
        FormSite::Action(n) => Some(AffordanceTarget::Action(Arc::from(n.as_str()))),
        _ => None,
    }
}
fn credits(b: &WorkBudget) -> [u64; W::ALL.len()] {
    W::ALL.map(|w| b.remaining(w))
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(context())
}
fn ambiguous() -> CoreError {
    CoreError::Validation(
        context().with_redacted_cause(640, "multiple actual Consumer providers support one Form"),
    )
}
