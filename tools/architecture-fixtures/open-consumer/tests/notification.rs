#![cfg(feature = "prepare")]
//! Public composed Event lifecycle evidence, without protocol interoperability claims.
use clinkz_wot_core::{consumer::*, host_accounts::HostResourceAccounts, notification::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::{
    consumer_notification::ConsumerNotificationRuntime, notification::*,
};
use std::{
    cell::Cell,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
};

const ENDPOINT: &str = "testevent://source/alarm";
#[derive(Default)]
struct Responses {
    sends: Cell<usize>,
    retire_polls: Cell<usize>,
    releases: Cell<usize>,
    pending_retirement: Cell<bool>,
    closes: Cell<usize>,
}
#[derive(Clone)]
struct EventBinding(Rc<Responses>);
struct Participation {
    responses: Rc<Responses>,
    recipient: NotificationIdentity,
    next: u8,
}
struct Response {
    responses: Rc<Responses>,
    sent: bool,
}
impl NotificationResponse for Response {
    fn poll_deliver(&mut self, _: &mut Context<'_>, _: &EventDataResponse) -> Poll<CoreResult<()>> {
        assert!(!self.sent);
        self.sent = true;
        self.responses.sends.set(self.responses.sends.get() + 1);
        Poll::Ready(Ok(()))
    }
    fn poll_retire(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<EventResponseDelivery>> {
        self.responses
            .retire_polls
            .set(self.responses.retire_polls.get() + 1);
        if self.responses.pending_retirement.get() {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(Ok(if self.sent {
                EventResponseDelivery::Delivered
            } else {
                EventResponseDelivery::NotAttempted
            }))
        }
    }
    fn abort_local(self) {}
}
impl Drop for Response {
    fn drop(&mut self) {
        self.responses
            .releases
            .set(self.responses.releases.get() + 1);
    }
}
impl ConsumerNotificationBinding for EventBinding {
    type Program = Rc<Responses>;
    type Configuration = u8;
    type Participation = Participation;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(90), BindingGeneration::INITIAL)
    }
    fn configuration(&self) -> &u8 {
        &1
    }
    fn support(
        &self,
        program: &Self::Program,
        kind: NotificationKind,
    ) -> CoreResult<NotificationSupport> {
        assert!(Rc::ptr_eq(&self.0, program));
        assert_eq!(kind, NotificationKind::Event);
        Ok(NotificationSupport {
            event_responses: true,
            cancellation: false,
        })
    }
    fn prepare(
        &mut self,
        program: Self::Program,
        recipient: &NotificationIdentity,
    ) -> Result<Participation, BindingInputRejection<Self::Program>> {
        self.support(&program, NotificationKind::Event).unwrap();
        Ok(Participation {
            responses: program,
            recipient: recipient.clone(),
            next: 1,
        })
    }
}
impl ConsumerNotificationPreparation for EventBinding {
    fn registration_identity(&self) -> BindingRegistrationIdentity {
        let (id, generation) = self.identity();
        BindingRegistrationIdentity::new(
            id,
            generation,
            BindingConfigurationDigest::new([90; 32]),
            BindingArtifactCompatibility::new(*b"event-response01"),
            0,
        )
    }
    fn preparation_bounds(&self) -> NotificationPreparationBounds {
        NotificationPreparationBounds {
            work: 32,
            temporary_bytes: 128,
            retained_bytes: 128,
        }
    }
    fn compile_notification(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<Self::Program>> {
        assert_eq!(input.target, &AffordanceTarget::Event(Arc::from("alarm")));
        assert_eq!(input.endpoint, ENDPOINT);
        assert!(input.schemas.subscription().is_none());
        assert!(input.schemas.cancellation().is_none());
        assert_eq!(
            input.schemas.data(),
            Some(value_type::TypeSchema::Type(value_type::ValueType::Integer))
        );
        assert_eq!(input.schemas.data_response(), input.schemas.data());
        Ok(Some(self.0.clone()))
    }
}
impl NotificationParticipation for Participation {
    type Response = Response;
    fn poll_start(
        &mut self,
        _: &mut Context<'_>,
        input: &SubscriptionInput,
    ) -> Poll<CoreResult<()>> {
        assert!(input.0.is_none());
        Poll::Ready(Ok(()))
    }
    fn poll_next(
        &mut self,
        _: &mut Context<'_>,
        max: usize,
    ) -> Poll<CoreResult<NotificationSignal<Response>>> {
        if self.next > 2 {
            return Poll::Pending;
        }
        let payload = Payload::new(self.next.to_string().as_bytes(), "application/json");
        assert!(notification_payload_backing(Some(&payload)).unwrap() <= max);
        self.next += 1;
        Poll::Ready(Ok(NotificationSignal::Data(NativeNotification {
            recipient: self.recipient.clone(),
            data: Some(payload),
            response: Some(Response {
                responses: self.responses.clone(),
                sent: false,
            }),
        })))
    }
    fn poll_close(
        &mut self,
        _: &mut Context<'_>,
        input: Option<&CancellationInput>,
    ) -> Poll<CoreResult<RemoteCancellation>> {
        assert!(input.is_none());
        self.responses.closes.set(self.responses.closes.get() + 1);
        Poll::Ready(Ok(RemoteCancellation::NotAttempted))
    }
    fn abort_local(self) {}
}
struct EventAccess;
impl ConsumerAccessService for EventAccess {
    fn authorize(
        &self,
        _: BindingRegistrationIdentity,
        endpoint: &str,
        op: Operation,
    ) -> CoreResult<()> {
        assert_eq!(endpoint, ENDPOINT);
        assert_eq!(op, Operation::SubscribeEvent);
        Ok(())
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        id: BindingRegistrationIdentity,
        endpoint: &str,
        op: Operation,
    ) -> CoreResult<()> {
        self.authorize(id, endpoint, op)
    }
    fn retained_bytes(&self) -> u64 {
        0
    }
    fn work(&self) -> u64 {
        1
    }
}
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000))
}
type Runtime = ConsumerNotificationRuntime<EventBinding>;
fn runtime(
    responses: &Rc<Responses>,
    bank: &Arc<HostResourceAccounts>,
) -> (ConsumerInstance, Runtime) {
    let definition = {
        let source = serde_json::from_value(serde_json::json!({
            "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"Optional response",
            "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
            "events":{"alarm":{"data":{"type":"integer"}, "dataResponse":{"type":"integer"},
                "forms":[{"href":ENDPOINT}]}}
        }))
        .unwrap();
        let registry =
            ConsumerNotificationRegistry::try_new(vec![ConsumerNotificationRegistration {
                binding: EventBinding(responses.clone()),
                services: ConsumerServices {
                    data: independent_open_consumer::services(Arc::new(
                        independent_open_consumer::Probe::default(),
                    ))
                    .data,
                    access: Arc::new(EventAccess),
                },
            }])
            .unwrap();
        registry
            .prepare_notification(
                &source,
                &AffordanceTarget::Event(Arc::from("alarm")),
                ConsumerSelector::default(),
                GatewayDefaultV1::LIMITS,
                bank.clone(),
                &mut work(),
            )
            .unwrap()
    }; // No source TD or preparation registry survives into execution.
    let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
    let runtime = Runtime::try_new(
        owner.reference(),
        definition,
        RawNotificationLimits {
            subscriptions: 1,
            queue_items: 2,
            queue_payload_bytes: 512,
            payload_backing_bytes: 256,
            control_backing_bytes: 256,
            target_name_bytes: 64,
            engine_bytes: 32768,
        },
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    (owner, runtime)
}
fn step(root: &mut Runtime, cursor: &NotificationCursor) -> NotificationState {
    root.progress(
        cursor.identity(),
        &mut Context::from_waker(Waker::noop()),
        &mut work(),
    )
    .unwrap()
}
fn finish(root: &mut Runtime, cursor: &NotificationCursor) -> NotificationSettlement {
    root.stop(cursor, None, &mut work())
        .unwrap_or_else(|_| panic!());
    for _ in 0..8 {
        if step(root, cursor) == NotificationState::Closed {
            return root.take_settlement(cursor.identity()).unwrap();
        }
    }
    panic!("local cleanup did not finish");
}
fn clean(bank: &HostResourceAccounts) {
    for kind in ResourceKind::ALL {
        assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
    }
}

#[test]
fn td_prepared_optional_response_can_retire_and_advance_without_sending_or_stopping() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let responses = Rc::new(Responses::default());
    let (_owner, mut root) = runtime(&responses, &bank);
    let mut cursor = root
        .subscribe(
            SubscriptionInput(None),
            ResponseRequirement::None,
            NotificationOverflow::Terminate,
            &mut work(),
        )
        .unwrap_or_else(|_| panic!());
    assert_eq!(step(&mut root, &cursor), NotificationState::Active);
    step(&mut root, &cursor);
    let first = root.front(&mut cursor, &mut work()).unwrap().unwrap();
    assert_eq!(first.response, EventResponseState::Awaiting);
    let original = Arc::downgrade(&first.data.unwrap().body);
    assert!(root.advance(&mut cursor, &mut work()).is_err());
    // Reproduce the admitted optional opportunity waiting for an explicit choice.
    for _ in 0..3 {
        step(&mut root, &cursor);
    }
    assert_eq!(responses.retire_polls.get(), 0);
    assert!(
        root.decline_response(&mut cursor, &mut WorkBudget::new())
            .is_err()
    );
    assert_eq!(
        root.front(&mut cursor, &mut work())
            .unwrap()
            .unwrap()
            .response,
        EventResponseState::Awaiting
    );
    let funded = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    let bytes = root.status(cursor.identity()).unwrap().queued_payload_bytes;
    responses.pending_retirement.set(true);
    root.decline_response(&mut cursor, &mut work()).unwrap();
    assert_eq!(step(&mut root, &cursor), NotificationState::Active);
    assert_eq!(
        root.front(&mut cursor, &mut work())
            .unwrap()
            .unwrap()
            .response,
        EventResponseState::Declining
    );
    assert!(root.advance(&mut cursor, &mut work()).is_err());
    assert!(original.upgrade().is_some());
    assert_eq!(responses.releases.get(), 0);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        funded
    );
    assert_eq!(
        root.status(cursor.identity()).unwrap().queued_payload_bytes,
        bytes
    );
    responses.pending_retirement.set(false);
    step(&mut root, &cursor);
    assert_eq!(
        root.front(&mut cursor, &mut work())
            .unwrap()
            .unwrap()
            .response,
        EventResponseState::Declined
    );
    root.advance(&mut cursor, &mut work()).unwrap();
    assert!(original.upgrade().is_none());
    let next = root.front(&mut cursor, &mut work()).unwrap().unwrap();
    assert_eq!(next.sequence, 1);
    assert_eq!(&*next.data.unwrap().body, b"2");
    root.decline_response(&mut cursor, &mut work()).unwrap();
    step(&mut root, &cursor);
    root.advance(&mut cursor, &mut work()).unwrap();
    assert_eq!(responses.releases.get(), 2);
    assert_eq!(responses.sends.get(), 0);
    assert_eq!(responses.closes.get(), 0);
    let report = finish(&mut root, &cursor);
    assert_eq!(report.delivered_items, 2);
    assert_eq!(report.responses_delivered, 0);
    assert_eq!(report.responses_uncertain, 0);
    drop(root);
    clean(&bank);
}

#[test]
fn composed_decline_rejects_required_and_already_submitted_responses() {
    for requirement in [ResponseRequirement::Required, ResponseRequirement::None] {
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let responses = Rc::new(Responses::default());
        let (_owner, mut root) = runtime(&responses, &bank);
        let mut cursor = root
            .subscribe(
                SubscriptionInput(None),
                requirement,
                NotificationOverflow::Terminate,
                &mut work(),
            )
            .unwrap_or_else(|_| panic!());
        step(&mut root, &cursor);
        step(&mut root, &cursor);
        root.front(&mut cursor, &mut work()).unwrap().unwrap();
        if requirement == ResponseRequirement::Required {
            assert!(root.decline_response(&mut cursor, &mut work()).is_err());
        }
        root.respond(
            &mut cursor,
            EventDataResponse(Some(Payload::new(b"7".as_slice(), "application/json"))),
            &mut work(),
        )
        .unwrap_or_else(|_| panic!());
        assert!(root.decline_response(&mut cursor, &mut work()).is_err());
        step(&mut root, &cursor);
        step(&mut root, &cursor);
        root.advance(&mut cursor, &mut work()).unwrap();
        assert_eq!(responses.sends.get(), 1);
        assert_eq!(finish(&mut root, &cursor).responses_delivered, 1);
        drop(root);
        clean(&bank);
    }
}
