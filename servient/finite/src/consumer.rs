//! Manually driven finite Consumer calls. This module depends only on Core and
//! Foundation. Notification establishment/data/stop has a separate owner/API.
use alloc::{rc::Rc, sync::Arc, vec::Vec};
use clinkz_wot_core::thing::AffordanceSite;
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

enum CallInput<'a> {
    Value(Option<Payload>),
    Members(&'a [&'a str]),
}

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
    fn matches(&self, target: AffordanceSite<'_>, operation: Operation) -> bool {
        self.input.operation == operation
            && match (&self.input.affordance, target) {
                (AffordanceTarget::Thing, AffordanceSite::Thing) => true,
                (AffordanceTarget::Property(a), AffordanceSite::Property(b))
                | (AffordanceTarget::Action(a), AffordanceSite::Action(b)) => a.as_ref() == b,
                _ => false,
            }
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
    fn check_dependencies(&self) -> CoreResult<()> {
        if self.input.execution.consumer_configuration() != Some(self.input.configuration)
            || self.input.execution.identity().configuration()
                != self.input.configuration.configuration()
            || self.input.services.data.policy() != self.data_policy
            || self.input.services.access.work() != self.access_work
            || !self.input.services.data.available()
            || !self
                .input
                .schemas
                .supports(self.input.services.data.as_ref())
            || !self
                .input
                .execution
                .supports_operation(self.input.operation)
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn check_current(
        &self,
        consumer: &ConsumerInstanceRef,
        selection: Option<collection::PropertySelection>,
    ) -> CoreResult<()> {
        if !consumer.is_current() {
            return Err(stale());
        }
        self.check_dependencies()?;
        if let (
            ConsumerFiniteSchemas::WriteAllProperties(schema)
            | ConsumerFiniteSchemas::WriteMultipleProperties(schema),
            Some(selected),
        ) = (&self.input.schemas, selection)
        {
            self.input.services.access.authorize_property_write(
                consumer,
                self.binding(),
                &self.input.target,
                self.input.operation,
                (schema, selected),
            )?;
        } else {
            self.input.services.access.authorize_instance(
                consumer,
                self.binding(),
                &self.input.target,
                self.input.operation,
            )?;
        }
        if !consumer.is_current() {
            return Err(stale());
        }
        // Permission can invalidate any captured dependency through interior
        // state. Every caller, including final admission and validated/unread
        // result handoff, needs the observations made after that callback.
        self.check_dependencies()
    }
    fn validate(
        &self,
        output: &InteractionOutput,
        selection: Option<collection::PropertySelection>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<()>> {
        if !self.input.services.data.available()
            || self.input.services.data.policy() != self.data_policy
        {
            return Poll::Ready(Err(invalid()));
        }
        let has_output = match &self.input.schemas {
            ConsumerFiniteSchemas::Value(s) => s.output.is_some(),
            ConsumerFiniteSchemas::WriteAllProperties(_)
            | ConsumerFiniteSchemas::WriteMultipleProperties(_) => false,
            ConsumerFiniteSchemas::ReadAllProperties(_)
            | ConsumerFiniteSchemas::ReadMultipleProperties(_)
            | ConsumerFiniteSchemas::QueryAllActions(_) => true,
        };
        if output.data().is_some() != has_output {
            return Poll::Ready(Err(invalid()));
        }
        if let Some(report) = output.metadata().property_write_report() {
            let selected = match (&self.input.schemas, selection) {
                (ConsumerFiniteSchemas::WriteAllProperties(schema), _) => schema.all(),
                (ConsumerFiniteSchemas::WriteMultipleProperties(_), Some(s)) => s,
                _ => return Poll::Ready(Err(invalid())),
            };
            if let Err(error) = report.validate_selection(selected) {
                return Poll::Ready(Err(error));
            }
        }
        // An absent data role has no decoding work. Preserve empty finite
        // Action/Write completion under zero codec credit.
        if !has_output {
            return Poll::Ready(Ok(()));
        }
        if output
            .data()
            .is_some_and(|p| p.body.len() as u64 > self.data_policy.body_bytes)
        {
            return Poll::Ready(Err(invalid()));
        }
        if !pay(budget, W::CodecInputBytes, self.data_policy.validation_work) {
            return Poll::Pending;
        }
        Poll::Ready(self.input.schemas.validate_output(
            self.input.services.data.as_ref(),
            selection,
            output.data(),
        ))
    }
    fn validate_payload(
        &self,
        payload: Option<&Payload>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<Option<collection::PropertySelection>>> {
        Poll::Ready(match (self.input.schemas.input(), payload) {
            (None, None) => Ok(None),
            (Some(_), Some(payload))
                if payload.body.len() as u64 <= self.data_policy.body_bytes =>
            {
                if !self.input.services.data.available()
                    || self.input.services.data.policy() != self.data_policy
                {
                    return Poll::Ready(Err(invalid()));
                }
                if !pay(budget, W::CodecInputBytes, self.data_policy.validation_work) {
                    return Poll::Pending;
                }
                self.input
                    .schemas
                    .validate_input_selection(self.input.services.data.as_ref(), Some(payload))
            }
            _ => Err(invalid()),
        })
    }
    fn start(
        &self,
        input: Option<Payload>,
        deadline: Option<Deadline>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<Option<Payload>>,
    > {
        let artifact = &self.input.artifact;
        let request = OutboundRequest::finite(
            BindingArtifactRef::new(artifact.identity(), SlotIndex::new(0)),
            self.input.operation,
            input,
            Default::default(),
            deadline,
        )?;
        self.input
            .execution
            .start_finite(request, artifact)
            .map_err(|e| {
                let (request, error) = e.into_parts();
                BindingInputRejection::new(request.into_input(), error)
            })
    }
}

/// Frozen source-free definitions attached to independent Consumer authority. An
/// accepted call always retains its original definition and original bank.
pub struct ConsumerRuntime {
    state: Rc<RefCell<State>>,
}
struct State {
    consumer: ConsumerInstanceRef,
    definitions: Vec<Arc<Definition>>,
    slots: Vec<Option<Call>>,
    bank: Arc<HostResourceAccounts>,
    active: bool,
    sequence: u64,
    next_slot: usize,
    // Last to drop, after all definitions, calls, native/result backing.
    _funding: Vec<HostCapacityReservation>,
}
struct Call {
    consumer: ConsumerInstanceRef,
    effect: ConsumerEffect,
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
    output_validated: bool,
    selection: Option<collection::PropertySelection>,
    abandoned: bool,
    deadline: Deadline,
    last_time: MonotonicInstant,
    _funding: Vec<HostCapacityReservation>,
}
impl ConsumerRuntime {
    /// Recoverable ownership handoff. Only this captured vector supplies native
    /// executors/services; runtime calls accept no compiler/configuration override.
    pub fn try_new(
        consumer: ConsumerInstanceRef,
        definitions: Vec<ConsumerDefinition>,
        funding: Vec<HostCapacityReservation>,
        bank: Arc<HostResourceAccounts>,
        calls: usize,
    ) -> Result<Self, BindingInputRejection<(Vec<ConsumerDefinition>, Vec<HostCapacityReservation>)>>
    {
        let error = if !consumer.is_current()
            || definitions.is_empty()
            || definitions.len() > 128
            || !(1..=MAX_CALLS).contains(&calls)
            || definitions.iter().any(|d| !Arc::ptr_eq(d.bank(), &bank))
            || definitions.iter().enumerate().any(|(i, d)| {
                definitions[..i].iter().any(|old| {
                    old.affordance() == d.affordance() && old.operation() == d.operation()
                })
            }) {
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
                consumer,
                definitions: entries,
                slots,
                bank,
                active: true,
                sequence: 0,
                next_slot: 0,
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
        self.start(
            AffordanceSite::Property(name),
            Operation::ReadProperty,
            CallInput::Value(None),
            deadline,
            now,
            budget,
        )
        .map_err(|e| e.error().error().clone())
    }
    /// One root status exchange. Returned native records confer no local
    /// invocation authority and do not query/cancel existing Consumer handles.
    pub fn query_all_actions(
        &self,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerCall> {
        self.start(
            AffordanceSite::Thing,
            Operation::QueryAllActions,
            CallInput::Value(None),
            deadline,
            now,
            budget,
        )
        .map_err(|e| e.error().error().clone())
    }
    /// One selected Thing-root native interaction; never individual reads.
    pub fn read_all_properties(
        &self,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerCall> {
        self.start(
            AffordanceSite::Thing,
            Operation::ReadAllProperties,
            CallInput::Value(None),
            deadline,
            now,
            budget,
        )
        .map_err(|e| e.error().error().clone())
    }
    /// One actual root interaction for this call's exact readable-member set.
    /// Names are borrowed only during admission; the retained schema and an
    /// inline selection survive independently of the caller and source TD.
    pub fn read_multiple_properties(
        &self,
        members: &[&str],
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerCall> {
        self.start(
            AffordanceSite::Thing,
            Operation::ReadMultipleProperties,
            CallInput::Members(members),
            deadline,
            now,
            budget,
        )
        .map_err(|e| e.error().error().clone())
    }
    /// A finite, explicitly synchronous Action. Rejection before native
    /// acceptance returns the original input, including all backing.
    pub fn invoke_action(
        &self,
        name: &str,
        input: Option<Payload>,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerCall, BindingInputRejection<Option<Payload>>> {
        self.start(
            AffordanceSite::Action(name),
            Operation::InvokeAction,
            CallInput::Value(input),
            deadline,
            now,
            budget,
        )
    }
    /// Validate and authorize a selected Property value before native effects.
    /// Local refusal returns the original value. An admitted call is never
    /// automatically replayed, even if its remote effect remains uncertain.
    pub fn write_property(
        &self,
        name: &str,
        input: Payload,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerCall, BindingInputRejection<Payload>> {
        self.start(
            AffordanceSite::Property(name),
            Operation::WriteProperty,
            CallInput::Value(Some(input)),
            deadline,
            now,
            budget,
        )
        .map_err(|e| {
            let (input, error) = e.into_parts();
            BindingInputRejection::new(input.expect("write input retained"), error)
        })
    }
    /// One native Thing-root write. Complete writable-map validation and group
    /// authorization precede native effects; refusal returns the original input.
    /// Error/cancellation can follow partial remote effects and never replays.
    pub fn write_all_properties(
        &self,
        input: Payload,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerCall, BindingInputRejection<Payload>> {
        self.start(
            AffordanceSite::Thing,
            Operation::WriteAllProperties,
            CallInput::Value(Some(input)),
            deadline,
            now,
            budget,
        )
        .map_err(|e| {
            let (input, error) = e.into_parts();
            BindingInputRejection::new(input.expect("collection write input retained"), error)
        })
    }
    /// One selected native root interaction. The validated input map supplies
    /// its exact writable set; omitted members are untouched by this intent.
    pub fn write_multiple_properties(
        &self,
        input: Payload,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerCall, BindingInputRejection<Payload>> {
        self.start(
            AffordanceSite::Thing,
            Operation::WriteMultipleProperties,
            CallInput::Value(Some(input)),
            deadline,
            now,
            budget,
        )
        .map_err(|e| {
            let (input, error) = e.into_parts();
            BindingInputRejection::new(input.expect("selected write input retained"), error)
        })
    }
    fn start(
        &self,
        target: AffordanceSite<'_>,
        operation: Operation,
        input: CallInput<'_>,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerCall, BindingInputRejection<Option<Payload>>> {
        let (input, members) = match input {
            CallInput::Value(input) => (input, None),
            CallInput::Members(members) => (None, Some(members)),
        };
        let mut state = self.state.borrow_mut();
        let checked = (|| -> CoreResult<_> {
            if !state.active || !state.consumer.is_current() {
                return Err(stale());
            }
            let name_len = match target {
                AffordanceSite::Thing => 0,
                AffordanceSite::Property(n)
                | AffordanceSite::Action(n)
                | AffordanceSite::Event(n) => n.len(),
            };
            if name_len > 256 {
                return Err(invalid());
            }
            if !pay(
                budget,
                W::PlanningItems,
                (state.definitions.len() * (name_len + 1)) as u64,
            ) {
                return Err(busy());
            }
            let definition = state
                .definitions
                .iter()
                .find(|d| d.matches(target, operation))
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
            definition.check_current(&state.consumer, None)?;
            let selection = match (&definition.input.schemas, members) {
                (ConsumerFiniteSchemas::ReadMultipleProperties(schema), Some(names)) => {
                    if names.len() > collection::PropertyCollectionSchema::MAX_MEMBERS {
                        return Err(invalid());
                    }
                    let work = names.len() as u64
                        * (collection::PropertyCollectionSchema::MAX_MEMBERS as u64 + 1)
                        * (collection::PropertyCollectionSchema::MAX_NAME_BYTES as u64 + 1)
                        + 1;
                    if !pay(budget, W::PlanningItems, work) {
                        return Err(busy());
                    }
                    Some(schema.select(names)?)
                }
                (ConsumerFiniteSchemas::ReadMultipleProperties(_), None) | (_, Some(_)) => {
                    return Err(invalid());
                }
                _ => None,
            };
            // Native lifetime, complete result opportunity and callback scratch are
            // acquired before invocation; unread/late results retain this charge.
            let resource = definition.resources();
            let policy = definition.data_policy();
            let (body, media) = definition.configuration().result_limits();
            let input_bytes = if selection.is_some() {
                policy.encoded_selection_bytes
            } else {
                clinkz_wot_core::notification::notification_payload_backing(input.as_ref())
                    .ok_or_else(invalid)? as u64
            };
            let lifetime = resource
                .admitted()
                .retained_bytes()
                .checked_add(body)
                .and_then(|n| n.checked_add(input_bytes))
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
            // Acquire encoded input only after its complete lifetime/scratch and
            // destruction allowances. On refusal it drops before this funding.
            let encoded =
                if let (ConsumerFiniteSchemas::ReadMultipleProperties(schema), Some(selected)) =
                    (&definition.input.schemas, selection)
                {
                    // Authorization may change service availability or its
                    // work/backing declarations. Check at this callback edge.
                    if !definition.input.services.data.available()
                        || definition.input.services.data.policy() != policy
                    {
                        return Err(invalid());
                    }
                    if !pay(budget, W::CodecInputBytes, policy.validation_work) {
                        return Err(busy());
                    }
                    let payload = definition
                        .input
                        .services
                        .data
                        .encode_property_selection(schema, selected)?;
                    if notification::notification_payload_backing(Some(&payload))
                        .ok_or_else(invalid)? as u64
                        > input_bytes
                    {
                        return Err(invalid());
                    }
                    Some(payload)
                } else {
                    None
                };
            if selection.is_some() {
                if !pay(budget, W::ProviderProbes, definition.access_work()) {
                    return Err(busy());
                }
                definition.check_current(&state.consumer, None)?;
            }
            let payload = encoded.as_ref().or(input.as_ref());
            let input_selection = match definition.validate_payload(payload, budget) {
                Poll::Ready(result) => result?,
                Poll::Pending => return Err(busy()),
            };
            // Codec callbacks may change current policy as well as authority.
            if payload.is_some() {
                if !pay(budget, W::ProviderProbes, definition.access_work()) {
                    return Err(busy());
                }
                definition.check_current(&state.consumer, input_selection.or(selection))?;
            }
            if !state.consumer.is_current() {
                return Err(stale());
            }
            Ok((
                slot,
                sequence,
                definition,
                funding,
                input_selection.or(selection),
                encoded,
            ))
        })();
        let (slot, sequence, definition, funding, selection, encoded) = match checked {
            Ok(checked) => checked,
            Err(e) => return Err(reject(input, e)),
        };
        let raw = match definition.start(encoded.or(input), deadline.instant().map(|_| deadline)) {
            Ok(raw) => raw,
            Err(rejection) if operation == Operation::ReadMultipleProperties => {
                let (payload, error) = rejection.into_parts();
                drop(payload); // encoded input dies before its original reservation
                return Err(BindingInputRejection::new(None, error));
            }
            Err(rejection) => return Err(rejection),
        };
        state.slots[slot] = Some(Call {
            consumer: state.consumer.clone(),
            effect: ConsumerEffect::NotAttempted,
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
            output_validated: false,
            selection,
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
    /// Closes this finite facet. Other facets of the Consumer stay independent;
    /// the ConsumerInstance owner revokes the whole incarnation.
    pub fn begin_close(&self) {
        self.state.borrow_mut().active = false;
    }
    /// Drive independent cleanup, including after every call handle was lost.
    pub fn step(&self, cx: &mut Context<'_>, now: MonotonicInstant, budget: &mut WorkBudget) {
        let mut state = self.state.borrow_mut();
        let active = state.active;
        let slots = state.slots.len();
        let start = state.next_slot;
        // Rotate priority across steps even when one native poll exhausts the
        // available credit. Still visit every slot for local fencing; an unpaid
        // later visit must not reset the next step's scheduling position.
        state.next_slot = (start + 1) % slots;
        for offset in 0..slots {
            let slot = &mut state.slots[(start + offset) % slots];
            if let Some(call) = slot {
                let _ = call.step(active, cx, now, budget);
                if call.abandoned && call.terminal {
                    *slot = None;
                }
            }
        }
    }
    pub fn is_idle(&self) -> bool {
        self.state.borrow().slots.iter().all(Option::is_none)
    }
    pub fn consumer(&self) -> ConsumerInstanceRef {
        self.state.borrow().consumer.clone()
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
        if call.step(active, cx, now, budget).is_pending() {
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
/// Payload access lends slices; share this owner to retain original funded bytes.
///
/// Neither normal nor rejected nor late/native outputs lend a cloneable body owner:
///
/// ```compile_fail,E0308
/// use clinkz_wot_servient_finite::consumer::ConsumerSettlement;
/// use std::sync::Arc;
/// fn detach(s: &ConsumerSettlement) -> Arc<[u8]> {
///     Arc::clone(&s.result().unwrap().data().unwrap().body)
/// }
/// ```
///
/// ```compile_fail,E0308
/// use clinkz_wot_servient_finite::consumer::ConsumerSettlement;
/// use std::sync::Arc;
/// fn detach(s: &ConsumerSettlement) -> Arc<[u8]> {
///     Arc::clone(&s.rejected_output().unwrap().data().unwrap().body)
/// }
/// ```
///
/// ```compile_fail,E0308
/// use clinkz_wot_servient_finite::consumer::{ConsumerNativeOutcome, ConsumerSettlement};
/// use std::sync::Arc;
/// fn detach(s: &ConsumerSettlement) -> Arc<[u8]> {
///     match s.native_outcome() {
///         ConsumerNativeOutcome::Returned(Ok(output)) => Arc::clone(&output.data().unwrap().body),
///         _ => panic!(),
///     }
/// }
/// ```
///
/// ```compile_fail,E0515
/// use clinkz_wot_servient_finite::consumer::ConsumerSettlement;
/// fn escape(s: ConsumerSettlement) -> &'static [u8] {
///     s.result().unwrap().data().unwrap().body
/// }
/// ```
pub struct ConsumerSettlement {
    call: Call,
}

/// Read-only original native outcome. A returned value may be late and does
/// not override the settlement's authoritative error. Cancellation keeps its
/// exact retry and cleanup evidence, without granting cleanup ownership.
#[derive(Clone, Copy, Debug)]
pub enum ConsumerNativeOutcome<'a> {
    /// Normal or late native result, independent of application acceptance.
    Returned(Result<InteractionOutputView<'a>, &'a CoreError>),
    /// Original native cancellation evidence under this settlement's lifetime.
    Cancelled {
        /// Original retry classification.
        retry_class: RetryClass,
        /// Original cleanup disposition; this borrow grants no ownership.
        disposition: &'a BindingCancellationDisposition<NoCleanupSuccessor>,
    },
}

impl ConsumerSettlement {
    /// Original writable meaning and invocation set, retained even after source
    /// destruction, replacement or native-report rejection. Report indices must
    /// be resolved through this schema, never through a successor definition.
    pub fn property_write_members(
        &self,
    ) -> Option<(
        &collection::PropertyCollectionSchema,
        collection::PropertySelection,
    )> {
        match (&self.call.definition.input.schemas, self.call.selection) {
            (
                ConsumerFiniteSchemas::WriteAllProperties(schema)
                | ConsumerFiniteSchemas::WriteMultipleProperties(schema),
                Some(selected),
            ) => Some((schema, selected)),
            _ => None,
        }
    }
    pub fn effect(&self) -> ConsumerEffect {
        self.call.effect
    }
    pub fn consumer(&self) -> &ConsumerInstanceRef {
        &self.call.consumer
    }
    pub fn operation(&self) -> Operation {
        self.call.definition.input.operation
    }
    /// Lends the original coordinate without exposing its shared name owner.
    pub fn affordance(&self) -> AffordanceSite<'_> {
        match &self.call.definition.input.affordance {
            AffordanceTarget::Thing => AffordanceSite::Thing,
            AffordanceTarget::Property(name) => AffordanceSite::Property(name),
            AffordanceTarget::Action(name) => AffordanceSite::Action(name),
            AffordanceTarget::Event(name) => AffordanceSite::Event(name),
        }
    }
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.call.first_cause.as_ref()
    }
    /// Lends original native output rejected by captured result validation.
    pub fn rejected_output(&self) -> Option<InteractionOutputView<'_>> {
        self.call
            .rejected_output
            .as_ref()
            .map(InteractionOutput::view)
    }
    pub fn refused_transfer(&self) -> Option<&CleanupTransferRequest> {
        self.call.refused_transfer.as_ref()
    }
    pub fn cleanup_error(&self) -> Option<&CoreError> {
        self.call.cleanup_error.as_ref()
    }
    /// Lends the complete original outcome without exposing payload owners.
    pub fn native_outcome(&self) -> ConsumerNativeOutcome<'_> {
        match self.call.outcome.as_ref().unwrap() {
            BindingCallSettlement::Returned(result) => {
                ConsumerNativeOutcome::Returned(result.as_ref().map(InteractionOutput::view))
            }
            BindingCallSettlement::Cancelled {
                retry_class,
                disposition,
            } => ConsumerNativeOutcome::Cancelled {
                retry_class: *retry_class,
                disposition,
            },
        }
    }
    /// Authoritative application outcome. A retained late native success does
    /// not override cancellation, revocation, policy or validation failure.
    pub fn result(&self) -> CoreResult<InteractionOutputView<'_>> {
        if let Some(error) = &self.call.first_cause {
            return Err(error.clone());
        }
        match self.native_outcome() {
            ConsumerNativeOutcome::Returned(Ok(output)) => {
                if let Some(report) = output.metadata().property_write_report() {
                    report.completion()?;
                }
                Ok(output)
            }
            ConsumerNativeOutcome::Returned(Err(error)) => Err(error.clone()),
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
    /// Ready requires a terminal outcome and paid current-policy checks in this
    /// step. A previously terminal, unread result alone cannot authorize delivery.
    fn step(
        &mut self,
        active: bool,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Poll<()> {
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
        if !active || !self.consumer.is_current() || self.abandoned {
            self.cancel(stale());
        }
        if !pay(budget, W::ProviderProbes, self.definition.access_work()) {
            return Poll::Pending;
        }
        if let Err(e) = self
            .definition
            .check_current(&self.consumer, self.selection)
        {
            self.cancel(e);
        }
        // A completed validation survives a yield for the final policy check.
        // The check above is fresh in this step; no codec/native callback can
        // change policy between it and this retained-result handoff.
        if self.terminal || self.output_validated {
            self.terminal = true;
            return Poll::Ready(());
        }
        if self.outcome.is_none() {
            // Raw callbacks receive only isolated native credit after Engine
            // bookkeeping prepayment; no zero-credit callback is invoked.
            if !pay(budget, W::HandlerSteps, 1) {
                return Poll::Pending;
            }
            if self.cancelling {
                if !pay(budget, W::CleanupItems, 1) {
                    return Poll::Pending;
                }
                let mut native = native_budget(budget);
                if native.is_exhausted() {
                    return Poll::Pending;
                }
                self.effect = ConsumerEffect::Unknown;
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
                    return Poll::Pending;
                }
                self.effect = ConsumerEffect::Unknown;
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
        if !self.consumer.is_current() {
            self.cancel(stale());
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
            return Poll::Pending;
        }
        if let Some(outcome) = self.outcome.as_mut() {
            if let BindingCallSettlement::Returned(Ok(output)) = outcome {
                // Authorization/service loss fences application success while
                // preserving the original result in the settlement owner.
                match self.definition.validate(output, self.selection, budget) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Ok(())) => {
                        self.output_validated = true;
                        if matches!(
                            self.definition.input.operation,
                            Operation::WriteAllProperties | Operation::WriteMultipleProperties
                        ) {
                            self.effect = if output
                                .metadata()
                                .property_write_report()
                                .is_some_and(|r| !r.is_complete())
                            {
                                ConsumerEffect::Reported
                            } else {
                                ConsumerEffect::Completed
                            };
                        }
                    }
                    Poll::Ready(Err(error)) => {
                        self.first_cause.get_or_insert(error.clone());
                        if let BindingCallSettlement::Returned(Ok(original)) =
                            core::mem::replace(outcome, BindingCallSettlement::Returned(Err(error)))
                        {
                            self.rejected_output = Some(original);
                        }
                    }
                }
            }
            if !self.consumer.is_current() {
                self.first_cause.get_or_insert_with(stale);
            }
            // The native and codec callbacks may change application policy.
            // Short current-check credit retains the complete original result.
            if self.first_cause.is_none()
                && matches!(outcome, BindingCallSettlement::Returned(Ok(_)))
            {
                if !pay(budget, W::ProviderProbes, self.definition.access_work()) {
                    return Poll::Pending;
                }
                if let Err(error) = self
                    .definition
                    .check_current(&self.consumer, self.selection)
                {
                    self.first_cause.get_or_insert(error);
                }
            }
            self.terminal = true;
            return Poll::Ready(());
        }
        Poll::Pending
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
