//! Producer-side raw Action invocation kernel. There is deliberately no
//! FiniteIngressBinding here: an initiating reply, subsequent control messages,
//! correlation and native resources remain with their actual Binding owner.
use super::{RawProducerServient, context};
use alloc::{
    boxed::Box,
    rc::{Rc, Weak},
    sync::Arc,
    vec::Vec,
};
use clinkz_wot_core::{
    ActionInvocationRef, AffordanceTarget, BindingGeneration, BindingId, CoreError, CoreResult,
    ErrorContext, ErrorPhase, InteractionInput, InteractionOutput, Operation, RetryClass,
    SecurityFailureReason,
    action::{
        ActionEffect, ActionMappingKey, ActionRoute, RawActionHandler, RawActionRequest,
        RawActionStart, RawActionWork, raw_action_output_shape,
    },
    interaction_output_backing,
    producer::ProducerInstanceKey,
    security::PrincipalId,
};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use core::{cell::Cell, mem::size_of, task::Poll};

const MAX_INVOCATIONS: usize = 128;

/// A registered application behavior, not a per-Binding instance. The query
/// and cancel flags represent actual supported control operations in this raw
/// facet; they do not qualify or publish any TD Forms.
pub struct RawActionBehavior {
    name: Arc<str>,
    handler: Box<dyn RawActionHandler>,
    work_bytes: usize,
    query: bool,
    cancel: bool,
}

impl RawActionBehavior {
    pub fn new(
        name: Arc<str>,
        handler: impl RawActionHandler + 'static,
        work_bytes: usize,
        query: bool,
        cancel: bool,
    ) -> Self {
        Self {
            name,
            handler: Box::new(handler),
            work_bytes,
            query,
            cancel,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub(crate) fn supports(&self, operation: Operation) -> bool {
        match operation {
            Operation::InvokeAction => true,
            Operation::QueryAction => self.query,
            Operation::CancelAction => self.cancel,
            _ => false,
        }
    }
}

/// Caller-funded Host envelope. Every admitted continuation reserves its
/// declared application backing until terminal acknowledgement, even after
/// cancellation or replacement. Native backing has a separate Binding owner.
#[derive(Clone, Copy, Debug)]
pub struct RawActionLimits {
    pub invocations: usize,
    pub application_work_bytes: usize,
    pub input_fields: usize,
    pub input_bytes: usize,
    pub principal_id_bytes: usize,
    pub output_body_bytes: usize,
    pub output_media_capacity: usize,
    pub output_retained_bytes: usize,
}

/// Non-forgeable local reference to one admitted obligation. Its numeric
/// invocation reference is diagnostic only; it cannot authorize control by
/// itself. Clones carry no right to accept work or shut down a Binding.
#[derive(Clone)]
pub struct RawActionHandle {
    id: ActionInvocationRef,
    token: Weak<Cell<bool>>,
}
impl RawActionHandle {
    pub fn reference(&self) -> ActionInvocationRef {
        self.id
    }
}
impl core::fmt::Debug for RawActionHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RawActionHandle")
            .field("reference", &self.id)
            .finish()
    }
}

pub enum RawActionState<'a> {
    Running,
    Complete(&'a CoreResult<InteractionOutput>),
}

/// Borrowed observation. Completion is retained until acknowledged; a cancel
/// request is not a completion or proof of remote cancellation.
pub struct RawActionView<'a> {
    pub state: RawActionState<'a>,
    pub cancel_requested: bool,
    pub cancel_failure: Option<&'a CoreError>,
}

/// Complete original semantic result after the application continuation has
/// released locally. Native delivery/remote effect is NOT inferred from this
/// acknowledgement and remains the Binding's separate obligation.
pub struct RawActionSettlement {
    pub reference: ActionInvocationRef,
    pub instance: ProducerInstanceKey,
    pub target: AffordanceTarget,
    pub binding: (BindingId, BindingGeneration),
    pub mapping: ActionMappingKey,
    pub query: Option<ActionRoute>,
    pub cancel: Option<ActionRoute>,
    pub result: CoreResult<InteractionOutput>,
    pub effect: ActionEffect,
    pub cancel_requested: bool,
    pub cancel_failure: Option<CoreError>,
    pub abandoned: bool,
    pub close_failure: Option<CoreError>,
}

impl core::fmt::Debug for RawActionSettlement {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RawActionSettlement")
            .field("reference", &self.reference)
            .field("instance", &self.instance)
            .field("binding", &self.binding)
            .field("mapping", &self.mapping)
            .field("effect", &self.effect)
            .field("cancel_requested", &self.cancel_requested)
            .field("abandoned", &self.abandoned)
            .field("error", &self.result.as_ref().err())
            .field("has_output", &self.result.is_ok())
            .finish()
    }
}

struct Invocation {
    id: ActionInvocationRef,
    // Shared with handles so step can select and debit the cleanup/handler
    // lookup lane before scanning entries. Only the owner can set this flag.
    token: Rc<Cell<bool>>,
    // The original logical permission cell survives replacement and guard
    // Drop. A later funded step can request cancellation even when no mutable
    // Servient was available to the guard at revocation time.
    current: Rc<Cell<bool>>,
    // Strong retention of the exact behavior/instance, never a successor lookup.
    instance: Rc<super::RawProducerInstance>,
    behavior: usize,
    binding: (BindingId, BindingGeneration),
    mapping: ActionMappingKey,
    query: Option<ActionRoute>,
    cancel: Option<ActionRoute>,
    principal: Option<PrincipalId>,
    work: Option<Box<dyn RawActionWork>>,
    result: Option<CoreResult<InteractionOutput>>,
    cancel_requested: bool,
    cancel_sent: bool,
    cancel_failure: Option<CoreError>,
    abandoned: bool,
    close_failure: Option<CoreError>,
    work_bytes: usize,
}

pub(super) struct ActionStore {
    entries: Vec<Invocation>,
    limits: RawActionLimits,
    used_work_bytes: usize,
    next: u32,
    retiring: bool,
    retire_cursor: usize,
}

impl Drop for ActionStore {
    fn drop(&mut self) {
        // No poll/await or remote callback in Drop. Every accepted live work
        // owner must supply a qualified finite local abort. Explicit retirement
        // is required to report results/cleanup errors before dropping the root.
        for entry in &mut self.entries {
            if let Some(work) = entry.work.take() {
                work.abort_local();
            }
        }
    }
}

fn denied() -> CoreError {
    CoreError::Security {
        reason: SecurityFailureReason::AuthorizationDenied,
        context: ErrorContext::new(ErrorPhase::Selection, RetryClass::Never),
    }
}

fn check_input(input: &InteractionInput, limits: RawActionLimits) -> bool {
    let scopes = input
        .principal
        .as_ref()
        .map_or(&[][..], |p| p.scopes.as_slice());
    let alternatives = input.accept.as_ref().and_then(|a| a.alternatives.as_ref());
    let fields = input
        .uri_variables
        .len()
        .checked_add(scopes.len())
        .and_then(|n| n.checked_add(alternatives.map_or(0, Vec::len)))
        .and_then(|n| n.checked_add(usize::from(input.data.is_some())))
        .and_then(|n| n.checked_add(usize::from(input.accept.is_some())));
    if fields.is_none_or(|n| n > limits.input_fields)
        || input
            .principal
            .as_ref()
            .is_some_and(|p| p.id.as_str().len() > limits.principal_id_bytes)
    {
        return false;
    }
    let mut bytes = 0usize;
    let mut charge = |n: usize| {
        bytes = bytes.saturating_add(n);
    };
    for (k, v) in &input.uri_variables {
        charge(k.len());
        charge(v.len());
    }
    for scope in scopes {
        charge(scope.len());
    }
    if let Some(p) = &input.principal {
        charge(p.id.as_str().len());
    }
    if let Some(p) = &input.data {
        charge(p.body.len());
        charge(p.content_type.len());
        if let Some(c) = &p.content_coding {
            charge(c.len());
        }
    }
    if let Some(a) = &input.accept {
        charge(a.preferred.as_str().len());
        if let Some(alts) = alternatives {
            for alt in alts {
                charge(alt.as_str().len());
            }
        }
    }
    bytes <= limits.input_bytes
}

fn check_output(output: &InteractionOutput, limits: RawActionLimits) -> CoreResult<()> {
    let fits = raw_action_output_shape(output)
        && output.data().is_none_or(|p| {
            p.body.len() <= limits.output_body_bytes
                && p.content_type.capacity() <= limits.output_media_capacity
                && p.content_coding
                    .as_ref()
                    .is_none_or(|c| c.capacity() <= limits.output_media_capacity)
        })
        && interaction_output_backing(output).is_some_and(|n| n <= limits.output_retained_bytes);
    if fits {
        Ok(())
    } else {
        Err(CoreError::Validation(ErrorContext::new(
            ErrorPhase::Validate,
            RetryClass::Never,
        )))
    }
}

impl ActionStore {
    pub(super) fn revoke(&mut self, key: ProducerInstanceKey) {
        for entry in &mut self.entries {
            if entry.instance.key() == key && entry.work.is_some() {
                entry.cancel_requested = true;
            }
        }
    }
    fn index(&self, handle: &RawActionHandle) -> Option<usize> {
        let token = handle.token.upgrade()?;
        self.entries
            .iter()
            .position(|entry| entry.id == handle.id && Rc::ptr_eq(&entry.token, &token))
    }
    fn view(entry: &Invocation) -> RawActionView<'_> {
        RawActionView {
            state: entry
                .result
                .as_ref()
                .map_or(RawActionState::Running, RawActionState::Complete),
            cancel_requested: entry.cancel_requested,
            cancel_failure: entry.cancel_failure.as_ref(),
        }
    }
}

impl RawProducerServient {
    /// Reserve a bounded independent Action obligation store at the *existing*
    /// logical owner, before ingress. Does not activate or publish a Thing.
    pub fn enable_raw_actions(&mut self, limits: RawActionLimits) -> CoreResult<()> {
        if self.actions.is_some()
            || !(1..=MAX_INVOCATIONS).contains(&limits.invocations)
            || limits.input_fields > MAX_INVOCATIONS
            || limits.output_retained_bytes < size_of::<InteractionOutput>()
        {
            return Err(CoreError::Validation(context()));
        }
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(limits.invocations)
            .map_err(|_| CoreError::Backpressure(context()))?;
        self.actions = Some(ActionStore {
            entries,
            limits,
            used_work_bytes: 0,
            next: 0,
            retiring: false,
            retire_cursor: 0,
        });
        Ok(())
    }

    /// Semantic acceptance, separate from the native initiating exchange.
    /// Failed admission/authorization never enters application behavior. Once
    /// entered, even an error may have had an effect and is never replayed.
    pub fn invoke_raw_action(
        &mut self,
        request: RawActionRequest<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<RawActionHandle> {
        let store = self
            .actions
            .as_mut()
            .ok_or_else(|| CoreError::UnsupportedOperation(context()))?;
        if store.retiring {
            return Err(CoreError::StaleHandle(context()));
        }
        let limits = store.limits;
        let AffordanceTarget::Action(name) = request.target else {
            return Err(CoreError::UnsupportedOperation(context()));
        };
        if name.len() > self.limits.target_name_bytes {
            return Err(CoreError::Backpressure(context()));
        }
        // Pay the whole bounded input traversal, lookup and maximum output
        // traversal before checking variable input or entering linked code.
        let units = self
            .limits
            .instances
            .checked_add(
                self.limits
                    .behaviors_per_instance
                    .saturating_mul(self.limits.target_name_bytes + 1),
            )
            .and_then(|n| n.checked_add(limits.input_bytes))
            .and_then(|n| n.checked_add(limits.input_fields))
            .and_then(|n| n.checked_add(limits.output_body_bytes))
            .and_then(|n| n.checked_add(limits.output_media_capacity.saturating_mul(2)))
            .and_then(|n| u64::try_from(n).ok())
            .ok_or_else(|| CoreError::Backpressure(context()))?;
        budget
            .consume(WorkClass::HandlerSteps, units)
            .map_err(|_| CoreError::Backpressure(context()))?;
        if !check_input(request.input, limits) {
            return Err(CoreError::Backpressure(context()));
        }
        let instance = self
            .instances
            .iter()
            .find(|e| e.current.get() && e.instance.key() == request.instance)
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        let index = instance
            .instance
            .actions
            .iter()
            .position(|a| a.name.as_ref() == name.as_ref())
            .ok_or_else(|| CoreError::UnsupportedOperation(context()))?;
        let behavior = &instance.instance.actions[index];
        if (request.query.is_some() && !behavior.query)
            || (request.cancel.is_some() && !behavior.cancel)
        {
            return Err(CoreError::UnsupportedOperation(context()));
        }
        if store.entries.len() == limits.invocations
            || behavior.work_bytes
                > limits
                    .application_work_bytes
                    .saturating_sub(store.used_work_bytes)
        {
            return Err(CoreError::Backpressure(context()));
        }
        let next = store
            .next
            .checked_add(1)
            .ok_or_else(|| CoreError::Backpressure(context()))?;
        behavior
            .handler
            .authorize(Operation::InvokeAction, request.input)?;
        let target = AffordanceTarget::Action(Arc::clone(&behavior.name));
        let ctx = clinkz_wot_core::HandlerContext::for_producer(
            instance.instance.key(),
            &target,
            Operation::InvokeAction,
            request.invoke.binding,
        )?;
        let token = Rc::new(Cell::new(false));
        let id = ActionInvocationRef::new(SlotIndex::new(next), Generation::INITIAL);
        let handle = RawActionHandle {
            id,
            token: Rc::downgrade(&token),
        };
        // Reserve and pin the original instance *before* invoking behavior.
        store.next = next;
        store.used_work_bytes += behavior.work_bytes;
        store.entries.push(Invocation {
            id,
            token,
            current: Rc::clone(&instance.current),
            instance: Rc::clone(&instance.instance),
            behavior: index,
            binding: request.invoke.binding,
            mapping: request.invoke.mapping,
            query: request.query,
            cancel: request.cancel,
            principal: request.input.principal.as_ref().map(|p| p.id.clone()),
            work: None,
            result: None,
            cancel_requested: false,
            cancel_sent: false,
            cancel_failure: None,
            abandoned: false,
            close_failure: None,
            work_bytes: behavior.work_bytes,
        });
        let started = behavior.handler.invoke(ctx, request.input);
        let entry = store.entries.last_mut().expect("reserved invocation");
        match started {
            Ok(RawActionStart::Continue(work)) => entry.work = Some(work),
            Ok(RawActionStart::Complete(value)) => {
                entry.result = Some(check_output(&value, limits).map(|()| value))
            }
            Err(error) => entry.result = Some(Err(error)),
        }
        Ok(handle)
    }

    fn control(
        &self,
        handle: &RawActionHandle,
        binding: (BindingId, BindingGeneration),
        mapping: ActionMappingKey,
        input: &InteractionInput,
        operation: Operation,
        budget: &mut WorkBudget,
    ) -> CoreResult<usize> {
        let store = self
            .actions
            .as_ref()
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        if store.retiring {
            return Err(CoreError::StaleHandle(context()));
        }
        let units = self
            .limits
            .instances
            .checked_add(
                self.limits
                    .behaviors_per_instance
                    .saturating_mul(self.limits.target_name_bytes + 1),
            )
            .and_then(|n| n.checked_add(store.limits.invocations))
            .and_then(|n| n.checked_add(store.limits.input_bytes))
            .and_then(|n| n.checked_add(store.limits.input_fields))
            .ok_or_else(|| CoreError::Backpressure(context()))?;
        budget
            .consume(
                WorkClass::HandlerSteps,
                u64::try_from(units).map_err(|_| CoreError::Backpressure(context()))?,
            )
            .map_err(|_| CoreError::Backpressure(context()))?;
        if !check_input(input, store.limits) {
            return Err(CoreError::Backpressure(context()));
        }
        let i = store
            .index(handle)
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        let entry = &store.entries[i];
        let expected = match operation {
            Operation::QueryAction => entry.query,
            Operation::CancelAction => entry.cancel,
            _ => None,
        };
        if expected.is_none() {
            return Err(CoreError::UnsupportedOperation(context()));
        }
        if entry.abandoned
            || expected != Some(ActionRoute { binding, mapping })
            || entry.principal.as_ref() != input.principal.as_ref().map(|p| &p.id)
        {
            return Err(denied());
        }
        if !self
            .instances
            .iter()
            .any(|e| e.current.get() && e.instance.key() == entry.instance.key())
        {
            return Err(CoreError::StaleHandle(context()));
        }
        let behavior = &entry.instance.actions[entry.behavior];
        if !match operation {
            Operation::QueryAction => behavior.query,
            Operation::CancelAction => behavior.cancel,
            _ => false,
        } {
            return Err(CoreError::UnsupportedOperation(context()));
        }
        behavior.handler.authorize(operation, input)?;
        Ok(i)
    }

    /// Fresh logical authority, mapping, principal and application policy are
    /// required even when the initiating exchange has long since settled.
    pub fn query_raw_action(
        &self,
        handle: &RawActionHandle,
        binding: (BindingId, BindingGeneration),
        mapping: ActionMappingKey,
        input: &InteractionInput,
        budget: &mut WorkBudget,
    ) -> CoreResult<RawActionView<'_>> {
        let i = self.control(
            handle,
            binding,
            mapping,
            input,
            Operation::QueryAction,
            budget,
        )?;
        Ok(ActionStore::view(
            &self.actions.as_ref().unwrap().entries[i],
        ))
    }

    /// Request application cancellation only. The caller's native cancel
    /// exchange, response and wire correlation remain with its Binding.
    /// `true` means a request was queued, never that the work was undone.
    pub fn cancel_raw_action(
        &mut self,
        handle: &RawActionHandle,
        binding: (BindingId, BindingGeneration),
        mapping: ActionMappingKey,
        input: &InteractionInput,
        budget: &mut WorkBudget,
    ) -> CoreResult<bool> {
        let i = self.control(
            handle,
            binding,
            mapping,
            input,
            Operation::CancelAction,
            budget,
        )?;
        let entry = &mut self.actions.as_mut().unwrap().entries[i];
        if entry.result.is_some() {
            return Ok(false);
        }
        entry.cancel_requested = true;
        Ok(true)
    }

    /// Local accepted-work owner abandons public control, requests cooperative
    /// cancellation, and still retains late results through acknowledgement.
    pub fn abandon_raw_action(&mut self, handle: &RawActionHandle) -> CoreResult<()> {
        let store = self
            .actions
            .as_mut()
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        let i = store
            .index(handle)
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        store.entries[i].abandoned = true;
        if store.entries[i].work.is_some() {
            store.entries[i].cancel_requested = true;
        }
        Ok(())
    }

    /// Stop waiting for an uncooperative application continuation. Close is
    /// independently budgeted and cannot release a pending work owner. It
    /// reports local cancellation, not a remote terminal Action state.
    pub fn close_raw_action(&mut self, handle: &RawActionHandle) -> CoreResult<()> {
        let store = self
            .actions
            .as_mut()
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        let i = store
            .index(handle)
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        let entry = &mut store.entries[i];
        entry.abandoned = true;
        if entry.work.is_some() {
            entry.cancel_requested = true;
        }
        entry.token.set(true);
        Ok(())
    }

    /// One bounded application/cleanup step. No native poll or handler relookup;
    /// revocation does not prevent an already accepted invocation from settling.
    pub fn step_raw_action(
        &mut self,
        handle: &RawActionHandle,
        budget: &mut WorkBudget,
    ) -> CoreResult<Poll<()>> {
        let store = self
            .actions
            .as_mut()
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        // A live token provides an O(1) owner-set closing hint. Pay for the
        // full bounded scan on that lane before looking through entries, even
        // when the work has already completed or this is a foreign handle.
        let closing = handle
            .token
            .upgrade()
            .ok_or_else(|| CoreError::StaleHandle(context()))?
            .get();
        let class = if closing {
            WorkClass::CleanupItems
        } else {
            WorkClass::HandlerSteps
        };
        if budget
            .consume(class, store.limits.invocations as u64)
            .is_err()
        {
            return Ok(Poll::Pending);
        }
        let i = store
            .index(handle)
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        store.progress(i, budget)
    }

    /// Explicit acknowledgement. Until this point the old instance, result,
    /// application reservation and local cleanup obligation stay charged.
    pub fn take_raw_action_settlement(
        &mut self,
        handle: &RawActionHandle,
    ) -> CoreResult<Option<RawActionSettlement>> {
        let store = self
            .actions
            .as_mut()
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        let i = store
            .index(handle)
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        Ok(store.take_at(i))
    }

    /// Fence new Action acceptance and public query/cancel without revoking
    /// unrelated finite Property work. The Servient itself remains the owner
    /// of EVERY admitted Action, including ones with no surviving handle.
    /// No application callback, native call, allocation or result destruction
    /// occurs at this local linearization point.
    pub fn begin_raw_action_retirement(&mut self) -> CoreResult<()> {
        let store = self
            .actions
            .as_mut()
            .ok_or_else(|| CoreError::UnsupportedOperation(context()))?;
        store.retiring = true;
        for entry in &mut store.entries {
            entry.abandoned = true;
            entry.token.set(true);
            if entry.work.is_some() {
                entry.cancel_requested = true;
            }
        }
        Ok(())
    }

    /// Funded owner-level cleanup/settlement when all handles have been lost.
    /// A Pending or error keeps the complete work owner and its reservation;
    /// the next call may progress another obligation. `Settled` transfers the
    /// complete terminal result to the caller, not a native delivery claim.
    /// Ready(Some) owns that settlement; Ready(None) means the local Action
    /// store is drained, not that a remote Action or Binding resource is gone.
    pub fn step_raw_action_retirement(
        &mut self,
        budget: &mut WorkBudget,
    ) -> CoreResult<Poll<Option<RawActionSettlement>>> {
        let store = self
            .actions
            .as_mut()
            .ok_or_else(|| CoreError::UnsupportedOperation(context()))?;
        if !store.retiring {
            return Err(CoreError::Lifecycle(context()));
        }
        if store.entries.is_empty() {
            return Ok(Poll::Ready(None));
        }
        // One prepaid bounded scan, including a possible Vec removal. Terminal
        // work is handed off first; otherwise round-robin avoids a stalled
        // application cleanup starving independent invocations.
        let scan = store.limits.invocations as u64 * 2 + 1;
        if budget.consume(WorkClass::CleanupItems, scan).is_err() {
            return Ok(Poll::Pending);
        }
        let i = store
            .entries
            .iter()
            .position(|entry| entry.work.is_none())
            .unwrap_or(store.retire_cursor % store.entries.len());
        store.retire_cursor = (i + 1) % store.entries.len();
        match store.progress(i, budget)? {
            Poll::Pending => Ok(Poll::Pending),
            Poll::Ready(()) => Ok(Poll::Ready(Some(
                store.take_at(i).expect("completed Action still owned"),
            ))),
        }
    }
}

impl ActionStore {
    fn progress(&mut self, i: usize, budget: &mut WorkBudget) -> CoreResult<Poll<()>> {
        let entry = &mut self.entries[i];
        if entry.work.is_none() {
            return Ok(Poll::Ready(()));
        }
        if !entry.current.get() {
            entry.cancel_requested = true;
        }
        let closing = entry.token.get();
        let class = if closing {
            WorkClass::CleanupItems
        } else {
            WorkClass::HandlerSteps
        };
        let units = if closing {
            1 + u64::from(entry.cancel_requested && !entry.cancel_sent)
        } else {
            u64::try_from(
                self.limits
                    .output_body_bytes
                    .saturating_add(self.limits.output_media_capacity.saturating_mul(2)),
            )
            .unwrap_or(u64::MAX)
            .saturating_add(1)
            .saturating_add(u64::from(entry.cancel_requested && !entry.cancel_sent))
        };
        if budget.consume(class, units).is_err() {
            return Ok(Poll::Pending);
        }
        let work = entry.work.as_mut().expect("checked work");
        if entry.cancel_requested && !entry.cancel_sent {
            entry.cancel_sent = true;
            entry.cancel_failure = work.request_cancel().err();
        }
        if closing {
            match work.poll_close() {
                Poll::Pending => return Ok(Poll::Pending),
                Poll::Ready(Err(error)) => {
                    let first = entry.close_failure.get_or_insert(error);
                    // An operational failure is visible NOW, but cannot
                    // surrender a still-live cleanup owner or change its
                    // original application result.
                    return Err(first.clone());
                }
                Poll::Ready(Ok(())) => {
                    entry.work = None;
                    entry.result.get_or_insert_with(|| {
                        Err(CoreError::Cancelled(ErrorContext::new(
                            ErrorPhase::Cleanup,
                            RetryClass::Never,
                        )))
                    });
                }
            }
        } else {
            match work.poll() {
                Poll::Pending => return Ok(Poll::Pending),
                Poll::Ready(result) => {
                    entry.result = Some(
                        result.and_then(|value| check_output(&value, self.limits).map(|()| value)),
                    );
                    entry.work = None;
                }
            }
        }
        Ok(Poll::Ready(()))
    }

    fn take_at(&mut self, i: usize) -> Option<RawActionSettlement> {
        if self.entries[i].work.is_some() {
            return None;
        }
        let entry = self.entries.remove(i);
        self.used_work_bytes -= entry.work_bytes;
        Some(RawActionSettlement {
            reference: entry.id,
            instance: entry.instance.key(),
            target: AffordanceTarget::Action(Arc::clone(
                &entry.instance.actions[entry.behavior].name,
            )),
            binding: entry.binding,
            mapping: entry.mapping,
            query: entry.query,
            cancel: entry.cancel,
            result: entry.result.expect("terminal without result"),
            effect: ActionEffect::PossiblyApplied,
            cancel_requested: entry.cancel_requested,
            cancel_failure: entry.cancel_failure,
            abandoned: entry.abandoned,
            close_failure: entry.close_failure,
        })
    }
}
