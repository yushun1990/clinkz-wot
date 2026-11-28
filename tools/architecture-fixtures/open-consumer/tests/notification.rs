#![cfg(feature = "prepare")]
//! Public composed Event lifecycle evidence, without protocol interoperability claims.
use clinkz_wot_core::{consumer::*, host_accounts::HostResourceAccounts, notification::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::notification::*;
#[path = "../../../../servient/finite/tests/support/composed_notification.rs"]
mod composed_fixture;
use composed_fixture::NotificationFixture;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
};

const ENDPOINT: &str = "testevent://source/alarm";
#[derive(Default)]
struct Responses {
    structured: bool,
    collection_structured: bool,
    one_way: bool,
    starts: Cell<usize>,
    compiled_data: Arc<std::sync::atomic::AtomicUsize>,
    last_body: RefCell<Option<std::sync::Weak<[u8]>>>,
    sends: Cell<usize>,
    response_wire: Option<std::net::UdpSocket>,
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
    fn poll_deliver(
        &mut self,
        _: &mut Context<'_>,
        data: &EventDataResponse,
    ) -> Poll<CoreResult<()>> {
        if self.responses.structured {
            assert_eq!(
                data.0.as_ref().unwrap().body.as_ref(),
                br#"{"accepted":true}"#
            );
        }
        assert!(!self.sent);
        if let Some(wire) = &self.responses.response_wire {
            wire.send(&data.0.as_ref().unwrap().body).unwrap();
        }
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
            event_responses: !program.one_way,
            cancellation: self.0.structured,
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
    fn compile_notification_form(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<Self::Program>> {
        if input.facts.content_coding.is_some() {
            return Ok(None);
        }
        assert_eq!(input.target, &AffordanceTarget::Event(Arc::from("alarm")));
        assert_eq!(input.endpoint, ENDPOINT);
        if self.0.structured {
            assert!(input.schemas.subscription().is_some());
            assert!(input.schemas.cancellation().is_some());
            assert_ne!(input.schemas.data(), input.schemas.data_response());
            let data_schema::PreparedSchema::Object(o) = input.schemas.data().unwrap() else {
                panic!()
            };
            self.0.compiled_data.store(
                o.properties().as_ptr() as usize,
                std::sync::atomic::Ordering::Relaxed,
            );
        } else {
            assert!(input.schemas.subscription().is_none());
            assert!(input.schemas.cancellation().is_none());
            assert_eq!(
                input.schemas.data(),
                Some(&data_schema::PreparedSchema::Type(
                    data_schema::ValueType::Integer
                ))
            );
            assert_eq!(input.schemas.data_response(), input.schemas.data());
        }
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
        if self.responses.structured {
            assert_eq!(input.0.as_ref().unwrap().body.as_ref(), br#"{"filter":1}"#);
        } else {
            assert!(input.0.is_none());
        }
        self.responses.starts.set(self.responses.starts.get() + 1);
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
        } else if self.responses.structured || self.responses.collection_structured {
            Payload::new(
                format!(r#"{{"reading":{{"level":{}}}}}"#, self.next).as_bytes(),
                "application/json",
            )
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
            response: (!self.responses.one_way).then(|| Response {
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
        if self.responses.structured {
            if let Some(input) = input {
                assert_eq!(
                    input.0.as_ref().unwrap().body.as_ref(),
                    br#"{"reason":"done"}"#
                );
            }
        } else {
            assert!(input.is_none());
        }
        self.responses.closes.set(self.responses.closes.get() + 1);
        Poll::Ready(Ok(RemoteCancellation::NotAttempted))
    }
    fn abort_local(self) {}
}
struct EventAccess;
impl ConsumerAccessService for EventAccess {
    fn qualification_revision(&self) -> u64 {
        0
    }

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
type Runtime = NotificationFixture<EventBinding>;
fn runtime(
    responses: &Rc<Responses>,
    bank: &Arc<HostResourceAccounts>,
) -> (ConsumerInstance, Runtime) {
    runtime_with_data(
        responses,
        bank,
        independent_open_consumer::services(Arc::default()).data,
    )
}
fn runtime_with_data(
    responses: &Rc<Responses>,
    bank: &Arc<HostResourceAccounts>,
    data: Arc<dyn ConsumerDataService>,
) -> (ConsumerInstance, Runtime) {
    let definition = {
        let source = serde_json::from_value(serde_json::json!({
            "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"Optional response",
            "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
            "events":{"alarm":{"data":{"type":"integer"}, "dataResponse":{"type":"integer"},
                "forms":[{"href":ENDPOINT}]}}
        }))
        .unwrap();
        let registry = ConsumerNotificationRegistry::try_new(
            vec![ConsumerNotificationRegistration {
                binding: EventBinding(responses.clone()),
                services: ConsumerServices {
                    data,
                    access: Arc::new(EventAccess),
                },
            }],
            &bank,
        )
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
        Arc::new(EventAccess),
        RawNotificationLimits {
            subscriptions: 1,
            queue_items: 2,
            queue_payload_bytes: 512,
            payload_backing_bytes: 256,
            control_backing_bytes: 256,
            target_name_bytes: 64,
            reports: 8,
        },
        bank,
    )
    .unwrap_or_else(|e| panic!("{:?}", e));
    (owner, runtime)
}
fn step(root: &mut Runtime, cursor: &NotificationCursor) -> NotificationState {
    root.progress(cursor.identity(), &mut Context::from_waker(Waker::noop()))
        .unwrap();
    root.status(cursor.identity()).unwrap().state
}
fn finish(root: &mut Runtime, cursor: &NotificationCursor) -> NotificationSettlement {
    root.stop(cursor, None).unwrap_or_else(|_| panic!());
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

struct StructuredData(Arc<std::sync::atomic::AtomicUsize>);
impl ConsumerDataService for StructuredData {
    fn qualification_revision(&self) -> u64 {
        0
    }

    fn content_type(&self) -> &str {
        "application/json"
    }
    fn supports_schema(&self, _: &data_schema::PreparedSchema) -> bool {
        true
    }
    fn validate(&self, schema: &data_schema::PreparedSchema, payload: &Payload) -> CoreResult<()> {
        assert_eq!(
            payload.content_type, "application/json",
            "only selected media reaches the codec"
        );
        if let data_schema::PreparedSchema::Object(o) = schema {
            if o.properties()
                .first()
                .is_some_and(|p| &*p.name == "reading")
            {
                assert_eq!(
                    o.properties().as_ptr() as usize,
                    self.0.load(std::sync::atomic::Ordering::Relaxed),
                    "runtime must borrow the compiler's original field backing"
                );
            }
        }
        clinkz_wot_codec_json::validate_payload(schema, payload)
    }
    fn policy(&self) -> ConsumerDataPolicy {
        ConsumerDataPolicy {
            retained_bytes: 0,
            temporary_bytes: 65_536,
            body_bytes: 4096,
            encoded_selection_bytes: 0,
            validation_work: 4096 * 120,
        }
    }
}
struct ControlAccess;
impl ConsumerAccessService for ControlAccess {
    fn qualification_revision(&self) -> u64 {
        0
    }

    fn authorize(
        &self,
        _: BindingRegistrationIdentity,
        endpoint: &str,
        op: Operation,
    ) -> CoreResult<()> {
        assert_eq!(endpoint, ENDPOINT);
        assert!(matches!(
            op,
            Operation::SubscribeEvent | Operation::UnsubscribeEvent
        ));
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
#[test]
fn downstream_binding_uses_shared_structured_meaning_for_four_distinct_event_roles() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let responses = Rc::new(Responses {
        structured: true,
        ..Responses::default()
    });
    let definition = {
        let source = serde_json::from_value(serde_json::json!({
            "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"Structured roles",
            "security":"n", "securityDefinitions":{"n":{"scheme":"nosec"}},
            "events":{"alarm":{
                "subscription":{"type":"object","required":["filter"],"properties":{"filter":{"type":"integer","minimum":1}}},
                "data":{"type":"object","required":["reading"],"properties":{"reading":{"type":"object","required":["level"],"properties":{"level":{"type":"integer","minimum":1,"maximum":2}}}}},
                "dataResponse":{"type":"object","required":["accepted"],"properties":{"accepted":{"type":"boolean"}}},
                "cancellation":{"type":"object","required":["reason"],"properties":{"reason":{"type":"string","minLength":4,"maxLength":4}}},
                "forms":[{"href":ENDPOINT}]
            }}
        })).unwrap();
        ConsumerNotificationRegistry::try_new(
            vec![ConsumerNotificationRegistration {
                binding: EventBinding(responses.clone()),
                services: ConsumerServices {
                    data: Arc::new(StructuredData(responses.compiled_data.clone())),
                    access: Arc::new(ControlAccess),
                },
            }],
            &bank,
        )
        .unwrap()
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
    };
    assert!(definition.input().schemas.retained_bytes() > 0);
    let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
    let mut root = Runtime::try_new(
        owner.reference(),
        definition,
        Arc::new(ControlAccess),
        RawNotificationLimits {
            subscriptions: 1,
            queue_items: 2,
            queue_payload_bytes: 512,
            payload_backing_bytes: 256,
            control_backing_bytes: 256,
            target_name_bytes: 64,
            reports: 8,
        },
        &bank,
    )
    .unwrap_or_else(|_| panic!());
    let payload = |s: &'static [u8]| Some(Payload::new(s, "application/json"));
    let original = payload(br#"{"filter":0}"#).unwrap();
    let weak = Arc::downgrade(&original.body);
    let refused = root
        .subscribe(
            SubscriptionInput(Some(original)),
            ResponseRequirement::Required,
            NotificationOverflow::Terminate,
        )
        .err()
        .unwrap()
        .into_input();
    assert!(weak.upgrade().is_some());
    assert_eq!(responses.starts.get(), 0);
    drop(refused);
    assert!(weak.upgrade().is_none());
    let wrong_media = Payload::new(br#"{"filter":1}"#.as_slice(), "application/octet-stream");
    let refused = root
        .subscribe(
            SubscriptionInput(Some(wrong_media.clone())),
            ResponseRequirement::Required,
            NotificationOverflow::Terminate,
        )
        .err()
        .unwrap()
        .into_input();
    assert_eq!(refused.0, Some(wrong_media));
    assert_eq!(responses.starts.get(), 0);
    let mut cursor = root
        .subscribe(
            SubscriptionInput(payload(br#"{"filter":1}"#)),
            ResponseRequirement::Required,
            NotificationOverflow::Terminate,
        )
        .ok()
        .unwrap();
    assert_eq!(step(&mut root, &cursor), NotificationState::Active);
    step(&mut root, &cursor);
    assert_eq!(responses.starts.get(), 1);
    assert_eq!(
        root.front(&mut cursor).unwrap().unwrap().data.unwrap().body,
        br#"{"reading":{"level":1}}"#
    );
    let body = responses.last_body.borrow_mut().take().unwrap();
    // Valid data/subscription shapes do not satisfy the reciprocal response role.
    assert!(
        root.respond(
            &mut cursor,
            EventDataResponse(payload(br#"{"reading":{"level":1}}"#))
        )
        .is_err()
    );
    assert_eq!(responses.sends.get(), 0);
    root.respond(
        &mut cursor,
        EventDataResponse(payload(br#"{"accepted":true}"#)),
    )
    .ok()
    .unwrap();
    responses.pending_retirement.set(true);
    step(&mut root, &cursor);
    assert_eq!(responses.sends.get(), 1);
    assert!(root.advance(&mut cursor).is_err());
    assert!(body.upgrade().is_some());
    let funded = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    responses.pending_retirement.set(false);
    step(&mut root, &cursor);
    root.advance(&mut cursor).unwrap();
    assert!(body.upgrade().is_none());
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        funded
    );
    // Invalid cancellation still fences delivery locally; a corrected control
    // can be admitted before native close starts. It never becomes Event data.
    assert!(
        root.stop(
            &cursor,
            Some(CancellationInput(payload(br#"{"reason":"x"}"#)))
        )
        .is_err()
    );
    let wrong_media = Payload::new(
        br#"{"reason":"done"}"#.as_slice(),
        "application/octet-stream",
    );
    let refused = root
        .stop(&cursor, Some(CancellationInput(Some(wrong_media.clone()))))
        .err()
        .unwrap()
        .into_input();
    assert_eq!(refused.unwrap().0, Some(wrong_media));
    root.stop(
        &cursor,
        Some(CancellationInput(payload(br#"{"reason":"done"}"#))),
    )
    .ok()
    .unwrap();
    let settlement = finish(&mut root, &cursor);
    assert_eq!(settlement.responses_delivered, 1);
    assert_eq!(responses.closes.get(), 1);
    drop(root);
    drop(settlement);
    clean(&bank);
}

#[test]
fn a_dual_format_codec_cannot_send_a_binary_response_on_the_selected_json_form() {
    use std::{
        net::UdpSocket,
        sync::atomic::{AtomicUsize, Ordering::Relaxed},
    };
    const BINARY: &str = "application/octet-stream";
    #[derive(Default)]
    struct IntegerCodec(AtomicUsize);
    impl ConsumerDataService for IntegerCodec {
        fn qualification_revision(&self) -> u64 {
            0
        }
        fn content_type(&self) -> &str {
            "application/json"
        }
        fn supports_media(&self, media: &str) -> bool {
            matches!(media, "application/json" | BINARY)
        }
        fn supports_schema(&self, s: &data_schema::PreparedSchema) -> bool {
            *s == data_schema::PreparedSchema::Type(data_schema::ValueType::Integer)
        }
        fn policy(&self) -> ConsumerDataPolicy {
            ConsumerDataPolicy {
                retained_bytes: std::mem::size_of::<Self>() as u64,
                ..CollectionData.policy()
            }
        }
        fn validate(&self, s: &data_schema::PreparedSchema, p: &Payload) -> CoreResult<()> {
            if p.content_type == BINARY {
                self.0.fetch_add(1, Relaxed);
                if self.supports_schema(s) && p.body.len() == 8 && p.content_coding.is_none() {
                    return Ok(());
                }
                return Err(invalid());
            }
            clinkz_wot_codec_json::validate_payload(s, p)
        }
    }
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_nonblocking(true).unwrap();
    let wire = UdpSocket::bind("127.0.0.1:0").unwrap();
    wire.connect(peer.local_addr().unwrap()).unwrap();
    let responses = Rc::new(Responses {
        response_wire: Some(wire),
        ..Default::default()
    });
    let codec = Arc::new(IntegerCodec::default());
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let (_owner, mut root) = runtime_with_data(&responses, &bank, codec.clone());
    let mut cursor = root
        .subscribe(
            SubscriptionInput(None),
            ResponseRequirement::Required,
            NotificationOverflow::Terminate,
        )
        .ok()
        .unwrap();
    step(&mut root, &cursor);
    step(&mut root, &cursor);
    assert_eq!(
        root.front(&mut cursor).unwrap().unwrap().response,
        EventResponseState::Awaiting
    );
    let binary = Payload::new(7_i64.to_be_bytes().as_slice(), BINARY);
    codec
        .validate(
            &data_schema::PreparedSchema::Type(data_schema::ValueType::Integer),
            &binary,
        )
        .unwrap();
    let original = Arc::downgrade(&binary.body);
    let refused = root
        .respond(&mut cursor, EventDataResponse(Some(binary)))
        .err()
        .expect("the Form selects JSON");
    assert!(matches!(refused.error().error(), CoreError::Validation(_)));
    let returned = refused.into_input().0.unwrap();
    assert!(Arc::ptr_eq(&returned.body, &original.upgrade().unwrap()));
    assert_eq!(returned.content_type, BINARY);
    assert_eq!(
        codec.0.load(Relaxed),
        1,
        "only the direct codec control ran"
    );
    step(&mut root, &cursor);
    let mut bytes = [0; 16];
    assert_eq!(
        peer.recv(&mut bytes).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    root.respond(
        &mut cursor,
        EventDataResponse(Some(Payload::new(b"7".as_slice(), "application/json"))),
    )
    .ok()
    .unwrap();
    step(&mut root, &cursor);
    peer.set_nonblocking(false).unwrap();
    peer.set_read_timeout(Some(std::time::Duration::from_secs(1)))
        .unwrap();
    let n = peer.recv(&mut bytes).unwrap();
    assert_eq!(&bytes[..n], b"7");
    assert_eq!(finish(&mut root, &cursor).responses_delivered, 1);
    drop(root);
    clean(&bank);
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
        )
        .unwrap_or_else(|_| panic!());
    assert_eq!(step(&mut root, &cursor), NotificationState::Active);
    step(&mut root, &cursor);
    let first = root.front(&mut cursor).unwrap().unwrap();
    assert_eq!(first.response, EventResponseState::Awaiting);
    let original = responses.last_body.borrow_mut().take().unwrap();
    assert_eq!(
        first.data.unwrap().body.as_ptr(),
        original.as_ptr() as *const u8
    );
    assert!(root.advance(&mut cursor).is_err());
    // Reproduce the admitted optional opportunity waiting for an explicit choice.
    for _ in 0..3 {
        step(&mut root, &cursor);
    }
    assert_eq!(responses.retire_polls.get(), 0);
    assert_eq!(
        root.front(&mut cursor).unwrap().unwrap().response,
        EventResponseState::Awaiting
    );
    let funded = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    let bytes = root.status(cursor.identity()).unwrap().queued_payload_bytes;
    responses.pending_retirement.set(true);
    root.decline_response(&mut cursor).unwrap();
    assert_eq!(step(&mut root, &cursor), NotificationState::Active);
    assert_eq!(
        root.front(&mut cursor).unwrap().unwrap().response,
        EventResponseState::Declining
    );
    assert!(root.advance(&mut cursor).is_err());
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
    step(&mut root, &cursor);
    assert_eq!(
        root.front(&mut cursor).unwrap().unwrap().response,
        EventResponseState::Declined
    );
    root.advance(&mut cursor).unwrap();
    assert!(original.upgrade().is_none());
    let next = root.front(&mut cursor).unwrap().unwrap();
    assert_eq!(next.sequence, 1);
    assert_eq!(next.data.unwrap().body, b"2");
    root.decline_response(&mut cursor).unwrap();
    step(&mut root, &cursor);
    root.advance(&mut cursor).unwrap();
    assert_eq!(responses.releases.get(), 2);
    assert_eq!(responses.sends.get(), 0);
    assert_eq!(responses.closes.get(), 0);
    let report = finish(&mut root, &cursor);
    assert_eq!(report.delivered_items, 2);
    assert_eq!(report.responses_delivered, 0);
    assert_eq!(report.responses_uncertain, 0);
    drop(root);
    drop(report);
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
            )
            .unwrap_or_else(|_| panic!());
        step(&mut root, &cursor);
        step(&mut root, &cursor);
        root.front(&mut cursor).unwrap().unwrap();
        if requirement == ResponseRequirement::Required {
            assert!(root.decline_response(&mut cursor).is_err());
        }
        root.respond(
            &mut cursor,
            EventDataResponse(Some(Payload::new(b"7".as_slice(), "application/json"))),
        )
        .unwrap_or_else(|_| panic!());
        assert!(root.decline_response(&mut cursor).is_err());
        step(&mut root, &cursor);
        step(&mut root, &cursor);
        root.advance(&mut cursor).unwrap();
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
    fn compile_notification_form(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<Self::Program>> {
        if input.facts.content_coding.is_some() {
            return Ok(None);
        }
        assert_eq!(input.target, &AffordanceTarget::Thing);
        let NotificationSchemas::SubscribeAllEvents(s) = input.schemas else {
            panic!()
        };
        assert_eq!(s.members().len(), 2);
        assert!(s.member("alarm").unwrap().data_response.is_some());
        assert!(s.member("ready").unwrap().data_response.is_some());
        Ok(Some(self.0.0.clone()))
    }
}
const COLLECTION_ENDPOINT: &str = "testevent://source/%7Bchannel%7D";
#[derive(Default)]
struct CollectionAccess {
    denied: bool,
}
impl ConsumerAccessService for CollectionAccess {
    fn qualification_revision(&self) -> u64 {
        0
    }

    fn authorize(
        &self,
        _: BindingRegistrationIdentity,
        endpoint: &str,
        op: Operation,
    ) -> CoreResult<()> {
        assert_eq!(endpoint, COLLECTION_ENDPOINT);
        assert_eq!(op, Operation::SubscribeAllEvents);
        if self.denied {
            return Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: context(),
            });
        }
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
fn collection_source(forms: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1","title":"Collection responses","base":"testevent://source/",
        "security":"n","securityDefinitions":{"n":{"scheme":"nosec"}},
        "events":{
            "alarm":{"data":{"type":"object","required":["reading"],"properties":{"reading":{"type":"object","required":["level"],"properties":{"level":{"type":"integer","minimum":1,"maximum":2}}}}},"dataResponse":{"type":"string","minLength":3,"maxLength":3},"forms":[{"href":ENDPOINT}]},
            "ready":{"data":{"type":"boolean"},"dataResponse":{"type":"integer","minimum":7,"maximum":7},"forms":[{"href":ENDPOINT}]}},
        "forms": forms
    })
}

fn prepare_collection(
    source: serde_json::Value,
    responses: &Rc<Responses>,
    access: Arc<CollectionAccess>,
    bank: &Arc<HostResourceAccounts>,
    budget: &mut WorkBudget,
) -> CoreResult<ConsumerNotificationDefinition<CollectionBinding>> {
    ConsumerNotificationRegistry::try_new(
        vec![ConsumerNotificationRegistration {
            binding: CollectionBinding(EventBinding(responses.clone())),
            services: ConsumerServices {
                data: Arc::new(CollectionData),
                access,
            },
        }],
        bank,
    )?
    .prepare_notification(
        &serde_json::from_value(source).unwrap(),
        &AffordanceTarget::Thing,
        NotificationKind::SubscribeAllEvents,
        ConsumerSelector::default(),
        GatewayDefaultV1::LIMITS,
        bank.clone(),
        budget,
    )
}

fn collection_runtime(
    owner: &ConsumerInstance,
    definition: ConsumerNotificationDefinition<CollectionBinding>,
    access: Arc<CollectionAccess>,
    bank: &Arc<HostResourceAccounts>,
) -> NotificationFixture<CollectionBinding> {
    NotificationFixture::try_new(
        owner.reference(),
        definition,
        access,
        RawNotificationLimits {
            subscriptions: 1,
            queue_items: 2,
            queue_payload_bytes: 512,
            payload_backing_bytes: 256,
            control_backing_bytes: 256,
            target_name_bytes: 64,
            reports: 8,
        },
        bank,
    )
    .unwrap()
}

#[test]
fn collection_responses_validate_the_original_occurrence_schema_and_retain_each_native_owner() {
    let source = collection_source(serde_json::json!([
        {"href":"testevent://source/{channel}","op":["subscribeallevents","unsubscribeallevents"]},
        {"href":"%7Bchannel%7D","op":["subscribeallevents","unsubscribeallevents"]}
    ]));
    exercise_collection_responses(source, 1);
}

#[test]
fn a_collection_without_a_td_base_authorizes_only_the_absolute_fallback() {
    let hrefs = [
        "%7Bchannel%7D",
        "/alarm",
        "//source/alarm",
        "?q=a:b",
        "#alarm",
        "",
        COLLECTION_ENDPOINT,
    ];
    let forms = hrefs.map(|href| {
        serde_json::json!({
            "href": href,
            "op": ["subscribeallevents", "unsubscribeallevents"]
        })
    });
    let mut source = collection_source(forms.into());
    source.as_object_mut().unwrap().remove("base");
    exercise_collection_responses(source, (hrefs.len() - 1) as u32);
}

fn exercise_collection_responses(source: serde_json::Value, expected_form: u32) {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let responses = Rc::new(Responses {
        collection_structured: true,
        ..Responses::default()
    });
    let access = Arc::new(CollectionAccess::default());
    let definition =
        prepare_collection(source, &responses, access.clone(), &bank, &mut work()).unwrap();
    assert_eq!(definition.input().form, expected_form);
    assert_eq!(&*definition.input().endpoint, COLLECTION_ENDPOINT);
    let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
    let mut root = collection_runtime(&owner, definition, access, &bank);
    let mut cursor = root
        .subscribe(
            SubscriptionInput(None),
            ResponseRequirement::None,
            NotificationOverflow::Terminate,
        )
        .unwrap_or_else(|_| panic!());
    let step = |r: &mut NotificationFixture<CollectionBinding>| {
        r.progress(cursor.identity(), &mut Context::from_waker(Waker::noop()))
            .unwrap();
        r.status(cursor.identity()).unwrap().state
    };
    assert_eq!(step(&mut root), NotificationState::Active);
    for (name, bad, good) in [
        ("alarm", b"7".as_slice(), br#""ack""#.as_slice()),
        ("ready", br#""ack""#, b"7"),
    ] {
        root.progress(cursor.identity(), &mut Context::from_waker(Waker::noop()))
            .unwrap();
        let item = root.front(&mut cursor).unwrap().unwrap();
        assert_eq!(
            item.target,
            clinkz_wot_core::thing::AffordanceSite::Event(name)
        );
        let body = responses.last_body.borrow_mut().take().unwrap();
        assert_eq!(item.data.unwrap().body.as_ptr(), body.as_ptr() as *const u8);
        let wrong = Payload::new(bad, "application/json");
        let original = wrong.body.clone();
        let returned = root
            .respond(&mut cursor, EventDataResponse(Some(wrong)))
            .err()
            .unwrap()
            .into_input();
        assert!(Arc::ptr_eq(&returned.0.unwrap().body, &original));
        root.respond(
            &mut cursor,
            EventDataResponse(Some(Payload::new(good, "application/json"))),
        )
        .unwrap_or_else(|_| panic!());
        responses.pending_retirement.set(true);
        for _ in 0..2 {
            root.progress(cursor.identity(), &mut Context::from_waker(Waker::noop()))
                .unwrap();
        }
        assert!(root.advance(&mut cursor).is_err());
        assert!(body.upgrade().is_some());
        responses.pending_retirement.set(false);
        for _ in 0..2 {
            root.progress(cursor.identity(), &mut Context::from_waker(Waker::noop()))
                .unwrap();
        }
        root.advance(&mut cursor).unwrap();
        assert!(body.upgrade().is_none());
    }
    assert_eq!(responses.sends.get(), 2);
    assert_eq!(responses.releases.get(), 2);
    root.stop(&cursor, None).unwrap_or_else(|_| panic!());
    for _ in 0..4 {
        root.progress(cursor.identity(), &mut Context::from_waker(Waker::noop()))
            .unwrap();
    }
    let report = root.take_settlement(cursor.identity()).unwrap();
    assert_eq!(
        report.target(),
        clinkz_wot_core::thing::AffordanceSite::Thing
    );
    assert_eq!(report.responses_delivered, 2);
    drop(root);
    drop(report);
    clean(&bank);
}

#[test]
fn root_event_controls_refuse_unusable_forms_under_allow_and_deny_policies() {
    use serde_json::json;
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let start = json!({"href": COLLECTION_ENDPOINT, "op": "subscribeallevents"});
    let combined =
        json!({"href": COLLECTION_ENDPOINT, "op": ["subscribeallevents", "unsubscribeallevents"]});
    let denied = Arc::new(CollectionAccess { denied: true });
    assert!(matches!(
        prepare_collection(
            collection_source(json!([combined])),
            &Rc::default(),
            denied,
            &bank,
            &mut work()
        ),
        Err(CoreError::Security {
            reason: SecurityFailureReason::AuthorizationDenied,
            ..
        })
    ));
    clean(&bank);
    for (control, form, native_stop) in [
        ("subscription", &start, false),
        ("cancellation", &combined, false),
        ("cancellation", &start, true),
    ] {
        let responses = Rc::new(Responses {
            structured: native_stop,
            ..Responses::default()
        });
        let mut source = collection_source(json!([form]));
        source["events"]["alarm"][control] = json!({"type": "integer"});
        for denied in [false, true] {
            let access = Arc::new(CollectionAccess { denied });
            let mut budget = work();
            if !native_stop && !denied {
                // The Form and member roles alone rule this candidate out.
                budget = budget.with_remaining(WorkClass::BindingPolls, 0);
            }
            assert!(
                matches!(
                    prepare_collection(source.clone(), &responses, access, &bank, &mut budget),
                    Err(CoreError::UnsupportedOperation(_))
                ),
                "{control}, native stop {native_stop}, denied {denied}"
            );
            assert_eq!(responses.starts.get(), 0);
            clean(&bank);
        }
    }
}

#[test]
fn a_root_event_with_member_cancellation_can_select_a_start_only_form_and_stop_locally() {
    use serde_json::json;
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let responses = Rc::new(Responses {
        collection_structured: true,
        ..Responses::default()
    });
    let access = Arc::new(CollectionAccess::default());
    let mut source = collection_source(json!([
        {"href": COLLECTION_ENDPOINT, "op": ["subscribeallevents", "unsubscribeallevents"]},
        {"href": COLLECTION_ENDPOINT, "op": "subscribeallevents"}
    ]));
    source["events"]["ready"]["cancellation"] = json!({"type": "array"});
    let units = EventBinding(responses.clone()).preparation_bounds().work;
    let mut budget = work().with_remaining(WorkClass::BindingPolls, units);
    let definition =
        prepare_collection(source, &responses, access.clone(), &bank, &mut budget).unwrap();
    assert_eq!(definition.input().form, 1);
    assert!(!definition.input().support.cancellation);
    let NotificationSchemas::SubscribeAllEvents(members) = &definition.input().schemas else {
        panic!()
    };
    let codec = &definition.input().data;
    assert!(codec.supports_schema(members.member("alarm").unwrap().data.as_ref().unwrap()));
    assert!(
        !codec.supports_schema(
            members
                .member("ready")
                .unwrap()
                .cancellation
                .as_ref()
                .unwrap()
        )
    );
    let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
    let mut root = collection_runtime(&owner, definition, access, &bank);
    let report = deliver_alarm_and_stop_locally(&mut root);
    assert_eq!(report.remote_cancellation, RemoteCancellation::NotAttempted);
    assert_eq!(responses.starts.get(), 1);
    assert_eq!(responses.closes.get(), 1);
    drop(report);
    drop(root);
    clean(&bank);
}

fn deliver_alarm_and_stop_locally(
    root: &mut NotificationFixture<CollectionBinding>,
) -> NotificationSettlement {
    let mut cursor = root
        .subscribe(
            SubscriptionInput(None),
            ResponseRequirement::None,
            NotificationOverflow::Terminate,
        )
        .ok()
        .unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    root.progress(cursor.identity(), &mut cx).unwrap();
    assert_eq!(
        root.status(cursor.identity()).unwrap().state,
        NotificationState::Active
    );
    root.progress(cursor.identity(), &mut cx).unwrap();
    let item = root.front(&mut cursor).unwrap().unwrap();
    assert_eq!(item.target, thing::AffordanceSite::Event("alarm"));
    assert_eq!(
        item.data.unwrap().body.as_ref(),
        br#"{"reading":{"level":1}}"#
    );
    root.stop(&cursor, None).ok().unwrap();
    for _ in 0..4 {
        if root.progress(cursor.identity(), &mut cx).unwrap() == NotificationTurn::Closed {
            break;
        }
    }
    root.take_settlement(cursor.identity()).unwrap()
}

#[test]
fn member_reply_schemas_require_a_codec_only_when_the_native_program_can_offer_replies() {
    use serde_json::json;
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let replies = Rc::new(Responses {
        collection_structured: true,
        ..Responses::default()
    });
    let one_way = Rc::new(Responses {
        collection_structured: true,
        one_way: true,
        ..Responses::default()
    });
    let mut source =
        collection_source(json!([{"href": COLLECTION_ENDPOINT, "op": "subscribeallevents"}]));
    source["events"]["alarm"]["dataResponse"] = json!({"type": "array"});
    let prepare = |native: &Rc<Responses>, denied| {
        prepare_collection(
            source.clone(),
            native,
            Arc::new(CollectionAccess { denied }),
            &bank,
            &mut work(),
        )
    };
    for denied in [false, true] {
        assert!(matches!(
            prepare(&replies, denied),
            Err(CoreError::UnsupportedOperation(_))
        ));
        clean(&bank);
    }
    assert!(matches!(
        prepare(&one_way, true),
        Err(CoreError::Security {
            reason: SecurityFailureReason::AuthorizationDenied,
            ..
        })
    ));
    clean(&bank);

    // Source-free construction must qualify the actual program against the retained schemas.
    let mut input = prepare(&one_way, false).unwrap().into_input();
    input.binding = CollectionBinding(EventBinding(replies.clone()));
    input.program = replies.clone();
    input.support = input
        .binding
        .support(&input.program, input.schemas.kind())
        .unwrap();
    let refused = ConsumerNotificationDefinition::try_new(input)
        .err()
        .expect("reply codec unavailable");
    assert!(Rc::ptr_eq(&refused.into_input().program, &replies));
    clean(&bank);

    let definition = prepare(&one_way, false).unwrap();
    let NotificationSchemas::SubscribeAllEvents(members) = &definition.input().schemas else {
        panic!()
    };
    assert!(
        !definition.input().data.supports_schema(
            members
                .member("alarm")
                .unwrap()
                .data_response
                .as_ref()
                .unwrap()
        )
    );
    let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
    let mut root = collection_runtime(&owner, definition, Arc::default(), &bank);
    let refusal = root
        .subscribe(
            SubscriptionInput(None),
            ResponseRequirement::Required,
            NotificationOverflow::Terminate,
        )
        .err()
        .unwrap();
    assert!(matches!(
        refusal.error().error(),
        CoreError::UnsupportedOperation(_)
    ));
    assert_eq!(one_way.starts.get(), 0);
    drop(refusal);
    let report = deliver_alarm_and_stop_locally(&mut root);
    assert_eq!(report.responses_delivered, 0);
    assert_eq!(one_way.releases.get(), 0);
    assert_eq!(one_way.closes.get(), 1);
    drop(report);
    drop(root);
    clean(&bank);
}

#[test]
fn a_codec_revision_change_during_native_reply_qualification_invalidates_the_search() {
    use data_schema::{PreparedSchema, ValueType};
    use std::sync::atomic::{AtomicU64, Ordering::SeqCst};

    struct ChangeOnReply(AtomicU64);
    impl ConsumerDataService for ChangeOnReply {
        fn qualification_revision(&self) -> u64 {
            self.0.load(SeqCst)
        }
        fn content_type(&self) -> &str {
            CollectionData.content_type()
        }
        fn supports_schema(&self, schema: &PreparedSchema) -> bool {
            if schema == &PreparedSchema::Type(ValueType::Array) {
                self.0.fetch_add(1, SeqCst);
            }
            CollectionData.supports_schema(schema)
        }
        fn policy(&self) -> ConsumerDataPolicy {
            ConsumerDataPolicy {
                retained_bytes: std::mem::size_of::<Self>() as u64,
                ..CollectionData.policy()
            }
        }
        fn validate(&self, schema: &PreparedSchema, payload: &Payload) -> CoreResult<()> {
            CollectionData.validate(schema, payload)
        }
    }

    let services = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let plans = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let replies = Rc::new(Responses::default());
    let codec = Arc::new(ChangeOnReply(0.into()));
    let registration = RetainedConsumerNotificationRegistration {
        binding: CollectionBinding(EventBinding(replies.clone())),
        services: RetainedConsumerServices {
            data: ConsumerServiceOwner::retain_data(codec.clone(), &services)
                .ok()
                .unwrap(),
            access: ConsumerServiceOwner::retain_access(
                Arc::new(CollectionAccess::default()),
                &services,
            )
            .ok()
            .unwrap(),
        },
    };
    let observed = codec.clone();
    let registry = ConsumerNotificationRegistry::try_new_revisioned(
        vec![registration.into_provider(std::convert::identity)],
        move || Some(observed.qualification_revision()),
    )
    .unwrap();
    let mut source = collection_source(
        serde_json::json!([{"href": COLLECTION_ENDPOINT, "op": "subscribeallevents"}]),
    );
    source["events"]["alarm"]["dataResponse"] = serde_json::json!({"type": "array"});
    let thing = serde_json::from_value(source).unwrap();
    let prepare = |budget: &mut WorkBudget| {
        registry.prepare_notification(
            &thing,
            &AffordanceTarget::Thing,
            NotificationKind::SubscribeAllEvents,
            ConsumerSelector::default(),
            GatewayDefaultV1::LIMITS,
            plans.clone(),
            budget,
        )
    };
    assert!(matches!(
        prepare(&mut work().with_remaining(WorkClass::BindingPolls, 0)),
        Err(CoreError::Backpressure(_))
    ));
    assert_eq!(codec.qualification_revision(), 0);
    clean(&plans);
    assert!(matches!(
        prepare(&mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    assert_eq!(codec.qualification_revision(), 1);
    assert_eq!(replies.starts.get(), 0);
    clean(&plans);
    drop(registry);
    clean(&services);
}

#[test]
fn source_free_root_definitions_return_the_native_program_for_unexecutable_member_controls() {
    use serde_json::json;
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    for mapped_stop in [false, true] {
        let responses = Rc::new(Responses {
            structured: mapped_stop,
            ..Responses::default()
        });
        let source = collection_source(
            json!([{"href": COLLECTION_ENDPOINT, "op": ["subscribeallevents", "unsubscribeallevents"]}]),
        );
        let mut input = prepare_collection(source, &responses, Arc::default(), &bank, &mut work())
            .unwrap()
            .into_input();
        assert_eq!(input.support.cancellation, mapped_stop);
        let NotificationSchemas::SubscribeAllEvents(schema) = &input.schemas else {
            panic!()
        };
        let mut members = schema.members().to_vec();
        let control = if mapped_stop {
            &mut members[0].cancellation
        } else {
            &mut members[0].subscription
        };
        *control = Some(data_schema::PreparedSchema::Type(
            data_schema::ValueType::Integer,
        ));
        input.schemas = NotificationSchemas::SubscribeAllEvents(Arc::new(
            collection::EventCollectionSchema::try_new(members).unwrap(),
        ));
        let refusal = ConsumerNotificationDefinition::try_new(input)
            .err()
            .expect("composed root control has no executable input");
        assert!(matches!(
            refusal.error().error(),
            CoreError::UnsupportedOperation(_)
        ));
        assert!(Rc::ptr_eq(&refusal.into_input().program, &responses));
        assert_eq!(responses.starts.get(), 0);
        clean(&bank);
    }
}

struct CollectionData;
impl ConsumerDataService for CollectionData {
    fn qualification_revision(&self) -> u64 {
        0
    }

    fn content_type(&self) -> &str {
        "application/json"
    }
    fn supports_schema(&self, schema: &data_schema::PreparedSchema) -> bool {
        !matches!(
            schema,
            data_schema::PreparedSchema::Type(data_schema::ValueType::Array)
        )
    }
    fn policy(&self) -> ConsumerDataPolicy {
        ConsumerDataPolicy {
            retained_bytes: 0,
            temporary_bytes: 65_536,
            body_bytes: 1024,
            encoded_selection_bytes: 0,
            validation_work: 4096 * 120 + 128,
        }
    }
    fn validate(&self, s: &data_schema::PreparedSchema, p: &Payload) -> CoreResult<()> {
        if !self.supports_schema(s) {
            return Err(CoreError::UnsupportedOperation(context()));
        }
        clinkz_wot_codec_json::validate_payload(s, p)
    }
}

#[test]
fn delayed_response_policy_receives_original_occurrence_and_denial_retires_the_exact_reply() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    struct ResponsePolicy(Arc<AtomicBool>, Arc<AtomicUsize>);
    impl ConsumerAccessService for ResponsePolicy {
        fn qualification_revision(&self) -> u64 {
            0
        }

        fn authorize(
            &self,
            _: BindingRegistrationIdentity,
            _: &str,
            _: Operation,
        ) -> CoreResult<()> {
            Ok(())
        }
        fn authorize_instance(
            &self,
            _: &ConsumerInstanceRef,
            _: BindingRegistrationIdentity,
            _: &str,
            _: Operation,
        ) -> CoreResult<()> {
            Ok(())
        }
        fn authorize_notification(
            &self,
            _: BindingRegistrationIdentity,
            _: &str,
            r: NotificationAuthorization<'_>,
        ) -> CoreResult<NotificationDecision> {
            assert_eq!(r.target, &AffordanceTarget::Thing);
            if r.permission == NotificationPermission::Respond {
                assert_eq!(
                    r.occurrence,
                    Some(&AffordanceTarget::Event(Arc::from("alarm")))
                );
                self.1.fetch_add(1, Ordering::Relaxed);
                if self.0.load(Ordering::Relaxed) {
                    return Err(CoreError::Security {
                        reason: SecurityFailureReason::AuthorizationDenied,
                        context: ErrorContext::new(ErrorPhase::Delivery, RetryClass::Never),
                    });
                }
            }
            Ok(NotificationDecision::Allow)
        }
        fn work(&self) -> u64 {
            1
        }
        fn retained_bytes(&self) -> u64 {
            64
        }
    }
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let responses = Rc::new(Responses::default());
    let denied = Arc::new(AtomicBool::new(false));
    let checks = Arc::new(AtomicUsize::new(0));
    let access: Arc<dyn ConsumerAccessService> =
        Arc::new(ResponsePolicy(denied.clone(), checks.clone()));
    let services = RetainedConsumerServices {
        data: ConsumerServiceOwner::retain_data(Arc::new(CollectionData), &bank)
            .unwrap_or_else(|_| panic!()),
        access: ConsumerServiceOwner::retain_access(access, &bank).unwrap_or_else(|_| panic!()),
    };
    let registry = ConsumerNotificationRegistry::try_from_retained(vec![
        RetainedConsumerNotificationRegistration {
            binding: CollectionBinding(EventBinding(responses.clone())),
            services: services.clone(),
        },
    ])
    .unwrap();
    let source = serde_json::from_value(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"Original occurrence policy",
        "security":"n","securityDefinitions":{"n":{"scheme":"nosec"}},
        "events":{
            "alarm":{"data":{"type":"integer"},"dataResponse":{"type":"integer"},"forms":[{"href":ENDPOINT}]},
            "ready":{"data":{"type":"boolean"},"dataResponse":{"type":"integer"},"forms":[{"href":ENDPOINT}]}},
        "forms":[{"href":ENDPOINT,"op":["subscribeallevents","unsubscribeallevents"]}]
    })).unwrap();
    let definition = Rc::new(
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
            .unwrap(),
    );
    let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
    let association = clinkz_wot_servient_finite::consumer_notification::ConsumerNotificationAssociation::try_new(owner.reference(), definition, services.access.clone()).unwrap_or_else(|_| panic!());
    let mut root =
        clinkz_wot_servient_finite::consumer_notification::ConsumerNotifications::try_new(
            RawNotificationLimits {
                subscriptions: 1,
                queue_items: 2,
                queue_payload_bytes: 512,
                payload_backing_bytes: 256,
                control_backing_bytes: 0,
                target_name_bytes: 64,
                reports: 8,
            },
        )
        .unwrap();
    drop((source, registry, services));
    let mut cursor = root
        .subscribe(
            &association,
            SubscriptionInput(None),
            ResponseRequirement::Required,
            NotificationOverflow::Terminate,
        )
        .unwrap_or_else(|_| panic!());
    drop(association);
    let mut cx = Context::from_waker(Waker::noop());
    root.progress(cursor.identity(), &mut cx).unwrap();
    root.progress(cursor.identity(), &mut cx).unwrap();
    assert!(root.front(&mut cursor).unwrap().is_some());
    root.respond(
        &mut cursor,
        EventDataResponse(Some(Payload::new(b"7".as_slice(), "application/json"))),
    )
    .unwrap_or_else(|_| panic!());
    denied.store(true, Ordering::Relaxed);
    assert!(root.progress(cursor.identity(), &mut cx).is_err());
    assert_eq!(checks.load(Ordering::Relaxed), 2);
    assert_eq!(responses.sends.get(), 0);
    for _ in 0..8 {
        if root.progress(cursor.identity(), &mut cx).unwrap() == NotificationTurn::Closed {
            break;
        }
    }
    let report = root.take_settlement(cursor.identity()).unwrap();
    assert_eq!(report.responses_delivered, 0);
    assert_eq!(responses.releases.get(), 1);
    assert!(
        responses
            .last_body
            .borrow()
            .as_ref()
            .unwrap()
            .upgrade()
            .is_none()
    );
    drop((report, root));
    clean(&bank);
}

#[test]
fn response_validation_dependency_failure_is_irreversible_after_restoration() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    struct MutatingData {
        armed: AtomicBool,
        changed: AtomicBool,
        calls: AtomicUsize,
    }
    impl ConsumerDataService for MutatingData {
        fn qualification_revision(&self) -> u64 {
            0
        }

        fn content_type(&self) -> &str {
            "application/json"
        }
        fn policy(&self) -> ConsumerDataPolicy {
            ConsumerDataPolicy {
                retained_bytes: 0,
                temporary_bytes: 4096,
                body_bytes: if self.changed.load(Ordering::Relaxed) {
                    511
                } else {
                    512
                },
                encoded_selection_bytes: 0,
                validation_work: 1024,
            }
        }
        fn validate(
            &self,
            schema: &data_schema::PreparedSchema,
            payload: &Payload,
        ) -> CoreResult<()> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let result = clinkz_wot_codec_json::validate_payload(schema, payload);
            if self.armed.swap(false, Ordering::Relaxed) {
                self.changed.store(true, Ordering::Relaxed);
            }
            result
        }
    }
    for invalid_response in [false, true] {
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let data = Arc::new(MutatingData {
            armed: AtomicBool::new(false),
            changed: AtomicBool::new(false),
            calls: AtomicUsize::new(0),
        });
        let responses = Rc::new(Responses::default());
        let source = serde_json::from_value(serde_json::json!({
            "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"Response dependency fencing",
            "security":"n", "securityDefinitions":{"n":{"scheme":"nosec"}},
            "events":{"alarm":{"data":{"type":"integer"},"dataResponse":{"type":"integer"},"forms":[{"href":ENDPOINT}]}}
        })).unwrap();
        let definition = ConsumerNotificationRegistry::try_new(
            vec![ConsumerNotificationRegistration {
                binding: EventBinding(responses.clone()),
                services: ConsumerServices {
                    data: data.clone(),
                    access: Arc::new(EventAccess),
                },
            }],
            &bank,
        )
        .unwrap()
        .prepare_notification(
            &source,
            &AffordanceTarget::Event(Arc::from("alarm")),
            NotificationKind::Event,
            ConsumerSelector::default(),
            GatewayDefaultV1::LIMITS,
            bank.clone(),
            &mut work(),
        )
        .unwrap();
        let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
        let mut root = Runtime::try_new(
            owner.reference(),
            definition,
            Arc::new(EventAccess),
            RawNotificationLimits {
                subscriptions: 1,
                queue_items: 2,
                queue_payload_bytes: 512,
                payload_backing_bytes: 256,
                control_backing_bytes: 256,
                target_name_bytes: 64,
                reports: 8,
            },
            &bank,
        )
        .unwrap();
        let mut cursor = root
            .subscribe(
                SubscriptionInput(None),
                ResponseRequirement::Required,
                NotificationOverflow::Terminate,
            )
            .unwrap_or_else(|_| panic!());
        step(&mut root, &cursor);
        step(&mut root, &cursor);
        assert!(root.front(&mut cursor).unwrap().is_some());
        let occurrence = responses.last_body.borrow().as_ref().unwrap().clone();
        let malformed = Payload::new(b"false".as_slice(), "application/json");
        let refused = root
            .respond(&mut cursor, EventDataResponse(Some(malformed.clone())))
            .err()
            .unwrap()
            .into_input();
        assert_eq!(refused.0, Some(malformed));
        assert_eq!(
            root.status(cursor.identity()).unwrap().state,
            NotificationState::Active
        );
        assert!(
            root.status(cursor.identity())
                .unwrap()
                .first_cause
                .is_none()
        );
        let original = Payload::new(
            if invalid_response {
                b"false".as_slice()
            } else {
                b"7".as_slice()
            },
            "application/json",
        );
        let original_ptr = original.body.as_ptr();
        let response_body = Arc::downgrade(&original.body);
        let calls = data.calls.load(Ordering::Relaxed);
        data.armed.store(true, Ordering::Relaxed);
        let input = root
            .respond(&mut cursor, EventDataResponse(Some(original)))
            .err()
            .unwrap()
            .into_input();
        assert_eq!(input.0.as_ref().unwrap().body.as_ptr(), original_ptr);
        assert_eq!(data.calls.load(Ordering::Relaxed), calls + 1);
        assert_eq!(responses.sends.get(), 0);
        assert!(response_body.upgrade().is_some());
        assert_eq!(
            root.status(cursor.identity()).unwrap().state,
            NotificationState::Closing,
            "an observed dependency failure must fence even if payload validation also refuses"
        );
        let first = root
            .status(cursor.identity())
            .unwrap()
            .first_cause
            .unwrap()
            .clone();
        data.changed.store(false, Ordering::Relaxed);
        let input = root.respond(&mut cursor, input).err().unwrap().into_input();
        assert_eq!(input.0.as_ref().unwrap().body.as_ptr(), original_ptr);
        assert_eq!(
            data.calls.load(Ordering::Relaxed),
            calls + 1,
            "restoration cannot re-enter validation on the original conversation"
        );
        assert!(root.front(&mut cursor).is_err());
        assert!(root.advance(&mut cursor).is_err());
        assert_eq!(
            root.status(cursor.identity()).unwrap().first_cause,
            Some(&first)
        );
        let funded = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
        responses.pending_retirement.set(true);
        root.progress(cursor.identity(), &mut Context::from_waker(Waker::noop()))
            .unwrap();
        assert_eq!(responses.releases.get(), 0);
        assert!(occurrence.upgrade().is_some());
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            funded
        );
        responses.pending_retirement.set(false);
        for _ in 0..8 {
            if root
                .progress(cursor.identity(), &mut Context::from_waker(Waker::noop()))
                .unwrap()
                == NotificationTurn::Closed
            {
                break;
            }
        }
        let report = root.take_settlement(cursor.identity()).unwrap();
        assert_eq!(report.first_cause.as_ref(), Some(&first));
        assert_eq!(report.responses_delivered, 0);
        assert_eq!(report.responses_uncertain, 0);
        assert_eq!(responses.sends.get(), 0);
        assert_eq!(responses.releases.get(), 1);
        assert_eq!(responses.closes.get(), 1);
        assert!(occurrence.upgrade().is_none());
        assert!(
            response_body.upgrade().is_some(),
            "refused caller input still belongs to its caller"
        );
        drop(input);
        assert!(response_body.upgrade().is_none());
        drop((report, root));
        clean(&bank);
    }
}
