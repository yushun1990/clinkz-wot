use super::*;
use clinkz_wot_core::{consumer::*, notification::*};
use clinkz_wot_servient_finite::notification::*;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    convert::Infallible,
    net::SocketAddr,
    rc::{Rc, Weak},
};

fn mixed(target: &str) -> clinkz_wot_td::thing::Thing {
    let mut td = serde_json::to_value(source(json!([{"href":target}]))).unwrap();
    td["actions"] = json!({"a":{"synchronous":true,"input":{"type":"integer"},"output":{"type":"integer"},"forms":[{"href":target,"op":"invokeaction"}]}});
    serde_json::from_value(td).unwrap()
}
fn finite(
    owner: &ConsumerInstance,
    prepared: PreparedConsumer,
    bank: Arc<HostResourceAccounts>,
) -> ConsumerRuntime {
    let (defs, funding) = prepared.into_execution();
    ConsumerRuntime::try_new(owner.reference(), defs, funding, bank, 2)
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn integer(n: &str) -> Option<Payload> {
    Some(Payload::new(n.as_bytes(), "application/json"))
}

#[test]
fn downstream_write_uses_its_own_native_completion_and_retains_late_success_without_replay() {
    for late in [false, true] {
        let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
        let probe = Arc::new(Probe::default());
        let (peer, target) = udp();
        let mut td = serde_json::to_value(source(json!([{"href":target}]))).unwrap();
        td["properties"]["a"]
            .as_object_mut()
            .unwrap()
            .remove("readOnly");
        let td = serde_json::from_value(td).unwrap();
        let registrations = registry(vec![registration(1, probe.clone())]);
        let prepared = prepare(&registrations, &td, bank.clone());
        assert_eq!(prepared.support().iter().filter(|s| s.selected).count(), 2);
        drop(td);
        drop(registrations);
        let owner = instance();
        let rt = finite(&owner, prepared, bank.clone());
        let mut read = rt
            .read_property("a", Deadline::NONE, now(), &mut work())
            .unwrap();
        assert!(
            read.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
                .is_pending()
        );
        exchange(&peer, b"11");
        drop(poll(&mut read));
        drop(read);
        let mut call = rt
            .write_property(
                "a",
                Payload::new(b"7".as_slice(), "application/json"),
                Deadline::NONE,
                now(),
                &mut work(),
            )
            .unwrap_or_else(|_| panic!());
        assert!(
            call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
                .is_pending()
        );
        let mut bytes = [0; 256];
        let (n, from) = peer.recv_from(&mut bytes).unwrap();
        assert_eq!(&bytes[..n], b"WRITE /temperature 7\n");
        if late {
            probe.late_on_cancel.store(true, Ordering::Relaxed);
            call.cancel();
        }
        peer.send_to(b"WRITTEN", from).unwrap();
        let settlement = poll(&mut call);
        assert_eq!(settlement.result().is_ok(), !late);
        assert_eq!(settlement.operation(), Operation::WriteProperty);
        assert_eq!(settlement.consumer(), &owner.reference());
        assert_eq!(settlement.effect(), ConsumerEffect::Unknown);
        assert!(
            matches!(settlement.native_outcome(),BindingCallSettlement::Returned(Ok(o)) if o.data().is_none() && o.metadata().binding_response().unwrap().status_code()==0)
        );
        assert!(bank.account(ResourceKind::CleanupBytesMax).used() > 0);
        drop(rt);
        assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
        drop(call);
        drop(settlement);
        clean(&bank);
        let port = *probe.ports.lock().unwrap().last().unwrap();
        drop(UdpSocket::bind(port).unwrap());
        peer.set_nonblocking(true).unwrap();
        assert!(
            matches!(peer.recv_from(&mut bytes),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock)
        );
        assert_eq!(
            probe.starts.load(Ordering::Relaxed),
            2,
            "no warm recompilation or replay"
        );
    }
}

#[test]
fn same_named_property_and_action_have_distinct_definitions_inputs_results_and_warm_io() {
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let p = Arc::new(Probe::default());
    let (peer, target) = udp();
    let td = mixed(&target);
    let registrations = registry(vec![registration(1, p.clone())]);
    let prepared = prepare(&registrations, &td, bank.clone());
    assert_eq!(prepared.support().iter().filter(|s| s.selected).count(), 2);
    drop(td);
    drop(registrations);
    assert_eq!(p.compiler_drops.load(Ordering::Relaxed), 1);
    let peer = std::thread::spawn(move || {
        let mut bytes = [0; 256];
        for expected in [
            b"READ /temperature\n".as_slice(),
            b"INVOKE /temperature 7\n",
            b"INVOKE /temperature 8\n",
        ] {
            let (n, from) = peer.recv_from(&mut bytes).unwrap();
            assert_eq!(&bytes[..n], expected);
            peer.send_to(b"11", from).unwrap();
        }
    });
    let owner = instance();
    let rt = finite(&owner, prepared, bank.clone());
    let mut read = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    let result = poll(&mut read);
    assert_eq!(result.operation(), Operation::ReadProperty);
    assert!(matches!(result.affordance(), AffordanceTarget::Property(n) if &**n == "a"));
    drop(result);
    for n in ["7", "8"] {
        let mut call = rt
            .invoke_action("a", integer(n), Deadline::NONE, now(), &mut work())
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let result = poll(&mut call);
        assert_eq!(result.consumer(), &owner.reference());
        assert_eq!(result.operation(), Operation::InvokeAction);
        assert!(matches!(result.affordance(), AffordanceTarget::Action(n) if &**n == "a"));
        assert_eq!(&*result.result().unwrap().data().unwrap().body, b"11");
    }
    assert_eq!(p.starts.load(Ordering::Relaxed), 2, "no warm compilation");
    drop(read);
    drop(rt);
    clean(&bank);
    peer.join().unwrap();
}

#[test]
fn action_roles_refuse_missing_invalid_oversize_unpaid_input_and_unsupported_continuations_before_io()
 {
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let p = Arc::new(Probe::default());
    let (_peer, target) = udp();
    let mut value = serde_json::to_value(mixed(&target)).unwrap();
    value["actions"]["unknown"] = value["actions"]["a"].clone();
    value["actions"]["unknown"]
        .as_object_mut()
        .unwrap()
        .remove("synchronous");
    value["actions"]["continuing"] = value["actions"]["a"].clone();
    value["actions"]["continuing"]["synchronous"] = json!(false);
    value["actions"]["a"]["forms"][0]["op"] =
        json!(["invokeaction", "queryaction", "cancelaction"]);
    let td = serde_json::from_value(value).unwrap();
    let registrations = registry(vec![registration(1, p.clone())]);
    let prepared = prepare(&registrations, &td, bank.clone());
    assert_eq!(prepared.support().iter().filter(|s| s.selected).count(), 2);
    assert_eq!(
        prepared
            .support()
            .iter()
            .filter(|s| matches!(
                s.site,
                clinkz_wot_planning::producer_composition::FormSite::Action(_)
            ) && !s.selected)
            .count(),
        4
    );
    let owner = instance();
    let rt = finite(&owner, prepared, bank.clone());
    let baseline = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    for input in [None, integer("false"), integer(&"7".repeat(1025))] {
        let original = input.clone();
        let error = rt
            .invoke_action("a", input, Deadline::NONE, now(), &mut work())
            .err()
            .unwrap();
        assert_eq!(error.into_input(), original);
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            baseline
        );
    }
    let mut short = work();
    short.set_remaining(WorkClass::CodecInputBytes, 4095);
    assert!(
        rt.invoke_action("a", integer("7"), Deadline::NONE, now(), &mut short)
            .is_err()
    );
    assert!(
        rt.invoke_action(
            "continuing",
            integer("7"),
            Deadline::NONE,
            now(),
            &mut work()
        )
        .is_err()
    );
    assert_eq!(p.native.load(Ordering::Relaxed), 0);
    drop(rt);
    clean(&bank);
}

#[test]
fn action_absent_data_roles_allow_empty_success_but_reject_unexpected_output() {
    for reply in [b"".as_slice(), b"11"] {
        let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
        let p = Arc::new(Probe::default());
        let (peer, target) = udp();
        let mut value = serde_json::to_value(mixed(&target)).unwrap();
        value["actions"]["a"]
            .as_object_mut()
            .unwrap()
            .remove("input");
        value["actions"]["a"]
            .as_object_mut()
            .unwrap()
            .remove("output");
        let td = serde_json::from_value(value).unwrap();
        let registrations = registry(vec![registration(1, p.clone())]);
        let owner = instance();
        let rt = finite(
            &owner,
            prepare(&registrations, &td, bank.clone()),
            bank.clone(),
        );
        let mut call = rt
            .invoke_action("a", None, Deadline::NONE, now(), &mut work())
            .unwrap_or_else(|_| panic!());
        let mut cx = Context::from_waker(Waker::noop());
        let no_codec = || {
            let mut budget = work();
            budget.set_remaining(WorkClass::CodecInputBytes, 0);
            budget
        };
        assert!(call.poll(&mut cx, now(), &mut no_codec()).is_pending());
        let mut bytes = [0; 256];
        let (n, from) = peer.recv_from(&mut bytes).unwrap();
        assert_eq!(&bytes[..n], b"INVOKE /temperature\n");
        peer.send_to(reply, from).unwrap();
        let mut result = None;
        for _ in 0..100 {
            if let Poll::Ready(settlement) = call.poll(&mut cx, now(), &mut no_codec()) {
                result = Some(settlement);
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let result = result.expect("absent output role must settle without codec credit");
        assert_eq!(result.result().is_ok(), reply.is_empty());
        assert_eq!(p.validates.load(Ordering::Relaxed), 0);
        if !reply.is_empty() {
            assert!(result.rejected_output().is_some());
        }
        drop(result);
        drop(call);
        drop(rt);
        clean(&bank);
    }
}

#[test]
fn validated_result_resumes_with_one_authorization_quantum_and_rechecks_between_phases() {
    for change in [
        "none",
        "revoke",
        "drop",
        "cancel",
        "access",
        "configuration",
        "data",
    ] {
        let p = Arc::new(Probe::default());
        let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
        let (peer, target) = udp();
        let registrations = registry(vec![registration(1, p.clone())]);
        let mut owner = Some(instance());
        let original = owner.as_ref().unwrap().reference();
        let rt = finite(
            owner.as_ref().unwrap(),
            prepare(
                &registrations,
                &source(json!([{"href":target}])),
                bank.clone(),
            ),
            bank.clone(),
        );
        let mut call = rt
            .read_property("a", Deadline::NONE, now(), &mut work())
            .unwrap();
        let mut cx = Context::from_waker(Waker::noop());
        let quantum = || {
            let mut budget = work();
            budget.set_remaining(WorkClass::ProviderProbes, 2);
            budget
        };
        assert!(call.poll(&mut cx, now(), &mut quantum()).is_pending());
        exchange(&peer, b"7");
        for _ in 0..100 {
            assert!(call.poll(&mut cx, now(), &mut quantum()).is_pending());
            if p.validates.load(Ordering::Relaxed) != 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            p.validates.load(Ordering::Relaxed),
            1,
            "{change}: result validated once before final-probe yield"
        );
        match change {
            "none" => {}
            "revoke" => owner.as_ref().unwrap().revoke(),
            "drop" => drop(owner.take()),
            "cancel" => call.cancel(),
            "access" => p.denied.store(true, Ordering::Relaxed),
            "configuration" => p.changed.store(true, Ordering::Relaxed),
            "data" => p.data_lost.store(true, Ordering::Relaxed),
            _ => unreachable!(),
        }
        let charged: Vec<_> = ResourceKind::ALL
            .iter()
            .map(|&kind| bank.account(kind).used())
            .collect();
        for credit in [0, 1] {
            let mut budget = WorkBudget::new().with_remaining(WorkClass::ProviderProbes, credit);
            assert!(call.poll(&mut cx, now(), &mut budget).is_pending());
            assert!(!rt.is_idle());
            assert_eq!(p.calls.load(Ordering::Relaxed), 0);
            assert_eq!(p.validates.load(Ordering::Relaxed), 1);
            assert_eq!(
                charged,
                ResourceKind::ALL
                    .iter()
                    .map(|&kind| bank.account(kind).used())
                    .collect::<Vec<_>>()
            );
        }
        // Only the remaining authorization callback has credit. Completed
        // decoding must not be repeated, including when authority/services changed.
        let mut budget = WorkBudget::new().with_remaining(WorkClass::ProviderProbes, 2);
        let Poll::Ready(result) = call.poll(&mut cx, now(), &mut budget) else {
            panic!("{change}: validated result must resume with one authorization quantum");
        };
        assert_eq!(result.consumer(), &original);
        match change {
            "none" => assert!(result.result().is_ok()),
            "revoke" | "drop" => assert!(matches!(result.result(), Err(CoreError::StaleHandle(_)))),
            "cancel" => assert!(matches!(result.result(), Err(CoreError::Cancelled(_)))),
            _ => assert!(matches!(result.result(), Err(CoreError::Validation(_)))),
        }
        assert!(matches!(
            result.native_outcome(),
            BindingCallSettlement::Returned(Ok(_))
        ));
        assert_eq!(p.validates.load(Ordering::Relaxed), 1);
        assert_eq!(p.calls.load(Ordering::Relaxed), 0);
        assert_eq!(
            charged,
            ResourceKind::ALL
                .iter()
                .map(|&kind| bank.account(kind).used())
                .collect::<Vec<_>>()
        );
        drop(result);
        assert_eq!(p.calls.load(Ordering::Relaxed), 1);
        drop(call);
        drop(rt);
        clean(&bank);
    }
}

#[test]
fn instance_and_operation_policy_and_callback_revocation_are_checked_before_effects() {
    struct Scope {
        inner: Arc<dyn ConsumerAccessService>,
        allowed: ConsumerInstanceRef,
        owner: Arc<ConsumerInstance>,
        revoke: std::sync::atomic::AtomicBool,
    }
    impl ConsumerAccessService for Scope {
        fn authorize(
            &self,
            id: BindingRegistrationIdentity,
            target: &str,
            op: Operation,
        ) -> CoreResult<()> {
            self.inner.authorize(id, target, op)
        }
        fn authorize_instance(
            &self,
            consumer: &ConsumerInstanceRef,
            id: BindingRegistrationIdentity,
            target: &str,
            op: Operation,
        ) -> CoreResult<()> {
            if self.revoke.swap(false, Ordering::Relaxed) {
                self.owner.revoke();
            }
            if consumer != &self.allowed || op == Operation::InvokeAction {
                return Err(invalid());
            }
            self.authorize(id, target, op)
        }
        fn work(&self) -> u64 {
            self.inner.work()
        }
        fn retained_bytes(&self) -> u64 {
            self.inner.retained_bytes() + 128
        }
    }
    let owner = Arc::new(instance());
    let foreign = instance();
    let p = Arc::new(Probe::default());
    let (_peer, target) = udp();
    let td = mixed(&target);
    let mut registration = registration(1, p.clone());
    let policy = Arc::new(Scope {
        inner: registration.services.access.clone(),
        allowed: owner.reference(),
        owner: owner.clone(),
        revoke: std::sync::atomic::AtomicBool::new(false),
    });
    registration.services.access = policy.clone();
    let registrations = registry(vec![registration]);
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let rt = finite(
        &owner,
        prepare(&registrations, &td, bank.clone()),
        bank.clone(),
    );
    let other = finite(
        &foreign,
        prepare(&registrations, &td, bank.clone()),
        bank.clone(),
    );
    assert!(
        other
            .read_property("a", Deadline::NONE, now(), &mut work())
            .is_err()
    );
    assert_eq!(
        rt.invoke_action("a", integer("7"), Deadline::NONE, now(), &mut work())
            .err()
            .unwrap()
            .into_input(),
        integer("7")
    );
    policy.revoke.store(true, Ordering::Relaxed);
    assert!(matches!(
        rt.read_property("a", Deadline::NONE, now(), &mut work())
            .err(),
        Some(CoreError::StaleHandle(_))
    ));
    assert_eq!(p.native.load(Ordering::Relaxed), 0);
    drop(rt);
    drop(other);
    clean(&bank);
}

#[test]
fn input_and_result_codec_callbacks_recheck_both_authority_and_changed_policy() {
    struct RevokeData {
        inner: Arc<dyn ConsumerDataService>,
        owner: Arc<ConsumerInstance>,
        denied: Option<Arc<Probe>>,
    }
    impl ConsumerDataService for RevokeData {
        fn content_type(&self) -> &str {
            self.inner.content_type()
        }
        fn validate(
            &self,
            schema: clinkz_wot_core::value_type::TypeSchema,
            payload: &Payload,
        ) -> CoreResult<()> {
            self.inner.validate(schema, payload)?;
            if let Some(probe) = &self.denied {
                probe.denied.store(true, Ordering::Relaxed);
            } else {
                self.owner.revoke();
            }
            Ok(())
        }
        fn policy(&self) -> ConsumerDataPolicy {
            self.inner.policy()
        }
        fn supports_schema(&self, schema: clinkz_wot_core::value_type::TypeSchema) -> bool {
            self.inner.supports_schema(schema)
        }
    }
    for (input_callback, change_policy) in
        [(true, false), (true, true), (false, false), (false, true)]
    {
        let owner = Arc::new(instance());
        let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
        let p = Arc::new(Probe::default());
        let (peer, target) = udp();
        let mut registration = registration(1, p.clone());
        registration.services.data = Arc::new(RevokeData {
            inner: registration.services.data.clone(),
            owner: owner.clone(),
            denied: change_policy.then(|| p.clone()),
        });
        let registrations = registry(vec![registration]);
        let rt = finite(
            &owner,
            prepare(&registrations, &mixed(&target), bank.clone()),
            bank.clone(),
        );
        if input_callback {
            let rejected = rt
                .invoke_action("a", integer("7"), Deadline::NONE, now(), &mut work())
                .err()
                .unwrap();
            assert_eq!(
                matches!(rejected.error().error(), CoreError::StaleHandle(_)),
                !change_policy
            );
            assert_eq!(rejected.into_input(), integer("7"));
            assert_eq!(p.native.load(Ordering::Relaxed), 0);
        } else {
            let peer = std::thread::spawn(move || exchange(&peer, b"7"));
            let mut call = rt
                .read_property("a", Deadline::NONE, now(), &mut work())
                .unwrap();
            let result = poll(&mut call);
            assert!(result.result().is_err());
            assert_eq!(
                matches!(result.result(), Err(CoreError::StaleHandle(_))),
                !change_policy
            );
            assert!(matches!(
                result.native_outcome(),
                BindingCallSettlement::Returned(Ok(_))
            ));
            peer.join().unwrap();
        }
        drop(rt);
        clean(&bank);
    }
}

#[test]
fn cancelled_action_retains_original_input_late_result_native_socket_and_charges_until_acknowledged()
 {
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let p = Arc::new(Probe::default());
    p.late_on_cancel.store(true, Ordering::Relaxed);
    let (peer, target) = udp();
    let registrations = registry(vec![registration(1, p.clone())]);
    let owner = instance();
    let rt = finite(
        &owner,
        prepare(&registrations, &mixed(&target), bank.clone()),
        bank.clone(),
    );
    let baseline = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    let mut call = rt
        .invoke_action("a", integer("7"), Deadline::NONE, now(), &mut work())
        .unwrap_or_else(|_| panic!());
    let mut native_only = work();
    native_only.set_remaining(WorkClass::CodecInputBytes, 0);
    assert!(
        call.poll(
            &mut Context::from_waker(Waker::noop()),
            now(),
            &mut native_only
        )
        .is_pending()
    );
    let mut bytes = [0; 256];
    let (n, from) = peer.recv_from(&mut bytes).unwrap();
    assert_eq!(&bytes[..n], b"INVOKE /temperature 7\n");
    call.cancel();
    peer.send_to(b"17", from).unwrap();
    let late = poll(&mut call);
    assert_eq!(late.operation(), Operation::InvokeAction);
    assert_eq!(late.effect(), ConsumerEffect::Unknown);
    assert!(matches!(late.result(), Err(CoreError::Cancelled(_))));
    assert!(matches!(
        late.native_outcome(),
        BindingCallSettlement::Returned(Ok(_))
    ));
    assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > baseline);
    assert_eq!(
        p.calls.load(Ordering::Relaxed),
        0,
        "settlement owns original native work"
    );
    drop(late);
    assert_eq!(p.calls.load(Ordering::Relaxed), 1);
    drop(UdpSocket::bind(from).expect("original socket physically released"));
    drop(call);
    drop(rt);
    clean(&bank);
}

// A test-only native observation facet. The Binding owns the shared socket,
// matching keys and bounded per-participation queues. Core sees no datagram case.
type Inbox = Rc<RefCell<VecDeque<Payload>>>;
struct Hub {
    socket: UdpSocket,
    peer: SocketAddr,
    next: Cell<u64>,
    receivers: RefCell<BTreeMap<u64, Weak<RefCell<VecDeque<Payload>>>>>,
    deny: Cell<bool>,
    hook: RefCell<Option<Box<dyn FnOnce()>>>,
    closes: Cell<usize>,
}
#[derive(Clone)]
struct Observe(Rc<Hub>);
struct Participation {
    hub: Rc<Hub>,
    inbox: Inbox,
    key: u64,
    id: NotificationIdentity,
}
impl ConsumerNotificationBinding for Observe {
    type Program = ();
    type Configuration = u8;
    type Participation = Participation;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(70), BindingGeneration::INITIAL)
    }
    fn configuration(&self) -> &u8 {
        &1
    }
    fn support(&self, _: &(), kind: NotificationKind) -> CoreResult<NotificationSupport> {
        assert_eq!(kind, NotificationKind::PropertyObservation);
        Ok(NotificationSupport {
            event_responses: false,
            cancellation: false,
        })
    }
    fn prepare(
        &mut self,
        _: (),
        id: &NotificationIdentity,
    ) -> Result<Participation, BindingInputRejection<()>> {
        let key = self.0.next.get();
        self.0.next.set(key.checked_add(1).unwrap());
        let inbox = Rc::new(RefCell::new(VecDeque::with_capacity(2)));
        self.0
            .receivers
            .borrow_mut()
            .insert(key, Rc::downgrade(&inbox));
        Ok(Participation {
            hub: self.0.clone(),
            inbox,
            key,
            id: id.clone(),
        })
    }
}
impl NotificationParticipation for Participation {
    type Response = Infallible;
    fn poll_start(&mut self, _: &mut Context<'_>, _: &SubscriptionInput) -> Poll<CoreResult<()>> {
        self.hub
            .socket
            .send_to(format!("OBSERVE {}", self.key).as_bytes(), self.hub.peer)
            .unwrap();
        Poll::Ready(Ok(()))
    }
    fn poll_next(
        &mut self,
        _: &mut Context<'_>,
        _: usize,
    ) -> Poll<CoreResult<NotificationSignal<Infallible>>> {
        let mut bytes = [0; 128];
        match self.hub.socket.recv_from(&mut bytes) {
            Ok((n, from)) => {
                assert_eq!(from, self.hub.peer);
                let (key, value) = std::str::from_utf8(&bytes[..n])
                    .unwrap()
                    .split_once(':')
                    .unwrap();
                if let Some(inbox) = self
                    .hub
                    .receivers
                    .borrow()
                    .get(&key.parse().unwrap())
                    .and_then(Weak::upgrade)
                {
                    let mut inbox = inbox.borrow_mut();
                    assert!(inbox.len() < 2, "native test envelope");
                    inbox.push_back(Payload::new(value.as_bytes(), "application/json"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            other => panic!("{other:?}"),
        }
        if let Some(hook) = self.hub.hook.borrow_mut().take() {
            hook();
        }
        self.inbox
            .borrow_mut()
            .pop_front()
            .map_or(Poll::Pending, |data| {
                Poll::Ready(Ok(NotificationSignal::Data(NativeNotification {
                    recipient: self.id.clone(),
                    data: Some(data),
                    response: None,
                })))
            })
    }
    fn poll_close(
        &mut self,
        _: &mut Context<'_>,
        cancel: Option<&CancellationInput>,
    ) -> Poll<CoreResult<RemoteCancellation>> {
        assert!(cancel.is_none());
        self.hub.closes.set(self.hub.closes.get() + 1);
        Poll::Ready(Ok(RemoteCancellation::Unsupported))
    }
    fn abort_local(self) {
        self.hub.receivers.borrow_mut().remove(&self.key);
    }
}
struct Access(Rc<Hub>);
impl NotificationAuthorizer for Access {
    fn authorize(
        &self,
        id: &NotificationIdentity,
        _: &AffordanceTarget,
        _: NotificationPermission,
    ) -> CoreResult<()> {
        assert!(id.consumer.is_current());
        if self.0.deny.get() {
            Err(invalid())
        } else {
            Ok(())
        }
    }
}
fn observations(
    owner: &ConsumerInstance,
    hub: Rc<Hub>,
) -> RawNotificationServient<Observe, Access> {
    RawNotificationServient::try_new(
        Observe(hub.clone()),
        1,
        Access(hub),
        owner.reference(),
        RawNotificationLimits {
            subscriptions: 2,
            queue_items: 2,
            queue_payload_bytes: 1024,
            payload_backing_bytes: 512,
            control_backing_bytes: 0,
            target_name_bytes: 256,
            engine_bytes: 32768,
        },
    )
    .unwrap_or_else(|_| panic!())
}
fn hub(peer: SocketAddr) -> Rc<Hub> {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.set_nonblocking(true).unwrap();
    Rc::new(Hub {
        socket,
        peer,
        next: Cell::new(0),
        receivers: RefCell::new(BTreeMap::new()),
        deny: Cell::new(false),
        hook: RefCell::new(None),
        closes: Cell::new(0),
    })
}
fn subscribe(root: &mut RawNotificationServient<Observe, Access>) -> NotificationCursor {
    root.subscribe(
        RawNotificationRequest {
            target: AffordanceTarget::Property(Arc::from("a")),
            subscription: SubscriptionInput(None),
            responses: ResponseRequirement::None,
            overflow: NotificationOverflow::Terminate,
        },
        (),
        &mut work(),
    )
    .unwrap_or_else(|_| panic!())
}
fn progress(
    root: &mut RawNotificationServient<Observe, Access>,
    cursor: &NotificationCursor,
) -> NotificationState {
    root.progress(
        cursor.identity(),
        &mut Context::from_waker(Waker::noop()),
        &mut work(),
    )
    .unwrap()
}
fn deliver(root: &mut RawNotificationServient<Observe, Access>, cursor: &mut NotificationCursor) {
    let start = Instant::now();
    loop {
        progress(root, cursor);
        if let Some(view) = root.front(cursor, &mut work()).unwrap() {
            assert_eq!(&*view.data.unwrap().body, b"17");
            return;
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::yield_now();
    }
}

#[test]
fn owner_cleanup_drains_external_revocation_with_retained_cursor_and_preserves_sibling() {
    for cause in ["revoke", "drop", "stop_then_revoke"] {
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        let hub = hub(peer.local_addr().unwrap());
        let local = hub.socket.local_addr().unwrap();
        let mut owner = Some(instance());
        let sibling_owner = instance();
        let mut root = observations(owner.as_ref().unwrap(), hub.clone());
        let baseline = root.reserved_engine_bytes();
        let cursor = subscribe(&mut root);
        assert_eq!(progress(&mut root, &cursor), NotificationState::Active);
        let mut sibling = observations(&sibling_owner, hub.clone());
        let mut sibling_cursor = subscribe(&mut sibling);
        assert_eq!(
            progress(&mut sibling, &sibling_cursor),
            NotificationState::Active
        );
        match cause {
            "revoke" => owner.as_ref().unwrap().revoke(),
            "drop" => drop(owner.take()),
            "stop_then_revoke" => {
                cursor.stop_handle().request_stop();
                owner.as_ref().unwrap().revoke();
            }
            _ => unreachable!(),
        }
        let mut cx = Context::from_waker(Waker::noop());
        assert_eq!(
            root.status(cursor.identity()).unwrap().state,
            NotificationState::Closing
        );
        assert!(root.step_cleanup(&mut cx, &mut WorkBudget::new()).is_err());
        assert_eq!(hub.closes.get(), 0);
        assert_eq!(root.retained_subscriptions(), 1);
        let mut result = None;
        for _ in 0..8 {
            if let Some(settlement) = root.step_cleanup(&mut cx, &mut work()).unwrap() {
                result = Some(settlement);
                break;
            }
        }
        let result = result.expect("owner cleanup must observe independent Consumer revocation");
        assert_eq!(
            &result.identity,
            cursor.identity(),
            "the original cursor is still retained"
        );
        if cause == "stop_then_revoke" {
            assert!(matches!(result.first_cause, Some(CoreError::Cancelled(_))));
        } else {
            assert!(matches!(
                result.first_cause,
                Some(CoreError::StaleHandle(_))
            ));
        }
        assert_eq!(result.remote_cancellation, RemoteCancellation::Unsupported);
        assert_eq!(hub.closes.get(), 1);
        assert_eq!(root.retained_subscriptions(), 0);
        assert_eq!(root.reserved_engine_bytes(), baseline);
        assert!(root.status(cursor.identity()).is_err());
        assert_eq!(hub.receivers.borrow().len(), 1);
        assert!(hub.receivers.borrow().contains_key(&1));
        peer.send_to(b"1:17", local).unwrap();
        deliver(&mut sibling, &mut sibling_cursor);
        drop(root);
        drop(sibling);
        drop(hub);
        drop(UdpSocket::bind(local).expect("shared physical owner releases its final socket"));
    }
}

#[test]
fn one_consumer_fences_calls_and_notifications_without_retargeting_replacement_or_stopping_siblings()
 {
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let p = Arc::new(Probe::default());
    p.late_on_cancel.store(true, Ordering::Relaxed);
    let (peer, target) = udp();
    let observation_peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    observation_peer
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let hub = hub(observation_peer.local_addr().unwrap());
    let local = hub.socket.local_addr().unwrap();
    let owner = instance();
    let sibling_owner = instance();
    assert_ne!(
        owner.reference(),
        sibling_owner.reference(),
        "same numeric slot is not the same incarnation"
    );
    let registrations = registry(vec![registration(1, p.clone())]);
    let td = mixed(&target);
    let rt = finite(
        &owner,
        prepare(&registrations, &td, bank.clone()),
        bank.clone(),
    );
    let mut first = observations(&owner, hub.clone());
    let mut sibling = observations(&sibling_owner, hub.clone());
    let mut a = subscribe(&mut first);
    let mut b = subscribe(&mut sibling);
    assert_eq!(a.identity().consumer, rt.consumer());
    assert_eq!(progress(&mut first, &a), NotificationState::Active);
    assert_eq!(progress(&mut sibling, &b), NotificationState::Active);
    assert!(
        first.front(&mut a, &mut work()).unwrap().is_none(),
        "no invented initial value"
    );
    let mut bytes = [0; 128];
    for _ in 0..2 {
        observation_peer.recv_from(&mut bytes).unwrap();
    }
    observation_peer.send_to(b"0:17", local).unwrap();
    deliver(&mut first, &mut a);
    let peer = std::thread::spawn(move || exchange(&peer, b"7"));
    let mut call = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(
        call.poll(&mut cx, now(), &mut WorkBudget::new())
            .is_pending()
    );
    let mut native_only = work();
    native_only.set_remaining(WorkClass::CodecInputBytes, 0);
    assert!(call.poll(&mut cx, now(), &mut native_only).is_pending());
    peer.join().unwrap();
    drop(owner); // sole logical authority; all retained references now fence
    assert!(first.front(&mut a, &mut WorkBudget::new()).is_err());
    let late = poll(&mut call);
    assert!(matches!(
        late.first_cause(),
        Some(CoreError::StaleHandle(_))
    ));
    assert!(matches!(
        late.native_outcome(),
        BindingCallSettlement::Returned(Ok(_))
    ));
    let replacement = instance();
    assert_ne!(late.consumer(), &replacement.reference());
    let mut replacement_root = observations(&replacement, hub.clone());
    assert!(
        replacement_root.front(&mut a, &mut work()).is_err(),
        "foreign cursor cannot retarget"
    );
    drop(replacement_root);
    for _ in 0..6 {
        progress(&mut first, &a);
    }
    let closed = first.take_settlement(a.identity()).unwrap();
    assert_eq!(closed.remote_cancellation, RemoteCancellation::Unsupported);
    assert!(matches!(
        closed.first_cause,
        Some(CoreError::StaleHandle(_))
    ));
    assert!(hub.receivers.borrow().contains_key(&1));
    observation_peer.send_to(b"1:17", local).unwrap();
    deliver(&mut sibling, &mut b);
    rt.begin_close();
    assert!(sibling_owner.reference().is_current());
    drop(late);
    drop(call);
    drop(rt);
    drop(first);
    drop(sibling);
    drop(hub);
    clean(&bank);
    drop(UdpSocket::bind(local).expect("shared socket released after final native owner"));
}

#[test]
fn closing_finite_facet_keeps_same_consumers_notification_live_and_callback_revoke_fences_late_data()
 {
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let p = Arc::new(Probe::default());
    let (_peer, target) = udp();
    let owner = Rc::new(instance());
    let registrations = registry(vec![registration(1, p)]);
    let rt = finite(
        &owner,
        prepare(&registrations, &mixed(&target), bank.clone()),
        bank.clone(),
    );
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    let hub = hub(peer.local_addr().unwrap());
    let local = hub.socket.local_addr().unwrap();
    let mut notifications = observations(&owner, hub.clone());
    let mut cursor = subscribe(&mut notifications);
    progress(&mut notifications, &cursor);
    rt.begin_close();
    drop(rt);
    peer.send_to(b"0:17", local).unwrap();
    deliver(&mut notifications, &mut cursor);
    notifications.advance(&mut cursor, &mut work()).unwrap();
    let revoke = owner.clone();
    *hub.hook.borrow_mut() = Some(Box::new(move || revoke.revoke()));
    peer.send_to(b"0:99", local).unwrap();
    assert_eq!(
        progress(&mut notifications, &cursor),
        NotificationState::Closing
    );
    assert!(notifications.front(&mut cursor, &mut work()).is_err());
    assert!(
        notifications
            .status(cursor.identity())
            .unwrap()
            .has_retained_handoff
    );
    for _ in 0..6 {
        progress(&mut notifications, &cursor);
    }
    assert!(matches!(
        notifications
            .take_settlement(cursor.identity())
            .unwrap()
            .first_cause,
        Some(CoreError::StaleHandle(_))
    ));
    clean(&bank);
}

struct ObserveAccess;
impl ConsumerAccessService for ObserveAccess {
    fn authorize(
        &self,
        id: BindingRegistrationIdentity,
        endpoint: &str,
        op: Operation,
    ) -> CoreResult<()> {
        assert_eq!(id.binding_id(), BindingId::new(70));
        assert!(endpoint.starts_with("testobserve://127.0.0.1:"));
        assert!(matches!(
            op,
            Operation::ObserveProperty | Operation::UnobserveProperty
        ));
        Ok(())
    }
    fn authorize_instance(
        &self,
        consumer: &ConsumerInstanceRef,
        id: BindingRegistrationIdentity,
        endpoint: &str,
        op: Operation,
    ) -> CoreResult<()> {
        assert!(consumer.is_current());
        self.authorize(id, endpoint, op)
    }
    fn work(&self) -> u64 {
        1
    }
    fn retained_bytes(&self) -> u64 {
        0
    }
}
// A downstream Cargo-linked Binding implements the same typed Property facet.
impl ConsumerNotificationPreparation for Observe {
    fn registration_identity(&self) -> BindingRegistrationIdentity {
        BindingRegistrationIdentity::new(
            BindingId::new(70),
            BindingGeneration::INITIAL,
            BindingConfigurationDigest::new([70; 32]),
            BindingArtifactCompatibility::new(*b"downstream-obs01"),
            0,
        )
    }
    fn preparation_bounds(&self) -> ConsumerPreparationBounds {
        ConsumerPreparationBounds {
            work: 256,
            temporary_bytes: 256,
            retained_bytes: 256,
        }
    }
    fn compile_notification(&self, input: ConsumerNotificationInput<'_>) -> CoreResult<Option<()>> {
        assert_eq!(input.target, &AffordanceTarget::Property(Arc::from("a")));
        assert_eq!(input.schemas.kind(), NotificationKind::PropertyObservation);
        assert_eq!(input.endpoint, format!("testobserve://{}", self.0.peer));
        assert!(input.document.property().observable);
        Ok(Some(()))
    }
}
#[test]
fn downstream_property_preparation_uses_real_datagrams_after_source_and_registry_drop() {
    use clinkz_wot_servient_finite::consumer_notification::ConsumerNotificationRuntime;
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let hub = hub(peer.local_addr().unwrap());
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let definition = {
        let mut td = serde_json::to_value(source(
            json!([{"href":format!("testobserve://{}",hub.peer),"op":"observeproperty"}]),
        ))
        .unwrap();
        td["properties"]["a"]["observable"] = json!(true);
        let thing = serde_json::from_value(td).unwrap();
        let registry =
            ConsumerNotificationRegistry::try_new(vec![ConsumerNotificationRegistration {
                binding: Observe(hub.clone()),
                services: ConsumerServices {
                    data: services(Arc::new(Probe::default())).data,
                    access: Arc::new(ObserveAccess),
                },
            }])
            .unwrap();
        registry
            .prepare_notification(
                &thing,
                &AffordanceTarget::Property(Arc::from("a")),
                ConsumerSelector::default(),
                &GatewayDefaultV1::LIMITS,
                bank.clone(),
                &mut work(),
            )
            .unwrap()
    };
    let owner = instance();
    let mut root = ConsumerNotificationRuntime::try_new(
        owner.reference(),
        definition,
        RawNotificationLimits {
            subscriptions: 2,
            queue_items: 2,
            queue_payload_bytes: 1024,
            payload_backing_bytes: 512,
            control_backing_bytes: 0,
            target_name_bytes: 256,
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
    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(
        root.progress(cursor.identity(), &mut cx, &mut work())
            .unwrap(),
        NotificationState::Active
    );
    let mut wire = [0; 128];
    let (n, from) = peer.recv_from(&mut wire).unwrap();
    assert_eq!(&wire[..n], b"OBSERVE 0");
    assert!(root.front(&mut cursor, &mut work()).unwrap().is_none());
    peer.send_to(b"0:17", from).unwrap();
    let start = Instant::now();
    loop {
        root.progress(cursor.identity(), &mut cx, &mut work())
            .unwrap();
        if let Some(view) = root.front(&mut cursor, &mut work()).unwrap() {
            assert_eq!(&*view.data.unwrap().body, b"17");
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    root.advance(&mut cursor, &mut work()).unwrap();
    root.stop(&cursor, None, &mut work())
        .unwrap_or_else(|_| panic!());
    while root
        .progress(cursor.identity(), &mut cx, &mut work())
        .unwrap()
        != NotificationState::Closed
    {
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    assert_eq!(
        root.take_settlement(cursor.identity())
            .unwrap()
            .delivered_items,
        1
    );
    drop(root);
    clean(&bank);
}
