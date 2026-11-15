#![cfg(feature = "broker-tests")]
//! Managed Consumer authority and borrowed execution turns over actual HTTP/MQTT.
#[path = "support/broker.rs"]
mod broker;
#[path = "support/mixed_http_peer.rs"]
mod http_peer;
mod support;
use broker::*;
use clinkz_wot_binding_http::{HttpActionConsumer, HttpConsumerConfig, try_consumer_registration};
use clinkz_wot_core::{consumer::*, consumer_action::ConsumerActionDefinition, notification::*};
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::{
    consumer::*, consumer_action::*, consumer_execution::*, consumer_notification::*,
    notification::*,
};
use http_peer::*;
use std::{
    future::{Future, poll_fn},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    task::{Poll, Wake},
    time::{Duration, Instant},
};
use support::*;

struct Access {
    original: ConsumerInstanceRef,
    denied: AtomicBool,
    calls: AtomicUsize,
}
struct PreparationAccess;
impl ConsumerAccessService for PreparationAccess {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
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
    fn retained_bytes(&self) -> u64 {
        0
    }
    fn work(&self) -> u64 {
        1
    }
}
impl ConsumerAccessService for Access {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
        Ok(())
    }
    fn authorize_instance(
        &self,
        instance: &ConsumerInstanceRef,
        _: BindingRegistrationIdentity,
        _: &str,
        _: Operation,
    ) -> CoreResult<()> {
        assert_eq!(instance, &self.original);
        assert!(instance.is_current());
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.denied.load(Ordering::Relaxed) {
            Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Admission, RetryClass::Never),
            })
        } else {
            Ok(())
        }
    }
    fn retained_bytes(&self) -> u64 {
        std::mem::size_of::<Self>() as u64
    }
    fn work(&self) -> u64 {
        1
    }
}
struct Managed {
    instance: ConsumerInstanceHandle,
    finite: ConsumerAssociation,
    action: ConsumerActionAssociation,
    notification: ConsumerNotificationAssociation<MqttNotificationConsumer>,
    access: [Arc<Access>; 3],
}
struct Engine {
    owner: ConsumerOwner,
    calls: ConsumerCalls,
    actions: ConsumerActions,
    notifications: ConsumerNotifications<MqttNotificationConsumer>,
    finite: Arc<ConsumerDefinition>,
    action: Rc<ConsumerActionDefinition>,
    notification: Rc<ConsumerNotificationDefinition<MqttNotificationConsumer>>,
    mqtt: MqttConsumerService,
    bank: Arc<HostResourceAccounts>,
    clock: Instant,
    wakes: Arc<AtomicUsize>,
}
fn retained(
    services: ConsumerServices,
    bank: &Arc<HostResourceAccounts>,
) -> RetainedConsumerServices {
    RetainedConsumerServices {
        data: ConsumerServiceOwner::retain_data(services.data, bank)
            .ok()
            .unwrap(),
        access: ConsumerServiceOwner::retain_access(services.access, bank)
            .ok()
            .unwrap(),
    }
}
impl Engine {
    fn new(peer: &HttpPeer, broker: &Broker, bank: Arc<HostResourceAccounts>) -> Self {
        let thing = serde_json::from_value(serde_json::json!({
            "@context":["https://www.w3.org/2022/wot/td/v1.1", {"mqv":"https://www.w3.org/2019/wot/mqtt#"}],
            "id":"urn:mixed:source", "title":"Mixed Consumer", "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
            "properties":{"level":{"type":"integer", "readOnly":true, "forms":[{"href":format!("http://{}/value",peer.address)}]}},
            "actions":{"run":{"synchronous":false,"input":{"type":"integer"},"output":{"type":"integer"},"uriVariables":{"id":{"type":"string"}},"forms":[
                {"href":format!("http://{}/run",peer.address),"op":"invokeaction","htv:methodName":"POST"},
                {"href":format!("http://{}/status{{?id}}",peer.address),"op":"queryaction","htv:methodName":"GET"},
                {"href":format!("http://{}/stop{{?id}}",peer.address),"op":"cancelaction","htv:methodName":"DELETE"}]}},
            "events":{"alarm":{"data":{"type":"integer"},"forms":[{"href":format!("mqtt://{}",broker.address),"mqv:filter":"mixed/alarm","mqv:qos":"0","mqv:retain":false}]}}
        })).unwrap();
        let mut limits = GatewayDefaultV1::LIMITS.clone();
        assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
        let config = HttpConsumerConfig::for_cleartext_ip().unwrap();
        let registry = ConsumerRegistry::try_new(
            vec![
                try_consumer_registration(
                    config,
                    BindingId::new(501),
                    BindingGeneration::INITIAL,
                    0,
                )
                .unwrap(),
            ],
            &bank,
        )
        .ok()
        .unwrap();
        let prepared = registry
            .prepare(
                &thing,
                &limits,
                bank.clone(),
                ConsumerSelector::default(),
                &mut budget(),
                |job| {
                    while !job.is_finished() {
                        job.step(&mut budget(), false);
                    }
                },
            )
            .unwrap();
        let (bindings, funding) = prepared.into_execution();
        let finite = bindings
            .iter()
            .find(|b| b.definition().operation() == Operation::ReadProperty)
            .unwrap()
            .definition()
            .clone();
        drop((bindings, funding, registry));
        let services = retained(config.consumer_services(), &bank);
        let registry = ConsumerActionRegistry::try_new(vec![ConsumerActionRegistration {
            preparation: Box::new(HttpActionConsumer::new(
                config,
                BindingId::new(502),
                BindingGeneration::INITIAL,
                1,
            )),
            services,
        }])
        .unwrap();
        let action = Rc::new(
            registry
                .prepare_action(
                    &thing,
                    "run",
                    ConsumerActionSelectors::default(),
                    &limits,
                    bank.clone(),
                    &mut budget(),
                )
                .unwrap(),
        );
        let mqtt = MqttConsumerService::new(
            MqttConsumerConfig::for_loopback(broker.address, 8192).unwrap(),
            BindingId::new(503),
            BindingGeneration::INITIAL,
        )
        .unwrap();
        let registry = ConsumerNotificationRegistry::try_new(
            vec![ConsumerNotificationRegistration {
                binding: mqtt.binding(),
                services: ConsumerServices {
                    data: Arc::new(MqttJsonData),
                    access: Arc::new(PreparationAccess),
                },
            }],
            &bank,
        )
        .unwrap();
        let notification = Rc::new(
            registry
                .prepare_notification(
                    &thing,
                    &AffordanceTarget::Event(Arc::from("alarm")),
                    NotificationKind::Event,
                    ConsumerSelector::default(),
                    &limits,
                    bank.clone(),
                    &mut budget(),
                )
                .unwrap(),
        );
        Self {
            owner: ConsumerOwner::try_new(bank.clone(), 16, 16).unwrap(),
            calls: ConsumerCalls::try_new(bank.clone(), 1).unwrap(),
            actions: ConsumerActions::try_new(1, bank.clone()).unwrap(),
            notifications: ConsumerNotifications::try_new(
                RawNotificationLimits {
                    subscriptions: 2,
                    queue_items: 4,
                    queue_payload_bytes: 24_000,
                    payload_backing_bytes: 4608,
                    control_backing_bytes: 0,
                    target_name_bytes: 64,
                    engine_bytes: 128_000,
                },
                bank.clone(),
            )
            .unwrap(),
            finite,
            action,
            notification,
            mqtt,
            bank,
            clock: Instant::now(),
            wakes: Arc::new(AtomicUsize::new(0)),
        }
    }
    fn register(&self, id: &str) -> Managed {
        self.bind(self.owner.register_thing(id, &mut budget()).unwrap())
    }
    fn bind(&self, instance: ConsumerInstanceHandle) -> Managed {
        let access: [Arc<Access>; 3] = std::array::from_fn(|_| {
            Arc::new(Access {
                original: instance.consumer(),
                denied: AtomicBool::new(false),
                calls: AtomicUsize::new(0),
            })
        });
        let owners: [Arc<ConsumerServiceOwner<dyn ConsumerAccessService>>; 3] =
            std::array::from_fn(|i| {
                ConsumerServiceOwner::retain_access(access[i].clone(), &self.bank)
                    .ok()
                    .unwrap()
            });
        let binding = self
            .finite
            .clone()
            .bind(owners[0].clone(), self.bank.clone(), &mut budget())
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let association = StandaloneConsumerAssociation::try_new(
            instance.consumer(),
            vec![binding],
            vec![],
            self.bank.clone(),
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let finite = self
            .owner
            .set_configuration(instance.thing_id(), "http", association, &mut budget())
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let action = ConsumerActionAssociation::try_new(
            instance.consumer(),
            self.action.clone(),
            ConsumerActionAccess {
                invoke: owners[1].clone(),
                query: Some(owners[1].clone()),
                cancel: Some(owners[1].clone()),
            },
            self.bank.clone(),
            &mut budget(),
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let notification = ConsumerNotificationAssociation::try_new(
            instance.consumer(),
            self.notification.clone(),
            owners[2].clone(),
            self.bank.clone(),
            &mut budget(),
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        Managed {
            instance,
            finite,
            action,
            notification,
            access,
        }
    }
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(ClockId::new(55), self.clock.elapsed().as_millis() as u64)
    }
    fn turn(&mut self, cx: &mut Context<'_>, credit: &mut impl FnMut(usize) -> WorkBudget) -> bool {
        let now = self.now();
        step_consumers(
            &mut [&mut self.calls, &mut self.actions, &mut self.notifications],
            cx,
            now,
            credit,
        )
    }
    async fn until(
        &mut self,
        mut credit: impl FnMut(usize) -> WorkBudget,
        mut observe: impl FnMut(&mut Self, &mut Context<'_>) -> bool,
    ) {
        let mut pulse = Box::pin(tokio::time::sleep(Duration::from_millis(5)));
        let mut native_waker = None;
        tokio::time::timeout(
            Duration::from_secs(4),
            poll_fn(|cx| {
                // Register a real deployment timer independently of native I/O.
                if pulse.as_mut().poll(cx).is_ready() {
                    pulse
                        .as_mut()
                        .reset(tokio::time::Instant::now() + Duration::from_millis(5));
                    assert!(pulse.as_mut().poll(cx).is_pending());
                }
                let waker = native_waker.get_or_insert_with(|| {
                    std::task::Waker::from(Arc::new(NativeWake {
                        parent: cx.waker().clone(),
                        count: self.wakes.clone(),
                    }))
                });
                let mut native = Context::from_waker(waker);
                self.turn(&mut native, &mut credit);
                if observe(self, &mut native) {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }),
        )
        .await
        .expect("mixed Consumer progress stalled");
    }
    fn subscribe(&mut self, consumer: &Managed) -> NotificationCursor {
        self.notifications
            .subscribe(
                &consumer.notification,
                SubscriptionInput(None),
                ResponseRequirement::None,
                NotificationOverflow::Terminate,
                &mut budget(),
            )
            .unwrap_or_else(|e| panic!("{:?}", e.error()))
    }
    fn read(&self, consumer: &Managed) -> CoreResult<ConsumerCall> {
        self.owner.read_property(
            consumer.instance.thing_id(),
            "http",
            &self.calls,
            "level",
            Deadline::NONE,
            self.now(),
            &mut budget(),
        )
    }
    fn invoke(&self, consumer: &Managed) -> ConsumerActionHandle {
        self.actions
            .invoke_action(
                &consumer.action,
                Some(Payload::new(b"12".as_slice(), "application/json")),
                Deadline::NONE,
                self.now(),
                &mut budget(),
            )
            .unwrap_or_else(|e| panic!("{:?}", e.error()))
    }
}
struct NativeWake {
    parent: std::task::Waker,
    count: Arc<AtomicUsize>,
}
impl Wake for NativeWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.parent.wake_by_ref();
    }
}
fn full(_: usize) -> WorkBudget {
    budget()
}
fn no_finite(index: usize) -> WorkBudget {
    if index == 0 {
        WorkBudget::new()
    } else {
        budget()
    }
}
fn clean(bank: &HostResourceAccounts) {
    for kind in ResourceKind::ALL {
        assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
    }
}

#[test]
fn managed_mixed_turns_keep_authorization_credit_controls_and_reports_independent() {
    runtime().block_on(async {
        let broker = Broker::start();
        let peer = HttpPeer::new();
        let bank = bank();
        let mut e = Engine::new(&peer, &broker, bank.clone());
        let a = e.register("urn:mixed:a");
        let b = e.register("urn:mixed:b");
        let idle: Vec<_> = (0..6)
            .map(|i| {
                e.owner
                    .register_thing(&format!("urn:mixed:idle:{i}"), &mut budget())
                    .unwrap()
            })
            .collect();
        a.access[0].denied.store(true, Ordering::Relaxed);
        assert!(matches!(e.read(&a), Err(CoreError::Security { .. })));
        let mut read = e.read(&b).unwrap();
        let action = e.invoke(&a);
        let mut ca = e.subscribe(&a);
        let mut cb = e.subscribe(&b);
        e.until(no_finite, |e, _| {
            action.state().unwrap() == ConsumerActionState::Pending
                && [ca.identity(), cb.identity()].into_iter().all(|id| {
                    e.notifications.status(id).unwrap().state == NotificationState::Active
                })
        })
        .await;
        assert_eq!(
            peer.observed.reads.load(Ordering::Acquire),
            0,
            "short finite credit cannot borrow Action credit"
        );
        let mut publisher = Peer::new(&broker);
        publisher.publish_raw("mixed/alarm", b"7", false);
        e.until(no_finite, |e, _| {
            [ca.identity(), cb.identity()]
                .into_iter()
                .all(|id| e.notifications.status(id).unwrap().queued_items == 1)
        })
        .await;
        assert!(
            e.wakes.load(Ordering::Relaxed) > 0,
            "actual native readiness wakes the Host task"
        );
        for cursor in [&mut ca, &mut cb] {
            assert_eq!(
                e.notifications
                    .front(cursor, &mut budget())
                    .unwrap()
                    .unwrap()
                    .data
                    .unwrap()
                    .body,
                b"7"
            );
            e.notifications.advance(cursor, &mut budget()).unwrap();
        }
        let mut finite_result = None;
        e.until(full, |e, cx| {
            // Result observation cannot do native I/O: only the composed turn can.
            if let Poll::Ready(result) = read.poll(
                cx,
                e.now(),
                &mut budget().with_remaining(WorkClass::BindingPolls, 0),
            ) {
                finite_result = Some(result);
                true
            } else {
                false
            }
        })
        .await;
        let finite_result = finite_result.unwrap();
        assert_eq!(finite_result.consumer(), &b.instance.consumer());
        assert_eq!(finite_result.result().unwrap().data().unwrap().body, b"7");
        action.query(e.now(), &mut budget()).unwrap();
        e.until(full, |_, _| {
            action.state().unwrap() == ConsumerActionState::Completed
        })
        .await;
        let action_result = action.take_settlement(e.now(), &mut budget()).unwrap();
        assert_eq!(action_result.identity().consumer(), &a.instance.consumer());
        assert_eq!(action_result.result().unwrap().data().unwrap().body, b"24");
        assert!(action_result.first_cause().is_none());

        let cancel = e.invoke(&b);
        e.until(full, |_, _| {
            cancel.state().unwrap() == ConsumerActionState::Pending
        })
        .await;
        cancel.request_cancel(e.now(), &mut budget()).unwrap();
        e.until(full, |_, _| {
            cancel.remote_cancellation().unwrap() == ConsumerActionCancellation::Requested
        })
        .await;
        assert_eq!(
            cancel.state().unwrap(),
            ConsumerActionState::Pending,
            "cancel acknowledgement is not completion"
        );
        cancel.query(e.now(), &mut budget()).unwrap();
        e.until(full, |_, _| {
            cancel.state().unwrap() == ConsumerActionState::Completed
        })
        .await;
        let cancelled = cancel.take_settlement(e.now(), &mut budget()).unwrap();
        assert_eq!(
            cancelled.remote_cancellation(),
            ConsumerActionCancellation::Requested
        );
        assert!(cancelled.first_cause().is_none());
        assert_eq!(peer.observed.cancels.load(Ordering::Acquire), 1);

        e.notifications
            .stop(&ca, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        e.until(full, |e, _| {
            e.notifications.status(ca.identity()).unwrap().state == NotificationState::Closed
        })
        .await;
        assert_eq!(
            e.notifications.retained_subscriptions(),
            2,
            "aggregate turns keep the live cursor's report"
        );
        drop(e.notifications.take_settlement(ca.identity()).unwrap());
        assert!(
            e.mqtt.connected(),
            "sibling retains the actual MQTT connection"
        );
        publisher.publish_raw("mixed/alarm", b"8", false);
        e.until(full, |e, _| {
            e.notifications.status(cb.identity()).unwrap().queued_items == 1
        })
        .await;
        assert_eq!(
            e.notifications
                .front(&mut cb, &mut budget())
                .unwrap()
                .unwrap()
                .data
                .unwrap()
                .body,
            b"8"
        );
        e.notifications.advance(&mut cb, &mut budget()).unwrap();
        e.owner.begin_close();
        e.until(full, |e, _| {
            e.notifications.status(cb.identity()).unwrap().state == NotificationState::Closed
        })
        .await;
        let report = e.notifications.take_settlement(cb.identity()).unwrap();
        assert!(matches!(
            report.first_cause,
            Some(CoreError::StaleHandle(_))
        ));
        assert!(!e.mqtt.connected());
        assert!(e.turn(&mut cx(), &mut full));
        drop((report, a, b, idle, read, action, cancel, ca, cb, e));
        assert!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0,
            "original result owners retain their funding after controller loss"
        );
        assert_eq!(action_result.result().unwrap().data().unwrap().body, b"24");
        drop((finite_result, action_result, cancelled));
        clean(&bank);
    });
}

#[test]
fn replacement_keeps_unread_original_results_and_shared_native_siblings() {
    runtime().block_on(async {
        let broker = Broker::start();
        let peer = HttpPeer::new();
        let bank = bank();
        let mut e = Engine::new(&peer, &broker, bank.clone());
        let a = e.register("urn:mixed:a");
        let b = e.register("urn:mixed:b");
        let mut read = e.read(&a).unwrap();
        let action = e.invoke(&a);
        let ca = e.subscribe(&a);
        let mut cb = e.subscribe(&b);
        e.until(full, |e, _| {
            action.state().unwrap() == ConsumerActionState::Pending
                && [ca.identity(), cb.identity()].into_iter().all(|id| {
                    e.notifications.status(id).unwrap().state == NotificationState::Active
                })
        })
        .await;
        let mut publisher = Peer::new(&broker);
        publisher.publish_raw("mixed/alarm", b"9", false);
        action.query(e.now(), &mut budget()).unwrap();
        e.until(full, |e, _| {
            action.state().unwrap() == ConsumerActionState::Completed
                && peer.observed.closed.load(Ordering::Acquire) == 3
                && [ca.identity(), cb.identity()]
                    .into_iter()
                    .all(|id| e.notifications.status(id).unwrap().queued_items == 1)
        })
        .await;

        let wakes = e.wakes.load(Ordering::Relaxed);
        let successor = e
            .owner
            .replace_thing(a.instance.thing_id(), &mut budget())
            .unwrap();
        assert_eq!(
            e.wakes.load(Ordering::Relaxed),
            wakes,
            "managed replacement remains a passive authority fence"
        );
        let successor = e.bind(successor);
        assert!(!a.instance.is_current() && !a.finite.is_current());
        assert!(b.instance.is_current() && successor.instance.is_current());
        assert_ne!(successor.instance.consumer(), a.instance.consumer());
        assert!(Rc::ptr_eq(
            a.action.definition(),
            successor.action.definition()
        ));
        assert!(Rc::ptr_eq(
            a.notification.definition(),
            successor.notification.definition()
        ));
        e.turn(&mut cx(), &mut |_| WorkBudget::new());
        assert_eq!(action.state().unwrap(), ConsumerActionState::Closed);
        assert!(matches!(
            action.query(e.now(), &mut budget()),
            Err(CoreError::StaleHandle(_))
        ));
        let action_result = action.take_settlement(e.now(), &mut budget()).unwrap();
        assert_eq!(action_result.identity().consumer(), &a.instance.consumer());
        assert!(matches!(
            action_result.first_cause(),
            Some(CoreError::StaleHandle(_))
        ));
        assert_eq!(
            action_result
                .native_result()
                .unwrap()
                .unwrap()
                .data()
                .unwrap()
                .body,
            b"24"
        );
        assert!(action_result.result().is_err());
        let mut finite_result = None;
        e.until(full, |e, cx| {
            if finite_result.is_none()
                && let Poll::Ready(result) = read.poll(
                    cx,
                    e.now(),
                    &mut budget().with_remaining(WorkClass::BindingPolls, 0),
                )
            {
                finite_result = Some(result);
            }
            finite_result.is_some()
                && e.notifications.status(ca.identity()).unwrap().state == NotificationState::Closed
        })
        .await;
        let finite_result = finite_result.unwrap();
        assert_eq!(finite_result.consumer(), &a.instance.consumer());
        assert!(matches!(
            finite_result.first_cause(),
            Some(CoreError::StaleHandle(_))
        ));
        let ConsumerNativeOutcome::Returned(Ok(original)) = finite_result.native_outcome() else {
            panic!("original native result was lost")
        };
        assert_eq!(original.data().unwrap().body, b"7");
        let report = e.notifications.take_settlement(ca.identity()).unwrap();
        assert_eq!(report.identity.consumer, a.instance.consumer());
        assert!(matches!(
            report.first_cause,
            Some(CoreError::StaleHandle(_))
        ));
        assert!(e.mqtt.connected());
        assert_eq!(
            e.notifications
                .front(&mut cb, &mut budget())
                .unwrap()
                .unwrap()
                .data
                .unwrap()
                .body,
            b"9"
        );
        e.notifications.advance(&mut cb, &mut budget()).unwrap();

        let replacement_action = e.invoke(&successor);
        let replacement_cursor = e.subscribe(&successor);
        e.until(full, |e, _| {
            replacement_action.state().unwrap() == ConsumerActionState::Pending
                && e.notifications
                    .status(replacement_cursor.identity())
                    .unwrap()
                    .state
                    == NotificationState::Active
        })
        .await;
        let before = e.wakes.load(Ordering::Relaxed);
        replacement_action.cancel_local();
        assert!(
            e.wakes.load(Ordering::Relaxed) > before,
            "explicit cancellation notifies the registered Host driver"
        );
        e.until(full, |_, _| {
            replacement_action.state().unwrap() == ConsumerActionState::Closed
        })
        .await;
        let replacement_result = replacement_action
            .take_settlement(e.now(), &mut budget())
            .unwrap();
        assert!(matches!(
            replacement_result.first_cause(),
            Some(CoreError::Cancelled(_))
        ));
        assert_eq!(
            peer.observed.cancels.load(Ordering::Acquire),
            0,
            "local cancellation invents no remote CancelAction"
        );
        e.owner.begin_close();
        drop((cb, replacement_cursor));
        e.until(full, |e, _| e.notifications.retained_subscriptions() == 0)
            .await;
        assert!(!e.mqtt.connected());
        assert!(e.turn(&mut cx(), &mut full));
        drop((
            report,
            a,
            b,
            successor,
            ca,
            read,
            action,
            replacement_action,
            replacement_result,
            e,
        ));
        assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
        drop((action_result, finite_result));
        clean(&bank);
    });
}

#[test]
fn host_maintenance_retires_abandoned_mixed_work_and_passive_owner_loss() {
    runtime().block_on(async {
        let broker = Broker::start();
        let peer = HttpPeer::new();
        peer.observed.hold_reads.store(true, Ordering::Release);
        let bank = bank();
        let mut e = Engine::new(&peer, &broker, bank.clone());
        let a = e.register("urn:mixed:a");
        let b = e.register("urn:mixed:b");
        let read = e.read(&a).unwrap();
        let action = e.invoke(&a);
        let ca = e.subscribe(&a);
        let mut cb = e.subscribe(&b);
        let stop_view = ca.stop_handle();
        e.until(full, |e, _| {
            peer.observed.reads.load(Ordering::Acquire) == 1
                && action.state().unwrap() == ConsumerActionState::Pending
                && [ca.identity(), cb.identity()].into_iter().all(|id| {
                    e.notifications.status(id).unwrap().state == NotificationState::Active
                })
        })
        .await;
        drop((read, action, ca));
        e.until(full, |e, _| {
            e.calls.is_idle()
                && e.actions.is_idle()
                && e.notifications.retained_subscriptions() == 1
                && peer.observed.closed.load(Ordering::Acquire) == 2
        })
        .await;
        assert!(e.mqtt.connected());
        assert!(a.instance.is_current() && b.instance.is_current());
        let mut publisher = Peer::new(&broker);
        publisher.publish_raw("mixed/alarm", b"10", false);
        e.until(full, |e, _| {
            e.notifications.status(cb.identity()).unwrap().queued_items == 1
        })
        .await;
        assert_eq!(
            e.notifications
                .front(&mut cb, &mut budget())
                .unwrap()
                .unwrap()
                .data
                .unwrap()
                .body,
            b"10"
        );
        e.notifications.advance(&mut cb, &mut budget()).unwrap();

        // With MQTT now silent, only the deployment pulse can trigger this
        // delayed management command and subsequent original cleanup turns.
        let command_at = e.clock.elapsed() + Duration::from_millis(20);
        let mut removed = false;
        e.until(full, |e, _| {
            if !removed && e.clock.elapsed() >= command_at {
                let wakes = e.wakes.load(Ordering::Relaxed);
                let original = std::mem::replace(
                    &mut e.owner,
                    ConsumerOwner::try_new(bank.clone(), 1, 0).unwrap(),
                );
                drop(original);
                assert_eq!(
                    e.wakes.load(Ordering::Relaxed),
                    wakes,
                    "logical owner Drop does not manufacture a wake"
                );
                removed = true;
            }
            removed
                && e.notifications.status(cb.identity()).unwrap().state == NotificationState::Closed
        })
        .await;
        assert!(!a.instance.is_current() && !b.instance.is_current());
        let report = e.notifications.take_settlement(cb.identity()).unwrap();
        assert!(matches!(
            report.first_cause,
            Some(CoreError::StaleHandle(_))
        ));
        assert!(!e.mqtt.connected());
        stop_view.request_stop();
        assert!(e.turn(&mut cx(), &mut full));
        drop((report, a, b, cb, stop_view, e));
        clean(&bank);
    });
}

#[test]
fn deployment_timer_observes_finite_and_action_deadlines_while_mqtt_stays_usable() {
    runtime().block_on(async {
        let broker = Broker::start();
        let peer = HttpPeer::new();
        peer.observed.hold_reads.store(true, Ordering::Release);
        let bank = bank();
        let mut e = Engine::new(&peer, &broker, bank.clone());
        let a = e.register("urn:mixed:a");
        let b = e.register("urn:mixed:b");
        let deadline = Deadline::at(e.now().checked_add_ticks(1000).unwrap());
        let mut read = e
            .owner
            .read_property(
                a.instance.thing_id(),
                "http",
                &e.calls,
                "level",
                deadline,
                e.now(),
                &mut budget(),
            )
            .unwrap();
        let action = e
            .actions
            .invoke_action(
                &a.action,
                Some(Payload::new(b"12".as_slice(), "application/json")),
                deadline,
                e.now(),
                &mut budget(),
            )
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let mut cursor = e.subscribe(&b);
        e.until(full, |e, _| {
            peer.observed.reads.load(Ordering::Acquire) == 1
                && action.state().unwrap() == ConsumerActionState::Pending
                && e.notifications.status(cursor.identity()).unwrap().state
                    == NotificationState::Active
        })
        .await;
        let mut result = None;
        e.until(full, |e, cx| {
            if result.is_none()
                && let Poll::Ready(settlement) = read.poll(
                    cx,
                    e.now(),
                    &mut budget().with_remaining(WorkClass::BindingPolls, 0),
                )
            {
                result = Some(settlement);
            }
            result.is_some()
                && action.state().unwrap() == ConsumerActionState::Closed
                && peer.observed.closed.load(Ordering::Acquire) == 2
        })
        .await;
        assert!(matches!(
            result.as_ref().unwrap().first_cause(),
            Some(CoreError::TimedOut(_))
        ));
        let action_result = action.take_settlement(e.now(), &mut budget()).unwrap();
        assert!(matches!(
            action_result.first_cause(),
            Some(CoreError::TimedOut(_))
        ));
        assert_eq!(peer.observed.cancels.load(Ordering::Acquire), 0);
        let mut publisher = Peer::new(&broker);
        publisher.publish_raw("mixed/alarm", b"11", false);
        e.until(full, |e, _| {
            e.notifications
                .status(cursor.identity())
                .unwrap()
                .queued_items
                == 1
        })
        .await;
        assert_eq!(
            e.notifications
                .front(&mut cursor, &mut budget())
                .unwrap()
                .unwrap()
                .data
                .unwrap()
                .body,
            b"11"
        );
        e.notifications.advance(&mut cursor, &mut budget()).unwrap();
        drop(cursor);
        e.until(full, |e, _| e.notifications.retained_subscriptions() == 0)
            .await;
        assert!(e.turn(&mut cx(), &mut full));
        drop((a, b, read, action, action_result, result, e));
        clean(&bank);
    });
}
