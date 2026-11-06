#![cfg(feature = "prepare")]
//! Public composed Event lifecycle evidence, without protocol interoperability claims.
use clinkz_wot_core::{consumer::*, host_accounts::HostResourceAccounts, notification::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::{
    consumer_notification::ConsumerNotificationRuntime, notification::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
};

const ENDPOINT: &str = "testevent://source/alarm";
#[derive(Default)]
struct Responses {
    last_body: RefCell<Option<std::sync::Weak<[u8]>>>,
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
    group: bool,
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
            group: false,
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
    fn preparation_bounds(&self) -> ConsumerPreparationBounds {
        ConsumerPreparationBounds {
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
            Some(&data_schema::PreparedSchema::Type(data_schema::ValueType::Integer))
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
        let ready = self.group && self.next == 2;
        let target = AffordanceTarget::Event(Arc::from(if ready { "ready" } else { "alarm" }));
        let payload = if ready {
            Payload::new(b"true".as_slice(), "application/json")
        } else {
            Payload::new(self.next.to_string().as_bytes(), "application/json")
        };
        assert!(notification_item_backing(&target, Some(&payload)).unwrap() <= max);
        *self.responses.last_body.borrow_mut() = Some(Arc::downgrade(&payload.body));
        self.next += 1;
        Poll::Ready(Ok(NotificationSignal::Data(NativeNotification {
            recipient: self.recipient.clone(),
            target,
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
                NotificationKind::Event,
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
    let original = responses.last_body.borrow_mut().take().unwrap();
    assert_eq!(
        first.data.unwrap().body.as_ptr(),
        original.as_ptr() as *const u8
    );
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
    assert_eq!(next.data.unwrap().body, b"2");
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

// A downstream collection Binding supplies actual per-item response owners.
// This fixture is lifecycle evidence, not a production protocol mapping.
#[derive(Clone)]
struct CollectionBinding(EventBinding);
impl ConsumerNotificationBinding for CollectionBinding {
    type Program = Rc<Responses>;
    type Configuration = u8;
    type Participation = Participation;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        self.0.identity()
    }
    fn configuration(&self) -> &u8 {
        self.0.configuration()
    }
    fn support(
        &self,
        p: &Self::Program,
        kind: NotificationKind,
    ) -> CoreResult<NotificationSupport> {
        assert_eq!(kind, NotificationKind::SubscribeAllEvents);
        self.0.support(p, NotificationKind::Event)
    }
    fn prepare(
        &mut self,
        p: Self::Program,
        id: &NotificationIdentity,
    ) -> Result<Participation, BindingInputRejection<Self::Program>> {
        let mut participant = self.0.prepare(p, id)?;
        participant.group = true;
        Ok(participant)
    }
}
impl ConsumerNotificationPreparation for CollectionBinding {
    fn registration_identity(&self) -> BindingRegistrationIdentity {
        self.0.registration_identity()
    }
    fn preparation_bounds(&self) -> ConsumerPreparationBounds {
        self.0.preparation_bounds()
    }
    fn compile_notification(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<Self::Program>> {
        assert_eq!(input.target, &AffordanceTarget::Thing);
        let ConsumerNotificationSchemas::SubscribeAllEvents(s) = input.schemas else {
            panic!()
        };
        assert_eq!(s.members().len(), 2);
        assert!(s.member("alarm").unwrap().data_response.is_some());
        assert!(s.member("ready").unwrap().data_response.is_some());
        Ok(Some(self.0.0.clone()))
    }
}
struct CollectionAccess;
impl ConsumerAccessService for CollectionAccess {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, op: Operation) -> CoreResult<()> {
        assert_eq!(op, Operation::SubscribeAllEvents);
        Ok(())
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        id: BindingRegistrationIdentity,
        t: &str,
        op: Operation,
    ) -> CoreResult<()> {
        self.authorize(id, t, op)
    }
    fn retained_bytes(&self) -> u64 {
        0
    }
    fn work(&self) -> u64 {
        1
    }
}
#[test]
fn collection_responses_validate_the_original_occurrence_schema_and_retain_each_native_owner() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let responses = Rc::new(Responses::default());
    let registry = ConsumerNotificationRegistry::try_new(vec![ConsumerNotificationRegistration {
        binding: CollectionBinding(EventBinding(responses.clone())),
        services: ConsumerServices {
            data: Arc::new(CollectionData),
            access: Arc::new(CollectionAccess),
        },
    }])
    .unwrap();
    let definition = {
        let source=serde_json::from_value(serde_json::json!({
            "@context":"https://www.w3.org/2022/wot/td/v1.1","title":"Collection responses",
            "security":"n","securityDefinitions":{"n":{"scheme":"nosec"}},
            "events":{
                "alarm":{"data":{"type":"integer"},"dataResponse":{"type":"string"},"forms":[{"href":ENDPOINT}]},
                "ready":{"data":{"type":"boolean"},"dataResponse":{"type":"integer"},"forms":[{"href":ENDPOINT}]}},
            "forms":[{"href":ENDPOINT,"op":["subscribeallevents","unsubscribeallevents"]}]
        })).unwrap();
        registry
            .prepare_notification(
                &source,
                &AffordanceTarget::Thing,
                NotificationKind::SubscribeAllEvents,
                ConsumerSelector::default(),
                GatewayDefaultV1::LIMITS,
                bank.clone(),
                &mut work(),
            )
            .unwrap()
    };
    drop(registry);
    let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
    let mut root = ConsumerNotificationRuntime::try_new(
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
    .unwrap_or_else(|_| panic!());
    let mut cursor = root
        .subscribe(
            SubscriptionInput(None),
            ResponseRequirement::None,
            NotificationOverflow::Terminate,
            &mut work(),
        )
        .unwrap_or_else(|_| panic!());
    let step = |r: &mut ConsumerNotificationRuntime<CollectionBinding>| {
        r.progress(
            cursor.identity(),
            &mut Context::from_waker(Waker::noop()),
            &mut work(),
        )
        .unwrap()
    };
    assert_eq!(step(&mut root), NotificationState::Active);
    for (name, bad, good) in [
        ("alarm", b"7".as_slice(), br#""ack""#.as_slice()),
        ("ready", br#""ack""#, b"7"),
    ] {
        root.progress(
            cursor.identity(),
            &mut Context::from_waker(Waker::noop()),
            &mut work(),
        )
        .unwrap();
        let item = root.front(&mut cursor, &mut work()).unwrap().unwrap();
        assert_eq!(
            item.target,
            clinkz_wot_core::thing::AffordanceSite::Event(name)
        );
        let body = responses.last_body.borrow_mut().take().unwrap();
        assert_eq!(item.data.unwrap().body.as_ptr(), body.as_ptr() as *const u8);
        let wrong = Payload::new(bad, "application/json");
        let original = wrong.body.clone();
        let returned = root
            .respond(&mut cursor, EventDataResponse(Some(wrong)), &mut work())
            .err()
            .unwrap()
            .into_input();
        assert!(Arc::ptr_eq(&returned.0.unwrap().body, &original));
        root.respond(
            &mut cursor,
            EventDataResponse(Some(Payload::new(good, "application/json"))),
            &mut work(),
        )
        .unwrap_or_else(|_| panic!());
        responses.pending_retirement.set(true);
        for _ in 0..2 {
            root.progress(
                cursor.identity(),
                &mut Context::from_waker(Waker::noop()),
                &mut work(),
            )
            .unwrap();
        }
        assert!(root.advance(&mut cursor, &mut work()).is_err());
        assert!(body.upgrade().is_some());
        responses.pending_retirement.set(false);
        root.progress(
            cursor.identity(),
            &mut Context::from_waker(Waker::noop()),
            &mut work(),
        )
        .unwrap();
        root.advance(&mut cursor, &mut work()).unwrap();
        assert!(body.upgrade().is_none());
    }
    assert_eq!(responses.sends.get(), 2);
    assert_eq!(responses.releases.get(), 2);
    root.stop(&cursor, None, &mut work())
        .unwrap_or_else(|_| panic!());
    for _ in 0..4 {
        root.progress(
            cursor.identity(),
            &mut Context::from_waker(Waker::noop()),
            &mut work(),
        )
        .unwrap();
    }
    let report = root.take_settlement(cursor.identity()).unwrap();
    assert_eq!(report.target, AffordanceTarget::Thing);
    assert_eq!(report.responses_delivered, 2);
    drop(root);
    clean(&bank);
}

struct CollectionData;
impl ConsumerDataService for CollectionData {
    fn content_type(&self) -> &str {
        "application/json"
    }
    fn policy(&self) -> ConsumerDataPolicy {
        ConsumerDataPolicy {
            retained_bytes: 0,
            temporary_bytes: 4096,
            body_bytes: 1024,
            encoded_selection_bytes: 0,
            validation_work: 4096 * 40 + 128,
        }
    }
    fn validate(&self, s: &data_schema::PreparedSchema, p: &Payload) -> CoreResult<()> {
        clinkz_wot_codec_json::validate_payload(s, p)
    }
}
