#![cfg(feature = "prepare")]
//! Qualified, managed, real-peer HTTP Action execution through the same Thing
//! authority as finite Properties. No test-only Action provider is involved.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{action::*, finite::*, producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::producer_composition::*;
use clinkz_wot_servient::{host_accounts::HostResourceAccounts, producer_composition::*};
use clinkz_wot_servient_finite::*;
use clinkz_wot_td::ValidatedProducerScope;
use std::{
    cell::Cell,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

fn budget() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, c| b.with_remaining(c, 1_000_000))
}
fn addr() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}
fn key(generation: Generation) -> ProducerInstanceAuthority {
    ProducerInstanceAuthority::try_new(ThingSlotId::new(SlotIndex::new(1), generation)).unwrap()
}
fn now() -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(1), 0)
}
fn cx() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
fn td(action: SocketAddr, property: Option<SocketAddr>) -> Arc<str> {
    let mut json = serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"HTTP Actions",
        "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "actions":{"run":{"synchronous":false, "input":{"type":"integer"}, "output":{"type":"integer"},
            "uriVariables":{"id":{"type":"string"}},
            "forms":[
                {"href":format!("http://{action}/run"), "op":"invokeaction", "htv:methodName":"POST"},
                {"href":format!("http://{action}/status{{?id}}"), "op":"queryaction", "htv:methodName":"GET"},
                {"href":format!("http://{action}/stop{{?id}}"), "op":"cancelaction", "htv:methodName":"DELETE"}
            ]}}
    });
    if let Some(property) = property {
        json["properties"] = serde_json::json!({"level":{"type":"integer","readOnly":true,
            "forms":[{"href":format!("http://{property}/value")}]}});
    }
    Arc::from(json.to_string())
}
#[derive(Default)]
struct Counters {
    invoked: Cell<usize>,
    cancelled: Cell<usize>,
    complete: Cell<bool>,
    invalid: Cell<bool>,
    immediate: Cell<bool>,
    deny_invoke: Cell<bool>,
    deny_query: Cell<bool>,
    deny_cancel: Cell<bool>,
}
struct Action(Rc<Counters>);
impl RawActionHandler for Action {
    fn authorize(&self, op: Operation, _: &InteractionInput) -> CoreResult<()> {
        if match op {
            Operation::InvokeAction => self.0.deny_invoke.get(),
            Operation::QueryAction => self.0.deny_query.get(),
            Operation::CancelAction => self.0.deny_cancel.get(),
            _ => false,
        } {
            Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Handler, RetryClass::Never),
            })
        } else {
            Ok(())
        }
    }
    fn invoke(
        &self,
        _: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<RawActionStart> {
        assert_eq!(input.data.as_ref().unwrap().body.as_ref(), b"12");
        self.0.invoked.set(self.0.invoked.get() + 1);
        if self.0.immediate.get() {
            return Ok(RawActionStart::Complete(InteractionOutput::with_data(
                Payload::new(b"\"invalid\"".to_vec(), "application/json"),
            )));
        }
        Ok(RawActionStart::Continue(Box::new(Work(Rc::clone(&self.0)))))
    }
}
struct Work(Rc<Counters>);
impl RawActionWork for Work {
    fn poll(&mut self) -> Poll<CoreResult<InteractionOutput>> {
        if !self.0.complete.get() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(InteractionOutput::with_data(Payload::new(
            if self.0.invalid.get() {
                b"\"invalid\"".to_vec()
            } else {
                b"24".to_vec()
            },
            "application/json",
        ))))
    }
    fn request_cancel(&mut self) -> CoreResult<()> {
        self.0.cancelled.set(self.0.cancelled.get() + 1);
        Ok(())
    }
    fn poll_close(&mut self) -> Poll<CoreResult<()>> {
        Poll::Ready(Ok(()))
    }
    fn abort_local(self: Box<Self>) {}
}
struct Property;
impl RawFiniteHandler for Property {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn handle(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput::with_data(Payload::new(
            b"42".to_vec(),
            "application/json",
        )))
    }
}
fn owner() -> RawProducerServient {
    let mut owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 2,
        behaviors_per_instance: 4,
        target_name_bytes: 64,
    })
    .unwrap();
    owner
        .enable_raw_actions(RawActionLimits {
            invocations: 4,
            application_work_bytes: 64,
            input_fields: 8,
            input_bytes: 4096 + 64,
            principal_id_bytes: 32,
            output_body_bytes: 4096,
            output_media_capacity: 64,
            output_retained_bytes: 8192,
        })
        .unwrap();
    owner
}
fn create(
    owner: &RawProducerServient,
    authority: ProducerInstanceAuthority,
    action: SocketAddr,
    property: Option<SocketAddr>,
    counts: Rc<Counters>,
    expected: Option<ProducerInstanceKey>,
    bank: Arc<HostResourceAccounts>,
) -> ComposedProducerExposure {
    let text = td(action, property);
    let key = authority.key();
    let instance = RawProducerInstance::new(
        authority,
        property.map_or_else(Vec::new, |_| {
            vec![RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("level")),
                Operation::ReadProperty,
                Property,
            )]
        }),
    )
    .with_actions(vec![RawActionBehavior::new(
        Arc::from("run"),
        Action(counts),
        0,
        true,
        true,
    )])
    .with_description(Arc::clone(&text));
    let action_provider = Rc::new(HttpActionProducer::new(
        HttpProducerConfig::for_loopback(action).unwrap(),
        BindingId::new(61),
        BindingGeneration::INITIAL,
    ));
    let property_provider = property.map(|addr| {
        Rc::new(
            HttpFiniteProducer::try_new(
                HttpProducerConfig::for_loopback(addr).unwrap(),
                BindingId::new(62),
                BindingGeneration::INITIAL,
                100,
            )
            .unwrap(),
        )
    });
    let mut providers: Vec<Rc<dyn ProducerCapability>> = vec![action_provider];
    if let Some(p) = property_provider {
        providers.push(p);
    }
    let borrowed: Vec<_> = providers.iter().map(|p| p.as_ref()).collect();
    let prepared = prepare_thing(
        &text,
        key,
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
        &borrowed,
        |scope, op| match scope {
            ValidatedProducerScope::Action(n) => instance.supports_action(n, op),
            ValidatedProducerScope::Property(n) => instance.supports_property(n, op),
            _ => false,
        },
    )
    .unwrap();
    let definitions: Vec<_> = prepared
        .operations()
        .iter()
        .filter_map(|o| o.definition::<HttpActionDefinition>().cloned())
        .collect();
    assert_eq!(definitions.len(), 3);
    let mut participants: Vec<Box<dyn OwnedProducerParticipation>> = vec![Box::new(
        HttpActionRuntime::try_new(owner, key, definitions, 4).unwrap(),
    )];
    if let Some(addr) = property {
        let program = prepared
            .operations()
            .iter()
            .find_map(|o| o.definition::<HttpProducerProgram>())
            .unwrap()
            .clone();
        let config = HttpFiniteConfig::try_new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            program,
            key,
            Arc::from("level"),
            100,
        )
        .unwrap();
        participants.push(Box::new(
            RawFiniteRuntime::try_new(
                HttpFiniteBinding::new(
                    config.clone(),
                    BindingId::new(62),
                    BindingGeneration::INITIAL,
                ),
                config,
                owner,
                RawFiniteLimits {
                    candidates: 1,
                    input_body_bytes: 0,
                    output_body_bytes: 4096,
                    output_media_capacity: 64,
                    output_retained_bytes: 8192,
                },
                FiniteOutputValidation::Required,
                Deadline::NONE,
            )
            .ok()
            .unwrap(),
        ));
    }
    ComposedProducerExposure::try_new(
        owner,
        ComposedProducerInput {
            instance,
            prepared,
            providers,
            participants,
            expected,
            handler_backing_bytes: 0,
        },
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        bank,
        &mut budget(),
    )
    .ok()
    .unwrap()
}
fn send(
    owner: &mut RawProducerServient,
    exposure: &mut ComposedProducerExposure,
    address: SocketAddr,
    method: &str,
    path: &str,
    body: &str,
) -> String {
    send_report(owner, exposure, address, method, path, body, &mut budget()).0
}
fn send_report(
    owner: &mut RawProducerServient,
    exposure: &mut ComposedProducerExposure,
    address: SocketAddr,
    method: &str,
    path: &str,
    body: &str,
    credits: &mut WorkBudget,
) -> (String, HttpActionDelivery) {
    let mut peer = TcpStream::connect(address).unwrap();
    peer.set_nonblocking(true).unwrap();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n{}Content-Length: {}\r\n\r\n{body}",
        if body.is_empty() {
            ""
        } else {
            "Content-Type: application/json\r\n"
        },
        body.len()
    );
    peer.write_all(head.as_bytes()).unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    let mut bytes = Vec::new();
    loop {
        let native = exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
        let _ = native.step(owner, credits).unwrap();
        if let Some(delivery) = native.take_delivery() {
            loop {
                let mut buf = [0; 512];
                match peer.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => bytes.extend_from_slice(&buf[..n]),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(e) => panic!("{e}"),
                }
                assert!(Instant::now() < end);
            }
            return (String::from_utf8(bytes).unwrap(), delivery);
        }
        assert!(Instant::now() < end, "HTTP Action stalled");
    }
}
fn read_property(
    owner: &mut RawProducerServient,
    exposure: &mut ComposedProducerExposure,
    address: SocketAddr,
) -> Vec<u8> {
    let mut peer = TcpStream::connect(address).unwrap();
    peer.set_nonblocking(true).unwrap();
    peer.write_all(
        format!("GET /value HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .unwrap();
    let mut bytes = Vec::new();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let native = exposure
            .participant_mut::<RawFiniteRuntime<HttpFiniteBinding>>(1)
            .unwrap();
        let _ = owner.step(native, &mut cx(), &mut budget(), now()).unwrap();
        let mut buf = [0; 512];
        match peer.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => bytes.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("{e}"),
        }
        assert!(Instant::now() < end, "Property read stalled");
    }
    bytes
}
#[test]
fn qualified_http_action_and_property_share_one_authority_with_independent_continuation() {
    let (action, property) = (addr(), addr());
    let mut owner = owner();
    let counts = Rc::new(Counters::default());
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut exposure = create(
        &owner,
        key(Generation::INITIAL),
        action,
        Some(property),
        Rc::clone(&counts),
        None,
        Arc::clone(&bank),
    );
    let logical_key = exposure.key();
    assert!(exposure.publish(&mut owner, &mut budget()).is_err());
    exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    assert_eq!(
        exposure
            .participant_mut::<RawFiniteRuntime<HttpFiniteBinding>>(1)
            .unwrap()
            .step(&mut cx(), &mut budget(), now()),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    assert!(
        exposure
            .participant_mut::<RawFiniteRuntime<HttpFiniteBinding>>(1)
            .unwrap()
            .activate_raw()
            .is_err()
    );
    exposure.publish(&mut owner, &mut budget()).unwrap();
    assert_eq!(
        owner.description(logical_key),
        Some(td(action, Some(property)).as_ref())
    );
    assert!(read_property(&mut owner, &mut exposure, property).ends_with(b"\r\n\r\n42"));
    let finite = exposure
        .participant_mut::<RawFiniteRuntime<HttpFiniteBinding>>(1)
        .unwrap();
    assert_eq!(
        finite.take_settlement().unwrap().recipient,
        Some(logical_key)
    );
    // A partial native request holds no semantic Action; disconnect releases
    // its socket without stealing the next complete peer's correlation.
    let mut partial = TcpStream::connect(action).unwrap();
    partial.write_all(b"POST /run HTTP/1.1\r\nHost:").unwrap();
    let _ = exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .step(&mut owner, &mut budget());
    drop(partial);
    for _ in 0..8 {
        let _ = exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .step(&mut owner, &mut budget());
    }
    assert_eq!(counts.invoked.get(), 0);
    let rejected = send(&mut owner, &mut exposure, action, "POST", "/run", "\"bad\"");
    assert!(rejected.starts_with("HTTP/1.1 400"));
    assert_eq!(counts.invoked.get(), 0);
    let accepted = send(&mut owner, &mut exposure, action, "POST", "/run", "12");
    assert!(accepted.starts_with("HTTP/1.1 202"), "{accepted}");
    let wire_id =
        serde_json::from_str::<serde_json::Value>(accepted.split("\r\n\r\n").nth(1).unwrap())
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
    let id: u32 = wire_id.split_once('-').unwrap().1.parse().unwrap();
    assert!(accepted.contains(&format!("Location: http://{action}/status?id={wire_id}")));
    assert!(
        send(
            &mut owner,
            &mut exposure,
            action,
            "GET",
            &format!("/status?id={wire_id}"),
            ""
        )
        .contains("\"running\"")
    );
    assert!(
        send(
            &mut owner,
            &mut exposure,
            action,
            "DELETE",
            &format!("/stop?id={wire_id}"),
            ""
        )
        .contains("\"cancelling\"")
    );
    let handle = exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .handle(id)
        .unwrap();
    assert_eq!(
        owner.step_raw_action(&handle, &mut budget()).unwrap(),
        Poll::Pending
    );
    assert_eq!(counts.cancelled.get(), 1);
    // Completing work after the initiating TCP connection was destroyed does
    // not change the original Action's authority or turn cancel into rollback.
    counts.complete.set(true);
    assert_eq!(
        owner.step_raw_action(&handle, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    assert!(
        send(
            &mut owner,
            &mut exposure,
            action,
            "GET",
            &format!("/status?id={wire_id}"),
            ""
        )
        .contains("\"output\":24")
    );
    exposure.begin_close();
    assert!(exposure.retire(&mut owner, &mut budget()).is_err());
    let result = exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .acknowledge(&mut owner, id)
        .unwrap()
        .unwrap();
    assert_eq!(result.instance, logical_key);
    assert_eq!(result.effect, ActionEffect::PossiblyApplied);
    assert!(result.cancel_requested);
    let finite = exposure
        .participant_mut::<RawFiniteRuntime<HttpFiniteBinding>>(1)
        .unwrap();
    while finite.state() != RawFiniteState::Closed {
        let _ = finite.step(&mut cx(), &mut budget(), now());
    }
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
    TcpListener::bind(action).unwrap();
    TcpListener::bind(property).unwrap();
}

#[test]
fn action_wire_refusals_distinguish_policy_capacity_and_stale_authority() {
    let address = addr();
    let mut owner = owner();
    let counts = Rc::new(Counters::default());
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut exposure = create(
        &owner,
        key(Generation::INITIAL),
        address,
        None,
        Rc::clone(&counts),
        None,
        Arc::clone(&bank),
    );
    let key = exposure.key();
    exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    exposure.publish(&mut owner, &mut budget()).unwrap();

    counts.deny_invoke.set(true);
    let (response, report) = send_report(
        &mut owner,
        &mut exposure,
        address,
        "POST",
        "/run",
        "12",
        &mut budget(),
    );
    assert!(response.starts_with("HTTP/1.1 403 Forbidden"), "{response}");
    assert!(response.ends_with("{\"status\":\"denied\"}"));
    assert!(report.locally_written && report.cause.is_none());
    assert!(matches!(
        report.request_error,
        Some(CoreError::Security { .. })
    ));
    counts.deny_invoke.set(false);

    // Native receive/write still have ample credit, but Servient handler
    // admission has none. This is retryable pressure, not policy denial.
    let mut low = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1000);
    let (response, report) = send_report(
        &mut owner,
        &mut exposure,
        address,
        "POST",
        "/run",
        "12",
        &mut low,
    );
    assert!(
        response.starts_with("HTTP/1.1 503 Service Unavailable"),
        "{response}"
    );
    assert!(matches!(
        report.request_error,
        Some(CoreError::Backpressure(_))
    ));
    assert_eq!(counts.invoked.get(), 0);

    let accepted = send(&mut owner, &mut exposure, address, "POST", "/run", "12");
    assert!(accepted.starts_with("HTTP/1.1 202"), "{accepted}");
    let wire =
        serde_json::from_str::<serde_json::Value>(accepted.split("\r\n\r\n").nth(1).unwrap())
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
    let query = format!("/status?id={wire}");
    let cancel = format!("/stop?id={wire}");

    for (op, path) in [
        (Operation::QueryAction, &query),
        (Operation::CancelAction, &cancel),
    ] {
        match op {
            Operation::QueryAction => counts.deny_query.set(true),
            Operation::CancelAction => counts.deny_cancel.set(true),
            _ => unreachable!(),
        }
        let method = if op == Operation::QueryAction {
            "GET"
        } else {
            "DELETE"
        };
        let (response, report) = send_report(
            &mut owner,
            &mut exposure,
            address,
            method,
            path,
            "",
            &mut budget(),
        );
        assert!(response.starts_with("HTTP/1.1 403 Forbidden"), "{response}");
        assert!(matches!(
            report.request_error,
            Some(CoreError::Security { .. })
        ));
        match op {
            Operation::QueryAction => counts.deny_query.set(false),
            Operation::CancelAction => counts.deny_cancel.set(false),
            _ => unreachable!(),
        }
        let mut low = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1000);
        let (response, report) = send_report(
            &mut owner,
            &mut exposure,
            address,
            method,
            path,
            "",
            &mut low,
        );
        assert!(
            response.starts_with("HTTP/1.1 503 Service Unavailable"),
            "{response}"
        );
        assert!(matches!(
            report.request_error,
            Some(CoreError::Backpressure(_))
        ));
    }
    assert_eq!(counts.cancelled.get(), 0);
    assert!(send(&mut owner, &mut exposure, address, "GET", &query, "").contains("running"));

    // The participant's own record cap is also a capacity refusal, not a
    // forged application authorization result.
    for _ in 0..3 {
        assert!(
            send(&mut owner, &mut exposure, address, "POST", "/run", "12")
                .starts_with("HTTP/1.1 202")
        );
    }
    let (response, report) = send_report(
        &mut owner,
        &mut exposure,
        address,
        "POST",
        "/run",
        "12",
        &mut budget(),
    );
    assert!(
        response.starts_with("HTTP/1.1 503 Service Unavailable"),
        "{response}"
    );
    assert!(matches!(
        report.request_error,
        Some(CoreError::Backpressure(_))
    ));
    assert_eq!(counts.invoked.get(), 4);

    // Direct logical revocation leaves a native gate open until the exposure
    // guard is retired; the Servient still refuses fresh access as stale.
    assert!(owner.revoke(key));
    for (method, path, body) in [("GET", query.as_str(), ""), ("DELETE", cancel.as_str(), "")] {
        let (response, report) = send_report(
            &mut owner,
            &mut exposure,
            address,
            method,
            path,
            body,
            &mut budget(),
        );
        assert!(response.starts_with("HTTP/1.1 410 Gone"), "{response}");
        assert!(matches!(
            report.request_error,
            Some(CoreError::StaleHandle(_))
        ));
    }
    // Free a native slot so the invoke reaches the same stale-authority check.
    let ids: Vec<_> = exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .action_ids()
        .collect();
    counts.complete.set(true);
    for id in ids {
        let native = exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
        let handle = native.handle(id).unwrap();
        assert_eq!(
            owner.step_raw_action(&handle, &mut budget()).unwrap(),
            Poll::Ready(())
        );
        native.acknowledge(&mut owner, id).unwrap().unwrap();
    }
    let (response, report) = send_report(
        &mut owner,
        &mut exposure,
        address,
        "POST",
        "/run",
        "12",
        &mut budget(),
    );
    assert!(response.starts_with("HTTP/1.1 410 Gone"), "{response}");
    assert!(matches!(
        report.request_error,
        Some(CoreError::StaleHandle(_))
    ));
    assert_eq!(counts.invoked.get(), 4);

    exposure.begin_close();
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
    TcpListener::bind(address).unwrap();
}

#[test]
fn published_action_cannot_skip_validation_through_raw_request() {
    let address = addr();
    let mut owner = owner();
    let counts = Rc::new(Counters::default());
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut exposure = create(
        &owner,
        key(Generation::INITIAL),
        address,
        None,
        Rc::clone(&counts),
        None,
        Arc::clone(&bank),
    );
    exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    exposure.publish(&mut owner, &mut budget()).unwrap();

    // RawActionRequest is public; a published instance must select its captured
    // service even when the native request bypasses HTTP's own ingress code.
    let target = AffordanceTarget::Action(Arc::from("run"));
    let input = InteractionInput::with_data(Payload::new(b"12".to_vec(), "application/json"));
    let invoke = ActionRoute {
        binding: (BindingId::new(61), BindingGeneration::INITIAL),
        mapping: ActionMappingKey(1),
    };
    let handle = owner
        .invoke_raw_action(
            RawActionRequest {
                instance: exposure.key(),
                target: &target,
                invoke,
                query: None,
                cancel: None,
                input: &input,
            },
            &mut budget(),
        )
        .unwrap();
    counts.complete.set(true);
    counts.invalid.set(true);
    assert_eq!(
        owner.step_raw_action(&handle, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    let settlement = owner.take_raw_action_settlement(&handle).unwrap().unwrap();
    assert!(matches!(settlement.result, Err(CoreError::Validation(_))));
    assert_eq!(counts.invoked.get(), 1);
    counts.immediate.set(true);
    let handle = owner
        .invoke_raw_action(
            RawActionRequest {
                instance: exposure.key(),
                target: &target,
                invoke,
                query: None,
                cancel: None,
                input: &input,
            },
            &mut budget(),
        )
        .unwrap();
    let settlement = owner.take_raw_action_settlement(&handle).unwrap().unwrap();
    assert!(matches!(settlement.result, Err(CoreError::Validation(_))));
    assert_eq!(counts.invoked.get(), 2);
    let invalid =
        InteractionInput::with_data(Payload::new(b"\"bad\"".to_vec(), "application/json"));
    assert!(matches!(
        owner.invoke_raw_action(
            RawActionRequest {
                instance: exposure.key(),
                target: &target,
                invoke,
                query: None,
                cancel: None,
                input: &invalid,
            },
            &mut budget(),
        ),
        Err(CoreError::Validation(_))
    ));
    assert_eq!(counts.invoked.get(), 2);
    assert!(matches!(
        owner.invoke_raw_action(
            RawActionRequest {
                instance: exposure.key(),
                target: &target,
                invoke: ActionRoute {
                    mapping: ActionMappingKey(99),
                    ..invoke
                },
                query: None,
                cancel: None,
                input: &input,
            },
            &mut budget(),
        ),
        Err(CoreError::UnsupportedOperation(_))
    ));
    exposure.begin_close();
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
}

#[test]
fn published_action_controls_cannot_be_selected_from_unprepared_routes() {
    let address = addr();
    let mut owner = owner();
    let counts = Rc::new(Counters::default());
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut exposure = create(
        &owner,
        key(Generation::INITIAL),
        address,
        None,
        Rc::clone(&counts),
        None,
        Arc::clone(&bank),
    );
    exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    exposure.publish(&mut owner, &mut budget()).unwrap();

    let target = AffordanceTarget::Action(Arc::from("run"));
    let input = InteractionInput::with_data(Payload::new(b"12".to_vec(), "application/json"));
    let binding = (BindingId::new(61), BindingGeneration::INITIAL);
    let invoke = ActionRoute {
        binding,
        mapping: ActionMappingKey(1),
    };
    let query = ActionRoute {
        binding,
        mapping: ActionMappingKey(2),
    };
    let cancel = ActionRoute {
        binding,
        mapping: ActionMappingKey(3),
    };
    let foreign = ActionRoute {
        binding: (BindingId::new(99), BindingGeneration::INITIAL),
        mapping: ActionMappingKey(99),
    };
    let unprepared = ActionRoute {
        binding,
        mapping: ActionMappingKey(99),
    };
    for (query, cancel) in [
        (Some(foreign), Some(cancel)),
        (Some(query), Some(foreign)),
        (Some(unprepared), Some(cancel)),
        (Some(query), Some(unprepared)),
        (Some(cancel), Some(cancel)), // an advertised cancel route is not a query route
    ] {
        assert!(matches!(
            owner.invoke_raw_action(
                RawActionRequest {
                    instance: exposure.key(),
                    target: &target,
                    invoke,
                    query,
                    cancel,
                    input: &input,
                },
                &mut budget(),
            ),
            Err(CoreError::UnsupportedOperation(_))
        ));
    }
    assert_eq!(counts.invoked.get(), 0);
    let handle = owner
        .invoke_raw_action(
            RawActionRequest {
                instance: exposure.key(),
                target: &target,
                invoke,
                query: Some(query),
                cancel: Some(cancel),
                input: &input,
            },
            &mut budget(),
        )
        .unwrap();
    for (op, route) in [
        (Operation::QueryAction, query),
        (Operation::CancelAction, cancel),
    ] {
        if op == Operation::QueryAction {
            assert!(matches!(
                owner
                    .query_raw_action(
                        &handle,
                        route.binding,
                        route.mapping,
                        &InteractionInput::empty(),
                        &mut budget()
                    )
                    .unwrap()
                    .state,
                RawActionState::Running
            ));
        } else {
            assert!(
                owner
                    .cancel_raw_action(
                        &handle,
                        route.binding,
                        route.mapping,
                        &InteractionInput::empty(),
                        &mut budget()
                    )
                    .unwrap()
            );
        }
    }
    counts.complete.set(true);
    assert_eq!(
        owner.step_raw_action(&handle, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    let settlement = owner.take_raw_action_settlement(&handle).unwrap().unwrap();
    assert_eq!(settlement.query, Some(query));
    assert_eq!(settlement.cancel, Some(cancel));
    exposure.begin_close();
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
}

#[test]
fn revoked_action_gate_drains_owned_partial_tcp_without_dispatch() {
    let address = addr();
    let mut owner = owner();
    let counts = Rc::new(Counters::default());
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut exposure = create(
        &owner,
        key(Generation::INITIAL),
        address,
        None,
        Rc::clone(&counts),
        None,
        Arc::clone(&bank),
    );
    let native = exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
    native.install_closed().unwrap();
    exposure.publish(&mut owner, &mut budget()).unwrap();
    let mut peer = TcpStream::connect(address).unwrap();
    peer.write_all(b"POST /run HTTP/1.1\r\nHost:").unwrap();
    let native = exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
    assert_eq!(
        native.step(&mut owner, &mut budget()).unwrap(),
        Poll::Pending
    );
    let gate = Rc::clone(native.ingress_gate());
    gate.set(false);
    peer.shutdown(std::net::Shutdown::Write).unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        if exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .step(&mut owner, &mut budget())
            .unwrap()
            .is_ready()
        {
            break;
        }
        assert!(
            Instant::now() < end,
            "revoked HTTP Action kept the partial exchange"
        );
    }
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    assert_eq!(peer.read(&mut [0; 1]).unwrap(), 0);
    assert_eq!(counts.invoked.get(), 0);
    assert_eq!(
        exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .records(),
        0
    );
    let mut next = TcpStream::connect(address).unwrap();
    next.write_all(format!("POST /run HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n12").as_bytes()).unwrap();
    assert_eq!(
        exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .step(&mut owner, &mut budget())
            .unwrap(),
        Poll::Pending
    );
    assert_eq!(counts.invoked.get(), 0);
    exposure.begin_close();
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    drop(next);
    drop(peer);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
    TcpListener::bind(address).unwrap();
}

#[test]
fn revoked_action_gate_finishes_owned_reply_and_action() {
    let address = addr();
    let mut owner = owner();
    let counts = Rc::new(Counters::default());
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut exposure = create(
        &owner,
        key(Generation::INITIAL),
        address,
        None,
        Rc::clone(&counts),
        None,
        Arc::clone(&bank),
    );
    exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    exposure.publish(&mut owner, &mut budget()).unwrap();

    let mut peer = TcpStream::connect(address).unwrap();
    peer.write_all(
        format!("POST /run HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n12").as_bytes(),
    ).unwrap();
    let native = exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
    assert_eq!(
        native.step(&mut owner, &mut budget()).unwrap(),
        Poll::Pending
    );
    assert_eq!(counts.invoked.get(), 1); // semantic admission, reply still owned
    let gate = Rc::clone(native.ingress_gate());
    gate.set(false);
    assert_eq!(
        native.step(&mut owner, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    assert!(native.take_delivery().unwrap().locally_written);
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    let mut response = String::new();
    peer.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 202 Accepted"), "{response}");
    assert_eq!(counts.invoked.get(), 1);
    assert_eq!(native.records(), 1);

    let id = native.action_ids().next().unwrap();
    counts.complete.set(true);
    assert_eq!(
        native.progress_actions(&mut owner, &mut budget()).unwrap(),
        Poll::Ready(Some(id))
    );
    assert!(native.acknowledge(&mut owner, id).unwrap().is_some());
    exposure.begin_close();
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
    TcpListener::bind(address).unwrap();
}

#[test]
fn revoked_action_gate_refuses_completed_input_from_owned_connection() {
    let address = addr();
    let mut owner = owner();
    let counts = Rc::new(Counters::default());
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut exposure = create(
        &owner,
        key(Generation::INITIAL),
        address,
        None,
        Rc::clone(&counts),
        None,
        Arc::clone(&bank),
    );
    let native = exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
    native.install_closed().unwrap();
    exposure.publish(&mut owner, &mut budget()).unwrap();
    let mut peer = TcpStream::connect(address).unwrap();
    peer.write_all(b"POST /run HTTP/1.1\r\nHost:").unwrap();
    let native = exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
    assert_eq!(
        native.step(&mut owner, &mut budget()).unwrap(),
        Poll::Pending
    );
    Rc::clone(native.ingress_gate()).set(false);
    peer.write_all(
        format!(" {address}\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n12")
            .as_bytes(),
    )
    .unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        if native.step(&mut owner, &mut budget()).unwrap().is_ready() {
            break;
        }
        assert!(
            Instant::now() < end,
            "revoked HTTP Action kept completed native input"
        );
    }
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    assert_eq!(peer.read(&mut [0; 1]).unwrap(), 0);
    assert_eq!(counts.invoked.get(), 0);
    assert_eq!(native.records(), 0);
    exposure.begin_close();
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
    TcpListener::bind(address).unwrap();
}

#[test]
fn unsupported_action_meaning_is_refused_before_any_native_setup() {
    let address = addr();
    let provider = HttpActionProducer::new(
        HttpProducerConfig::for_loopback(address).unwrap(),
        BindingId::new(61),
        BindingGeneration::INITIAL,
    );
    let text = td(address, None);
    let key = key(Generation::INITIAL).key();
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(
        prepare_thing(
            &text,
            key,
            &GatewayDefaultV1::LIMITS,
            &mut budget(),
            &[&provider],
            |_, op| op == Operation::InvokeAction
        )
        .is_err()
    );
    for change in 0..9 {
        let mut candidate = value.clone();
        let action = &mut candidate["actions"]["run"];
        match change {
            0 => action["input"]["minimum"] = 3.into(),
            1 => action["output"]["type"] = "object".into(), // supported type alone, but still complete
            2 => action["synchronous"] = true.into(),
            3 => action["forms"][1]["htv:methodName"] = "POST".into(),
            4 => action["forms"][2]["contentType"] = "text/plain".into(),
            5 => action["forms"][2]["op"] = serde_json::json!(["queryaction", "cancelaction"]),
            6 => action["forms"][1]["href"] = format!("http://{address}/status").into(),
            7 => action["forms"][0]["security"] = serde_json::json!(["missing"]),
            _ => action["uriVariables"]["id"]["pattern"] = "[a-z]+".into(),
        }
        let result = prepare_thing(
            &candidate.to_string(),
            key,
            &GatewayDefaultV1::LIMITS,
            &mut budget(),
            &[&provider],
            |_, _| true,
        );
        if change == 1 {
            assert!(result.is_ok(), "simple object type is supported");
        } else {
            assert!(
                result.is_err(),
                "unsupported case {change} reached native setup"
            );
        }
    }
    value["actions"]["run"]["output"]["properties"] =
        serde_json::json!({"value":{"type":"integer"}});
    assert!(
        prepare_thing(
            &value.to_string(),
            key,
            &GatewayDefaultV1::LIMITS,
            &mut budget(),
            &[&provider],
            |_, _| true
        )
        .is_err()
    );
    let complete = prepare_thing(
        &text,
        key,
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
        &[&provider],
        |_, _| true,
    )
    .unwrap();
    let mut partial: Vec<_> = complete
        .operations()
        .iter()
        .map(|o| o.definition::<HttpActionDefinition>().unwrap().clone())
        .collect();
    partial.pop();
    assert!(HttpActionRuntime::try_new(&owner(), key, partial, 4).is_err());
    TcpListener::bind(address).unwrap();
}

#[test]
fn late_invalid_result_and_replacement_never_retarget_old_http_correlation() {
    let (old_addr, new_addr) = (addr(), addr());
    let mut owner = owner();
    let old = Rc::new(Counters::default());
    let newer = Rc::new(Counters::default());
    let old_bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let new_bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut first = create(
        &owner,
        key(Generation::INITIAL),
        old_addr,
        None,
        Rc::clone(&old),
        None,
        Arc::clone(&old_bank),
    );
    let old_key = first.key();
    first
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    first.publish(&mut owner, &mut budget()).unwrap();
    let accepted = send(&mut owner, &mut first, old_addr, "POST", "/run", "12");
    let old_wire =
        serde_json::from_str::<serde_json::Value>(accepted.split("\r\n\r\n").nth(1).unwrap())
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
    let id: u32 = old_wire.split_once('-').unwrap().1.parse().unwrap();
    let old_handle = first
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .handle(id)
        .unwrap();
    assert!(first.retire(&mut owner, &mut budget()).is_err());

    let next = key(Generation::INITIAL.checked_next().unwrap());
    let mut second = create(
        &owner,
        next,
        new_addr,
        None,
        Rc::clone(&newer),
        Some(old_key),
        Arc::clone(&new_bank),
    );
    second
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    second.publish(&mut owner, &mut budget()).unwrap();
    assert_eq!(owner.description(old_key), None);
    // Replacement displaces the old logical authority; its still-live native
    // lease cannot turn a stale request into an authorized Action.
    let (stale, report) = send_report(
        &mut owner,
        &mut first,
        old_addr,
        "POST",
        "/run",
        "12",
        &mut budget(),
    );
    assert!(stale.starts_with("HTTP/1.1 410 Gone"), "{stale}");
    assert!(matches!(
        report.request_error,
        Some(CoreError::StaleHandle(_))
    ));
    assert_eq!(old.invoked.get(), 1);
    assert_eq!(newer.invoked.get(), 0);
    let new_accepted = send(&mut owner, &mut second, new_addr, "POST", "/run", "12");
    let new_wire =
        serde_json::from_str::<serde_json::Value>(new_accepted.split("\r\n\r\n").nth(1).unwrap())
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
    assert_ne!(old_wire, new_wire);
    assert!(
        send(
            &mut owner,
            &mut second,
            new_addr,
            "GET",
            &format!("/status?id={old_wire}"),
            ""
        )
        .starts_with("HTTP/1.1 404")
    );
    assert_eq!(
        owner.step_raw_action(&old_handle, &mut budget()).unwrap(),
        Poll::Pending
    );
    assert_eq!(old.cancelled.get(), 1);
    old.complete.set(true);
    old.invalid.set(true);
    assert_eq!(
        owner.step_raw_action(&old_handle, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    assert!(first.retire(&mut owner, &mut budget()).is_err());
    let settlement = first
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .acknowledge(&mut owner, id)
        .unwrap()
        .unwrap();
    assert_eq!(settlement.instance, old_key);
    assert!(matches!(settlement.result, Err(CoreError::Validation(_))));
    first.begin_close();
    first.retire(&mut owner, &mut budget()).unwrap();
    drop(first);
    assert_eq!(
        old_bank
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used(),
        0
    );
    assert!(
        new_bank
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used()
            > 0
    );
    let new_id: u32 = new_wire.split_once('-').unwrap().1.parse().unwrap();
    let new_handle = second
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .handle(new_id)
        .unwrap();
    newer.complete.set(true);
    assert_eq!(
        owner.step_raw_action(&new_handle, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    second.begin_close();
    second
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .acknowledge(&mut owner, new_id)
        .unwrap();
    second.retire(&mut owner, &mut budget()).unwrap();
    drop(second);
    assert_eq!(
        new_bank
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used(),
        0
    );
    TcpListener::bind(old_addr).unwrap();
    TcpListener::bind(new_addr).unwrap();
}

#[test]
fn occupied_action_listener_cannot_publish_and_releases_partial_native_owner() {
    let address = addr();
    let occupied = TcpListener::bind(address).unwrap();
    let mut owner = owner();
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut exposure = create(
        &owner,
        key(Generation::INITIAL),
        address,
        None,
        Rc::new(Counters::default()),
        None,
        Arc::clone(&bank),
    );
    assert!(
        exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .install_closed()
            .is_err()
    );
    assert!(exposure.publish(&mut owner, &mut budget()).is_err());
    assert_eq!(owner.description(exposure.key()), None);
    exposure.begin_close();
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    drop(occupied);
    TcpListener::bind(address).unwrap();
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
}

#[test]
fn terminal_retention_expiry_never_discards_running_action() {
    let address = addr();
    let mut owner = owner();
    let counts = Rc::new(Counters::default());
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut exposure = create(
        &owner,
        key(Generation::INITIAL),
        address,
        None,
        Rc::clone(&counts),
        None,
        Arc::clone(&bank),
    );
    exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    exposure.publish(&mut owner, &mut budget()).unwrap();
    let accepted = send(&mut owner, &mut exposure, address, "POST", "/run", "12");
    let wire =
        serde_json::from_str::<serde_json::Value>(accepted.split("\r\n\r\n").nth(1).unwrap())
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
    assert_eq!(
        exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .progress_actions(&mut owner, &mut budget())
            .unwrap(),
        Poll::Pending
    );
    assert!(
        exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .reap_completed(&mut owner, Duration::ZERO)
            .unwrap()
            .is_none()
    );
    assert!(
        send(
            &mut owner,
            &mut exposure,
            address,
            "GET",
            &format!("/status?id={wire}"),
            ""
        )
        .contains("running")
    );
    counts.complete.set(true);
    assert!(matches!(
        exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .progress_actions(&mut owner, &mut budget()),
        Ok(Poll::Ready(Some(_)))
    ));
    assert!(
        exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .reap_completed(&mut owner, Duration::ZERO)
            .unwrap()
            .is_some()
    );
    assert!(
        send(
            &mut owner,
            &mut exposure,
            address,
            "GET",
            &format!("/status?id={wire}"),
            ""
        )
        .starts_with("HTTP/1.1 404")
    );
    exposure.begin_close();
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
    TcpListener::bind(address).unwrap();
}

#[test]
fn lost_native_and_exposure_guards_keep_validator_and_original_work_until_acknowledged() {
    let address = addr();
    let mut owner = owner();
    let counts = Rc::new(Counters::default());
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let mut exposure = create(
        &owner,
        key(Generation::INITIAL),
        address,
        None,
        Rc::clone(&counts),
        None,
        Arc::clone(&bank),
    );
    let key = exposure.key();
    exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    exposure.publish(&mut owner, &mut budget()).unwrap();
    let accepted = send(&mut owner, &mut exposure, address, "POST", "/run", "12");
    let wire =
        serde_json::from_str::<serde_json::Value>(accepted.split("\r\n\r\n").nth(1).unwrap())
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
    let id: u32 = wire.split_once('-').unwrap().1.parse().unwrap();
    let handle = exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .handle(id)
        .unwrap();
    drop(exposure);
    assert_eq!(owner.description(key), None);
    assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
    assert!(
        owner
            .retire_revoked_publication(key, &mut budget())
            .is_err()
    );
    counts.invalid.set(true);
    counts.complete.set(true);
    assert_eq!(
        owner.step_raw_action(&handle, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    let result = owner.take_raw_action_settlement(&handle).unwrap().unwrap();
    assert!(result.cancel_requested);
    assert!(matches!(result.result, Err(CoreError::Validation(_))));
    owner
        .retire_revoked_publication(key, &mut budget())
        .unwrap();
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
    TcpListener::bind(address).unwrap();
}
