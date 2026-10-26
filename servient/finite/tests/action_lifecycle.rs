//! Deterministic source-free Engine semantics; no native transport or TD
//! publication is inferred from these in-memory Binding observations.
use clinkz_wot_core::{
    AcceptHint, ActionInvocationRef, AffordanceTarget, BindingGeneration, BindingId, CoreError,
    CoreResult, ErrorContext, ErrorPhase, HandlerContext, InteractionInput, InteractionOutput,
    InteractionStatus, Operation, Payload, Principal, PrincipalId, RetryClass, ThingSlotId,
    action::{
        ActionMappingKey, ActionRoute, RawActionHandler, RawActionRequest, RawActionStart,
        RawActionWork,
    },
    finite::RawFiniteHandler,
    producer::{ProducerInstanceAuthority, ProducerInstanceKey},
};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_servient_finite::{
    RawActionBehavior, RawActionLimits, RawActionState, RawFiniteBehavior, RawProducerInstance,
    RawProducerRegistryLimits, RawProducerServient,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    task::Poll,
};

fn key(slot: u32) -> ProducerInstanceAuthority {
    ProducerInstanceAuthority::try_new(ThingSlotId::new(SlotIndex::new(slot), Generation::INITIAL))
        .unwrap()
}
fn route(binding: u32, mapping: u64) -> ActionRoute {
    ActionRoute {
        binding: (BindingId::new(binding), BindingGeneration::INITIAL),
        mapping: ActionMappingKey(mapping),
    }
}
fn caller(who: &str) -> InteractionInput {
    InteractionInput {
        principal: Some(Principal {
            id: PrincipalId::from(who),
            scopes: vec!["allow".into()],
        }),
        ..InteractionInput::empty()
    }
}
fn request<'a>(
    key: ProducerInstanceKey,
    target: &'a AffordanceTarget,
    input: &'a InteractionInput,
) -> RawActionRequest<'a> {
    RawActionRequest {
        instance: key,
        target,
        input,
        invoke: route(1, 11),
        query: Some(route(2, 22)),
        cancel: Some(route(3, 33)),
    }
}
fn work_budget() -> WorkBudget {
    WorkBudget::new()
        .with_remaining(WorkClass::HandlerSteps, 1_000_000)
        .with_remaining(WorkClass::CleanupItems, 100)
}
fn limits() -> RawActionLimits {
    RawActionLimits {
        invocations: 1,
        application_work_bytes: 64,
        input_fields: 8,
        input_bytes: 256,
        principal_id_bytes: 32,
        output_body_bytes: 64,
        output_media_capacity: 64,
        output_retained_bytes: 2048,
    }
}
fn output(bytes: &[u8]) -> InteractionOutput {
    InteractionOutput::with_data(Payload::new(bytes.to_vec(), "application/octet-stream"))
}

#[derive(Default)]
struct Trace {
    started: usize,
    polls: usize,
    cancels: usize,
    closes: usize,
    local_aborts: usize,
    dropped: usize,
    operations: Vec<Operation>,
    instances: Vec<ProducerInstanceKey>,
}
struct App {
    trace: Rc<RefCell<Trace>>,
    ready: Rc<Cell<bool>>,
    complete_now: bool,
    cancel_fails: bool,
    bad_output: bool,
    close_attempts: Rc<Cell<u8>>,
}
impl RawActionHandler for App {
    fn authorize(&self, operation: Operation, input: &InteractionInput) -> CoreResult<()> {
        if !input
            .principal
            .as_ref()
            .is_some_and(|p| p.scopes.iter().any(|s| s == "allow"))
        {
            return Err(CoreError::Security {
                reason: clinkz_wot_core::SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Selection, RetryClass::Never),
            });
        }
        self.trace.borrow_mut().operations.push(operation);
        Ok(())
    }
    fn invoke(
        &self,
        context: HandlerContext<'_>,
        _: &InteractionInput,
    ) -> CoreResult<RawActionStart> {
        assert_eq!(context.operation(), Operation::InvokeAction);
        let mut trace = self.trace.borrow_mut();
        trace.started += 1;
        trace.instances.push(context.producer_instance().unwrap());
        drop(trace);
        if self.complete_now {
            return Ok(RawActionStart::Complete(output(b"now")));
        }
        Ok(RawActionStart::Continue(Box::new(Work {
            trace: Rc::clone(&self.trace),
            ready: Rc::clone(&self.ready),
            cancel_fails: self.cancel_fails,
            bad_output: self.bad_output,
            close_attempts: Rc::clone(&self.close_attempts),
        })))
    }
}
struct Work {
    trace: Rc<RefCell<Trace>>,
    ready: Rc<Cell<bool>>,
    cancel_fails: bool,
    bad_output: bool,
    close_attempts: Rc<Cell<u8>>,
}
impl RawActionWork for Work {
    fn poll(&mut self) -> Poll<CoreResult<InteractionOutput>> {
        self.trace.borrow_mut().polls += 1;
        if !self.ready.get() {
            return Poll::Pending;
        }
        let result = if self.bad_output {
            output(b"bad").with_status(InteractionStatus::Created)
        } else {
            output(b"late")
        };
        Poll::Ready(Ok(result))
    }
    fn request_cancel(&mut self) -> CoreResult<()> {
        self.trace.borrow_mut().cancels += 1;
        if self.cancel_fails {
            Err(CoreError::Cancelled(ErrorContext::new(
                ErrorPhase::Handler,
                RetryClass::Never,
            )))
        } else {
            Ok(())
        }
    }
    fn poll_close(&mut self) -> Poll<CoreResult<()>> {
        self.trace.borrow_mut().closes += 1;
        let attempt = self.close_attempts.get();
        self.close_attempts.set(attempt + 1);
        match attempt {
            0 => Poll::Pending,
            1 => Poll::Ready(Err(CoreError::Cleanup(ErrorContext::new(
                ErrorPhase::Cleanup,
                RetryClass::Never,
            )))),
            _ => Poll::Ready(Ok(())),
        }
    }
    fn abort_local(self: Box<Self>) {
        self.trace.borrow_mut().local_aborts += 1;
        // No waiting, native I/O or escaped work. Destroy the owned local
        // continuation after recording its finite abort.
    }
}
impl Drop for Work {
    fn drop(&mut self) {
        self.trace.borrow_mut().dropped += 1;
    }
}

struct Property;
impl RawFiniteHandler for Property {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn handle(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(output(b"property"))
    }
}
fn instance(
    authority: ProducerInstanceAuthority,
    trace: Rc<RefCell<Trace>>,
    ready: Rc<Cell<bool>>,
    complete_now: bool,
    cancel_fails: bool,
    bad_output: bool,
    close_attempts: Rc<Cell<u8>>,
) -> RawProducerInstance {
    // The finite and Action behaviors coexist on ONE logical authority.
    RawProducerInstance::new(
        authority,
        vec![RawFiniteBehavior::new(
            AffordanceTarget::Property(Arc::from("value")),
            Operation::ReadProperty,
            Property,
        )],
    )
    .with_actions(vec![RawActionBehavior::new(
        Arc::from("run"),
        App {
            trace,
            ready,
            complete_now,
            cancel_fails,
            bad_output,
            close_attempts,
        },
        64,
        true,
        true,
    )])
}
fn owner(instance: RawProducerInstance) -> RawProducerServient {
    let mut owner = RawProducerServient::try_new(
        vec![instance],
        RawProducerRegistryLimits {
            instances: 2,
            behaviors_per_instance: 4,
            target_name_bytes: 16,
        },
    )
    .ok()
    .unwrap();
    owner.enable_raw_actions(limits()).unwrap();
    owner
}

#[test]
fn action_outlives_initiating_exchange_and_replacement_without_retargeting() {
    let old = key(1);
    let old_key = old.key();
    let trace = Rc::new(RefCell::new(Trace::default()));
    let ready = Rc::new(Cell::new(false));
    let close = Rc::new(Cell::new(0));
    let mut owner = owner(instance(
        old,
        Rc::clone(&trace),
        Rc::clone(&ready),
        false,
        false,
        false,
        Rc::clone(&close),
    ));
    let target = AffordanceTarget::Action(Arc::from("run"));
    let input = caller("alice");
    let mut budget = work_budget();
    let handle = owner
        .invoke_raw_action(request(old_key, &target, &input), &mut budget)
        .unwrap();
    let id = handle.reference();
    // A Binding could dispose of its initiating exchange here. The Engine
    // holds no reply, wire correlation or native continuation resource.
    assert!(matches!(
        owner
            .query_raw_action(
                &handle,
                route(2, 22).binding,
                ActionMappingKey(22),
                &input,
                &mut budget
            )
            .unwrap()
            .state,
        RawActionState::Running
    ));
    assert!(
        owner
            .cancel_raw_action(
                &handle,
                route(3, 33).binding,
                ActionMappingKey(33),
                &input,
                &mut budget
            )
            .unwrap()
    );
    assert!(matches!(
        owner.step_raw_action(&handle, &mut budget),
        Ok(Poll::Pending)
    ));
    assert_eq!(trace.borrow().cancels, 1);
    assert_eq!(trace.borrow().polls, 1);

    let successor = key(1);
    let successor_key = successor.key();
    owner
        .replace(
            old_key,
            instance(
                successor,
                Rc::clone(&trace),
                Rc::clone(&ready),
                true,
                false,
                false,
                close,
            ),
        )
        .ok()
        .unwrap();
    assert!(matches!(
        owner.query_raw_action(
            &handle,
            route(2, 22).binding,
            ActionMappingKey(22),
            &input,
            &mut budget
        ),
        Err(CoreError::StaleHandle(_))
    ));
    assert!(matches!(
        owner.invoke_raw_action(request(old_key, &target, &input), &mut budget),
        Err(CoreError::StaleHandle(_))
    ));
    // The original slot is still occupied despite replacement and settlement
    // of the initiating exchange; no implicit replay on capacity exhaustion.
    assert!(matches!(
        owner.invoke_raw_action(request(successor_key, &target, &input), &mut budget),
        Err(CoreError::Backpressure(_))
    ));
    ready.set(true);
    assert!(matches!(
        owner.step_raw_action(&handle, &mut budget),
        Ok(Poll::Ready(()))
    ));
    owner.abandon_raw_action(&handle).unwrap();
    let late = owner.take_raw_action_settlement(&handle).unwrap().unwrap();
    assert_eq!(late.reference, id);
    assert_eq!(late.instance, old_key);
    assert_eq!(late.binding, route(1, 11).binding);
    assert_eq!(late.mapping, ActionMappingKey(11));
    assert_eq!(
        late.effect,
        clinkz_wot_core::action::ActionEffect::PossiblyApplied
    );
    assert_eq!(late.query, Some(route(2, 22)));
    assert_eq!(late.cancel, Some(route(3, 33)));
    assert_eq!(late.result.unwrap().data().unwrap().body.as_ref(), b"late");
    assert!(late.cancel_requested);
    assert!(late.abandoned);
    assert!(matches!(
        owner.take_raw_action_settlement(&handle),
        Err(CoreError::StaleHandle(_))
    ));
    let next = owner
        .invoke_raw_action(request(successor_key, &target, &input), &mut budget)
        .unwrap();
    assert_ne!(next.reference(), id);
    assert_eq!(
        owner
            .take_raw_action_settlement(&next)
            .unwrap()
            .unwrap()
            .instance,
        successor_key
    );
    assert_eq!(trace.borrow().instances, vec![old_key, successor_key]);
}

#[test]
fn query_cancel_need_their_own_mapping_principal_and_fresh_policy() {
    let authority = key(2);
    let actual = authority.key();
    let trace = Rc::new(RefCell::new(Trace::default()));
    let mut owner = owner(instance(
        authority,
        Rc::clone(&trace),
        Rc::new(Cell::new(false)),
        false,
        true,
        false,
        Rc::new(Cell::new(0)),
    ));
    assert!(trace.borrow().instances.is_empty());
    let target = AffordanceTarget::Action(Arc::from("run"));
    let input = caller("alice");
    let mut budget = work_budget();
    let handle = owner
        .invoke_raw_action(request(actual, &target, &input), &mut budget)
        .unwrap();
    assert!(matches!(
        owner.query_raw_action(
            &handle,
            route(1, 11).binding,
            ActionMappingKey(11),
            &input,
            &mut budget
        ),
        Err(CoreError::Security { .. })
    ));
    assert!(matches!(
        owner.cancel_raw_action(
            &handle,
            route(3, 33).binding,
            ActionMappingKey(99),
            &input,
            &mut budget
        ),
        Err(CoreError::Security { .. })
    ));
    assert!(matches!(
        owner.query_raw_action(
            &handle,
            route(2, 22).binding,
            ActionMappingKey(22),
            &caller("bob"),
            &mut budget
        ),
        Err(CoreError::Security { .. })
    ));
    let mut denied = caller("alice");
    denied.principal.as_mut().unwrap().scopes.clear();
    assert!(matches!(
        owner.cancel_raw_action(
            &handle,
            route(3, 33).binding,
            ActionMappingKey(33),
            &denied,
            &mut budget
        ),
        Err(CoreError::Security { .. })
    ));
    assert!(
        owner
            .cancel_raw_action(
                &handle,
                route(3, 33).binding,
                ActionMappingKey(33),
                &input,
                &mut budget
            )
            .unwrap()
    );
    assert!(matches!(
        owner.step_raw_action(&handle, &mut budget),
        Ok(Poll::Pending)
    ));
    assert!(
        owner
            .query_raw_action(
                &handle,
                route(2, 22).binding,
                ActionMappingKey(22),
                &input,
                &mut budget
            )
            .unwrap()
            .cancel_failure
            .is_some()
    );
    owner.abandon_raw_action(&handle).unwrap();
    assert!(matches!(
        owner.query_raw_action(
            &handle,
            route(2, 22).binding,
            ActionMappingKey(22),
            &input,
            &mut budget
        ),
        Err(CoreError::Security { .. })
    ));
    assert_eq!(trace.borrow().started, 1);
    assert_eq!(trace.borrow().cancels, 1);
    assert_eq!(
        trace.borrow().operations,
        vec![
            Operation::InvokeAction,
            Operation::CancelAction,
            Operation::QueryAction
        ]
    );
}

#[test]
fn saturated_and_failed_admission_never_enter_behavior_and_close_retains_owner() {
    let authority = key(4);
    let instance_key = authority.key();
    let trace = Rc::new(RefCell::new(Trace::default()));
    let close_attempts = Rc::new(Cell::new(0));
    let mut owner = owner(instance(
        authority,
        Rc::clone(&trace),
        Rc::new(Cell::new(false)),
        false,
        false,
        false,
        Rc::clone(&close_attempts),
    ));
    let target = AffordanceTarget::Action(Arc::from("run"));
    let input = caller("alice");
    let mut budget = work_budget();
    let mut unfunded = WorkBudget::new();
    assert!(matches!(
        owner.invoke_raw_action(request(instance_key, &target, &input), &mut unfunded),
        Err(CoreError::Backpressure(_))
    ));
    let mut oversized = caller("alice");
    oversized.data = Some(Payload::new(vec![0; 512], "application/json"));
    assert!(matches!(
        owner.invoke_raw_action(request(instance_key, &target, &oversized), &mut budget),
        Err(CoreError::Backpressure(_))
    ));
    let mut denied = caller("alice");
    denied.principal.as_mut().unwrap().scopes.clear();
    assert!(matches!(
        owner.invoke_raw_action(request(instance_key, &target, &denied), &mut budget),
        Err(CoreError::Security { .. })
    ));
    assert_eq!(trace.borrow().started, 0);
    let handle = owner
        .invoke_raw_action(request(instance_key, &target, &input), &mut budget)
        .unwrap();
    assert!(matches!(
        owner.invoke_raw_action(request(instance_key, &target, &input), &mut budget),
        Err(CoreError::Backpressure(_))
    ));
    owner.close_raw_action(&handle).unwrap();
    for attempt in 0..2 {
        let step = owner.step_raw_action(&handle, &mut budget);
        if attempt == 0 {
            assert!(matches!(step, Ok(Poll::Pending)));
        } else {
            assert!(matches!(step, Err(CoreError::Cleanup(_))));
        }
        assert!(owner.take_raw_action_settlement(&handle).unwrap().is_none());
        assert!(matches!(
            owner.invoke_raw_action(request(instance_key, &target, &input), &mut budget),
            Err(CoreError::Backpressure(_))
        ));
        assert_eq!(trace.borrow().dropped, 0);
    }
    assert!(matches!(
        owner.step_raw_action(&handle, &mut budget),
        Ok(Poll::Ready(()))
    ));
    assert_eq!(trace.borrow().dropped, 1);
    let settlement = owner.take_raw_action_settlement(&handle).unwrap().unwrap();
    assert!(settlement.abandoned);
    assert!(matches!(settlement.result, Err(CoreError::Cancelled(_))));
    assert!(matches!(
        settlement.close_failure,
        Some(CoreError::Cleanup(_))
    ));
    let next = owner
        .invoke_raw_action(request(instance_key, &target, &input), &mut budget)
        .unwrap();
    assert_ne!(next.reference(), handle.reference());
    assert_ne!(
        next.reference(),
        ActionInvocationRef::new(SlotIndex::new(0), Generation::INITIAL)
    );
}

#[test]
fn invalid_late_result_is_retained_and_cannot_be_reported_as_success() {
    let authority = key(5);
    let instance_key = authority.key();
    let trace = Rc::new(RefCell::new(Trace::default()));
    let mut owner = owner(instance(
        authority,
        trace,
        Rc::new(Cell::new(true)),
        false,
        false,
        true,
        Rc::new(Cell::new(0)),
    ));
    let target = AffordanceTarget::Action(Arc::from("run"));
    let input = caller("alice");
    let mut budget = work_budget();
    let handle = owner
        .invoke_raw_action(request(instance_key, &target, &input), &mut budget)
        .unwrap();
    assert!(matches!(
        owner.step_raw_action(&handle, &mut budget),
        Ok(Poll::Ready(()))
    ));
    assert!(matches!(
        owner
            .query_raw_action(
                &handle,
                route(2, 22).binding,
                ActionMappingKey(22),
                &input,
                &mut budget
            )
            .unwrap()
            .state,
        RawActionState::Complete(Err(CoreError::Validation(_)))
    ));
    assert!(matches!(
        owner
            .take_raw_action_settlement(&handle)
            .unwrap()
            .unwrap()
            .result,
        Err(CoreError::Validation(_))
    ));
}

#[test]
fn action_only_instance_needs_no_finite_runtime_and_unpublished_controls_are_unsupported() {
    let authority = key(6);
    let instance_key = authority.key();
    let trace = Rc::new(RefCell::new(Trace::default()));
    let ready = Rc::new(Cell::new(false));
    let app = App {
        trace: Rc::clone(&trace),
        ready,
        complete_now: true,
        cancel_fails: false,
        bad_output: false,
        close_attempts: Rc::new(Cell::new(0)),
    };
    let only_action =
        RawProducerInstance::new(authority, vec![]).with_actions(vec![RawActionBehavior::new(
            Arc::from("run"),
            app,
            8,
            false,
            false,
        )]);
    let mut owner = owner(only_action);
    let target = AffordanceTarget::Action(Arc::from("run"));
    let input = caller("alice");
    let mut budget = work_budget();
    assert!(matches!(
        owner.invoke_raw_action(request(instance_key, &target, &input), &mut budget),
        Err(CoreError::UnsupportedOperation(_))
    ));
    let handle = owner
        .invoke_raw_action(
            RawActionRequest {
                instance: instance_key,
                target: &target,
                input: &input,
                invoke: route(8, 800),
                query: None,
                cancel: None,
            },
            &mut budget,
        )
        .unwrap();
    assert!(matches!(
        owner.query_raw_action(
            &handle,
            route(2, 22).binding,
            ActionMappingKey(22),
            &input,
            &mut budget
        ),
        Err(CoreError::UnsupportedOperation(_))
    ));
    assert!(matches!(
        owner.cancel_raw_action(
            &handle,
            route(3, 33).binding,
            ActionMappingKey(33),
            &input,
            &mut budget
        ),
        Err(CoreError::UnsupportedOperation(_))
    ));
    owner.close_raw_action(&handle).unwrap();
    let result = owner.take_raw_action_settlement(&handle).unwrap().unwrap();
    assert!(!result.cancel_requested); // Completed before close: no request was queued.
    assert_eq!(result.binding, route(8, 800).binding);
    assert_eq!(result.result.unwrap().data().unwrap().body.as_ref(), b"now");
    assert_eq!(trace.borrow().operations, vec![Operation::InvokeAction]);
}

#[test]
fn stalled_invocation_and_foreign_same_number_handle_do_not_block_or_alias_sibling() {
    let a = key(7);
    let b = key(8);
    let a_key = a.key();
    let b_key = b.key();
    let trace = Rc::new(RefCell::new(Trace::default()));
    let a_ready = Rc::new(Cell::new(false));
    let b_ready = Rc::new(Cell::new(true));
    let mut owner = RawProducerServient::try_new(
        vec![
            instance(
                a,
                Rc::clone(&trace),
                a_ready,
                false,
                false,
                false,
                Rc::new(Cell::new(0)),
            ),
            instance(
                b,
                Rc::clone(&trace),
                b_ready,
                false,
                false,
                false,
                Rc::new(Cell::new(0)),
            ),
        ],
        RawProducerRegistryLimits {
            instances: 2,
            behaviors_per_instance: 4,
            target_name_bytes: 16,
        },
    )
    .ok()
    .unwrap();
    owner
        .enable_raw_actions(RawActionLimits {
            invocations: 2,
            application_work_bytes: 128,
            ..limits()
        })
        .unwrap();
    let target = AffordanceTarget::Action(Arc::from("run"));
    let mut budget = work_budget();
    let a_handle = {
        // The initiating input is destroyed immediately after acceptance.
        let originating_input = caller("alice");
        owner
            .invoke_raw_action(request(a_key, &target, &originating_input), &mut budget)
            .unwrap()
    };
    let b_handle = owner
        .invoke_raw_action(request(b_key, &target, &caller("bob")), &mut budget)
        .unwrap();
    assert!(matches!(
        owner.step_raw_action(&a_handle, &mut budget),
        Ok(Poll::Pending)
    ));
    assert!(matches!(
        owner.step_raw_action(&b_handle, &mut budget),
        Ok(Poll::Ready(()))
    ));
    let b_result = owner
        .take_raw_action_settlement(&b_handle)
        .unwrap()
        .unwrap();
    assert_eq!(b_result.instance, b_key);
    assert_eq!(
        b_result.result.unwrap().data().unwrap().body.as_ref(),
        b"late"
    );

    let foreign = key(9);
    let foreign_key = foreign.key();
    let mut other = self::owner(instance(
        foreign,
        trace,
        Rc::new(Cell::new(false)),
        true,
        false,
        false,
        Rc::new(Cell::new(0)),
    ));
    let foreign_handle = other
        .invoke_raw_action(request(foreign_key, &target, &caller("alice")), &mut budget)
        .unwrap();
    assert_eq!(foreign_handle.reference(), a_handle.reference());
    assert!(matches!(
        other.take_raw_action_settlement(&a_handle),
        Err(CoreError::StaleHandle(_))
    ));
    owner.close_raw_action(&a_handle).unwrap();
    let mut no_cleanup_credit = WorkBudget::new();
    assert!(matches!(
        owner.step_raw_action(&a_handle, &mut no_cleanup_credit),
        Ok(Poll::Pending)
    ));
    assert!(
        owner
            .take_raw_action_settlement(&a_handle)
            .unwrap()
            .is_none()
    );
}

#[test]
fn lost_handles_do_not_orphan_owner_level_retirement_or_terminal_results() {
    let a = key(20);
    let b = key(21);
    let a_key = a.key();
    let b_key = b.key();
    let trace = Rc::new(RefCell::new(Trace::default()));
    let mut owner = RawProducerServient::try_new(
        vec![
            instance(
                a,
                Rc::clone(&trace),
                Rc::new(Cell::new(false)),
                false,
                false,
                false,
                Rc::new(Cell::new(0)),
            ),
            instance(
                b,
                Rc::clone(&trace),
                Rc::new(Cell::new(true)),
                true,
                false,
                false,
                Rc::new(Cell::new(0)),
            ),
        ],
        RawProducerRegistryLimits {
            instances: 2,
            behaviors_per_instance: 4,
            target_name_bytes: 16,
        },
    )
    .ok()
    .unwrap();
    owner
        .enable_raw_actions(RawActionLimits {
            invocations: 2,
            application_work_bytes: 128,
            ..limits()
        })
        .unwrap();
    let target = AffordanceTarget::Action(Arc::from("run"));
    let input = caller("alice");
    let mut budget = work_budget();
    let a_handle = owner
        .invoke_raw_action(request(a_key, &target, &input), &mut budget)
        .unwrap();
    let a_ref = a_handle.reference();
    let b_handle = owner
        .invoke_raw_action(request(b_key, &target, &input), &mut budget)
        .unwrap();
    let b_ref = b_handle.reference();
    drop(a_handle);
    drop(b_handle);

    assert!(matches!(
        owner.step_raw_action_retirement(&mut budget),
        Err(CoreError::Lifecycle(_))
    ));
    owner.begin_raw_action_retirement().unwrap();
    assert_eq!(trace.borrow().closes, 0);
    assert!(matches!(
        owner.invoke_raw_action(request(a_key, &target, &input), &mut budget),
        Err(CoreError::StaleHandle(_))
    ));
    assert!(matches!(
        owner.step_raw_action_retirement(&mut WorkBudget::new()),
        Ok(Poll::Pending)
    ));
    assert_eq!(trace.borrow().closes, 0);
    let Poll::Ready(Some(completed)) = owner.step_raw_action_retirement(&mut budget).unwrap()
    else {
        panic!("terminal result must be returned before pending cleanup");
    };
    assert_eq!(completed.reference, b_ref);
    assert_eq!(completed.instance, b_key);
    assert_eq!(
        completed.result.unwrap().data().unwrap().body.as_ref(),
        b"now"
    );
    assert!(matches!(
        owner.step_raw_action_retirement(&mut budget),
        Ok(Poll::Pending)
    ));
    assert!(matches!(
        owner.step_raw_action_retirement(&mut budget),
        Err(CoreError::Cleanup(_))
    ));
    let Poll::Ready(Some(cleaned)) = owner.step_raw_action_retirement(&mut budget).unwrap() else {
        panic!("failed cleanup still owns the original invocation");
    };
    assert_eq!(cleaned.reference, a_ref);
    assert_eq!(cleaned.instance, a_key);
    assert!(matches!(cleaned.result, Err(CoreError::Cancelled(_))));
    assert!(matches!(cleaned.close_failure, Some(CoreError::Cleanup(_))));
    assert!(cleaned.abandoned);
    assert!(matches!(
        owner.step_raw_action_retirement(&mut WorkBudget::new()),
        Ok(Poll::Ready(None))
    ));
    drop(owner);
    assert_eq!(trace.borrow().local_aborts, 0);
    assert_eq!(trace.borrow().dropped, 1);
}

#[test]
fn stalled_cleanup_does_not_block_sibling_and_root_drop_locally_aborts_the_rest() {
    let a = key(22);
    let b = key(23);
    let a_key = a.key();
    let b_key = b.key();
    let trace = Rc::new(RefCell::new(Trace::default()));
    let mut owner = RawProducerServient::try_new(
        vec![
            instance(
                a,
                Rc::clone(&trace),
                Rc::new(Cell::new(false)),
                false,
                false,
                false,
                Rc::new(Cell::new(0)),
            ),
            instance(
                b,
                Rc::clone(&trace),
                Rc::new(Cell::new(false)),
                false,
                false,
                false,
                Rc::new(Cell::new(2)),
            ),
        ],
        RawProducerRegistryLimits {
            instances: 2,
            behaviors_per_instance: 4,
            target_name_bytes: 16,
        },
    )
    .ok()
    .unwrap();
    owner
        .enable_raw_actions(RawActionLimits {
            invocations: 2,
            application_work_bytes: 128,
            ..limits()
        })
        .unwrap();
    let target = AffordanceTarget::Action(Arc::from("run"));
    let mut budget = work_budget();
    for instance_key in [a_key, b_key] {
        let handle = owner
            .invoke_raw_action(
                request(instance_key, &target, &caller("alice")),
                &mut budget,
            )
            .unwrap();
        drop(handle);
    }
    owner.begin_raw_action_retirement().unwrap();
    assert!(matches!(
        owner.step_raw_action_retirement(&mut budget),
        Ok(Poll::Pending)
    ));
    let Poll::Ready(Some(sibling)) = owner.step_raw_action_retirement(&mut budget).unwrap() else {
        panic!("stalled cleanup must not block a sibling");
    };
    assert!(matches!(sibling.result, Err(CoreError::Cancelled(_))));
    assert_eq!(trace.borrow().dropped, 1);
    drop(owner);
    assert_eq!(trace.borrow().local_aborts, 1);
    assert_eq!(trace.borrow().dropped, 2);
}

#[test]
fn root_drop_without_retirement_uses_qualified_finite_local_abort() {
    let authority = key(24);
    let key = authority.key();
    let trace = Rc::new(RefCell::new(Trace::default()));
    let mut owner = owner(instance(
        authority,
        Rc::clone(&trace),
        Rc::new(Cell::new(false)),
        false,
        false,
        false,
        Rc::new(Cell::new(0)),
    ));
    let target = AffordanceTarget::Action(Arc::from("run"));
    let handle = owner
        .invoke_raw_action(request(key, &target, &caller("alice")), &mut work_budget())
        .unwrap();
    drop(handle);
    drop(owner);
    assert_eq!(trace.borrow().local_aborts, 1);
    assert_eq!(trace.borrow().closes, 0);
    assert_eq!(trace.borrow().dropped, 1);
}

#[test]
fn cancel_then_close_sends_application_request_before_immediate_local_retirement() {
    let authority = key(25);
    let instance_key = authority.key();
    let trace = Rc::new(RefCell::new(Trace::default()));
    // A compliant work owner may complete local close on its first call.
    let mut owner = owner(instance(
        authority,
        Rc::clone(&trace),
        Rc::new(Cell::new(false)),
        false,
        false,
        false,
        Rc::new(Cell::new(2)),
    ));
    let target = AffordanceTarget::Action(Arc::from("run"));
    let input = caller("alice");
    let mut budget = work_budget();
    let handle = owner
        .invoke_raw_action(request(instance_key, &target, &input), &mut budget)
        .unwrap();
    assert!(
        owner
            .cancel_raw_action(
                &handle,
                route(3, 33).binding,
                ActionMappingKey(33),
                &input,
                &mut budget
            )
            .unwrap()
    );
    owner.close_raw_action(&handle).unwrap();
    assert_eq!(trace.borrow().cancels, 0);
    let mut one_credit = WorkBudget::new().with_remaining(WorkClass::CleanupItems, 2);
    assert!(matches!(
        owner.step_raw_action(&handle, &mut one_credit),
        Ok(Poll::Pending)
    ));
    assert_eq!(one_credit.remaining(WorkClass::CleanupItems), 1);
    assert_eq!(trace.borrow().cancels, 0);
    // The lookup is prepaid, and the two callbacks share a funded close step.
    let mut cleanup_only = WorkBudget::new().with_remaining(WorkClass::CleanupItems, 3);
    assert!(matches!(
        owner.step_raw_action(&handle, &mut cleanup_only),
        Ok(Poll::Ready(()))
    ));
    assert_eq!(cleanup_only.remaining(WorkClass::CleanupItems), 0);
    let settled = owner.take_raw_action_settlement(&handle).unwrap().unwrap();
    assert!(settled.cancel_requested);
    assert!(settled.cancel_failure.is_none());
    assert!(matches!(settled.result, Err(CoreError::Cancelled(_))));
    let trace = trace.borrow();
    assert_eq!(
        (trace.cancels, trace.closes, trace.polls, trace.dropped),
        (1, 1, 0, 1)
    );
}

#[test]
fn revoke_then_owner_retirement_keeps_cancel_and_close_failures_until_settlement() {
    let authority = key(26);
    let instance_key = authority.key();
    let trace = Rc::new(RefCell::new(Trace::default()));
    let mut owner = owner(instance(
        authority,
        Rc::clone(&trace),
        Rc::new(Cell::new(false)),
        false,
        true,
        false,
        Rc::new(Cell::new(0)),
    ));
    let target = AffordanceTarget::Action(Arc::from("run"));
    let mut budget = work_budget();
    let handle = owner
        .invoke_raw_action(
            request(instance_key, &target, &caller("alice")),
            &mut budget,
        )
        .unwrap();
    assert!(owner.revoke(instance_key));
    drop(handle);
    owner.begin_raw_action_retirement().unwrap();
    assert!(matches!(
        owner.step_raw_action_retirement(&mut WorkBudget::new()),
        Ok(Poll::Pending)
    ));
    assert_eq!(trace.borrow().cancels, 0);
    assert!(matches!(
        owner.step_raw_action_retirement(&mut budget),
        Ok(Poll::Pending)
    ));
    assert_eq!(trace.borrow().cancels, 1);
    assert!(matches!(
        owner.step_raw_action_retirement(&mut budget),
        Err(CoreError::Cleanup(_))
    ));
    assert_eq!(trace.borrow().dropped, 0);
    let Poll::Ready(Some(settled)) = owner.step_raw_action_retirement(&mut budget).unwrap() else {
        panic!("failed close must retain the original result until local cleanup completes");
    };
    assert!(settled.cancel_requested);
    assert!(matches!(
        settled.cancel_failure,
        Some(CoreError::Cancelled(_))
    ));
    assert!(matches!(settled.close_failure, Some(CoreError::Cleanup(_))));
    assert!(matches!(settled.result, Err(CoreError::Cancelled(_))));
    let trace = trace.borrow();
    assert_eq!(
        (trace.cancels, trace.closes, trace.polls, trace.dropped),
        (1, 3, 0, 1)
    );
}

#[test]
fn input_and_last_slot_lookup_are_prepaid_before_bounded_traversal() {
    let trace = Rc::new(RefCell::new(Trace::default()));
    // Fill the allowed 128 fields across all three traversed input collections.
    let mut dense = caller("alice");
    dense.uri_variables = (0..48)
        .map(|i| (format!("var{i}"), "value".into()))
        .collect();
    dense.principal.as_mut().unwrap().scopes = (0..48).map(|_| "allow".into()).collect();
    dense.accept = Some(
        AcceptHint::single("application/json")
            .with_alternatives((0..30).map(|_| "application/json".into())),
    );
    dense.data = Some(Payload::new(vec![1; 16], "application/octet-stream"));
    let mut invalid = dense.clone();
    invalid
        .accept
        .as_mut()
        .unwrap()
        .alternatives
        .as_mut()
        .unwrap()[29] = "x".repeat(8192).into();
    let target = AffordanceTarget::Action(Arc::from("run"));
    // A funded invalid input spends the declared envelope before rejecting its
    // last oversized alternative; a zero-credit path cannot do this traversal.
    let authority = key(28);
    let instance_key = authority.key();
    let mut owner = RawProducerServient::try_new(
        vec![instance(
            authority,
            Rc::clone(&trace),
            Rc::new(Cell::new(false)),
            false,
            false,
            false,
            Rc::new(Cell::new(0)),
        )],
        RawProducerRegistryLimits {
            instances: 2,
            behaviors_per_instance: 4,
            target_name_bytes: 16,
        },
    )
    .ok()
    .unwrap();
    owner
        .enable_raw_actions(RawActionLimits {
            invocations: 128,
            application_work_bytes: 128 * 64,
            input_fields: 128,
            input_bytes: 4096,
            ..limits()
        })
        .unwrap();
    let mut no_credit = WorkBudget::new();
    for _ in 0..8 {
        assert!(matches!(
            owner.invoke_raw_action(request(instance_key, &target, &invalid), &mut no_credit),
            Err(CoreError::Backpressure(_))
        ));
    }
    assert_eq!(trace.borrow().started, 0);
    let mut budget = work_budget();
    let before = budget.remaining(WorkClass::HandlerSteps);
    assert!(matches!(
        owner.invoke_raw_action(request(instance_key, &target, &invalid), &mut budget),
        Err(CoreError::Backpressure(_))
    ));
    assert!(budget.remaining(WorkClass::HandlerSteps) < before);
    assert_eq!(trace.borrow().started, 0);

    let mut handles = Vec::new();
    for _ in 0..128 {
        handles.push(
            owner
                .invoke_raw_action(request(instance_key, &target, &dense), &mut budget)
                .unwrap(),
        );
    }
    let last = handles.last().unwrap();
    for _ in 0..8 {
        assert!(matches!(
            owner.query_raw_action(
                last,
                route(2, 22).binding,
                ActionMappingKey(22),
                &invalid,
                &mut no_credit
            ),
            Err(CoreError::Backpressure(_))
        ));
        assert!(matches!(
            owner.cancel_raw_action(
                last,
                route(3, 33).binding,
                ActionMappingKey(33),
                &invalid,
                &mut no_credit
            ),
            Err(CoreError::Backpressure(_))
        ));
        assert!(matches!(
            owner.step_raw_action(last, &mut no_credit),
            Ok(Poll::Pending)
        ));
    }
    assert_eq!(trace.borrow().operations.len(), 128);
    assert_eq!(trace.borrow().polls, 0);
    let before = budget.remaining(WorkClass::HandlerSteps);
    assert!(matches!(
        owner.query_raw_action(
            last,
            route(2, 22).binding,
            ActionMappingKey(22),
            &invalid,
            &mut budget
        ),
        Err(CoreError::Backpressure(_))
    ));
    assert!(budget.remaining(WorkClass::HandlerSteps) < before);
    let mut lookup_only = WorkBudget::new().with_remaining(WorkClass::HandlerSteps, 128);
    assert!(matches!(
        owner.step_raw_action(last, &mut lookup_only),
        Ok(Poll::Pending)
    ));
    assert_eq!(lookup_only.remaining(WorkClass::HandlerSteps), 0);
    assert_eq!(trace.borrow().polls, 0);
    let mut underfunded = WorkBudget::new().with_remaining(WorkClass::HandlerSteps, 127);
    assert!(matches!(
        owner.step_raw_action(last, &mut underfunded),
        Ok(Poll::Pending)
    ));
    assert_eq!(underfunded.remaining(WorkClass::HandlerSteps), 127);
    assert!(matches!(
        owner.step_raw_action(last, &mut budget),
        Ok(Poll::Pending)
    ));
    assert_eq!(trace.borrow().polls, 1);
    owner.close_raw_action(last).unwrap();
    let mut cleanup_only = WorkBudget::new().with_remaining(WorkClass::CleanupItems, 130);
    assert!(matches!(
        owner.step_raw_action(last, &mut cleanup_only),
        Ok(Poll::Pending)
    ));
    assert_eq!(cleanup_only.remaining(WorkClass::CleanupItems), 0);
    assert_eq!(trace.borrow().cancels, 1);
    assert_eq!(trace.borrow().polls, 1);
}
