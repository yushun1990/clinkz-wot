//! A deliberately nonstandard test-native group protocol. TD-prepared Consumer
//! and raw Producer exercise richer outcomes without borrowing HTTP semantics.
use super::*;
use clinkz_wot_core::{collection::*, consumer::ConsumerEffect, finite::*, producer::*};
use clinkz_wot_servient_finite::*;
use std::{
    cell::{Cell, RefCell},
    net::SocketAddr,
    rc::Rc,
};

fn source_for(target: &str) -> clinkz_wot_td::thing::Thing {
    serde_json::from_value(json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"group fixture",
        "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
        "properties":{
            "a":{"type":"boolean","forms":[{"href":"unsupported:a"}]},
            "b":{"type":"string","writeOnly":true,"forms":[{"href":"unsupported:b"}]},
            "c":{"type":"integer","forms":[{"href":"unsupported:c"}]}
        },
        "forms":[{"op":"writemultipleproperties","href":target}]
    }))
    .unwrap()
}
fn schema() -> Arc<PropertyCollectionSchema> {
    use data_schema::{PreparedSchema, ValueType};
    Arc::new(
        PropertyCollectionSchema::try_new(
            [
                ("a", ValueType::Boolean),
                ("b", ValueType::String),
                ("c", ValueType::Integer),
            ]
            .into_iter()
            .map(|(name, t)| PropertyMemberSchema {
                name: name.into(),
                schema: PreparedSchema::Type(t),
            })
            .collect(),
        )
        .unwrap(),
    )
}
fn report(
    s: &PropertyCollectionSchema,
    bits: u32,
    done: u32,
    refused: u32,
    atomicity: WriteAtomicity,
) -> PropertyWriteReport {
    PropertyWriteReport::try_new(
        s.selection_from_bits(bits).unwrap(),
        done,
        refused,
        atomicity,
    )
    .unwrap()
}
fn client_for(
    target: &str,
    probe: Arc<Probe>,
    bank: Arc<HostResourceAccounts>,
    owner: &ConsumerInstance,
) -> (StandaloneConsumerAssociation, ConsumerCalls) {
    let td = source_for(target);
    let registry = registry(vec![registration(1, probe)]);
    let p = prepare(&registry, &td, bank.clone());
    let (d, f) = p.into_execution();
    assert_eq!(d.len(), 1);
    drop(td);
    drop(registry);
    StandaloneConsumerAssociation::try_new(owner.reference(), d, f, bank.clone())
        .map(|association| (association, ConsumerCalls::try_new(bank, 1).unwrap()))
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn write(rt: &(StandaloneConsumerAssociation, ConsumerCalls), bytes: &[u8]) -> ConsumerCall {
    rt.0.write_multiple_properties(
        &rt.1,
        Payload::new(bytes, "application/json"),
        Deadline::NONE,
        now(),
        &mut work(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()))
}

#[derive(Clone, Eq, PartialEq)]
struct Config {
    address: SocketAddr,
    key: ProducerInstanceKey,
    schema: Arc<PropertyCollectionSchema>,
}
struct Native {
    config: Config,
    socket: Option<UdpSocket>,
    sent: bool,
    hold: Rc<Cell<bool>>,
}
impl FiniteValidator for Native {
    fn additional_validation_work(&self) -> u64 {
        4096 * 80
    }
    fn validate_input(&self, _: &FiniteCandidate, input: &InteractionInput) -> CoreResult<()> {
        clinkz_wot_codec_json::validate_property_write_map(
            &self.config.schema,
            input.data.as_ref().unwrap(),
        )
        .map(|_| ())
    }
    fn validate_output(
        &self,
        _: &FiniteCandidate,
        input: &InteractionInput,
        output: &InteractionOutput,
    ) -> CoreResult<()> {
        let selected = clinkz_wot_codec_json::validate_property_write_map(
            &self.config.schema,
            input.data.as_ref().unwrap(),
        )?;
        output
            .metadata()
            .property_write_report()
            .unwrap()
            .validate_selection(selected)
    }
}
impl FiniteIngressBinding for Native {
    type Configuration = Config;
    type Reply = SocketAddr;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(201), BindingGeneration::INITIAL)
    }
    fn configuration(&self) -> &Config {
        &self.config
    }
    fn validator(&self) -> Option<&dyn FiniteValidator> {
        Some(self)
    }
    fn poll_start(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
        let socket = UdpSocket::bind(self.config.address).unwrap();
        socket.set_nonblocking(true).unwrap();
        self.socket = Some(socket);
        Poll::Ready(Ok(()))
    }
    fn poll_receive(
        &mut self,
        _: &mut Context<'_>,
        _: MonotonicInstant,
    ) -> Poll<CoreResult<FiniteIngress<SocketAddr>>> {
        let mut bytes = [0; 1025];
        match self.socket.as_ref().unwrap().recv_from(&mut bytes) {
            Ok((n, from)) if n <= 1024 => {
                let body = bytes[..n]
                    .strip_prefix(b"GROUP /temperature ")
                    .unwrap()
                    .strip_suffix(b"\n")
                    .unwrap();
                self.sent = false;
                Poll::Ready(Ok(FiniteIngress {
                    candidates: vec![FiniteCandidate {
                        instance: self.config.key,
                        target: AffordanceTarget::Thing,
                        operation: Operation::WriteMultipleProperties,
                    }],
                    input: InteractionInput::with_data(Payload::new(body, "application/json")),
                    reply: from,
                }))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Poll::Pending,
            _ => panic!("bounded test-native ingress"),
        }
    }
    fn poll_send(
        &mut self,
        _: &mut Context<'_>,
        reply: &RawFiniteReply<SocketAddr>,
    ) -> Poll<CoreResult<()>> {
        if self.hold.get() {
            return Poll::Pending;
        }
        let output = reply.result.as_ref().unwrap();
        let r = output.metadata().property_write_report().unwrap();
        let done = (0..3)
            .filter(|&i| r.effect(i) == Some(MemberWriteEffect::Completed))
            .fold(0, |bits, i| bits | (1 << i));
        let refused = (0..3)
            .filter(|&i| r.effect(i) == Some(MemberWriteEffect::NotApplied))
            .fold(0, |bits, i| bits | (1 << i));
        let atomicity = match r.atomicity() {
            WriteAtomicity::Unspecified => "U",
            WriteAtomicity::Atomic => "A",
            WriteAtomicity::NonAtomic => "N",
        };
        let mut bytes = format!(
            "REPORT {} {done} {refused} {atomicity}",
            r.selection().bits()
        );
        for i in 0..3 {
            bytes.push(' ');
            bytes.push_str(
                &r.native_status(i)
                    .map_or_else(|| "x".into(), |s| s.to_string()),
            );
        }
        assert_eq!(
            self.socket
                .as_ref()
                .unwrap()
                .send_to(bytes.as_bytes(), reply.ingress.reply)
                .unwrap(),
            bytes.len()
        );
        self.sent = true;
        Poll::Ready(Ok(()))
    }
    fn poll_close(
        &mut self,
        _: &mut Context<'_>,
        _: Option<&FiniteIngress<SocketAddr>>,
        _: Option<&RawFiniteReply<SocketAddr>>,
        _: Option<&CoreError>,
    ) -> Poll<CoreResult<FiniteDelivery>> {
        self.socket.take();
        Poll::Ready(Ok(if self.sent {
            FiniteDelivery::Delivered
        } else {
            FiniteDelivery::NotDelivered
        }))
    }
}
struct App {
    schema: Arc<PropertyCollectionSchema>,
    report: Cell<PropertyWriteReport>,
    applied: RefCell<Value>,
    calls: Cell<usize>,
}
struct Behavior(Rc<App>);
impl RawFiniteHandler for Behavior {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn handle(
        &self,
        _: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.0.calls.set(self.0.calls.get() + 1);
        let values: Value = serde_json::from_slice(&input.data.as_ref().unwrap().body).unwrap();
        let r = self.0.report.get();
        for (i, m) in self.0.schema.members().iter().enumerate() {
            if r.effect(i) == Some(MemberWriteEffect::Completed) {
                self.0.applied.borrow_mut()[m.name.as_ref()] = values[m.name.as_ref()].clone();
            }
        }
        Ok(InteractionOutput::empty()
            .try_with_metadata(InteractionOutputMetadata::default().with_property_write_report(r))
            .unwrap())
    }
}

#[test]
fn real_datagram_group_preserves_member_status_atomicity_and_late_evidence_in_both_roles() {
    for fault in ["mixed", "atomic-refusal", "complete", "cancel", "replace"] {
        let s = schema();
        let r = match fault {
            "atomic-refusal" => report(&s, 7, 0, 7, WriteAtomicity::Atomic)
                .with_native_status(0, 424)
                .unwrap()
                .with_native_status(1, 409)
                .unwrap()
                .with_native_status(2, 424)
                .unwrap(),
            "complete" => report(&s, 7, 7, 0, WriteAtomicity::Unspecified),
            _ => report(&s, 7, 1, 2, WriteAtomicity::NonAtomic)
                .with_native_status(0, 0)
                .unwrap()
                .with_native_status(1, 712)
                .unwrap()
                .with_native_status(2, 900)
                .unwrap(),
        };
        let app = Rc::new(App {
            schema: s.clone(),
            report: Cell::new(r),
            applied: RefCell::new(json!({})),
            calls: Cell::new(0),
        });
        let authority = ProducerInstanceAuthority::try_new(ThingSlotId::new(
            SlotIndex::new(8),
            Generation::INITIAL,
        ))
        .unwrap();
        let key = authority.key();
        let mut producer = RawProducerServient::try_new(
            vec![RawProducerInstance::new(
                authority,
                vec![RawFiniteBehavior::new(
                    AffordanceTarget::Thing,
                    Operation::WriteMultipleProperties,
                    Behavior(app.clone()),
                )],
            )],
            RawProducerRegistryLimits {
                instances: 1,
                behaviors_per_instance: 1,
                target_name_bytes: 256,
            },
        )
        .unwrap_or_else(|_| panic!());
        let reserved = UdpSocket::bind("127.0.0.1:0").unwrap();
        let address = reserved.local_addr().unwrap();
        drop(reserved);
        let config = Config {
            address,
            key,
            schema: s,
        };
        let hold = Rc::new(Cell::new(matches!(fault, "cancel" | "replace")));
        let mut native = RawFiniteRuntime::try_new(
            Native {
                config: config.clone(),
                socket: None,
                sent: false,
                hold: hold.clone(),
            },
            config,
            &producer,
            RawFiniteLimits {
                candidates: 1,
                input_body_bytes: 1024,
                output_body_bytes: 1024,
                output_media_capacity: 16,
                output_retained_bytes: 4096,
            },
            FiniteValidation::Required,
            Deadline::NONE,
        )
        .unwrap_or_else(|_| panic!());
        assert_eq!(
            native.step(&mut Context::from_waker(Waker::noop()), &mut work(), now()),
            Poll::Ready(RawFiniteState::InstalledClosed)
        );
        native.activate_raw().unwrap();
        let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
        let probe = Arc::new(Probe::default());
        let owner = instance();
        let rt = client_for(
            &format!("linkedudp://{address}/temperature"),
            probe.clone(),
            bank.clone(),
            &owner,
        );
        let mut call = write(&rt, br#"{"a":true,"b":"new","c":7}"#);
        let start = Instant::now();
        let mut remote = None;
        loop {
            let _ = producer
                .step(
                    &mut native,
                    &mut Context::from_waker(Waker::noop()),
                    &mut work(),
                    now(),
                )
                .unwrap();
            assert!(
                call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
                    .is_pending()
            );
            if app.calls.get() == 1 {
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(3));
        }
        if matches!(fault, "cancel" | "replace") {
            probe.late_on_cancel.store(true, Ordering::Relaxed);
            if fault == "cancel" {
                call.cancel();
            } else {
                owner.revoke();
            }
            let retained = ResourceKind::ALL.map(|k| bank.account(k).used());
            assert!(
                call.poll(
                    &mut Context::from_waker(Waker::noop()),
                    now(),
                    &mut WorkBudget::new()
                )
                .is_pending()
            );
            assert_eq!(retained, ResourceKind::ALL.map(|k| bank.account(k).used()));
            hold.set(false);
        }
        let local = loop {
            let _ = producer
                .step(
                    &mut native,
                    &mut Context::from_waker(Waker::noop()),
                    &mut work(),
                    now(),
                )
                .unwrap();
            if let Some(s) = native.take_settlement() {
                remote = Some(s);
            }
            if let Poll::Ready(s) =
                call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            {
                break s;
            }
            assert!(start.elapsed() < Duration::from_secs(3));
        };
        let remote = remote.unwrap();
        let (original_schema, selected) = local.property_write_members().unwrap();
        assert_eq!(
            original_schema
                .members()
                .iter()
                .map(|m| m.name.as_ref())
                .collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        assert_eq!(selected.bits(), 7);
        assert_eq!(
            remote.result.unwrap().metadata().property_write_report(),
            Some(&r)
        );
        assert_eq!(
            remote.effect,
            if fault == "complete" {
                FiniteEffect::Completed
            } else {
                FiniteEffect::Reported
            }
        );
        assert_eq!(remote.delivery, FiniteDelivery::Delivered);
        assert!(
            matches!(local.native_outcome(),ConsumerNativeOutcome::Returned(Ok(o)) if o.metadata().property_write_report()==Some(&r))
        );
        assert_eq!(local.result().is_ok(), fault == "complete");
        assert_eq!(
            local.effect(),
            if fault == "complete" {
                ConsumerEffect::Completed
            } else {
                ConsumerEffect::Reported
            }
        );
        assert_eq!(
            local.first_cause().is_some(),
            matches!(fault, "cancel" | "replace")
        );
        assert_eq!(app.calls.get(), 1);
        if fault == "atomic-refusal" {
            assert_eq!(*app.applied.borrow(), json!({}));
        } else if fault != "complete" {
            assert_eq!(*app.applied.borrow(), json!({"a":true}));
        }
        let port = *probe.ports.lock().unwrap().last().unwrap();
        drop(rt);
        drop(call);
        assert!(bank.account(ResourceKind::CleanupBytesMax).used() > 0);
        assert!(UdpSocket::bind(port).is_err());
        drop(local);
        clean(&bank);
        drop(UdpSocket::bind(port).unwrap());
        native.begin_close();
        let _ = native.step(&mut Context::from_waker(Waker::noop()), &mut work(), now());
        drop(native);
        drop(UdpSocket::bind(address).unwrap());
    }
}

#[test]
fn unrequested_native_member_report_is_retained_as_rejected_output() {
    let (peer, target) = udp();
    let probe = Arc::new(Probe::default());
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let owner = instance();
    let rt = client_for(&target, probe.clone(), bank.clone(), &owner);
    let mut call = write(&rt, br#"{"a":true}"#);
    assert!(
        call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending()
    );
    let mut bytes = [0; 1024];
    let (n, from) = peer.recv_from(&mut bytes).unwrap();
    assert_eq!(&bytes[..n], b"GROUP /temperature {\"a\":true}\n");
    peer.send_to(b"REPORT 7 1 2 N 0 712 900", from).unwrap();
    let result = poll(&mut call);
    assert!(result.result().is_err());
    assert_eq!(result.effect(), ConsumerEffect::Unknown);
    assert_eq!(result.property_write_members().unwrap().1.bits(), 1);
    assert_eq!(
        result
            .rejected_output()
            .unwrap()
            .metadata()
            .property_write_report()
            .unwrap()
            .selection()
            .bits(),
        7
    );
    drop(result);
    drop(call);
    drop(rt);
    clean(&bank);
    peer.set_nonblocking(true).unwrap();
    assert!(
        matches!(peer.recv_from(&mut bytes),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock)
    );
}
