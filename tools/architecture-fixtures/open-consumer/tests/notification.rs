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
    starts: Cell<usize>,
    compiled_data: Arc<std::sync::atomic::AtomicUsize>,
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
    fn compile_notification(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<Self::Program>> {
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
                    data: independent_open_consumer::services(Arc::new(
                        independent_open_consumer::Probe::default(),
                    ))
                    .data,
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
            engine_bytes: 32768,
        },
    )
    .unwrap_or_else(|e| panic!("{:?}", e));
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

struct StructuredData(Arc<std::sync::atomic::AtomicUsize>);
impl ConsumerDataService for StructuredData {
    fn content_type(&self) -> &str {
        "application/json"
    }
    fn supports_schema(&self, _: &data_schema::PreparedSchema) -> bool {
        true
    }
    fn validate(&self, schema: &data_schema::PreparedSchema, payload: &Payload) -> CoreResult<()> {
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
            engine_bytes: 32768,
        },
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
            &mut work(),
        )
        .err()
        .unwrap()
        .into_input();
    assert!(weak.upgrade().is_some());
    assert_eq!(responses.starts.get(), 0);
    drop(refused);
    assert!(weak.upgrade().is_none());
    let mut cursor = root
        .subscribe(
            SubscriptionInput(payload(br#"{"filter":1}"#)),
            ResponseRequirement::Required,
            NotificationOverflow::Terminate,
            &mut work(),
        )
        .ok()
        .unwrap();
    assert_eq!(step(&mut root, &cursor), NotificationState::Active);
    step(&mut root, &cursor);
    assert_eq!(responses.starts.get(), 1);
    assert_eq!(
        root.front(&mut cursor, &mut work())
            .unwrap()
            .unwrap()
            .data
            .unwrap()
            .body,
        br#"{"reading":{"level":1}}"#
    );
    let body = responses.last_body.borrow_mut().take().unwrap();
    // Valid data/subscription shapes do not satisfy the reciprocal response role.
    assert!(
        root.respond(
            &mut cursor,
            EventDataResponse(payload(br#"{"reading":{"level":1}}"#)),
            &mut work()
        )
        .is_err()
    );
    assert!(
        root.respond(
            &mut cursor,
            EventDataResponse(payload(br#"{"accepted":true}"#)),
            &mut WorkBudget::new()
        )
        .is_err()
    );
    assert_eq!(responses.sends.get(), 0);
    root.respond(
        &mut cursor,
        EventDataResponse(payload(br#"{"accepted":true}"#)),
        &mut work(),
    )
    .ok()
    .unwrap();
    responses.pending_retirement.set(true);
    step(&mut root, &cursor);
    assert_eq!(responses.sends.get(), 1);
    assert!(root.advance(&mut cursor, &mut work()).is_err());
    assert!(body.upgrade().is_some());
    let funded = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    responses.pending_retirement.set(false);
    step(&mut root, &cursor);
    root.advance(&mut cursor, &mut work()).unwrap();
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
            Some(CancellationInput(payload(br#"{"reason":"x"}"#))),
            &mut work()
        )
        .is_err()
    );
    root.stop(
        &cursor,
        Some(CancellationInput(payload(br#"{"reason":"done"}"#))),
        &mut work(),
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
        let NotificationSchemas::SubscribeAllEvents(s) = input.schemas else {
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
    let responses = Rc::new(Responses {
        collection_structured: true,
        ..Responses::default()
    });
    let registry = ConsumerNotificationRegistry::try_new(
        vec![ConsumerNotificationRegistration {
            binding: CollectionBinding(EventBinding(responses.clone())),
            services: ConsumerServices {
                data: Arc::new(CollectionData),
                access: Arc::new(CollectionAccess),
            },
        }],
        &bank,
    )
    .unwrap();
    let definition = {
        let source=serde_json::from_value(serde_json::json!({
            "@context":"https://www.w3.org/2022/wot/td/v1.1","title":"Collection responses",
            "security":"n","securityDefinitions":{"n":{"scheme":"nosec"}},
            "events":{
                "alarm":{"data":{"type":"object","required":["reading"],"properties":{"reading":{"type":"object","required":["level"],"properties":{"level":{"type":"integer","minimum":1,"maximum":2}}}}},"dataResponse":{"type":"string","minLength":3,"maxLength":3},"forms":[{"href":ENDPOINT}]},
                "ready":{"data":{"type":"boolean"},"dataResponse":{"type":"integer","minimum":7,"maximum":7},"forms":[{"href":ENDPOINT}]}},
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
    let mut root = NotificationFixture::try_new(
        owner.reference(),
        definition,
        Arc::new(CollectionAccess),
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
    let step = |r: &mut NotificationFixture<CollectionBinding>| {
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
    drop(report);
    clean(&bank);
}

struct CollectionData;
impl ConsumerDataService for CollectionData {
    fn content_type(&self) -> &str {
        "application/json"
    }
    fn supports_schema(&self, _: &data_schema::PreparedSchema) -> bool {
        true
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
        clinkz_wot_codec_json::validate_payload(s, p)
    }
}

#[test]
fn delayed_response_policy_receives_original_occurrence_and_denial_retires_the_exact_reply() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    struct ResponsePolicy(Arc<AtomicBool>, Arc<AtomicUsize>);
    impl ConsumerAccessService for ResponsePolicy {
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
        ) -> CoreResult<()> {
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
            Ok(())
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
    let association = clinkz_wot_servient_finite::consumer_notification::ConsumerNotificationAssociation::try_new(owner.reference(), definition, services.access.clone(), bank.clone(), &mut work()).unwrap_or_else(|_| panic!());
    let mut root =
        clinkz_wot_servient_finite::consumer_notification::ConsumerNotifications::try_new(
            RawNotificationLimits {
                subscriptions: 1,
                queue_items: 2,
                queue_payload_bytes: 512,
                payload_backing_bytes: 256,
                control_backing_bytes: 0,
                target_name_bytes: 64,
                engine_bytes: 32768,
            },
            bank.clone(),
        )
        .unwrap();
    drop((source, registry, services));
    let mut cursor = root
        .subscribe(
            &association,
            SubscriptionInput(None),
            ResponseRequirement::Required,
            NotificationOverflow::Terminate,
            &mut work(),
        )
        .unwrap_or_else(|_| panic!());
    drop(association);
    let mut cx = Context::from_waker(Waker::noop());
    root.progress(cursor.identity(), &mut cx, &mut work())
        .unwrap();
    root.progress(cursor.identity(), &mut cx, &mut work())
        .unwrap();
    assert!(root.front(&mut cursor, &mut work()).unwrap().is_some());
    root.respond(
        &mut cursor,
        EventDataResponse(Some(Payload::new(b"7".as_slice(), "application/json"))),
        &mut work(),
    )
    .unwrap_or_else(|_| panic!());
    denied.store(true, Ordering::Relaxed);
    assert!(
        root.progress(cursor.identity(), &mut cx, &mut work())
            .is_err()
    );
    assert_eq!(checks.load(Ordering::Relaxed), 2);
    assert_eq!(responses.sends.get(), 0);
    for _ in 0..8 {
        if root
            .progress(cursor.identity(), &mut cx, &mut work())
            .unwrap()
            == NotificationState::Closed
        {
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
