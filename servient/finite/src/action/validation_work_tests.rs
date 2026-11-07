//! Focused captured-service invariants. The HTTP integration tests separately
//! exercise complete managed publication and structured validation over TCP.
use super::*;
use crate::{CapturedActionRoute, RawProducerInstance, RawProducerRegistryLimits};
use alloc::vec;
use clinkz_wot_core::{ThingSlotId, producer::ProducerInstanceAuthority};

#[derive(Clone, Copy)]
enum Change {
    Before,
    Input,
    Authorize,
    Immediate,
    Continuing,
    Output,
    None,
    Overflow,
}
struct Service {
    work: Cell<u64>,
    change: Change,
    inputs: Cell<usize>,
    outputs: Cell<usize>,
    authorized: Cell<usize>,
    invoked: Cell<usize>,
    polls: Cell<usize>,
    closes: Cell<usize>,
}
impl ActionValidator for Service {
    fn additional_validation_work(&self) -> u64 {
        self.work.get()
    }
    fn validate_input(&self, _: &InteractionInput) -> CoreResult<()> {
        self.inputs.set(self.inputs.get() + 1);
        if matches!(self.change, Change::Input) {
            self.work.set(20);
        }
        Ok(())
    }
    fn validate_output(&self, _: &InteractionOutput) -> CoreResult<()> {
        self.outputs.set(self.outputs.get() + 1);
        if matches!(self.change, Change::Output) {
            self.work.set(20);
        }
        Ok(())
    }
}
struct App(Rc<Service>);
impl RawActionHandler for App {
    fn authorize(&self, _: Operation, _: &InteractionInput) -> CoreResult<()> {
        self.0.authorized.set(self.0.authorized.get() + 1);
        if matches!(self.0.change, Change::Authorize) {
            self.0.work.set(20);
        }
        Ok(())
    }
    fn invoke(
        &self,
        _: clinkz_wot_core::HandlerContext<'_>,
        _: &InteractionInput,
    ) -> CoreResult<RawActionStart> {
        self.0.invoked.set(self.0.invoked.get() + 1);
        if matches!(self.0.change, Change::Immediate | Change::Output) {
            if matches!(self.0.change, Change::Immediate) {
                self.0.work.set(20);
            }
            return Ok(RawActionStart::Complete(InteractionOutput::empty()));
        }
        Ok(RawActionStart::Continue(Box::new(Work(self.0.clone()))))
    }
}
struct Work(Rc<Service>);
impl RawActionWork for Work {
    fn poll(&mut self) -> Poll<CoreResult<InteractionOutput>> {
        self.0.polls.set(self.0.polls.get() + 1);
        if matches!(self.0.change, Change::Continuing) {
            self.0.work.set(20);
        }
        Poll::Ready(Ok(InteractionOutput::empty()))
    }
    fn request_cancel(&mut self) -> CoreResult<()> {
        Ok(())
    }
    fn poll_close(&mut self) -> Poll<CoreResult<()>> {
        self.0.closes.set(self.0.closes.get() + 1);
        Poll::Ready(Ok(()))
    }
    fn abort_local(self: Box<Self>) {}
}
fn fixture(change: Change) -> (RawProducerServient, Rc<Service>, ProducerInstanceKey) {
    let service = Rc::new(Service {
        work: Cell::new(if matches!(change, Change::Overflow) {
            u64::MAX
        } else {
            10
        }),
        change,
        inputs: Cell::new(0),
        outputs: Cell::new(0),
        authorized: Cell::new(0),
        invoked: Cell::new(0),
        polls: Cell::new(0),
        closes: Cell::new(0),
    });
    let authority = ProducerInstanceAuthority::try_new(ThingSlotId::new(
        SlotIndex::new(1),
        Generation::INITIAL,
    ))
    .unwrap();
    let key = authority.key();
    let instance =
        RawProducerInstance::new(authority, Vec::new()).with_actions(vec![RawActionBehavior::new(
            Arc::from("run"),
            App(service.clone()),
            size_of::<Work>(),
            false,
            false,
        )]);
    let mut owner = RawProducerServient::try_new(
        vec![instance],
        RawProducerRegistryLimits {
            instances: 1,
            behaviors_per_instance: 1,
            target_name_bytes: 16,
        },
    )
    .ok()
    .unwrap();
    owner
        .enable_raw_actions(RawActionLimits {
            invocations: 1,
            application_work_bytes: size_of::<Work>(),
            input_fields: 0,
            input_bytes: 0,
            principal_id_bytes: 0,
            output_body_bytes: 0,
            output_media_capacity: 0,
            output_retained_bytes: 1024,
        })
        .unwrap();
    // Supply the original captured service directly to this unit fixture;
    // it makes no TD or native-publication claim.
    owner.instances[0].action_routes = Some(vec![CapturedActionRoute {
        target: AffordanceTarget::Action(Arc::from("run")),
        operation: Operation::InvokeAction,
        route: route(),
        validator: Some(service.clone()),
        validation_work: service.work.get(),
    }]);
    (owner, service, key)
}
fn route() -> ActionRoute {
    ActionRoute {
        binding: (BindingId::new(1), BindingGeneration::INITIAL),
        mapping: ActionMappingKey(1),
    }
}
fn invoke(
    owner: &mut RawProducerServient,
    key: ProducerInstanceKey,
) -> CoreResult<RawActionHandle> {
    owner.invoke_raw_action(
        RawActionRequest {
            instance: key,
            target: &AffordanceTarget::Action(Arc::from("run")),
            invoke: route(),
            query: None,
            cancel: None,
            input: &InteractionInput::empty(),
        },
        &mut WorkBudget::new().with_remaining(WorkClass::HandlerSteps, 10_000),
    )
}

#[test]
fn captured_work_cannot_refresh_across_validation_authorization_and_completion_callbacks() {
    for change in [
        Change::Before,
        Change::Input,
        Change::Authorize,
        Change::Immediate,
        Change::Continuing,
        Change::Output,
        Change::Overflow,
    ] {
        let (mut owner, service, key) = fixture(change);
        if matches!(change, Change::Before) {
            service.work.set(20);
        }
        let result = invoke(&mut owner, key);
        if matches!(
            change,
            Change::Before | Change::Input | Change::Authorize | Change::Overflow
        ) {
            assert!(result.is_err());
            assert_eq!(service.invoked.get(), 0);
            if matches!(change, Change::Before | Change::Overflow) {
                assert_eq!(service.inputs.get(), 0);
            }
            if !matches!(change, Change::Authorize) {
                assert_eq!(service.authorized.get(), 0);
            }
        } else {
            let handle = result.unwrap();
            if matches!(change, Change::Continuing) {
                assert_eq!(
                    owner
                        .step_raw_action(
                            &handle,
                            &mut WorkBudget::new().with_remaining(WorkClass::HandlerSteps, 10_000)
                        )
                        .unwrap(),
                    Poll::Ready(())
                );
            }
            // Restoring a declaration cannot reopen this original terminal failure.
            service.work.set(10);
            let settled = owner.take_raw_action_settlement(&handle).unwrap().unwrap();
            assert!(matches!(settled.result(), Err(CoreError::Validation(_))));
            assert_eq!(settled.effect, ActionEffect::PossiblyApplied);
            assert_eq!(service.invoked.get(), 1);
        }
        assert_eq!(
            service.outputs.get(),
            usize::from(matches!(change, Change::Output)),
            "a changed declaration must not enter an unpaid output validator"
        );
    }
}

#[test]
fn validator_drift_cannot_make_original_cleanup_require_handler_or_codec_work() {
    let (mut owner, service, key) = fixture(Change::None);
    let handle = invoke(&mut owner, key).unwrap();
    service.work.set(u64::MAX);
    owner.close_raw_action(&handle).unwrap();
    assert_eq!(
        owner
            .step_raw_action(
                &handle,
                &mut WorkBudget::new().with_remaining(WorkClass::CleanupItems, 10)
            )
            .unwrap(),
        Poll::Ready(())
    );
    assert_eq!(service.polls.get(), 0);
    assert_eq!(service.outputs.get(), 0);
    assert_eq!(service.closes.get(), 1);
    let settled = owner.take_raw_action_settlement(&handle).unwrap().unwrap();
    assert!(matches!(settled.result(), Err(CoreError::Cancelled(_))));
    assert_eq!(settled.effect, ActionEffect::PossiblyApplied);
}
