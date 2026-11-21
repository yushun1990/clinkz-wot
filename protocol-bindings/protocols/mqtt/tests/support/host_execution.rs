//! Application-level execution uses only aggregate Producer/Consumer turns.
//! Typed loans below are for native installation, explicit emission and result
//! observation/acknowledgement, never for duplicating Engine progression.
use super::*;
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{action::*, finite::*};
use std::{
    cell::Cell,
    io::{Read, Write},
    net::TcpStream,
};

type Finite = RawFiniteRuntime<HttpFiniteBinding>;
#[derive(Default)]
struct App {
    value: u8,
    denied: Cell<bool>,
    reads: Cell<usize>,
    complete: Cell<bool>,
    polls: Cell<usize>,
    cancels: Cell<usize>,
    closes: Cell<usize>,
    release: Cell<bool>,
}
struct Property(Rc<App>);
impl RawFiniteHandler for Property {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        if self.0.denied.get() {
            Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Admission, RetryClass::Never),
            })
        } else {
            Ok(())
        }
    }
    fn handle(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<InteractionOutput> {
        self.0.reads.set(self.0.reads.get() + 1);
        Ok(InteractionOutput::with_data(Payload::new(
            self.0.value.to_string().into_bytes(),
            "application/json",
        )))
    }
}
struct Action(Rc<App>);
struct Job(Rc<App>);
impl RawActionHandler for Action {
    fn authorize(&self, _: Operation, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn invoke(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<RawActionStart> {
        Ok(RawActionStart::Continue(Box::new(Job(self.0.clone()))))
    }
}
impl RawActionWork for Job {
    fn poll(&mut self) -> Poll<CoreResult<InteractionOutput>> {
        self.0.polls.set(self.0.polls.get() + 1);
        if self.0.complete.get() {
            Poll::Ready(Ok(InteractionOutput::with_data(Payload::new(
                b"24".as_slice(),
                "application/json",
            ))))
        } else {
            Poll::Pending
        }
    }
    fn request_cancel(&mut self) -> CoreResult<()> {
        self.0.cancels.set(self.0.cancels.get() + 1);
        Ok(())
    }
    fn poll_close(&mut self) -> Poll<CoreResult<()>> {
        self.0.closes.set(self.0.closes.get() + 1);
        if self.0.closes.get() == 1 {
            Poll::Ready(Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Cleanup,
                RetryClass::Never,
            ))))
        } else if self.0.release.get() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
    fn abort_local(self: Box<Self>) {}
}

fn description(
    http: SocketAddr,
    action: SocketAddr,
    mqtt: SocketAddr,
    mixed: bool,
    path: &str,
) -> serde_json::Value {
    let mut td = serde_json::json!({
        "@context":["https://www.w3.org/2022/wot/td/v1.1", {"mqv":"https://www.w3.org/2019/wot/mqtt#"}],
        "id":format!("urn:host:{path}"),"title":path,"security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
        "properties":{"level":{"type":"integer","readOnly":true,"forms":[{"href":format!("http://{http}/{path}")}]}}
    });
    if mixed {
        td["actions"] = serde_json::json!({"run":{"synchronous":false,"input":{"type":"integer"},"output":{"type":"integer"},
            "uriVariables":{"id":{"type":"string"}},"forms":[
                {"href":format!("http://{action}/run"),"op":"invokeaction","htv:methodName":"POST"},
                {"href":format!("http://{action}/status{{?id}}"),"op":"queryaction","htv:methodName":"GET"},
                {"href":format!("http://{action}/stop{{?id}}"),"op":"cancelaction","htv:methodName":"DELETE"}]}});
        td["events"] = serde_json::json!({"alarm":{"data":{"type":"integer"},"forms":[{
            "href":format!("mqtt://{mqtt}"),"mqv:filter":"host/alarm","mqv:qos":"0","mqv:retain":false}]}});
    }
    td
}
fn finite_limits() -> RawFiniteLimits {
    RawFiniteLimits {
        candidates: 1,
        input_body_bytes: 0,
        output_body_bytes: 4096,
        output_media_capacity: 64,
        output_retained_bytes: 8192,
    }
}
struct Native {
    listener: HttpFiniteListener,
    finite: Rc<HttpFiniteProducer>,
    actions: Rc<HttpActionProducer>,
    mqtt: Rc<MqttNotificationProducer>,
    service: MqttNotificationService,
    capacity: ProducerFiniteCapacity,
}
impl Native {
    fn stage(
        &self,
        owner: &RawProducerServient,
        authority: ProducerInstanceAuthority,
        td: &serde_json::Value,
        app: Rc<App>,
        bank: &Arc<HostResourceAccounts>,
        expected: Option<ProducerInstanceKey>,
    ) -> (ComposedProducerExposure, HttpProducerDefinition) {
        let mut instance = RawProducerInstance::new(
            authority,
            vec![RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("level")),
                Operation::ReadProperty,
                Property(app.clone()),
            )],
        )
        .with_description(Arc::from(td.to_string()));
        let mut providers: Vec<Rc<dyn ProducerCapability>> = vec![self.finite.clone()];
        let mixed = td.get("actions").is_some();
        if mixed {
            instance = instance
                .with_actions(vec![RawActionBehavior::new(
                    Arc::from("run"),
                    Action(app),
                    std::mem::size_of::<Job>(),
                    true,
                    true,
                )])
                .with_notification_sources(vec![ProducerNotificationSource::event("alarm")]);
            providers.extend([
                self.actions.clone() as Rc<dyn ProducerCapability>,
                self.mqtt.clone(),
            ]);
        }
        let prepared = prepare_thing(
            instance.description().unwrap(),
            instance.key(),
            GatewayDefaultV1::LIMITS,
            &mut budget(),
            &providers.iter().map(|p| p.as_ref()).collect::<Vec<_>>(),
            |site, op| match site {
                ValidatedProducerScope::Property(n) => instance.supports_property(n, op),
                ValidatedProducerScope::Action(n) => instance.supports_action(n, op),
                ValidatedProducerScope::Event(n) => instance.supports_event(n, op),
                ValidatedProducerScope::Thing => false,
            },
        )
        .unwrap();
        let key = instance.key();
        let program = prepared
            .operations()
            .iter()
            .find_map(|o| o.definition::<HttpProducerProgram>())
            .unwrap()
            .clone();
        let definition = HttpProducerDefinition::from_program(&program).unwrap();
        let config = HttpFiniteConfig::try_new(
            HttpProducerConfig::for_loopback(program.listen_address()).unwrap(),
            program,
            key,
            AffordanceTarget::Property(Arc::from("level")),
            5_000,
        )
        .unwrap()
        .with_listener(&self.listener)
        .unwrap();
        let finite = RawFiniteRuntime::try_new(
            HttpFiniteBinding::new(
                config.clone(),
                self.finite.identity().0,
                self.finite.identity().1,
            ),
            config,
            owner,
            finite_limits(),
            FiniteValidation::Required,
            Deadline::NONE,
        )
        .ok()
        .unwrap()
        .with_capacity(self.capacity.clone())
        .ok()
        .unwrap();
        let mut participants = vec![OwnedProducerParticipation::finite(finite)];
        if mixed {
            let action = HttpActionRuntime::try_new(
                &owner.participation_token(),
                key,
                prepared
                    .operations()
                    .iter()
                    .filter_map(|o| o.definition::<HttpActionDefinition>().cloned())
                    .collect(),
                8,
            )
            .unwrap();
            let mqtt = self
                .service
                .participation(
                    &owner.participation_token(),
                    key,
                    prepared
                        .operations()
                        .iter()
                        .filter_map(|o| o.definition::<MqttNotificationDefinition>().cloned())
                        .collect(),
                )
                .unwrap();
            participants.extend([
                OwnedProducerParticipation::actions(action),
                OwnedProducerParticipation::notifications(mqtt),
            ]);
        }
        let exposure = ComposedProducerExposure::try_new(
            owner,
            ComposedProducerInput {
                instance,
                prepared,
                providers: ProducerRegistrations::new(providers),
                participants,
                expected,
                handler_backing_bytes: std::mem::size_of::<App>(),
            },
            limits(),
            bank.clone(),
            &mut budget(),
        )
        .unwrap_or_else(|r| panic!("{:?}", r.error()));
        (exposure, definition)
    }
}

/// A deployment-supplied cyclic turn. Observing native delivery reports is
/// explicit application policy; no library turn discards them for this loop.
async fn until(
    owner: &mut RawProducerServient,
    exposures: &mut [&mut ComposedProducerExposure],
    e: &mut Engine,
    mut observe: impl FnMut(
        &mut RawProducerServient,
        &mut [&mut ComposedProducerExposure],
        &mut Engine,
    ) -> bool,
) {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let now = e.now();
        for exposure in exposures.iter_mut() {
            let result = exposure.step(owner, &mut cx(), now, |_, _| budget());
            assert!(
                matches!(result, Ok(()) | Err(CoreError::Validation(_))),
                "{result:?}"
            );
            if let Some(native) = exposure.participant_mut::<HttpActionRuntime>(1)
                && let Some(report) = native.take_delivery()
            {
                assert!(report.locally_written || report.cause.is_some());
            }
        }
        e.turn(&mut cx(), &mut full);
        if observe(owner, exposures, e) {
            return;
        }
        assert!(Instant::now() < end, "Host execution stalled");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
fn connect(address: SocketAddr, path: &str) -> TcpStream {
    let mut peer = TcpStream::connect(address).unwrap();
    peer.write_all(
        format!("GET /{path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .unwrap();
    peer.set_nonblocking(true).unwrap();
    peer
}
fn read(peer: &mut TcpStream, bytes: &mut Vec<u8>) -> bool {
    let mut buf = [0; 4096];
    match peer.read(&mut buf) {
        Ok(0) => true,
        Ok(n) => {
            bytes.extend_from_slice(&buf[..n]);
            false
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => false,
        Err(e) => panic!("{e}"),
    }
}
fn take(exposure: &mut ComposedProducerExposure) -> Box<ProducerFiniteSettlement> {
    exposure
        .participant_mut::<Finite>(0)
        .unwrap()
        .take_retained_settlement()
        .unwrap()
}
fn acknowledge(
    owner: &mut RawProducerServient,
    exposure: &mut ComposedProducerExposure,
) -> RawActionSettlement {
    let native = exposure.participant_mut::<HttpActionRuntime>(1).unwrap();
    let id = native.action_ids().next().unwrap();
    owner.acknowledge_action(native, id).unwrap().unwrap()
}

#[test]
fn host_turns_join_independent_publications_all_families_replacement_and_complete_disposal() {
    runtime().block_on(async {
        let broker = Broker::start();
        // Hold both endpoint probes until distinct addresses have been chosen.
        let probes = [
            TcpListener::bind("127.0.0.1:0").unwrap(),
            TcpListener::bind("127.0.0.1:0").unwrap(),
        ];
        let [http, action] = probes.each_ref().map(|p| p.local_addr().unwrap());
        drop(probes);
        let definition_bank = bank();
        let work_bank = bank();
        let consumer_bank = bank();
        let publication_banks = [bank(), bank(), bank()];
        let mut owner = owner();
        owner
            .enable_raw_actions(RawActionLimits {
                invocations: 8,
                application_work_bytes: 1024,
                input_fields: 8,
                input_bytes: 4160,
                principal_id_bytes: 32,
                output_body_bytes: 4096,
                output_media_capacity: 64,
                output_retained_bytes: 8192,
            })
            .unwrap();
        let listen = HttpProducerConfig::for_loopback(http).unwrap();
        let compiler = Rc::new(
            HttpFiniteProducer::compiling(
                listen,
                BindingId::new(601),
                BindingGeneration::INITIAL,
                5_000,
                definition_bank.clone(),
            )
            .unwrap(),
        );
        let mqtt = Rc::new(MqttNotificationProducer::new(
            MqttProducerConfig::for_loopback(broker.address, 8192).unwrap(),
            BindingId::new(602),
            BindingGeneration::INITIAL,
        ));
        let mut native = Native {
            listener: HttpFiniteListener::new(listen, 5_000).unwrap(),
            finite: compiler.clone(),
            actions: Rc::new(HttpActionProducer::new(
                HttpProducerConfig::for_loopback(action).unwrap(),
                BindingId::new(603),
                BindingGeneration::INITIAL,
            )),
            service: MqttNotificationService::new(&mqtt).unwrap(),
            mqtt,
            capacity: ProducerFiniteCapacity::try_new(
                &owner,
                finite_limits(),
                1,
                8,
                work_bank.clone(),
            )
            .unwrap(),
        };
        let td = description(http, action, broker.address, true, "alpha");
        let apps = [
            Rc::new(App {
                value: 7,
                ..App::default()
            }),
            Rc::new(App {
                value: 8,
                ..App::default()
            }),
            Rc::new(App {
                value: 9,
                ..App::default()
            }),
        ];
        let (mut first, shared) = native.stage(
            &owner,
            authority(1),
            &td,
            apps[0].clone(),
            &publication_banks[0],
            None,
        );
        first
            .participant_mut::<HttpActionRuntime>(1)
            .unwrap()
            .install_closed()
            .unwrap();
        assert_eq!(compiler.native_compilations(), 1);
        native.finite = Rc::new(
            HttpFiniteProducer::reusing(
                listen,
                BindingId::new(601),
                BindingGeneration::INITIAL,
                5_000,
                &shared,
            )
            .unwrap(),
        );
        let (mut sibling, sibling_definition) = native.stage(
            &owner,
            authority(2),
            &description(http, action, broker.address, false, "beta"),
            apps[1].clone(),
            &publication_banks[1],
            None,
        );
        assert!(shared.shares_compilation_with(&sibling_definition));
        assert_eq!(native.finite.native_compilations(), 0);
        let thing = serde_json::from_value(td.clone()).unwrap();
        let mut e = Engine::from_thing(&thing, &broker, consumer_bank.clone());
        drop(thing);
        let consumer = e.register("urn:host:alpha");
        // Zero credit cannot acquire native readiness or dispatch.
        first
            .step(&mut owner, &mut cx(), e.now(), |_, _| WorkBudget::new())
            .unwrap();
        assert!(first.publish(&mut owner, &mut budget()).is_err());
        until(
            &mut owner,
            &mut [&mut first, &mut sibling],
            &mut e,
            |owner, exposures, _| {
                for exposure in exposures {
                    if !exposure.is_committed() {
                        match exposure.publish(owner, &mut budget()) {
                            Ok(()) => {}
                            Err(CoreError::Backpressure(_)) => return false,
                            Err(e) => panic!("{e:?}"),
                        }
                    }
                }
                true
            },
        )
        .await;
        let mut call = e.read(&consumer).unwrap();
        let invocation = e.invoke(&consumer);
        let mut cursor = e.subscribe(&consumer);
        let mut peer = connect(http, "beta");
        let mut sibling_bytes = Vec::new();
        let mut peer_done = false;
        let mut result = None;
        until(
            &mut owner,
            &mut [&mut first, &mut sibling],
            &mut e,
            |_, _, e| {
                peer_done |= read(&mut peer, &mut sibling_bytes);
                if result.is_none()
                    && let Poll::Ready(s) = call.poll(
                        &mut cx(),
                        e.now(),
                        &mut budget().with_remaining(WorkClass::BindingPolls, 0),
                    )
                {
                    result = Some(s);
                }
                peer_done
                    && result.is_some()
                    && invocation.state().unwrap() == ConsumerActionState::Pending
                    && e.notifications.status(cursor.identity()).unwrap().state
                        == NotificationState::Active
            },
        )
        .await;
        assert!(sibling_bytes.ends_with(b"8"));
        let result = result.unwrap();
        assert_eq!(result.result().unwrap().data().unwrap().body, b"7");
        assert_eq!(native.capacity.active_work(), 0);
        assert_eq!(native.capacity.retained_work(), 2);
        // Both original Producer results stay unread while the publication serves again.
        assert_eq!((apps[0].reads.get(), apps[1].reads.get()), (1, 1));
        apps[1].denied.set(true);
        let mut denied = connect(http, "beta");
        let mut denied_bytes = Vec::new();
        until(
            &mut owner,
            &mut [&mut first, &mut sibling],
            &mut e,
            |_, _, _| read(&mut denied, &mut denied_bytes),
        )
        .await;
        assert!(denied_bytes.starts_with(b"HTTP/1.1 403"));
        assert_eq!(apps[1].reads.get(), 1);
        assert_eq!(native.capacity.retained_work(), 3);
        let first_result = take(&mut first);
        let sibling_result = take(&mut sibling);
        let refusal = take(&mut sibling);
        assert_eq!(first_result.recipient(), Some(first.key()));
        assert_eq!(sibling_result.recipient(), Some(sibling.key()));
        assert!(first_result.sequence() != sibling_result.sequence());
        assert_eq!(refusal.effect(), FiniteEffect::NotInvoked);
        assert_eq!(first_result.delivery(), FiniteDelivery::Delivered);

        let key = first.key();
        let mut emission = owner
            .publish_notification(
                first.participant_mut::<MqttNotificationRuntime>(2).unwrap(),
                key,
                AffordanceTarget::Event(Arc::from("alarm")),
                data(b"31"),
                Deadline::NONE,
                e.now(),
                &mut budget(),
            )
            .ok()
            .unwrap();
        until(
            &mut owner,
            &mut [&mut first, &mut sibling],
            &mut e,
            |_, _, e| {
                emission.step(&mut cx(), &mut budget(), e.now()).is_ready()
                    && e.notifications
                        .status(cursor.identity())
                        .unwrap()
                        .queued_items
                        == 1
            },
        )
        .await;
        let event = emission.acknowledge().ok().unwrap();
        assert_eq!(event.native.effect, PublicationEffect::LocallyPublished);
        assert_eq!(
            e.notifications
                .front(&mut cursor, &mut budget())
                .unwrap()
                .unwrap()
                .data
                .unwrap()
                .body,
            b"31"
        );
        e.notifications.advance(&mut cursor, &mut budget()).unwrap();
        // Native ingress gets zero credit; accepted Action work still progresses.
        let polls = apps[0].polls.get();
        apps[0].complete.set(true);
        first
            .step(&mut owner, &mut cx(), e.now(), |_, lane| {
                if lane == ProducerWork::Participation {
                    WorkBudget::new()
                } else {
                    budget()
                }
            })
            .ok();
        assert!(apps[0].polls.get() > polls);
        invocation.query(e.now(), &mut budget()).unwrap();
        until(
            &mut owner,
            &mut [&mut first, &mut sibling],
            &mut e,
            |_, _, _| invocation.state().unwrap() == ConsumerActionState::Completed,
        )
        .await;
        let action_result = invocation.take_settlement(e.now(), &mut budget()).unwrap();
        assert_eq!(action_result.result().unwrap().data().unwrap().body, b"24");
        let producer_action = acknowledge(&mut owner, &mut first);
        assert!(producer_action.result().is_ok());

        apps[0].complete.set(false);
        let pending = e.invoke(&consumer);
        until(
            &mut owner,
            &mut [&mut first, &mut sibling],
            &mut e,
            |_, _, _| pending.state().unwrap() == ConsumerActionState::Pending,
        )
        .await;
        // Stage its native lease before closing the last old MQTT lease. This
        // service deliberately cannot reopen after complete physical shutdown.
        let replacement_authority = ProducerInstanceAuthority::try_new(ThingSlotId::new(
            key.slot().slot(),
            Generation::new(2).unwrap(),
        ))
        .unwrap();
        let (mut replacement, replacement_definition) = native.stage(
            &owner,
            replacement_authority,
            &td,
            apps[2].clone(),
            &publication_banks[2],
            Some(key),
        );
        assert!(shared.shares_compilation_with(&replacement_definition));
        first.begin_close();
        replacement
            .participant_mut::<HttpActionRuntime>(1)
            .unwrap()
            .install_closed()
            .unwrap();
        // Cleanup errors and Pending retain the old original work and first error.
        until(
            &mut owner,
            &mut [&mut first, &mut sibling],
            &mut e,
            |_, _, _| apps[0].closes.get() >= 3,
        )
        .await;
        assert!(matches!(
            first.progress_failure(1),
            Some(CoreError::Validation(_))
        ));
        assert!(first.retire(&mut owner, &mut budget()).is_err());
        let closing = apps[0].closes.get();
        first
            .step(&mut owner, &mut cx(), e.now(), |_, _| WorkBudget::new())
            .unwrap();
        assert_eq!(apps[0].closes.get(), closing);
        assert_eq!(apps[0].cancels.get(), 1);

        until(
            &mut owner,
            &mut [&mut first, &mut sibling, &mut replacement],
            &mut e,
            |owner, exposures, _| match exposures[2].publish(owner, &mut budget()) {
                Ok(()) => true,
                Err(CoreError::Backpressure(_)) => false,
                Err(e) => panic!("{e:?}"),
            },
        )
        .await;
        assert!(owner.description(key).is_none());
        assert!(owner.description(replacement.key()).is_some());
        let mut successor_peer = connect(http, "alpha");
        let mut successor_bytes = Vec::new();
        until(
            &mut owner,
            &mut [&mut first, &mut sibling, &mut replacement],
            &mut e,
            |_, _, _| read(&mut successor_peer, &mut successor_bytes),
        )
        .await;
        assert!(successor_bytes.ends_with(b"9"));
        assert_eq!(apps[2].reads.get(), 1);
        assert!(!apps[0].release.get());
        assert!(native.service.connected());
        let successor_result = take(&mut replacement);
        assert_eq!(successor_result.recipient(), Some(replacement.key()));
        apps[0].release.set(true);
        let mut closed_action = None;
        until(
            &mut owner,
            &mut [&mut first, &mut sibling, &mut replacement],
            &mut e,
            |owner, exposures, _| {
                let native = exposures[0]
                    .participant_mut::<HttpActionRuntime>(1)
                    .unwrap();
                let id = native.action_ids().next().unwrap();
                closed_action = owner.acknowledge_action(native, id).unwrap();
                closed_action.is_some()
            },
        )
        .await;
        let closed_action = closed_action.unwrap();
        assert_eq!(closed_action.instance, key);
        assert!(closed_action.close_failure().is_some());
        assert!(matches!(
            closed_action.result(),
            Err(CoreError::Cancelled(_))
        ));
        assert_eq!(closed_action.effect, ActionEffect::PossiblyApplied);
        first.retire(&mut owner, &mut budget()).unwrap();
        assert!(
            native.service.connected(),
            "successor retains shared physical session"
        );
        drop(first);

        e.owner.begin_close();
        drop((cursor, pending));
        sibling.begin_close();
        replacement.begin_close();
        until(
            &mut owner,
            &mut [&mut sibling, &mut replacement],
            &mut e,
            |owner, exposures, e| {
                e.calls.is_idle()
                    && e.actions.is_idle()
                    && e.notifications.retained_subscriptions() == 0
                    && exposures
                        .iter_mut()
                        .all(|p| p.retire(owner, &mut budget()).is_ok())
            },
        )
        .await;
        assert!(!e.mqtt.connected());
        assert!(!native.service.connected());
        assert_eq!(native.capacity.active_work(), 0);
        assert_eq!(native.capacity.retained_work(), 4);
        assert_eq!(compiler.native_compilations(), 1);
        assert_eq!(native.finite.native_compilations(), 0);
        drop((
            sibling,
            replacement,
            consumer,
            call,
            invocation,
            e,
            native,
            compiler,
            shared,
            sibling_definition,
            replacement_definition,
            owner,
        ));
        TcpListener::bind(http).unwrap();
        TcpListener::bind(action).unwrap();
        assert!(
            publication_banks[0]
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        assert!(
            work_bank
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        assert!(
            consumer_bank
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        assert_eq!(first_result.result().unwrap().data().unwrap().body, b"7");
        drop((
            result,
            action_result,
            producer_action,
            closed_action,
            first_result,
            sibling_result,
            refusal,
            successor_result,
        ));
        for bank in [&definition_bank, &work_bank, &consumer_bank]
            .into_iter()
            .chain(publication_banks.iter())
        {
            clean(bank);
        }
    });
}
