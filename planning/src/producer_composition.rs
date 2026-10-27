//! Whole-Thing Producer selection. A prepared operation is an immutable,
//! source-independent Binding definition, not a logical instance or a native
//! listener. Each Form/operation can select a different registered capability.
#![allow(clippy::result_large_err)]
use crate::{finite_producer::PRODUCER_DESCRIPTION_BYTES, producer_property_read::td_error};
use alloc::{boxed::Box, string::String, vec::Vec};
use clinkz_wot_core::{
    BindingGeneration, BindingId, CoreError, CoreResult, ErrorContext, ErrorPhase, Operation,
    RetryClass, producer::ProducerInstanceKey, thing::AffordanceSite,
};
use clinkz_wot_foundation::{AdmissionLedger, ResourceKind, ResourceLimits, WorkBudget, WorkClass};
use clinkz_wot_td::{
    ValidatedProducerEvent, ValidatedProducerScope, ValidatedProducerStep, ValidatedSemanticError,
    ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress,
    semantic::{FormOwner, ResolvedSemanticDocument},
    thing::Thing,
};
use core::any::Any;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormSite {
    Thing,
    Property(String),
    Action(String),
    Event(String),
}
impl FormSite {
    fn from_scope(scope: ValidatedProducerScope<'_>) -> Self {
        match scope {
            ValidatedProducerScope::Thing => Self::Thing,
            ValidatedProducerScope::Property(n) => Self::Property(n.into()),
            ValidatedProducerScope::Action(n) => Self::Action(n.into()),
            ValidatedProducerScope::Event(n) => Self::Event(n.into()),
        }
    }
    pub fn loan(&self) -> ValidatedProducerScope<'_> {
        match self {
            Self::Thing => ValidatedProducerScope::Thing,
            Self::Property(n) => ValidatedProducerScope::Property(n),
            Self::Action(n) => ValidatedProducerScope::Action(n),
            Self::Event(n) => ValidatedProducerScope::Event(n),
        }
    }
    pub fn owner(&self) -> FormOwner<'_> {
        match self {
            Self::Thing => FormOwner::Thing,
            Self::Property(n) => FormOwner::Property(n),
            Self::Action(n) => FormOwner::Action(n),
            Self::Event(n) => FormOwner::Event(n),
        }
    }
    pub fn site(&self) -> AffordanceSite<'_> {
        match self {
            Self::Thing => AffordanceSite::Thing,
            Self::Property(n) => AffordanceSite::Property(n),
            Self::Action(n) => AffordanceSite::Action(n),
            Self::Event(n) => AffordanceSite::Event(n),
        }
    }
    fn name_bytes(&self) -> usize {
        match self {
            Self::Thing => 0,
            Self::Property(n) | Self::Action(n) | Self::Event(n) => n.len(),
        }
    }
}

/// Temporary common meaning. A Binding interprets native vocabularies and
/// returns only independently owned preparation. No handler or authority is
/// passed to a compiler, and Action/notification controls are not finite calls.
pub struct ProducerOperationInput<'a> {
    pub document: &'a ResolvedSemanticDocument<'a>,
    pub site: &'a FormSite,
    pub form: u32,
    pub operation: Operation,
    pub target: &'a str,
}

/// An actual startup registration with its own concrete definition type.
/// `None` is a nonmatching candidate; `Err` is a failed selected preparation.
/// The final check must compare all immutable services, mapping operands and
/// native endpoint facts against the SAME owned definition. Native readiness,
/// collisions and leases are checked separately before Servient publication.
pub trait ProducerCapability {
    fn identity(&self) -> (BindingId, BindingGeneration);
    /// Exact retained definition footprint including indirect owned backing.
    /// Native infrastructure/work and the logical Thing are separate owners.
    fn definition_bytes(&self, definition: &dyn Any) -> Option<usize>;
    fn prepare(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<Box<dyn Any>>>;
    fn check_final(
        &self,
        input: ProducerOperationInput<'_>,
        definition: &dyn Any,
        budget: &mut WorkBudget,
    ) -> CoreResult<bool>;
}

pub struct PreparedOperation {
    pub site: FormSite,
    pub form: u32,
    pub operation: Operation,
    pub binding: (BindingId, BindingGeneration),
    provider: usize,
    definition: Box<dyn Any>,
}
impl PreparedOperation {
    /// The concrete Binding obtains its own typed, immutable definition. This
    /// loan confers no right to create logical authority or shut down I/O.
    pub fn definition<T: 'static>(&self) -> Option<&T> {
        self.definition.downcast_ref()
    }
    pub fn definition_any(&self) -> &dyn Any {
        self.definition.as_ref()
    }
}
pub struct PreparedThing {
    operations: Vec<PreparedOperation>,
    retained_bytes: usize,
}
impl PreparedThing {
    pub fn operations(&self) -> &[PreparedOperation] {
        &self.operations
    }
    pub fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
}

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
fn busy() -> CoreError {
    CoreError::Backpressure(context())
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(context())
}
fn semantic_error(error: ValidatedSemanticError) -> CoreError {
    match error {
        ValidatedSemanticError::Admission(cause) => td_error(cause),
        ValidatedSemanticError::Resolution(clinkz_wot_td::semantic::SemanticError::Allocation) => {
            busy()
        }
        _ => unsupported(),
    }
}

/// Pure whole-description selection. `behavior` queries the actual staged
/// Servient registration (including query/cancel), not a nominal operation
/// whitelist. Missing or ambiguous provider, empty op set, missing behavior,
/// exhausted work and unsupported selected meaning all fail before native setup.
/// No single-compiler, single-Form or single-family invariant is imposed.
pub fn prepare_thing(
    description: &str,
    instance: ProducerInstanceKey,
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
    providers: &[&dyn ProducerCapability],
    behavior: impl Fn(ValidatedProducerScope<'_>, Operation) -> bool,
) -> CoreResult<PreparedThing> {
    prepare_checked(
        description,
        instance,
        limits,
        budget,
        providers,
        &behavior,
        None,
    )
}

fn prepare_checked(
    description: &str,
    instance: ProducerInstanceKey,
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
    providers: &[&dyn ProducerCapability],
    behavior: &impl Fn(ValidatedProducerScope<'_>, Operation) -> bool,
    previous: Option<&PreparedThing>,
) -> CoreResult<PreparedThing> {
    if description.len() > PRODUCER_DESCRIPTION_BYTES {
        return Err(unsupported());
    }
    budget
        .consume(WorkClass::CodecInputBytes, description.len() as u64)
        .map_err(|_| busy())?;
    let td: Thing =
        serde_json::from_str(description).map_err(|_| CoreError::Validation(context()))?;
    let config = ValidatedThingAdmissionConfig::try_from_limits(limits)
        .map_err(|_| CoreError::Validation(context()))?;
    let temporary = limits
        .get(ResourceKind::AdmissionTemporaryBytesPerOperationMax)
        .unwrap_or(0);
    let mut cursor = ValidatedThingCursor::from_thing(
        &td,
        &config,
        AdmissionLedger::new(
            instance.slot().slot(),
            instance.slot().generation(),
            0,
            temporary,
            0,
            0,
            0,
            0,
        ),
    );
    let validated = loop {
        budget
            .consume(WorkClass::PlanningItems, 1)
            .map_err(|_| busy())?;
        let before = WorkClass::ALL.map(|c| budget.remaining(c));
        cursor = match cursor.step(budget, false) {
            ValidatedThingProgress::Complete(value) => break value,
            ValidatedThingProgress::Failed(cause) => return Err(td_error(cause)),
            ValidatedThingProgress::Pending(next) => next,
        };
        if before == WorkClass::ALL.map(|c| budget.remaining(c)) {
            return Err(busy());
        }
    };
    let mut cursor = validated.into_producer();
    let mut operations = Vec::new();
    let mut retained_bytes = core::mem::size_of::<PreparedThing>();
    loop {
        budget
            .consume(WorkClass::PlanningItems, 1)
            .map_err(|_| busy())?;
        let before = WorkClass::ALL.map(|c| budget.remaining(c));
        match cursor.step(budget, false).map_err(td_error)? {
            ValidatedProducerStep::Done => break,
            ValidatedProducerStep::Pending => {}
            ValidatedProducerStep::Ready(ValidatedProducerEvent::Scope { .. }) => {
                cursor.acknowledge()
            }
            ValidatedProducerStep::Ready(ValidatedProducerEvent::Form(form)) => {
                let site = FormSite::from_scope(form.scope);
                let index = form.index;
                let ops = form.effective_operations.to_vec();
                if ops.is_empty() {
                    return Err(unsupported());
                }
                for operation in ops {
                    if !behavior(site.loan(), operation) {
                        return Err(unsupported());
                    }
                    let old = if let Some(p) = previous {
                        Some(p.operations.get(operations.len()).ok_or_else(unsupported)?)
                    } else {
                        None
                    };
                    if let Some(old) = old {
                        if old.site != site || old.form != index || old.operation != operation {
                            return Err(unsupported());
                        }
                    }
                    let selected = cursor
                        .with_form_semantics(site.loan(), index, budget, |doc, target, budget| {
                            let mut chosen = None;
                            for (i, provider) in providers.iter().enumerate() {
                                budget
                                    .consume(WorkClass::PlanningItems, 1)
                                    .map_err(|_| busy())?;
                                let input = ProducerOperationInput {
                                    document: doc,
                                    site: &site,
                                    form: index,
                                    operation,
                                    target,
                                };
                                if let Some(definition) = provider.prepare(input, budget)? {
                                    if chosen.is_some() {
                                        return Err(unsupported());
                                    }
                                    if let Some(old) = old {
                                        if old.provider != i
                                            || old.binding != provider.identity()
                                            || old.definition.as_ref().type_id()
                                                != definition.as_ref().type_id()
                                            || !provider.check_final(
                                                ProducerOperationInput {
                                                    document: doc,
                                                    site: &site,
                                                    form: index,
                                                    operation,
                                                    target,
                                                },
                                                old.definition.as_ref(),
                                                budget,
                                            )?
                                        {
                                            return Err(unsupported());
                                        }
                                    }
                                    chosen = Some((i, provider.identity(), definition));
                                }
                            }
                            chosen.ok_or_else(unsupported)
                        })
                        .map_err(semantic_error)?
                        .ok_or_else(busy)??;
                    let binding_bytes = providers[selected.0]
                        .definition_bytes(selected.2.as_ref())
                        .ok_or_else(unsupported)?;
                    let old_capacity = operations.capacity();
                    operations.try_reserve(1).map_err(|_| busy())?;
                    let next = retained_bytes
                        .checked_add(
                            (operations.capacity() - old_capacity)
                                * core::mem::size_of::<PreparedOperation>(),
                        )
                        .and_then(|n| n.checked_add(site.name_bytes()))
                        .and_then(|n| n.checked_add(binding_bytes))
                        .ok_or_else(busy)?;
                    if u64::try_from(next).map_or(true, |n| {
                        n > limits
                            .get(ResourceKind::EngineLiveBytesGlobalMax)
                            .unwrap_or(0)
                    }) {
                        return Err(busy());
                    }
                    retained_bytes = next;
                    operations.push(PreparedOperation {
                        site: site.clone(),
                        form: index,
                        operation,
                        binding: selected.1,
                        provider: selected.0,
                        definition: selected.2,
                    });
                }
                cursor.acknowledge();
            }
        }
        if before == WorkClass::ALL.map(|c| budget.remaining(c)) {
            return Err(busy());
        }
    }
    if operations.is_empty() || previous.is_some_and(|p| p.operations.len() != operations.len()) {
        return Err(unsupported());
    }
    Ok(PreparedThing {
        operations,
        retained_bytes,
    })
}

/// Reinterpret the entire final TD against the original actual registrations.
/// Call only after every owned participant is closed and check its physical
/// endpoints, collision and readiness separately. This comparison cannot turn
/// a new Form into a formerly checked operation or swap a service silently.
pub fn check_final_thing(
    description: &str,
    instance: ProducerInstanceKey,
    limits: &ResourceLimits,
    budget: &mut WorkBudget,
    providers: &[&dyn ProducerCapability],
    prepared: &PreparedThing,
    behavior: impl Fn(ValidatedProducerScope<'_>, Operation) -> bool,
) -> CoreResult<()> {
    // Requalify *all* common semantics and candidates, not only the first leaf.
    // Compare exact coordinates and selected providers, then verify their owned
    // definition with the same final semantic input.
    let _final = prepare_checked(
        description,
        instance,
        limits,
        budget,
        providers,
        &behavior,
        Some(prepared),
    )?;
    Ok(())
}
