//! Manually driven finite Consumer calls. This module depends only on Core and
//! Foundation. Notification establishment/data/stop has a separate owner/API.
use alloc::{rc::Rc, sync::Arc, vec::Vec};
use clinkz_wot_core::{consumer::*, host_accounts::*, *};
use clinkz_wot_foundation::{
    Generation, MonotonicInstant, ResourceKind as K, SlotIndex, WorkBudget, WorkClass as W,
};
use core::{
    cell::RefCell,
    cmp::Ordering,
    task::{Context, Poll},
};

const MAX_CALLS: usize = 32;

struct Definition {
    input: ConsumerDefinitionInput,
    data_policy: ConsumerDataPolicy,
    access_work: u64,
}
impl Definition {
    fn from_prepared(definition: ConsumerDefinition) -> Self {
        let (input, data_policy, access_work) = definition.into_parts();
        Self {
            input,
            data_policy,
            access_work,
        }
    }
    fn name(&self) -> &str {
        &self.input.name
    }
    fn binding(&self) -> BindingRegistrationIdentity {
        self.input.execution.identity()
    }
    fn resources(&self) -> BindingResourceDeclarations {
        self.input.execution.resources()
    }
    fn configuration(&self) -> ConsumerBindingConfiguration {
        self.input.configuration
    }
    fn data_policy(&self) -> ConsumerDataPolicy {
        self.data_policy
    }
    fn access_work(&self) -> u64 {
        self.access_work
    }
    fn check_current(&self) -> CoreResult<()> {
        if self.input.execution.consumer_configuration() != Some(self.input.configuration)
            || self.input.execution.identity().configuration()
                != self.input.configuration.configuration()
            || self.input.services.data.policy() != self.data_policy
            || self.input.services.access.work() != self.access_work
            || !self.input.services.data.available()
            || !self.input.services.data.supports_schema(self.input.schema)
        {
            return Err(invalid());
        }
        self.input.services.access.authorize(
            self.binding(),
            &self.input.target,
            Operation::ReadProperty,
        )
    }
    fn validate(&self, output: &InteractionOutput) -> CoreResult<()> {
        if !self.input.services.data.available()
            || self.input.services.data.policy() != self.data_policy
        {
            return Err(invalid());
        }
        let payload = output.data().ok_or_else(invalid)?;
        if payload.body.len() as u64 > self.data_policy.body_bytes {
            return Err(invalid());
        }
        self.input.services.data.validate(self.input.schema, output)
    }
    fn start(
        &self,
        deadline: Option<Deadline>,
    ) -> CoreResult<HostBindingCallBox<CoreResult<InteractionOutput>>> {
        let artifact = &self.input.artifact;
        let request = OutboundRequest::property_read(
            BindingArtifactRef::new(artifact.identity(), SlotIndex::new(0)),
            Default::default(),
            deadline,
        )?;
        self.input
            .execution
            .start_consumer_property_read(request, artifact)
            .map_err(|e| e.error().error().clone())
    }
}

/// Frozen source-free definitions plus fresh local logical authority. An
/// accepted call always retains its original definition and original bank.
pub struct ConsumerRuntime {
    state: Rc<RefCell<State>>,
}
struct State {
    definitions: Vec<Arc<Definition>>,
    slots: Vec<Option<Call>>,
    bank: Arc<HostResourceAccounts>,
    active: bool,
    sequence: u64,
    // Last to drop, after all definitions, calls, native/result backing.
    _funding: Vec<HostCapacityReservation>,
}
struct Call {
    sequence: u64,
    raw: Option<HostBindingCallBox<CoreResult<InteractionOutput>>>,
    outcome: Option<BindingCallSettlement<CoreResult<InteractionOutput>>>,
    rejected_output: Option<InteractionOutput>,
    definition: Arc<Definition>,
    first_cause: Option<CoreError>,
    cleanup_error: Option<CoreError>,
    refused_transfer: Option<CleanupTransferRequest>,
    cancelling: bool,
    cancel_started: bool,
    terminal: bool,
    abandoned: bool,
    deadline: Deadline,
    last_time: MonotonicInstant,
    _funding: Vec<HostCapacityReservation>,
}
impl ConsumerRuntime {
    /// Recoverable ownership handoff. Only this captured vector supplies native
    /// executors/services; runtime calls accept no compiler/configuration override.
    pub fn try_new(
        definitions: Vec<ConsumerDefinition>,
        funding: Vec<HostCapacityReservation>,
        bank: Arc<HostResourceAccounts>,
        calls: usize,
    ) -> Result<Self, BindingInputRejection<(Vec<ConsumerDefinition>, Vec<HostCapacityReservation>)>>
    {
        let error = if definitions.is_empty()
            || definitions.len() > 128
            || !(1..=MAX_CALLS).contains(&calls)
            || definitions.iter().any(|d| !Arc::ptr_eq(d.bank(), &bank))
            || definitions
                .iter()
                .enumerate()
                .any(|(i, d)| definitions[..i].iter().any(|old| old.name() == d.name()))
        {
            Some(invalid())
        } else {
            None
        };
        if let Some(error) = error {
            return Err(reject((definitions, funding), error));
        }
        let bytes = (core::mem::size_of::<State>()
            + calls * core::mem::size_of::<Option<Call>>()
            + definitions.len()
                * (core::mem::size_of::<Arc<ConsumerDefinition>>()
                    + core::mem::size_of::<ConsumerDefinition>()
                    + 32)) as u64;
        let extra = match reserve(&bank, &[(K::EngineLiveBytesGlobalMax, bytes)]) {
            Ok(extra) => extra,
            Err(e) => return Err(reject((definitions, funding), e)),
        };
        let mut slots = Vec::new();
        let mut entries = Vec::new();
        if slots.try_reserve_exact(calls).is_err()
            || entries.try_reserve_exact(definitions.len()).is_err()
        {
            return Err(reject((definitions, funding), busy()));
        }
        slots.resize_with(calls, || None);
        entries.extend(
            definitions
                .into_iter()
                .map(|d| Arc::new(Definition::from_prepared(d))),
        );
        let mut funding = funding;
        funding.extend(extra);
        Ok(Self {
            state: Rc::new(RefCell::new(State {
                definitions: entries,
                slots,
                bank,
                active: true,
                sequence: 0,
                _funding: funding,
            })),
        })
    }
    pub fn read_property(
        &self,
        name: &str,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerCall> {
        let mut state = self.state.borrow_mut();
        if !state.active {
            return Err(stale());
        }
        if name.len() > 256 {
            return Err(invalid());
        }
        if !pay(
            budget,
            W::PlanningItems,
            (state.definitions.len() * (name.len() + 1)) as u64,
        ) {
            return Err(busy());
        }
        let definition = state
            .definitions
            .iter()
            .find(|d| d.name() == name)
            .ok_or_else(unsupported)?
            .clone();
        match deadline.checked_is_elapsed_at(now) {
            Some(false) => {}
            Some(true) => return Err(CoreError::TimedOut(context())),
            None => return Err(invalid()),
        }
        let slot = state
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or_else(busy)?;
        let sequence = state.sequence.checked_add(1).ok_or_else(busy)?;
        if !pay(budget, W::ProviderProbes, definition.access_work()) {
            return Err(busy());
        }
        definition.check_current()?;
        // Native lifetime, complete result opportunity and callback scratch are
        // acquired before invocation; unread/late results retain this charge.
        let resource = definition.resources();
        let policy = definition.data_policy();
        let (body, media) = definition.configuration().result_limits();
        let lifetime = resource
            .admitted()
            .retained_bytes()
            .checked_add(body)
            .and_then(|n| n.checked_add(media.checked_mul(2)?))
            .and_then(|n| n.checked_add(core::mem::size_of::<Call>() as u64 + 256))
            .ok_or_else(invalid)?;
        let temporary = resource
            .transient()
            .peak_bytes()
            .max(policy.temporary_bytes);
        if temporary
            > state
                .bank
                .account(K::BindingPollTemporaryBytesPerCallMax)
                .limit()
        {
            return Err(CoreError::LimitExceeded {
                resource: K::BindingPollTemporaryBytesPerCallMax,
                limit: state
                    .bank
                    .account(K::BindingPollTemporaryBytesPerCallMax)
                    .limit(),
                requested: Some(temporary),
                observed: None,
                context: context(),
            });
        }
        let funding = reserve(
            &state.bank,
            &[
                (
                    K::EngineLiveBytesGlobalMax,
                    lifetime.checked_add(temporary).ok_or_else(invalid)?,
                ),
                (K::BindingPollTemporaryBytesGlobalMax, temporary),
                (K::CleanupBytesMax, lifetime),
                (K::CleanupItemsMax, 1),
                (K::PendingHandlerCallsGlobalMax, 1),
                (K::BindingResponseBufferBytesGlobalMax, body),
            ],
        )?;
        // Prepay local-disposal bookkeeping and the one native invocation. The
        // selected ConsumerOwnedHostV2 facet owes finite nonblocking local Drop.
        if budget.remaining(W::BindingPolls) < 1 || budget.remaining(W::CleanupItems) < 1 {
            return Err(busy());
        }
        budget.consume(W::BindingPolls, 1).unwrap();
        budget.consume(W::CleanupItems, 1).unwrap();
        let raw = definition.start(deadline.instant().map(|_| deadline))?;
        state.slots[slot] = Some(Call {
            sequence,
            definition,
            raw: Some(raw),
            outcome: None,
            rejected_output: None,
            first_cause: None,
            cleanup_error: None,
            refused_transfer: None,
            cancelling: false,
            cancel_started: false,
            terminal: false,
            abandoned: false,
            deadline,
            last_time: now,
            _funding: funding,
        });
        state.sequence = sequence;
        Ok(ConsumerCall {
            state: self.state.clone(),
            slot,
            sequence,
        })
    }
    /// Revokes this incarnation. Accepted work cannot move to a replacement.
    pub fn revoke(&self) {
        self.state.borrow_mut().active = false;
    }
    /// Drive independent cleanup, including after every call handle was lost.
    pub fn step(&self, cx: &mut Context<'_>, now: MonotonicInstant, budget: &mut WorkBudget) {
        let mut state = self.state.borrow_mut();
        let active = state.active;
        for slot in &mut state.slots {
            if let Some(call) = slot {
                call.step(active, cx, now, budget);
                if call.abandoned && call.terminal {
                    *slot = None;
                }
            }
        }
    }
    pub fn is_idle(&self) -> bool {
        self.state.borrow().slots.iter().all(Option::is_none)
    }
}
impl Drop for ConsumerRuntime {
    fn drop(&mut self) {
        self.state.borrow_mut().active = false;
    }
}

/// A logical call handle; the runtime owns abandoned work until retirement.
/// Cloning a native Binding or swapping a definition is not exposed here.
pub struct ConsumerCall {
    state: Rc<RefCell<State>>,
    slot: usize,
    sequence: u64,
}
impl ConsumerCall {
    pub fn cancel(&self) {
        if let Some(call) = self.state.borrow_mut().slots[self.slot]
            .as_mut()
            .filter(|c| c.sequence == self.sequence)
        {
            call.cancel(CoreError::Cancelled(context()));
        }
    }
    pub fn poll(
        &mut self,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Poll<ConsumerSettlement> {
        let mut state = self.state.borrow_mut();
        let active = state.active;
        let slot = &mut state.slots[self.slot];
        let Some(call) = slot.as_mut().filter(|c| c.sequence == self.sequence) else {
            return Poll::Pending;
        };
        call.step(active, cx, now, budget);
        if !call.terminal {
            return Poll::Pending;
        }
        Poll::Ready(ConsumerSettlement {
            call: slot.take().unwrap(),
        })
    }
}
impl Drop for ConsumerCall {
    fn drop(&mut self) {
        if let Some(call) = self.state.borrow_mut().slots[self.slot]
            .as_mut()
            .filter(|c| c.sequence == self.sequence)
        {
            call.abandoned = true;
            call.cancel(CoreError::Cancelled(context()));
        }
    }
}
/// Complete original normal/late result and retained cleanup/native owner.
/// Dropping this owner performs finite local disposal, then releases funding.
/// A cancellation cause does not overwrite a late result or claim remote success.
pub struct ConsumerSettlement {
    call: Call,
}
impl ConsumerSettlement {
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.call.first_cause.as_ref()
    }
    pub fn rejected_output(&self) -> Option<&InteractionOutput> {
        self.call.rejected_output.as_ref()
    }
    pub fn refused_transfer(&self) -> Option<&CleanupTransferRequest> {
        self.call.refused_transfer.as_ref()
    }
    pub fn cleanup_error(&self) -> Option<&CoreError> {
        self.call.cleanup_error.as_ref()
    }
    pub fn native_outcome(&self) -> &BindingCallSettlement<CoreResult<InteractionOutput>> {
        self.call.outcome.as_ref().unwrap()
    }
    /// Authoritative application outcome. A retained late native success does
    /// not override cancellation, revocation, policy or validation failure.
    pub fn result(&self) -> CoreResult<&InteractionOutput> {
        if let Some(error) = &self.call.first_cause {
            return Err(error.clone());
        }
        match self.native_outcome() {
            BindingCallSettlement::Returned(Ok(output)) => Ok(output),
            BindingCallSettlement::Returned(Err(error)) => Err(error.clone()),
            _ => Err(CoreError::Cancelled(context())),
        }
    }
    pub fn binding(&self) -> BindingRegistrationIdentity {
        self.call.definition.binding()
    }
}
impl Call {
    fn cancel(&mut self, error: CoreError) {
        if self.first_cause.is_none() {
            self.first_cause = Some(error);
        }
        if !self.terminal {
            self.cancelling = true;
        }
    }
    fn step(
        &mut self,
        active: bool,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) {
        match now.checked_cmp(self.last_time) {
            Some(Ordering::Equal | Ordering::Greater) => self.last_time = now,
            _ => self.cancel(CoreError::InternalInvariant(ErrorContext::new(
                ErrorPhase::Binding,
                RetryClass::Never,
            ))),
        }
        match self.deadline.checked_is_elapsed_at(now) {
            Some(false) => {}
            Some(true) => self.cancel(CoreError::TimedOut(context())),
            None => self.cancel(CoreError::InternalInvariant(ErrorContext::new(
                ErrorPhase::Binding,
                RetryClass::Never,
            ))),
        }
        if !active || self.abandoned {
            self.cancel(stale());
        }
        if !pay(budget, W::ProviderProbes, self.definition.access_work()) {
            return;
        }
        if let Err(e) = self.definition.check_current() {
            self.cancel(e);
        }
        if self.terminal {
            return;
        }
        if self.outcome.is_none() {
            // Raw callbacks receive only isolated native credit after Engine
            // bookkeeping prepayment; no zero-credit callback is invoked.
            if !pay(budget, W::HandlerSteps, 1) {
                return;
            }
            if self.cancelling {
                if !pay(budget, W::CleanupItems, 1) {
                    return;
                }
                let mut native = native_budget(budget);
                if native.is_exhausted() {
                    return;
                }
                let raw = self.raw.as_mut().unwrap();
                let outcome = if self.cancel_started {
                    match raw.as_pin_mut().poll_cancel(cx, &mut native) {
                        Poll::Pending => None,
                        Poll::Ready(Ok(value)) => Some(value),
                        Poll::Ready(Err(error)) => {
                            self.cleanup_error.get_or_insert(error);
                            None
                        }
                    }
                } else {
                    let cleanup = CleanupPhaseContext::bind(
                        CleanupReservation::new(
                            CleanupSlotId::new(SlotIndex::new(0), Generation::INITIAL),
                            self.definition.resources().admitted(),
                            0,
                            WorkBudget::new().with_remaining(W::CleanupItems, 1),
                        ),
                        CleanupOperation::CancelRequest,
                        self.first_cause.as_ref().unwrap().clone(),
                        Deadline::NONE,
                    );
                    match raw.as_pin_mut().start_cancel(cx, cleanup, &mut native) {
                        Ok(StartStatus::Pending) => {
                            self.cancel_started = true;
                            None
                        }
                        Ok(StartStatus::Ready(value)) => Some(value),
                        Err(error) => {
                            self.cleanup_error.get_or_insert(error);
                            None
                        }
                    }
                };
                refund_native(budget, native);
                self.outcome = outcome;
            } else {
                let mut native = native_budget(budget);
                if native.is_exhausted() {
                    return;
                }
                if let Poll::Ready(result) = self
                    .raw
                    .as_mut()
                    .unwrap()
                    .as_pin_mut()
                    .poll_result(cx, &mut native)
                {
                    if let Err(error) = &result {
                        self.first_cause.get_or_insert(error.clone());
                    }
                    self.outcome = Some(BindingCallSettlement::Returned(result));
                }
                refund_native(budget, native);
            }
        }
        if matches!(
            self.outcome,
            Some(BindingCallSettlement::Cancelled {
                disposition: BindingCancellationDisposition::TransferRequired(_),
                ..
            })
        ) {
            // This owner granted no named external transfer destination. Retain
            // the complete original work and drive manual cleanup; a fabricated
            // request is not a terminal disposition or an acknowledgement.
            if let Some(BindingCallSettlement::Cancelled {
                disposition: BindingCancellationDisposition::TransferRequired(request),
                ..
            }) = self.outcome.take()
            {
                self.refused_transfer = Some(request);
            }
            self.cleanup_error
                .get_or_insert(CoreError::InternalInvariant(ErrorContext::new(
                    ErrorPhase::Cleanup,
                    RetryClass::Never,
                )));
            self.cancel_started = true;
            return;
        }
        if let Some(outcome) = self.outcome.as_mut() {
            if let BindingCallSettlement::Returned(Ok(output)) = outcome {
                if !pay(
                    budget,
                    W::CodecInputBytes,
                    self.definition.data_policy().validation_work,
                ) {
                    return;
                }
                // Authorization/service loss fences application success while
                // preserving the original result in the settlement owner.
                if let Err(error) = self.definition.validate(output) {
                    self.first_cause.get_or_insert(error.clone());
                    if let BindingCallSettlement::Returned(Ok(original)) =
                        core::mem::replace(outcome, BindingCallSettlement::Returned(Err(error)))
                    {
                        self.rejected_output = Some(original);
                    }
                }
            }
            self.terminal = true;
        }
    }
}
fn native_budget(b: &mut WorkBudget) -> WorkBudget {
    let n = b.remaining(W::BindingPolls).min(1);
    b.consume(W::BindingPolls, n).unwrap();
    WorkBudget::new().with_remaining(W::BindingPolls, n)
}
fn refund_native(b: &mut WorkBudget, native: WorkBudget) {
    b.set_remaining(
        W::BindingPolls,
        b.remaining(W::BindingPolls) + native.remaining(W::BindingPolls),
    );
}
fn stale() -> CoreError {
    CoreError::StaleHandle(context())
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(context())
}
fn reject<T>(input: T, e: CoreError) -> BindingInputRejection<T> {
    BindingInputRejection::new(input, BindingOperationalError::new(e))
}
