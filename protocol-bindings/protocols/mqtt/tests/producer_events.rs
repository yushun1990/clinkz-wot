#![cfg(feature = "broker-tests")]
#[path = "support/broker.rs"]
mod broker;
#[path = "support/services.rs"]
mod services;
mod support;
use broker::*;
use services::*;
use std::time::{Duration, Instant};
use support::*;

fn registration(address: SocketAddr, chunk: usize) -> MqttNotificationProducer {
    MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(address, chunk).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    )
}

struct RevokingBinding<'a> {
    native: &'a mut MqttNotificationRuntime,
    publication: Rc<ProducerPublication>,
    during_retire: bool,
    fail_cleanup_once: bool,
}
struct RevokingPublication {
    native: MqttPublication,
    publication: Rc<ProducerPublication>,
    during_retire: bool,
    fail_cleanup_once: bool,
}
impl ProducerNotificationBinding for RevokingBinding<'_> {
    type Publication = RevokingPublication;
    fn notification_route(&self, target: &AffordanceTarget) -> Option<&NotificationRoute> {
        self.native.notification_route(target)
    }
    fn start_publication(
        &mut self,
        route: &NotificationRoute,
        data: PublicationData,
    ) -> Result<RevokingPublication, BindingInputRejection<PublicationData>> {
        self.native
            .start_publication(route, data)
            .map(|native| RevokingPublication {
                native,
                publication: Rc::clone(&self.publication),
                during_retire: self.during_retire,
                fail_cleanup_once: self.fail_cleanup_once,
            })
    }
}
impl NativeNotificationPublication for RevokingPublication {
    fn poll_publish(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
        cancel: bool,
    ) -> std::task::Poll<NativePublicationOutcome> {
        let result = self.native.poll_publish(cx, budget, cancel);
        if result.is_ready() && !self.during_retire && !self.fail_cleanup_once {
            self.publication.revoke();
        }
        result
    }
    fn poll_retire(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> std::task::Poll<CoreResult<()>> {
        if self.fail_cleanup_once {
            self.fail_cleanup_once = false;
            return std::task::Poll::Ready(Err(CoreError::Cleanup(ErrorContext::new(
                ErrorPhase::Cleanup,
                RetryClass::Never,
            ))));
        }
        let result = self.native.poll_retire(cx, budget);
        if result.is_ready() && self.during_retire {
            self.publication.revoke();
        }
        result
    }
}

#[test]
fn callback_revocation_and_cleanup_failure_retain_original_effect_and_owner() {
    runtime().block_on(async {
        let broker = Broker::start();
        let provider = registration(broker.address, 8192);
        for (during_retire, fail_cleanup_once) in [(false, false), (true, false), (false, true)] {
            let service = MqttNotificationService::new(&provider).unwrap();
            let mut owner = owner();
            let instance = instance(
                authority(1),
                &td(broker.address, "callback/alarm").to_string(),
            );
            let key = instance.key();
            let prepared = prepare(&instance, &[&provider]).unwrap();
            let definitions = prepared
                .operations()
                .iter()
                .map(|o| {
                    o.definition::<MqttNotificationDefinition>()
                        .unwrap()
                        .clone()
                })
                .collect();
            let mut native = service.participation(&owner, key, definitions).unwrap();
            let mut publication = owner
                .prepare_publication(instance, None)
                .unwrap_or_else(|_| panic!("staging"));
            let end = Instant::now() + Duration::from_secs(3);
            while !native.ready_for(&owner, key) {
                let _ = native.step(&mut owner, &mut cx(), &mut budget()).unwrap();
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            publish_prepared_thing(
                &mut owner,
                &mut publication,
                &prepared,
                &[&provider],
                &mut [&mut native],
                GatewayDefaultV1::LIMITS,
                &mut budget(),
            )
            .unwrap();
            let publication = Rc::new(publication);
            let mut peer = Peer::new(&broker);
            peer.subscribe("callback/alarm");
            let mut binding = RevokingBinding {
                native: &mut native,
                publication: Rc::clone(&publication),
                during_retire,
                fail_cleanup_once,
            };
            let mut e = owner
                .publish_notification(
                    &mut binding,
                    key,
                    AffordanceTarget::Event(Arc::from("alarm")),
                    data(b"12"),
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .ok()
                .unwrap();
            if fail_cleanup_once {
                assert!(e.step(&mut cx(), &mut budget(), now()).is_pending());
                assert!(matches!(e.first_cause(), Some(CoreError::Cleanup(_))));
                assert!(e.cleanup_error().is_some());
                e = e
                    .acknowledge()
                    .expect_err("failed cleanup retains the entire emission");
            }
            finish(&mut e).await;
            let s = e.acknowledge().ok().unwrap();
            if fail_cleanup_once {
                assert!(matches!(s.cause, Some(CoreError::Cleanup(_))));
                assert!(s.cleanup_error.is_some());
            } else {
                assert!(matches!(s.cause, Some(CoreError::StaleHandle(_))));
            }
            assert_eq!(s.native.effect, PublicationEffect::LocallyPublished);
            peer.event("callback/alarm", b"12");
            assert!(owner.description(key).is_none());
            drop(binding);
            native.begin_retire();
            let _ = native.step(&mut owner, &mut cx(), &mut budget()).unwrap();
            let mut publication = Rc::try_unwrap(publication).ok().unwrap();
            owner.retire_composed(&mut publication, &[&native]).unwrap();
            assert!(!service.connected());
        }
    });
}

#[test]
fn captured_actual_security_and_validation_are_used_for_every_emission() {
    runtime().block_on(async {
        let broker = Broker::start();
        let provider = registration(broker.address, 8192);
        let service = MqttNotificationService::new(&provider).unwrap();
        let accounts = bank();
        let mut owner = owner();
        let (input, probe) = intercept(
            &owner,
            authority(1),
            &provider,
            &service,
            &td(broker.address, "policy/alarm").to_string(),
            false,
        );
        let mut exposure = stage(&owner, input, &accounts);
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            let p = exposure.participant_mut::<ServicesParticipant>(0).unwrap();
            let _ = p.native.step(&mut owner, &mut cx(), &mut budget()).unwrap();
            if exposure.publish(&mut owner, &mut budget()).is_ok() {
                break;
            }
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let mut peer = Peer::new(&broker);
        peer.subscribe("policy/alarm");
        let key = exposure.key();
        probe.denied.set(true);
        let p = exposure.participant_mut::<ServicesParticipant>(0).unwrap();
        let denied = owner
            .publish_notification(
                &mut p.native,
                key,
                AffordanceTarget::Event(Arc::from("alarm")),
                data(b"1"),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .err()
            .unwrap();
        assert!(matches!(denied.error().error(), CoreError::Security { .. }));
        assert_eq!(probe.authorized.get(), 1);
        assert_eq!(probe.validated.get(), 0);
        assert_eq!(service.locally_published(), 0);
        probe.denied.set(false);
        assert!(
            owner
                .publish_notification(
                    &mut p.native,
                    key,
                    AffordanceTarget::Event(Arc::from("alarm")),
                    data(b"\"bad\""),
                    Deadline::NONE,
                    now(),
                    &mut budget()
                )
                .is_err()
        );
        let mut e = owner
            .publish_notification(
                &mut p.native,
                key,
                AffordanceTarget::Event(Arc::from("alarm")),
                data(b"2"),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .ok()
            .unwrap();
        finish(&mut e).await;
        peer.event("policy/alarm", b"2");
        drop(e);
        assert_eq!(probe.authorized.get(), 3);
        assert_eq!(probe.validated.get(), 2);
        exposure.begin_close();
        let p = exposure.participant_mut::<ServicesParticipant>(0).unwrap();
        assert!(
            p.native
                .step(&mut owner, &mut cx(), &mut budget())
                .unwrap()
                .is_ready()
        );
        exposure.retire(&mut owner, &mut budget()).unwrap();
        drop(exposure);
        drop(probe);
        assert_empty(&accounts);
    });
}

#[test]
fn service_substitution_after_staging_refuses_final_publication() {
    runtime().block_on(async {
        let broker = Broker::start();
        let provider = registration(broker.address, 8192);
        let service = MqttNotificationService::new(&provider).unwrap();
        let accounts = bank();
        let mut owner = owner();
        let (input, probe) = intercept(
            &owner,
            authority(1),
            &provider,
            &service,
            &td(broker.address, "changed/alarm").to_string(),
            false,
        );
        let mut exposure = stage(&owner, input, &accounts);
        let key = exposure.key();
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            let p = exposure.participant_mut::<ServicesParticipant>(0).unwrap();
            let _ = p.native.step(&mut owner, &mut cx(), &mut budget()).unwrap();
            if p.native.ready_for(&owner, key) {
                break;
            }
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let p = exposure.participant_mut::<ServicesParticipant>(0).unwrap();
        p.services = Some(Rc::clone(&probe.original));
        assert!(exposure.publish(&mut owner, &mut budget()).is_err());
        assert!(owner.description(key).is_none());
        assert!(exposure.publication_failure().is_some());
        assert_eq!(service.locally_published(), 0);
        let p = exposure.participant_mut::<ServicesParticipant>(0).unwrap();
        assert!(
            p.native
                .step(&mut owner, &mut cx(), &mut budget())
                .unwrap()
                .is_ready()
        );
        exposure.retire(&mut owner, &mut budget()).unwrap();
        drop(exposure);
        drop(probe);
        assert_empty(&accounts);
    });
}

#[test]
fn qualified_td_event_reaches_real_peers_through_one_native_publication() {
    runtime().block_on(async {
        let broker = Broker::start();
        let provider = registration(broker.address, 8192);
        let service = MqttNotificationService::new(&provider).unwrap();
        let accounts = bank();
        let mut owner = owner();
        let text = td(broker.address, "things/a/alarm").to_string();
        let input = input(&owner, authority(1), &provider, &service, &text, None);
        drop(text); // no preparation source or semantic loan survives into execution
        let mut exposure = stage(&owner, input, &accounts);
        let key = exposure.key();
        let mut exact = Peer::new(&broker);
        exact.subscribe("things/a/alarm");
        let mut wildcard = Peer::new(&broker);
        wildcard.subscribe("things/+/alarm");
        let mut other = Peer::new(&broker);
        other.subscribe("other/#");
        assert!(
            owner
                .publish_notification(
                    native(&mut exposure),
                    key,
                    AffordanceTarget::Event(Arc::from("alarm")),
                    data(b"1"),
                    Deadline::NONE,
                    now(),
                    &mut budget()
                )
                .is_err()
        );
        assert_eq!(service.locally_published(), 0);
        exact.absent();
        publish(&mut owner, &mut exposure).await;
        exact.absent(); // establishment/publication fabricates no initial Event
        assert!(owner.description(key).unwrap().contains("events"));
        let mut emission = emit(&mut owner, &mut exposure, b"12");
        finish(&mut emission).await;
        assert_eq!(
            emission.settlement().unwrap().native.effect,
            PublicationEffect::LocallyPublished
        );
        assert!(emission.settlement().unwrap().cause.is_none());
        exact.event("things/a/alarm", b"12");
        wildcard.event("things/a/alarm", b"12");
        other.absent();
        assert_eq!(
            service.locally_published(),
            1,
            "Engine/native publisher sent only once for multiple peers"
        );
        emission.acknowledge().ok().unwrap();
        exact.unsubscribe("things/a/alarm");
        let mut second = emit(&mut owner, &mut exposure, b"13");
        finish(&mut second).await;
        second.acknowledge().ok().unwrap();
        wildcard.event("things/a/alarm", b"13");
        exact.absent();
        close(&mut owner, &mut exposure);
        assert!(!service.connected());
        drop(exposure);
        assert_empty(&accounts);
        assert_eq!(broker.logs().matches("as cz").count(), 1);
    });
}

#[test]
fn one_thing_keeps_distinct_event_schemas_and_native_topics() {
    runtime().block_on(async {
        let broker = Broker::start();
        let provider = registration(broker.address, 8192);
        let service = MqttNotificationService::new(&provider).unwrap();
        let accounts = bank();
        let mut owner = owner();
        let mut json = td(broker.address, "multi/alarm");
        json["events"]["notice"] = json["events"]["alarm"].clone();
        json["events"]["notice"]["data"]["type"] = serde_json::json!("string");
        json["events"]["notice"]["forms"][0]["mqv:filter"] = serde_json::json!("multi/notice");
        let instance = RawProducerInstance::new(authority(1), vec![])
            .with_notification_sources(vec![
                ProducerNotificationSource::event("alarm"),
                ProducerNotificationSource::event("notice"),
            ])
            .with_description(Arc::from(json.to_string()));
        drop(json);
        let prepared = prepare(&instance, &[&provider]).unwrap();
        assert_eq!(prepared.operations().len(), 4);
        let definitions = prepared
            .operations()
            .iter()
            .map(|o| {
                o.definition::<MqttNotificationDefinition>()
                    .unwrap()
                    .clone()
            })
            .collect();
        let participant = service
            .participation(&owner, instance.key(), definitions)
            .unwrap();
        let mut exposure = stage(
            &owner,
            ComposedProducerInput {
                instance,
                prepared,
                providers: vec![Rc::new(provider.clone())],
                participants: vec![Box::new(participant)],
                expected: None,
                handler_backing_bytes: 0,
            },
            &accounts,
        );
        publish(&mut owner, &mut exposure).await;
        let mut peer = Peer::new(&broker);
        peer.subscribe("multi/+");
        let mut alarm = emit(&mut owner, &mut exposure, b"7");
        finish(&mut alarm).await;
        assert_eq!(
            alarm.settlement().unwrap().target,
            AffordanceTarget::Event(Arc::from("alarm"))
        );
        drop(alarm);
        peer.event("multi/alarm", b"7");
        let key = exposure.key();
        assert!(
            owner
                .publish_notification(
                    native(&mut exposure),
                    key,
                    AffordanceTarget::Event(Arc::from("notice")),
                    data(b"7"),
                    Deadline::NONE,
                    now(),
                    &mut budget()
                )
                .is_err()
        );
        assert!(
            owner
                .publish_notification(
                    native(&mut exposure),
                    key,
                    AffordanceTarget::Event(Arc::from("unknown")),
                    data(b"7"),
                    Deadline::NONE,
                    now(),
                    &mut budget()
                )
                .is_err()
        );
        let mut notice = owner
            .publish_notification(
                native(&mut exposure),
                key,
                AffordanceTarget::Event(Arc::from("notice")),
                data(b"\"ok\""),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .ok()
            .unwrap();
        finish(&mut notice).await;
        assert_eq!(
            notice.settlement().unwrap().target,
            AffordanceTarget::Event(Arc::from("notice"))
        );
        drop(notice);
        peer.event("multi/notice", b"\"ok\"");
        assert_eq!(service.locally_published(), 2);
        close(&mut owner, &mut exposure);
        drop(exposure);
        assert_empty(&accounts);
    });
}

#[test]
fn no_subscribers_is_a_successful_local_publication_without_replay() {
    runtime().block_on(async {
        let broker = Broker::start();
        let provider = registration(broker.address, 8192);
        let service = MqttNotificationService::new(&provider).unwrap();
        let accounts = bank();
        let mut owner = owner();
        let mut exposure = stage(
            &owner,
            input(
                &owner,
                authority(1),
                &provider,
                &service,
                &td(broker.address, "empty/alarm").to_string(),
                None,
            ),
            &accounts,
        );
        publish(&mut owner, &mut exposure).await;
        let mut e = emit(&mut owner, &mut exposure, b"5");
        finish(&mut e).await;
        assert_eq!(
            e.acknowledge().ok().unwrap().native.effect,
            PublicationEffect::LocallyPublished
        );
        let mut late = Peer::new(&broker);
        late.subscribe("empty/alarm");
        late.absent();
        let mut e = emit(&mut owner, &mut exposure, b"6");
        finish(&mut e).await;
        late.event("empty/alarm", b"6");
        drop(e);
        close(&mut owner, &mut exposure);
        drop(exposure);
        assert_empty(&accounts);
    });
}

#[test]
fn validation_and_bounded_unread_settlements_return_original_refused_data() {
    runtime().block_on(async {
        let broker = Broker::start();
        let provider = registration(broker.address, 8192);
        let service = MqttNotificationService::new(&provider).unwrap();
        let accounts = bank();
        let mut owner = owner();
        let mut exposure = stage(
            &owner,
            input(
                &owner,
                authority(1),
                &provider,
                &service,
                &td(broker.address, "bounded/alarm").to_string(),
                None,
            ),
            &accounts,
        );
        publish(&mut owner, &mut exposure).await;
        let key = exposure.key();
        for d in [
            data(b"\"bad\""),
            data(b"true"),
            data(b"{broken}"),
            data(b"1.00000001"),
            PublicationData::EventData(Payload::new(
                vec![b'1'; MAX_NOTIFICATION_BYTES + 1],
                "application/json",
            )),
            PublicationData::EventData(Payload::new(b"1".to_vec(), "text/plain")),
            PublicationData::EventData(
                Payload::new(b"1".to_vec(), "application/json").with_content_coding("gzip"),
            ),
        ] {
            let original = Arc::clone(&d.payload().body);
            let r = owner
                .publish_notification(
                    native(&mut exposure),
                    key,
                    AffordanceTarget::Event(Arc::from("alarm")),
                    d,
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .err()
                .unwrap();
            assert!(Arc::ptr_eq(&original, &r.into_input().payload().body));
        }
        assert_eq!(service.locally_published(), 0);
        let mut peer = Peer::new(&broker);
        peer.subscribe("bounded/alarm");
        let mut e = emit(&mut owner, &mut exposure, b"1.20e1");
        finish(&mut e).await;
        peer.event("bounded/alarm", b"1.20e1");
        let d = data(b"2");
        let original = Arc::clone(&d.payload().body);
        let r = owner
            .publish_notification(
                native(&mut exposure),
                key,
                AffordanceTarget::Event(Arc::from("alarm")),
                d,
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .err()
            .unwrap();
        assert!(matches!(r.error().error(), CoreError::Backpressure(_)));
        assert!(Arc::ptr_eq(&original, &r.into_input().payload().body));
        assert_eq!(service.locally_published(), 1);
        peer.absent();
        e.acknowledge().ok().unwrap();
        let mut next = emit(&mut owner, &mut exposure, b"2");
        finish(&mut next).await;
        peer.event("bounded/alarm", b"2");
        drop(next);
        close(&mut owner, &mut exposure);
        drop(exposure);
        assert_empty(&accounts);
    });
}

#[test]
fn cancel_deadline_and_replacement_do_not_publish_or_retarget_pending_data() {
    runtime().block_on(async {
        let broker = Broker::start();
        let provider = registration(broker.address, 8192);
        let service = MqttNotificationService::new(&provider).unwrap();
        let accounts = bank();
        let mut owner = owner();
        let text = td(broker.address, "current/alarm").to_string();
        let mut old = stage(
            &owner,
            input(&owner, authority(1), &provider, &service, &text, None),
            &accounts,
        );
        publish(&mut owner, &mut old).await;
        let mut peer = Peer::new(&broker);
        peer.subscribe("current/alarm");
        let mut e = emit(&mut owner, &mut old, b"1");
        e.cancel();
        assert!(
            e.step(&mut cx(), &mut WorkBudget::new(), now())
                .is_pending()
        );
        assert!(matches!(e.first_cause(), Some(CoreError::Cancelled(_))));
        finish(&mut e).await;
        let s = e.acknowledge().ok().unwrap();
        assert!(matches!(s.cause, Some(CoreError::Cancelled(_))));
        assert_eq!(s.native.effect, PublicationEffect::NotPublished);
        let key = old.key();
        let mut timed = owner
            .publish_notification(
                native(&mut old),
                key,
                AffordanceTarget::Event(Arc::from("alarm")),
                data(b"2"),
                Deadline::at(now().checked_add_ticks(1).unwrap()),
                now(),
                &mut budget(),
            )
            .ok()
            .unwrap();
        assert!(
            timed
                .step(
                    &mut cx(),
                    &mut budget(),
                    now().checked_add_ticks(1).unwrap()
                )
                .is_ready()
        );
        assert!(matches!(
            timed.acknowledge().ok().unwrap().cause,
            Some(CoreError::TimedOut(_))
        ));
        let mut pending = emit(&mut owner, &mut old, b"3");
        let mut successor = stage(
            &owner,
            input(&owner, authority(1), &provider, &service, &text, Some(key)),
            &accounts,
        );
        publish(&mut owner, &mut successor).await;
        finish(&mut pending).await;
        let s = pending.acknowledge().ok().unwrap();
        assert_eq!(s.instance, key);
        assert!(matches!(s.cause, Some(CoreError::StaleHandle(_))));
        assert_eq!(s.native.effect, PublicationEffect::NotPublished);
        assert!(
            owner
                .publish_notification(
                    native(&mut old),
                    key,
                    AffordanceTarget::Event(Arc::from("alarm")),
                    data(b"4"),
                    Deadline::NONE,
                    now(),
                    &mut budget()
                )
                .is_err()
        );
        peer.absent();
        assert_eq!(service.locally_published(), 0);
        close(&mut owner, &mut old);
        drop(old);
        assert!(service.connected());
        let mut e = emit(&mut owner, &mut successor, b"5");
        finish(&mut e).await;
        peer.event("current/alarm", b"5");
        drop(e);
        close(&mut owner, &mut successor);
        drop(successor);
        assert_empty(&accounts);
        assert!(!service.connected());
    });
}

#[test]
fn shared_connection_outlives_one_thing_and_releases_after_final_use() {
    runtime().block_on(async {
        let broker = Broker::start();
        let provider = registration(broker.address, 8192);
        let service = MqttNotificationService::new(&provider).unwrap();
        let accounts = bank();
        let mut owner = owner();
        let mut a = stage(
            &owner,
            input(
                &owner,
                authority(1),
                &provider,
                &service,
                &td(broker.address, "a/alarm").to_string(),
                None,
            ),
            &accounts,
        );
        let mut b = stage(
            &owner,
            input(
                &owner,
                authority(2),
                &provider,
                &service,
                &td(broker.address, "b/alarm").to_string(),
                None,
            ),
            &accounts,
        );
        publish(&mut owner, &mut a).await;
        publish(&mut owner, &mut b).await;
        assert_eq!(
            broker.logs().matches("as cz").count(),
            1,
            "Things share one real broker connection"
        );
        let mut peer = Peer::new(&broker);
        peer.subscribe("+/alarm");
        close(&mut owner, &mut a);
        drop(a);
        assert!(service.connected());
        let mut e = emit(&mut owner, &mut b, b"9");
        finish(&mut e).await;
        peer.event("b/alarm", b"9");
        b.begin_close();
        assert!(
            native(&mut b)
                .step(&mut owner, &mut cx(), &mut budget())
                .unwrap()
                .is_pending()
        );
        assert!(b.retire(&mut owner, &mut budget()).is_err());
        assert!(service.connected());
        e.acknowledge().ok().unwrap();
        close(&mut owner, &mut b);
        drop(b);
        assert!(!service.connected());
        assert_empty(&accounts);
        let end = Instant::now() + Duration::from_secs(1);
        while !broker
            .logs()
            .lines()
            .any(|l| l.contains("Client cz") && l.contains("closed its connection"))
        {
            assert!(
                Instant::now() < end,
                "broker did not observe actual socket release: {}",
                broker.logs()
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });
}

#[test]
fn partial_write_cancellation_is_uncertain_and_fences_shared_native_service() {
    runtime().block_on(async {
        let broker = Broker::start();
        let provider = registration(broker.address, 1);
        let service = MqttNotificationService::new(&provider).unwrap();
        let accounts = bank();
        let mut owner = owner();
        let mut a = stage(
            &owner,
            input(
                &owner,
                authority(1),
                &provider,
                &service,
                &td(broker.address, "a/alarm").to_string(),
                None,
            ),
            &accounts,
        );
        let mut b = stage(
            &owner,
            input(
                &owner,
                authority(2),
                &provider,
                &service,
                &td(broker.address, "b/alarm").to_string(),
                None,
            ),
            &accounts,
        );
        publish(&mut owner, &mut a).await;
        publish(&mut owner, &mut b).await;
        let mut e = emit(&mut owner, &mut a, b"1");
        assert!(e.step(&mut cx(), &mut budget(), now()).is_pending());
        e.cancel();
        finish(&mut e).await;
        let s = e.acknowledge().ok().unwrap();
        assert!(matches!(s.cause, Some(CoreError::Cancelled(_))));
        assert_eq!(s.native.effect, PublicationEffect::Uncertain);
        assert!(!service.connected());
        assert!(service.first_failure().is_some());
        assert_eq!(service.locally_published(), 0);
        let _ = native(&mut a)
            .step(&mut owner, &mut cx(), &mut budget())
            .unwrap();
        let _ = native(&mut b)
            .step(&mut owner, &mut cx(), &mut budget())
            .unwrap();
        assert!(owner.description(a.key()).is_none());
        assert!(owner.description(b.key()).is_none());
        assert!(native(&mut a).first_failure().is_some());
        assert!(native(&mut b).first_failure().is_some());
        close(&mut owner, &mut a);
        close(&mut owner, &mut b);
        drop(a);
        drop(b);
        assert_empty(&accounts);
    });
}

#[test]
fn late_native_success_retains_original_effect_and_funding_after_root_loss() {
    runtime().block_on(async {
        let broker = Broker::start();
        let provider = registration(broker.address, 8192);
        let service = MqttNotificationService::new(&provider).unwrap();
        let accounts = bank();
        let mut owner = owner();
        let mut exposure = stage(
            &owner,
            input(
                &owner,
                authority(1),
                &provider,
                &service,
                &td(broker.address, "late/alarm").to_string(),
                None,
            ),
            &accounts,
        );
        publish(&mut owner, &mut exposure).await;
        let mut peer = Peer::new(&broker);
        peer.subscribe("late/alarm");
        let mut e = emit(&mut owner, &mut exposure, b"42");
        let mut no_cleanup = budget();
        no_cleanup.set_remaining(WorkClass::CleanupItems, 0);
        assert!(e.step(&mut cx(), &mut no_cleanup, now()).is_pending());
        peer.event("late/alarm", b"42");
        drop(exposure);
        drop(owner);
        assert!(service.connected());
        assert!(
            accounts
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        finish(&mut e).await;
        let s = e.acknowledge().ok().unwrap();
        assert!(matches!(s.cause, Some(CoreError::StaleHandle(_))));
        assert_eq!(s.native.effect, PublicationEffect::LocallyPublished);
        assert!(!service.connected());
        assert_empty(&accounts);
    });
}

#[test]
fn real_broker_loss_reports_first_failure_and_retires_without_replay() {
    runtime().block_on(async {
        let mut broker = Broker::start();
        let provider = registration(broker.address, 8192);
        let service = MqttNotificationService::new(&provider).unwrap();
        let accounts = bank();
        let mut owner = owner();
        let mut exposure = stage(
            &owner,
            input(
                &owner,
                authority(1),
                &provider,
                &service,
                &td(broker.address, "failure/alarm").to_string(),
                None,
            ),
            &accounts,
        );
        publish(&mut owner, &mut exposure).await;
        let mut pending = emit(&mut owner, &mut exposure, b"1");
        broker.stop();
        let end = Instant::now() + Duration::from_secs(2);
        while native(&mut exposure).first_failure().is_none() {
            let _ = native(&mut exposure)
                .step(&mut owner, &mut cx(), &mut budget())
                .unwrap();
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let first = native(&mut exposure).first_failure().unwrap().clone();
        finish(&mut pending).await;
        let s = pending.acknowledge().ok().unwrap();
        assert!(matches!(s.cause, Some(CoreError::StaleHandle(_))));
        assert_eq!(s.native.effect, PublicationEffect::NotPublished);
        assert_eq!(service.locally_published(), 0);
        assert!(!service.connected());
        close(&mut owner, &mut exposure);
        assert_eq!(native(&mut exposure).first_failure(), Some(&first));
        drop(exposure);
        assert_empty(&accounts);
    });
}
