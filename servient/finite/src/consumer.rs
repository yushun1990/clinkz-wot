//! Manually driven finite Consumer calls. This module depends only on Core and
//! Foundation. Notification establishment/data/stop has a separate owner/API.
use alloc::{rc::Rc, sync::Arc, vec::Vec};
use clinkz_wot_core::execution::{FiniteCallDriver, FiniteCallState, NativeProgress};
use clinkz_wot_core::thing::AffordanceSite;
use clinkz_wot_core::{consumer::*, host_accounts::*, *};
use clinkz_wot_foundation::{
    Generation, MonotonicInstant, ResourceKind as K, SlotIndex, WorkBudget, WorkClass as W,
};
use core::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    task::{Context, Poll},
};

const MAX_CALLS: usize = 32;

enum CallInput<'a> {
    Value(Option<Payload>),
    Members(&'a [&'a str]),
}

struct Definition {
    input: ConsumerBindingInput,
    data_policy: ConsumerDataPolicy,
    access_work: u64,
}
impl Definition {
    fn from_prepared(definition: ConsumerBinding) -> Self {
        let (input, data_policy, access_work) = definition.into_parts();
        Self {
            input,
            data_policy,
            access_work,
        }
    }
    fn matches(&self, target: AffordanceSite<'_>, operation: Operation) -> bool {
        self.input.definition.operation() == operation
            && match (self.input.definition.affordance(), target) {
                (AffordanceTarget::Thing, AffordanceSite::Thing) => true,
                (AffordanceTarget::Property(a), AffordanceSite::Property(b))
                | (AffordanceTarget::Action(a), AffordanceSite::Action(b)) => a.as_ref() == b,
                _ => false,
            }
    }
    fn binding(&self) -> BindingRegistrationIdentity {
        self.input.definition.execution().identity()
    }
    fn resources(&self) -> BindingResourceDeclarations {
        self.input.definition.execution().resources()
    }
    fn configuration(&self) -> ConsumerBindingConfiguration {
        self.input.definition.configuration()
    }
    fn data_policy(&self) -> ConsumerDataPolicy {
        self.data_policy
    }
    fn access_work(&self) -> u64 {
        self.access_work
    }
    fn check_dependencies(&self) -> CoreResult<()> {
        self.input.definition.check_dependencies()?;
        if !self
            .input
            .access
            .supports_security(self.input.definition.content().security())
        {
            return Err(invalid());
        }
        if self.input.access.security_revision() != self.input.security_revision
            || self.input.access.work() != self.access_work
            || self.input.access.retained_bytes() != self.input.access.backing_bytes()
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
        ) = (self.input.definition.content().schemas(), selection)
        {
            self.input.access.authorize_property_write(
                consumer,
                self.binding(),
                self.input.definition.target(),
                self.input.definition.operation(),
                (schema, selected),
            )?;
        } else {
            self.input.access.authorize_instance(
                consumer,
                self.binding(),
                self.input.definition.target(),
                self.input.definition.operation(),
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
        if !self.input.definition.data().available()
            || self.input.definition.data().policy() != self.data_policy
        {
            return Poll::Ready(Err(invalid()));
        }
        let invalid_response = || {
            CoreError::Validation(
                ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
                    .with_operation(self.input.definition.operation()),
            )
        };
        let content = self.input.definition.content();
        if let Err(error) = content
            .response_shape()
            .validate(output.metadata().response_selection(), output.view())
        {
            return Poll::Ready(Err(error));
        }
        let response = content
            .responses()
            .and_then(|r| r.get(output.metadata().response_selection()));
        let has_output = response
            .is_some_and(|r| matches!(r.data, execution::responses::ResponseData::Value(_)))
            || content.schemas().has_output();
        if output.data().is_some() != has_output {
            return Poll::Ready(Err(invalid_response()));
        }
        if let Some(report) = output.metadata().property_write_report() {
            let selected = match (self.input.definition.content().schemas(), selection) {
                (ConsumerFiniteSchemas::WriteAllProperties(schema), _) => schema.all(),
                (ConsumerFiniteSchemas::WriteMultipleProperties(_), Some(s)) => s,
                _ => return Poll::Ready(Err(invalid_response())),
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
            return Poll::Ready(Err(invalid_response()));
        }
        if !pay(budget, W::CodecInputBytes, self.data_policy.validation_work) {
            return Poll::Pending;
        }
        if let Some(p) = output.data() {
            if response.is_some_and(|r| r.content_type.as_ref() != p.content_type.as_str())
                || p.content_coding.is_some()
            {
                return Poll::Ready(Err(invalid_response()));
            }
        }
        Poll::Ready(match response.map(|r| &r.data) {
            Some(execution::responses::ResponseData::Value(schema)) => self
                .input
                .definition
                .data()
                .validate(schema, output.data().unwrap()),
            _ => content.schemas().validate_output(
                self.input.definition.data(),
                selection,
                output.data(),
            ),
        })
    }
    fn validate_payload(
        &self,
        payload: Option<&Payload>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<Option<collection::PropertySelection>>> {
        Poll::Ready(
            match (self.input.definition.content().schemas().input(), payload) {
                (None, None) => Ok(None),
                (Some(_), Some(payload))
                    if payload.body.len() as u64 <= self.data_policy.body_bytes
                        && payload.content_type == self.input.definition.request_media() =>
                {
                    if !self.input.definition.data().available()
                        || self.input.definition.data().policy() != self.data_policy
                    {
                        return Poll::Ready(Err(invalid()));
                    }
                    if !pay(budget, W::CodecInputBytes, self.data_policy.validation_work) {
                        return Poll::Pending;
                    }
                    self.input
                        .definition
                        .content()
                        .schemas()
                        .validate_input_selection(self.input.definition.data(), Some(payload))
                }
                _ => Err(invalid()),
            },
        )
    }
    fn start(
        &self,
        consumer: &ConsumerInstanceRef,
        input: Option<Payload>,
        deadline: Option<Deadline>,
        budget: &mut WorkBudget,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<Option<Payload>>,
    > {
        // Issuance is a distinct linked callback. The preceding permission
        // payment cannot fund a second traversal of the credential service.
        if !pay(budget, W::ProviderProbes, self.access_work()) {
            return Err(BindingInputRejection::new(
                input,
                BindingOperationalError::new(busy()),
            ));
        }
        let current = if consumer.is_current() {
            self.check_dependencies()
        } else {
            Err(stale())
        };
        if let Err(error) = current {
            return Err(BindingInputRejection::new(
                input,
                BindingOperationalError::new(error),
            ));
        }
        let artifact = self.input.definition.artifact();
        let request = OutboundRequest::finite(
            BindingArtifactRef::new(artifact.identity(), SlotIndex::new(0)),
            self.input.definition.operation(),
            input,
            Default::default(),
            deadline,
        )?;
        let security = match self.input.access.outbound_security(
            consumer,
            self.binding(),
            self.input.definition.target(),
            self.input.definition.operation(),
            self.input.definition.content().security(),
        ) {
            Ok(capability) => capability,
            Err(error) => {
                return Err(BindingInputRejection::new(
                    request.into_input(),
                    BindingOperationalError::new(error),
                ));
            }
        };
        // Issuance may invalidate its own credentials, wrapped policy or the
        // original instance. Refuse before constructing native work.
        let current = if consumer.is_current() {
            self.check_dependencies()
        } else {
            Err(stale())
        };
        if let Err(error) = current {
            return Err(BindingInputRejection::new(
                request.into_input(),
                BindingOperationalError::new(error),
            ));
        }
        let request = request
            .with_security(security)
            .with_responses(self.input.definition.content().response_shape());
        self.input
            .definition
            .execution()
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
    state: Rc<State>,
}
struct State {
    consumer: ConsumerInstanceRef,
    definitions: Vec<Arc<Definition>>,
    bank: Arc<HostResourceAccounts>,
    active: Cell<bool>,
    registry: RefCell<Registry>,
    // Last to drop, after all definitions, calls, native/result backing.
    _funding: Vec<HostCapacityReservation>,
}
struct Registry {
    slots: Vec<Slot>,
    sequence: u64,
    next_slot: usize,
}
// Inline slots are prepaid at construction; boxing each active call would add
// an allocation/retirement edge to checkout. Vacant slots retain that capacity.
#[allow(clippy::large_enum_variant)]
enum Slot {
    Vacant,
    Reserved {
        sequence: u64,
        control: Option<Rc<CallControl>>,
    },
    Ready(Call),
}
impl Slot {
    fn control(&self) -> Option<&Rc<CallControl>> {
        match self {
            Self::Vacant => None,
            Self::Reserved { control, .. } => control.as_ref(),
            Self::Ready(call) => Some(&call.work.control),
        }
    }
}

/// Control remains reachable while the complete call is outside the registry.
/// Recording a cause is passive; waking and destroying wakers happen separately.
struct CallControl {
    cause: RefCell<Option<CoreError>>,
    last_time: Cell<MonotonicInstant>,
    abandoned: Cell<bool>,
    waker: RefCell<Option<core::task::Waker>>,
    recheck: Cell<bool>,
    _funding: HostCapacityReservation,
}
impl CallControl {
    fn new(bank: &Arc<HostResourceAccounts>, now: MonotonicInstant) -> CoreResult<Rc<Self>> {
        let bytes = (core::mem::size_of::<Self>() + 2 * core::mem::size_of::<usize>()) as u64;
        let funding = bank
            .reserve(K::EngineLiveBytesGlobalMax, bytes)
            .ok_or_else(busy)?;
        Ok(Rc::new(Self {
            cause: RefCell::new(None),
            last_time: Cell::new(now),
            abandoned: Cell::new(false),
            waker: RefCell::new(None),
            recheck: Cell::new(false),
            _funding: funding,
        }))
    }
    fn record(&self, cause: CoreError) {
        self.cause.borrow_mut().get_or_insert(cause);
    }
    fn cause(&self) -> Option<CoreError> {
        self.cause.borrow().clone()
    }
    // Only public API entry observes caller time. A resumed sweep consumes this
    // latest observation instead of replaying its pre-callback timestamp.
    fn observe_time(&self, now: MonotonicInstant) {
        match now.checked_cmp(self.last_time.get()) {
            Some(Ordering::Equal | Ordering::Greater) => self.last_time.set(now),
            _ => self.record(clock_error()),
        }
    }
    fn register(&self, waker: &core::task::Waker) {
        let next = waker.clone();
        let old = self.waker.borrow_mut().replace(next);
        drop(old);
    }
    fn wake(&self) {
        let waker = self.waker.borrow_mut().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// A reserved slot stays occupied during admission/progress, including nested
/// entry. Unwinding restores the complete original call. Neither restoration
/// nor removal destroys linked owners under the registry borrow.
struct SlotLease<'a> {
    state: &'a State,
    index: usize,
    sequence: u64,
    call: Option<Call>,
}
impl State {
    fn current(&self) -> bool {
        self.active.get() && self.consumer.is_current()
    }
    fn reserve_slot(&self) -> CoreResult<SlotLease<'_>> {
        let mut registry = self.registry.borrow_mut();
        let index = registry
            .slots
            .iter()
            .position(|s| matches!(s, Slot::Vacant))
            .ok_or_else(busy)?;
        let sequence = registry.sequence.checked_add(1).ok_or_else(busy)?;
        registry.sequence = sequence;
        registry.slots[index] = Slot::Reserved {
            sequence,
            control: None,
        };
        Ok(SlotLease {
            state: self,
            index,
            sequence,
            call: None,
        })
    }
    fn checkout(
        &self,
        index: usize,
        sequence: Option<u64>,
        waker: &core::task::Waker,
    ) -> Option<SlotLease<'_>> {
        let (lease, control) = {
            let mut registry = self.registry.borrow_mut();
            let slot = &mut registry.slots[index];
            match slot {
                Slot::Ready(call) if sequence.is_none_or(|s| s == call.work.sequence) => {
                    let Slot::Ready(call) = core::mem::replace(slot, Slot::Vacant) else {
                        unreachable!()
                    };
                    let sequence = call.work.sequence;
                    let control = call.work.control.clone();
                    *slot = Slot::Reserved {
                        sequence,
                        control: Some(control.clone()),
                    };
                    (
                        Some(SlotLease {
                            state: self,
                            index,
                            sequence,
                            call: Some(call),
                        }),
                        control,
                    )
                }
                Slot::Reserved {
                    sequence: original,
                    control: Some(control),
                } if sequence.is_none_or(|s| s == *original) => {
                    control.recheck.set(true);
                    (None, control.clone())
                }
                _ => return None,
            }
        };
        // Retain driver interest even when another stack frame owns this call.
        // Waker clone/drop may reenter; the slot is already reserved and a live
        // lease restores its original owner even if registration unwinds.
        control.register(waker);
        lease
    }
    fn revoke(&self) {
        self.active.set(false);
        // First record every original cause without calling linked code. A
        // reentrant wake cannot make sibling cancellation win over this close.
        {
            let registry = self.registry.borrow();
            for control in registry.slots.iter().filter_map(Slot::control) {
                control.record(stale());
            }
        }
    }
    fn close(&self) {
        self.revoke();
        let count = self.registry.borrow().slots.len();
        for index in 0..count {
            let control = self.registry.borrow().slots[index].control().cloned();
            if let Some(control) = control {
                control.wake();
            }
        }
    }
}
impl SlotLease<'_> {
    fn attach(&self, control: Rc<CallControl>) {
        let mut registry = self.state.registry.borrow_mut();
        let Slot::Reserved { control: slot, .. } = &mut registry.slots[self.index] else {
            unreachable!()
        };
        *slot = Some(control);
    }
    fn take(mut self) -> Call {
        self.call.take().unwrap()
    }
}
impl Drop for SlotLease<'_> {
    fn drop(&mut self) {
        let replacement = self.call.take().map_or(Slot::Vacant, Slot::Ready);
        let old = {
            let mut registry = self.state.registry.borrow_mut();
            core::mem::replace(&mut registry.slots[self.index], replacement)
        };
        let Slot::Reserved { sequence, control } = old else {
            unreachable!()
        };
        debug_assert_eq!(sequence, self.sequence);
        if let Some(control) = control
            && control.recheck.replace(false)
        {
            control.wake();
        }
    }
}
struct Call {
    progress: FiniteCallState,
    work: CallWork,
}
struct CallWork {
    consumer: ConsumerInstanceRef,
    control: Rc<CallControl>,
    effect: ConsumerEffect,
    sequence: u64,
    raw: Option<HostBindingCallBox<CoreResult<InteractionOutput>>>,
    outcome: Option<BindingCallSettlement<CoreResult<InteractionOutput>>>,
    rejected_output: Option<InteractionOutput>,
    definition: Arc<Definition>,
    cleanup_error: Option<CoreError>,
    refused_transfer: Option<CleanupTransferRequest>,
    selection: Option<collection::PropertySelection>,
    deadline: Deadline,
    _funding: Vec<HostCapacityReservation>,
}
impl ConsumerRuntime {
    /// Recoverable ownership handoff. Only this captured vector supplies native
    /// executors/services; runtime calls accept no compiler/configuration override.
    pub fn try_new(
        consumer: ConsumerInstanceRef,
        definitions: Vec<ConsumerBinding>,
        funding: Vec<HostCapacityReservation>,
        bank: Arc<HostResourceAccounts>,
        calls: usize,
    ) -> Result<Self, BindingInputRejection<(Vec<ConsumerBinding>, Vec<HostCapacityReservation>)>>
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
            + calls * core::mem::size_of::<Slot>()
            + definitions.len()
                * (core::mem::size_of::<Arc<Definition>>()
                    + core::mem::size_of::<Definition>()
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
        slots.resize_with(calls, || Slot::Vacant);
        entries.extend(
            definitions
                .into_iter()
                .map(|d| Arc::new(Definition::from_prepared(d))),
        );
        let mut funding = funding;
        funding.extend(extra);
        Ok(Self {
            state: Rc::new(State {
                consumer,
                definitions: entries,
                bank,
                active: Cell::new(true),
                registry: RefCell::new(Registry {
                    slots,
                    sequence: 0,
                    next_slot: 0,
                }),
                _funding: funding,
            }),
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
        let state = &self.state;
        if !state.current() {
            return Err(reject(input, stale()));
        }
        let mut reserved = match state.reserve_slot() {
            Ok(reserved) => reserved,
            Err(error) => return Err(reject(input, error)),
        };
        let control = match CallControl::new(&state.bank, now) {
            Ok(control) => control,
            Err(error) => return Err(reject(input, error)),
        };
        reserved.attach(control.clone());
        let checked = (|| -> CoreResult<_> {
            if !state.current() {
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
            if !pay(budget, W::ProviderProbes, definition.access_work()) {
                return Err(busy());
            }
            definition.check_current(&state.consumer, None)?;
            let selection = match (definition.input.definition.content().schemas(), members) {
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
                    (definition.input.definition.content().schemas(), selection)
                {
                    // Authorization may change service availability or its
                    // work/backing declarations. Check at this callback edge.
                    if !definition.input.definition.data().available()
                        || definition.input.definition.data().policy() != policy
                    {
                        return Err(invalid());
                    }
                    if !pay(budget, W::CodecInputBytes, policy.validation_work) {
                        return Err(busy());
                    }
                    let payload = definition
                        .input
                        .definition
                        .data()
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
            if !state.current() {
                return Err(stale());
            }
            if let Some(cause) = control.cause() {
                return Err(cause);
            }
            match deadline.checked_is_elapsed_at(control.last_time.get()) {
                Some(false) => {}
                Some(true) => return Err(CoreError::TimedOut(context())),
                None => return Err(invalid()),
            }
            Ok((definition, funding, input_selection.or(selection), encoded))
        })();
        let (definition, funding, selection, encoded) = match checked {
            Ok(checked) => checked,
            Err(e) => return Err(reject(input, e)),
        };
        let raw = match definition.start(
            &self.state.consumer,
            encoded.or(input),
            deadline.instant().map(|_| deadline),
            budget,
        ) {
            Ok(raw) => raw,
            Err(rejection) if operation == Operation::ReadMultipleProperties => {
                let (payload, error) = rejection.into_parts();
                drop(payload); // encoded input dies before its original reservation
                return Err(BindingInputRejection::new(None, error));
            }
            Err(rejection) => return Err(rejection),
        };
        let slot = reserved.index;
        let sequence = reserved.sequence;
        let mut call = Call {
            progress: FiniteCallState::default(),
            work: CallWork {
                consumer: state.consumer.clone(),
                control: control.clone(),
                effect: ConsumerEffect::NotAttempted,
                sequence,
                definition,
                raw: Some(raw),
                outcome: None,
                rejected_output: None,
                cleanup_error: None,
                refused_transfer: None,
                selection,
                deadline,
                _funding: funding,
            },
        };
        // Native construction is local, but can revoke/close through a linked
        // callback. Retain that complete work for cancellation before any poll.
        call.work.fence(state);
        reserved.call = Some(call);
        drop(reserved);
        Ok(ConsumerCall {
            state: self.state.clone(),
            slot,
            sequence,
            control,
        })
    }
    /// Closes this finite facet. Other facets of the Consumer stay independent;
    /// the ConsumerInstance owner revokes the whole incarnation.
    pub fn begin_close(&self) {
        self.state.close();
    }
    /// Drive independent cleanup, including after every call handle was lost.
    pub fn step(&self, cx: &mut Context<'_>, now: MonotonicInstant, budget: &mut WorkBudget) {
        let (start, count) = {
            let mut registry = self.state.registry.borrow_mut();
            // Latch one API-entry observation before any linked callback. A
            // nested entry can advance all original controls, including busy
            // slots, and a newly admitted call keeps its own later observation.
            for control in registry.slots.iter().filter_map(Slot::control) {
                control.observe_time(now);
            }
            let count = registry.slots.len();
            let start = registry.next_slot;
            registry.next_slot = (start + 1) % count;
            (start, count)
        };
        for offset in 0..count {
            if let Some(mut lease) = self
                .state
                .checkout((start + offset) % count, None, cx.waker())
            {
                let call = lease.call.as_mut().unwrap();
                let _ = call.step(&self.state, cx, budget);
                if call.work.control.abandoned.get() && call.progress.is_terminal() {
                    // Vacate the slot before disposing linked native owners.
                    drop(lease.take());
                }
            }
        }
    }
    pub fn is_idle(&self) -> bool {
        self.state
            .registry
            .borrow()
            .slots
            .iter()
            .all(|s| matches!(s, Slot::Vacant))
    }
    pub fn consumer(&self) -> ConsumerInstanceRef {
        self.state.consumer.clone()
    }
}
impl Drop for ConsumerRuntime {
    fn drop(&mut self) {
        self.state.revoke();
    }
}

/// A logical call handle. Cancellation and abandonment remain reachable while
/// its native work is checked out; each incarnation has a distinct control.
pub struct ConsumerCall {
    state: Rc<State>,
    slot: usize,
    sequence: u64,
    control: Rc<CallControl>,
}
impl ConsumerCall {
    pub fn cancel(&self) {
        self.control.record(CoreError::Cancelled(context()));
        self.control.wake();
    }
    pub fn poll(
        &mut self,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Poll<ConsumerSettlement> {
        self.control.observe_time(now);
        let Some(mut lease) = self
            .state
            .checkout(self.slot, Some(self.sequence), cx.waker())
        else {
            return Poll::Pending;
        };
        if lease
            .call
            .as_mut()
            .unwrap()
            .step(&self.state, cx, budget)
            .is_pending()
        {
            return Poll::Pending;
        }
        Poll::Ready(ConsumerSettlement { call: lease.take() })
    }
}
impl Drop for ConsumerCall {
    fn drop(&mut self) {
        self.control.abandoned.set(true);
        self.control.record(CoreError::Cancelled(context()));
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
        match (
            self.call
                .work
                .definition
                .input
                .definition
                .content()
                .schemas(),
            self.call.work.selection,
        ) {
            (
                ConsumerFiniteSchemas::WriteAllProperties(schema)
                | ConsumerFiniteSchemas::WriteMultipleProperties(schema),
                Some(selected),
            ) => Some((schema, selected)),
            _ => None,
        }
    }
    pub fn effect(&self) -> ConsumerEffect {
        self.call.work.effect
    }
    pub fn consumer(&self) -> &ConsumerInstanceRef {
        &self.call.work.consumer
    }
    pub fn operation(&self) -> Operation {
        self.call.work.definition.input.definition.operation()
    }
    /// Lends the original coordinate without exposing its shared name owner.
    pub fn affordance(&self) -> AffordanceSite<'_> {
        match self.call.work.definition.input.definition.affordance() {
            AffordanceTarget::Thing => AffordanceSite::Thing,
            AffordanceTarget::Property(name) => AffordanceSite::Property(name),
            AffordanceTarget::Action(name) => AffordanceSite::Action(name),
            AffordanceTarget::Event(name) => AffordanceSite::Event(name),
        }
    }
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.call.progress.first_cause()
    }
    /// Lends original native output rejected by captured result validation.
    pub fn rejected_output(&self) -> Option<InteractionOutputView<'_>> {
        self.call
            .work
            .rejected_output
            .as_ref()
            .map(InteractionOutput::view)
    }
    pub fn refused_transfer(&self) -> Option<&CleanupTransferRequest> {
        self.call.work.refused_transfer.as_ref()
    }
    pub fn cleanup_error(&self) -> Option<&CoreError> {
        self.call.work.cleanup_error.as_ref()
    }
    /// Lends the complete original outcome without exposing payload owners.
    pub fn native_outcome(&self) -> ConsumerNativeOutcome<'_> {
        match self.call.work.outcome.as_ref().unwrap() {
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
        if let Some(error) = self.call.progress.first_cause() {
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
        self.call.work.definition.binding()
    }
}
impl Call {
    fn step(&mut self, state: &State, cx: &mut Context<'_>, budget: &mut WorkBudget) -> Poll<()> {
        self.progress.step(
            &mut HostCallDriver {
                work: &mut self.work,
                state,
            },
            cx,
            budget,
        )
    }
}
impl CallWork {
    fn fence(&mut self, state: &State) -> Option<CoreError> {
        match self
            .deadline
            .checked_is_elapsed_at(self.control.last_time.get())
        {
            Some(false) => {}
            Some(true) => self.control.record(CoreError::TimedOut(context())),
            None => self.control.record(clock_error()),
        }
        if !state.current() {
            self.control.record(stale());
        }
        self.control.cause()
    }
}
struct HostCallDriver<'a> {
    work: &'a mut CallWork,
    state: &'a State,
}
impl FiniteCallDriver for HostCallDriver<'_> {
    fn fence(&mut self) -> Option<CoreError> {
        self.work.fence(self.state)
    }
    fn record(&mut self, error: CoreError) {
        self.work.control.record(error);
    }
    fn access_work(&self) -> u64 {
        self.work.definition.access_work()
    }
    fn check_current(&mut self) -> CoreResult<()> {
        self.work
            .definition
            .check_current(&self.work.consumer, self.work.selection)
    }
    fn poll_native(
        &mut self,
        cx: &mut Context<'_>,
        cancel: Option<(&CoreError, bool)>,
        budget: &mut WorkBudget,
    ) -> NativeProgress {
        let work = &mut self.work;
        work.effect = ConsumerEffect::Unknown;
        let raw = work.raw.as_mut().unwrap();
        let mut progress = NativeProgress::Pending;
        work.outcome = if let Some((cause, started)) = cancel {
            if started {
                match raw.as_pin_mut().poll_cancel(cx, budget) {
                    Poll::Pending => None,
                    Poll::Ready(Ok(value)) => Some(value),
                    Poll::Ready(Err(error)) => {
                        work.cleanup_error.get_or_insert(error);
                        None
                    }
                }
            } else {
                let cleanup = CleanupPhaseContext::bind(
                    CleanupReservation::new(
                        CleanupSlotId::new(SlotIndex::new(0), Generation::INITIAL),
                        work.definition.resources().admitted(),
                        0,
                        WorkBudget::new().with_remaining(W::CleanupItems, 1),
                    ),
                    CleanupOperation::CancelRequest,
                    cause.clone(),
                    Deadline::NONE,
                );
                match raw.as_pin_mut().start_cancel(cx, cleanup, budget) {
                    Ok(StartStatus::Pending) => {
                        progress = NativeProgress::Cancelling;
                        None
                    }
                    Ok(StartStatus::Ready(value)) => Some(value),
                    Err(error) => {
                        work.cleanup_error.get_or_insert(error);
                        None
                    }
                }
            }
        } else {
            match raw.as_pin_mut().poll_result(cx, budget) {
                Poll::Pending => None,
                Poll::Ready(result) => {
                    if let Err(error) = &result {
                        work.control.record(error.clone());
                    }
                    Some(BindingCallSettlement::Returned(result))
                }
            }
        };
        if matches!(
            work.outcome,
            Some(BindingCallSettlement::Cancelled {
                disposition: BindingCancellationDisposition::TransferRequired(_),
                ..
            })
        ) {
            // No external destination was granted. Keep original work and drive
            // its cleanup; a request alone acknowledges no ownership transfer.
            if let Some(BindingCallSettlement::Cancelled {
                disposition: BindingCancellationDisposition::TransferRequired(request),
                ..
            }) = work.outcome.take()
            {
                work.refused_transfer = Some(request);
            }
            work.cleanup_error
                .get_or_insert(CoreError::InternalInvariant(ErrorContext::new(
                    ErrorPhase::Cleanup,
                    RetryClass::Never,
                )));
            return NativeProgress::Cancelling;
        }
        if work.outcome.is_some() {
            NativeProgress::Settled
        } else {
            progress
        }
    }
    fn has_success(&self) -> bool {
        matches!(
            self.work.outcome,
            Some(BindingCallSettlement::Returned(Ok(_)))
        )
    }
    fn validate_output(&mut self, budget: &mut WorkBudget) -> Poll<CoreResult<()>> {
        let work = &mut self.work;
        let outcome = work.outcome.as_mut().unwrap();
        let BindingCallSettlement::Returned(Ok(output)) = outcome else {
            unreachable!()
        };
        match work.definition.validate(output, work.selection, budget) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                if output.status() == InteractionStatus::Ok
                    && output.metadata().response_selection() == ResponseSelection::Primary
                    && matches!(
                        work.definition.input.definition.operation(),
                        Operation::WriteProperty
                            | Operation::WriteAllProperties
                            | Operation::WriteMultipleProperties
                    )
                {
                    work.effect = if output
                        .metadata()
                        .property_write_report()
                        .is_some_and(|r| !r.is_complete())
                    {
                        ConsumerEffect::Reported
                    } else {
                        ConsumerEffect::Completed
                    };
                }
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => {
                if let BindingCallSettlement::Returned(Ok(original)) =
                    core::mem::replace(outcome, BindingCallSettlement::Returned(Err(error.clone())))
                {
                    work.rejected_output = Some(original);
                }
                Poll::Ready(Err(error))
            }
        }
    }
}
fn stale() -> CoreError {
    CoreError::StaleHandle(context())
}
fn clock_error() -> CoreError {
    CoreError::InternalInvariant(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never))
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(context())
}
fn reject<T>(input: T, e: CoreError) -> BindingInputRejection<T> {
    BindingInputRejection::new(input, BindingOperationalError::new(e))
}
