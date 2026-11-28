#![cfg(feature = "prepare")]

use clinkz_wot_core::{
    consumer::*, host_accounts::HostResourceAccounts, notification::*,
    security::SecurityRequirement, *,
};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::{consumer_notification::*, notification::*};
use clinkz_wot_td::thing::Thing;
use independent_open_consumer::event::*;
use serde_json::json;
use std::{
    cell::Cell,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, UdpSocket},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering::SeqCst},
    },
    task::{Context, Waker},
    time::Duration,
};

#[path = "event_selection/qualification.rs"]
mod qualification;

#[derive(Default)]
struct Access {
    missing: AtomicBool,
    denied: AtomicBool,
    // An eligibility writer can run inside a candidate callback.
    change: std::sync::Mutex<Option<Box<dyn FnOnce() + Send>>>,
}
impl ConsumerAccessService for Access {
    fn qualification_revision(&self) -> u64 {
        u64::from(self.missing.load(SeqCst))
    }
    fn supports_security(&self, r: &SecurityRequirement) -> bool {
        *r == SecurityRequirement::NOSEC && !self.missing.load(SeqCst)
    }
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
        if let Some(change) = self.change.lock().unwrap().take() {
            change();
        }
        if self.denied.load(SeqCst) {
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
        s: &str,
        op: Operation,
    ) -> CoreResult<()> {
        self.authorize(id, s, op)
    }
    fn work(&self) -> u64 {
        1
    }
    fn retained_bytes(&self) -> u64 {
        64
    }
}

fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, c| b.with_remaining(c, 1_000_000))
}
fn bank() -> Arc<HostResourceAccounts> {
    Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS))
}
fn clean(bank: &HostResourceAccounts) {
    for k in ResourceKind::ALL {
        assert_eq!(bank.account(k).used(), 0, "{k:?}");
    }
}
fn td(addr: SocketAddr, specs: &[(Option<&str>, Option<&str>)]) -> Thing {
    let forms: Vec<_> = specs
        .iter()
        .map(|(protocol, coding)| {
            let mut form = json!({
                "href": format!("testevent://{addr}/alarm"),
                "op": ["subscribeevent", "unsubscribeevent"]
            });
            if let Some(protocol) = protocol {
                form["subprotocol"] = json!(protocol);
            }
            if let Some(coding) = coding {
                form["contentCoding"] = json!(coding);
            }
            form
        })
        .collect();
    serde_json::from_value(json!({
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "title": "Dual transport events",
        "security": "n",
        "securityDefinitions": {"n": {"scheme": "nosec"}},
        "events": {"alarm": {"data": {"type": "integer"}, "forms": forms}}
    }))
    .unwrap()
}
struct Peers {
    stream: TcpListener,
    datagram: UdpSocket,
    addr: SocketAddr,
}
impl Peers {
    fn new() -> Self {
        Self::on(std::net::Ipv4Addr::LOCALHOST.into())
    }
    fn on(ip: std::net::IpAddr) -> Self {
        for _ in 0..16 {
            let stream = TcpListener::bind((ip, 0)).unwrap();
            let addr = stream.local_addr().unwrap();
            let datagram = match UdpSocket::bind(addr) {
                Ok(socket) => socket,
                Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => continue,
                Err(e) => panic!("could not bind test datagram endpoint: {e}"),
            };
            stream.set_nonblocking(true).unwrap();
            datagram
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            return Self {
                stream,
                datagram,
                addr,
            };
        }
        panic!("could not reserve a loopback port for both transports")
    }
    fn quiet(&self) {
        assert_eq!(
            self.stream.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        self.datagram.set_nonblocking(true).unwrap();
        assert_eq!(
            self.datagram.recv(&mut [0; 16]).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        self.datagram.set_nonblocking(false).unwrap();
    }
    fn packet(&self, expected: &[u8]) -> SocketAddr {
        let mut buf = [0; 16];
        let (n, from) = self.datagram.recv_from(&mut buf).unwrap();
        assert_eq!(&buf[..n], expected);
        from
    }
}
struct Deployment {
    stream: Stream,
    datagram: Datagram,
    stream_data: Arc<independent_open_consumer::Probe>,
    stream_access: Arc<Access>,
    datagram_access: Arc<Access>,
    services: Arc<HostResourceAccounts>,
    stream_services: RetainedConsumerServices,
    datagram_services: RetainedConsumerServices,
    revision: Arc<AtomicU64>,
}
impl Deployment {
    fn new(peer: SocketAddr) -> Self {
        let services = bank();
        let stream_data = Arc::default();
        let stream_access = Arc::default();
        let datagram_access = Arc::default();
        let stream_services = Self::retained(
            &services,
            Arc::clone(&stream_data),
            Arc::clone(&stream_access),
        );
        let datagram_services =
            Self::retained(&services, Arc::default(), Arc::clone(&datagram_access));
        Self {
            stream: Stream {
                peer,
                compilations: Arc::default(),
            },
            datagram: Datagram {
                config: DatagramConfig {
                    peer,
                    require_hex: false,
                },
                compilations: Rc::default(),
            },
            stream_data,
            stream_access,
            datagram_access,
            services,
            stream_services,
            datagram_services,
            revision: Arc::default(),
        }
    }
    fn retained(
        bank: &Arc<HostResourceAccounts>,
        probe: Arc<independent_open_consumer::Probe>,
        access: Arc<Access>,
    ) -> RetainedConsumerServices {
        RetainedConsumerServices {
            data: ConsumerServiceOwner::retain_data(
                independent_open_consumer::services(probe).data,
                bank,
            )
            .ok()
            .unwrap(),
            access: ConsumerServiceOwner::retain_access(access, bank)
                .ok()
                .unwrap(),
        }
    }
    fn providers(&self, reverse: bool) -> Vec<ConsumerNotificationProvider<SelectedEvent>> {
        let mut entries = vec![
            RetainedConsumerNotificationRegistration {
                binding: self.stream.clone(),
                services: self.stream_services.clone(),
            }
            .into_provider(SelectedEvent::Stream),
            RetainedConsumerNotificationRegistration {
                binding: self.datagram.clone(),
                services: self.datagram_services.clone(),
            }
            .into_provider(SelectedEvent::Datagram),
        ];
        if reverse {
            entries.reverse();
        }
        entries
    }
    fn registry(&self, reverse: bool) -> ConsumerNotificationRegistry<SelectedEvent> {
        let revision = self.revision.clone();
        ConsumerNotificationRegistry::try_new_revisioned(self.providers(reverse), move || {
            Some(revision.load(SeqCst))
        })
        .unwrap()
    }
}
fn select(
    registry: &ConsumerNotificationRegistry<SelectedEvent>,
    thing: &Thing,
    pin: ConsumerSelector,
    original: &Arc<HostResourceAccounts>,
    budget: &mut WorkBudget,
) -> CoreResult<SelectedEvent> {
    registry.prepare_notification(
        thing,
        &alarm(),
        NotificationKind::Event,
        pin,
        GatewayDefaultV1::LIMITS,
        original.clone(),
        budget,
    )
}
fn pin(form: Option<u32>, binding: Option<u32>) -> ConsumerSelector {
    ConsumerSelector {
        form,
        binding: binding.map(BindingId::new),
    }
}

struct Conversation<B: ConsumerNotificationBinding> {
    work: ConsumerNotifications<B>,
    cursor: NotificationCursor,
    owner: ConsumerInstance,
}
impl<B: ConsumerNotificationBinding + Clone + 'static> Conversation<B>
where
    B::Program: Clone + 'static,
    B::Configuration: Clone + 'static,
{
    fn start(d: ConsumerNotificationDefinition<B>, original: &Arc<HostResourceAccounts>) -> Self {
        let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
        let access = ConsumerServiceOwner::retain_access(Arc::new(Access::default()), original)
            .ok()
            .unwrap();
        let assoc = ConsumerNotificationAssociation::try_new(owner.reference(), Rc::new(d), access)
            .ok()
            .unwrap();
        let mut capacity = ConsumerNotifications::try_new(RawNotificationLimits {
            subscriptions: 1,
            queue_items: 1,
            queue_payload_bytes: 240,
            payload_backing_bytes: 200,
            control_backing_bytes: 0,
            target_name_bytes: 16,
            reports: 8,
        })
        .unwrap();
        let cursor = capacity
            .subscribe(
                &assoc,
                SubscriptionInput(None),
                ResponseRequirement::None,
                NotificationOverflow::Terminate,
            )
            .ok()
            .unwrap();
        assert!(
            capacity
                .subscribe(
                    &assoc,
                    SubscriptionInput(None),
                    ResponseRequirement::None,
                    NotificationOverflow::Terminate
                )
                .is_err()
        );
        Self {
            work: capacity,
            cursor,
            owner,
        }
    }
    async fn until(&mut self, state: NotificationState) {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                self.work
                    .progress(
                        self.cursor.identity(),
                        &mut Context::from_waker(Waker::noop()),
                    )
                    .unwrap();
                if self.work.status(self.cursor.identity()).unwrap().state == state {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap()
    }
    async fn receive(&mut self, expected: &[u8]) {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                self.until(NotificationState::Active).await;
                if let Some(item) = self.work.front(&mut self.cursor).unwrap() {
                    assert_eq!(item.data.unwrap().body, expected);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        self.work.advance(&mut self.cursor).unwrap();
    }
    async fn finish(mut self, mapped: bool) -> NotificationSettlement {
        if mapped {
            self.work
                .stop(&self.cursor, Some(CancellationInput(None)))
                .ok()
                .unwrap();
        } else {
            self.owner.revoke();
        }
        self.until(NotificationState::Closed).await;
        self.work.take_settlement(self.cursor.identity()).unwrap()
    }
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

#[test]
fn preferred_stream_and_later_coded_datagram_reach_their_peers_in_either_registration_order() {
    runtime().block_on(async {
        for reverse in [false, true] {
            let peers = Peers::new();
            let d = Deployment::new(peers.addr);
            let registry = d.registry(reverse);
            let thing = td(
                peers.addr,
                &[
                    (Some("stream"), None),
                    (Some("datagram"), Some("x-test-hex")),
                ],
            );
            let plans = bank();
            let SelectedEvent::Stream(stream) =
                select(&registry, &thing, pin(None, None), &plans, &mut work()).unwrap()
            else {
                panic!("first Form should win")
            };
            assert_eq!(stream.input().form, 0);
            assert_eq!(
                (
                    d.stream.compilations.load(SeqCst),
                    d.datagram.compilations.get()
                ),
                (1, 1)
            );
            let SelectedEvent::Datagram(pinned) = select(
                &registry,
                &thing,
                pin(Some(1), Some(72)),
                &plans,
                &mut work(),
            )
            .unwrap() else {
                panic!()
            };
            assert!(pinned.input().program.hex);
            drop(pinned);
            // The earlier Form loses its actual preparation service. Existing
            // definitions still require fresh permission at association and use.
            d.stream_access.missing.store(true, SeqCst);
            d.revision.fetch_add(1, SeqCst);
            assert!(matches!(
                select(&registry, &thing, pin(Some(0), None), &plans, &mut work()),
                Err(CoreError::UnsupportedOperation(_))
            ));
            let SelectedEvent::Datagram(datagram) =
                select(&registry, &thing, pin(None, None), &plans, &mut work()).unwrap()
            else {
                panic!("later executable Form should win")
            };
            assert_eq!(datagram.input().form, 1);
            assert!(datagram.input().program.hex);
            let original_services = d.services.clone();
            drop((thing, registry, d));
            peers.quiet();
            let active = bank();
            let mut a = Conversation::start(stream, &active);
            let mut b = Conversation::start(datagram, &active);
            peers.quiet(); // acceptance is still local construction
            a.until(NotificationState::Active).await;
            let (mut wire, _) = peers.stream.accept().unwrap();
            wire.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
            let mut control = [0];
            wire.read_exact(&mut control).unwrap();
            assert_eq!(&control, b"+");
            b.until(NotificationState::Active).await;
            let destination = peers.packet(b"LISTEN");
            wire.write_all(b"4").unwrap();
            peers.datagram.send_to(b"37", destination).unwrap();
            a.receive(b"4").await;
            b.receive(b"7").await;
            let a = a.finish(false).await;
            assert!(matches!(a.first_cause, Some(CoreError::StaleHandle(_))));
            assert_eq!(
                wire.read(&mut control).unwrap(),
                0,
                "revoked stream closed locally"
            );
            peers.datagram.send_to(b"38", destination).unwrap();
            b.receive(b"8").await;
            let b = b.finish(true).await;
            assert_eq!(peers.packet(b"UNLISTEN"), destination);
            assert!(matches!(b.first_cause, Some(CoreError::Cancelled(_))));
            assert_eq!(b.remote_cancellation, RemoteCancellation::Unknown);
            drop(UdpSocket::bind(destination).expect("datagram lease ended"));
            assert!(
                plans.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0,
                "reports retain original definitions"
            );
            drop((a, b));
            clean(&plans);
            clean(&active);
            clean(&original_services);
        }
    });
}

#[test]
fn a_new_competitor_changes_the_next_decision_and_pins_execute_after_registry_drop() {
    runtime().block_on(async {
        let peers = Peers::new();
        let d = Deployment::new(peers.addr);
        let plans = bank();
        let thing = td(peers.addr, &[(None, None)]);
        let registry = d.registry(false);
        d.stream_data.data_lost.store(true, SeqCst);
        let SelectedEvent::Datagram(existing) =
            select(&registry, &thing, pin(None, None), &plans, &mut work()).unwrap()
        else {
            panic!()
        };
        d.stream_data.data_lost.store(false, SeqCst);
        d.revision.fetch_add(1, SeqCst);
        for reverse in [false, true] {
            let competing = d.registry(reverse);
            assert!(matches!(
                select(&competing, &thing, pin(None, None), &plans, &mut work()),
                Err(CoreError::Selection {
                    reason: SelectionFailureReason::AmbiguousBindingOwner,
                    ..
                })
            ));
        }
        let SelectedEvent::Stream(pinned) = select(
            &registry,
            &thing,
            pin(Some(0), Some(71)),
            &plans,
            &mut work(),
        )
        .unwrap() else {
            panic!()
        };
        let original_services = d.services.clone();
        drop((registry, thing, d));
        peers.quiet();
        let mut stream = Conversation::start(pinned, &plans);
        stream.until(NotificationState::Active).await;
        let (mut peer, _) = peers.stream.accept().unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let mut byte = [0];
        peer.read_exact(&mut byte).unwrap();
        assert_eq!(&byte, b"+");
        peer.write_all(b"2").unwrap();
        stream.receive(b"2").await;
        drop(stream.finish(true).await);
        peer.read_exact(&mut byte).unwrap();
        assert_eq!(&byte, b"-");
        let mut original = Conversation::start(existing, &plans);
        original.until(NotificationState::Active).await;
        let port = peers.packet(b"LISTEN");
        peers.datagram.send_to(b"9", port).unwrap();
        original.receive(b"9").await;
        drop(original.finish(true).await);
        assert_eq!(peers.packet(b"UNLISTEN"), port);
        clean(&plans);
        clean(&original_services);
    });
}

#[test]
fn a_reload_fences_both_a_rejected_earlier_form_and_a_rejected_competitor() {
    let peers = Peers::new();
    for first_protocol in [Some("stream"), None] {
        let d = Deployment::new(peers.addr);
        let plans = bank();
        let thing = td(
            peers.addr,
            &[(first_protocol, None), (Some("datagram"), None)],
        );
        d.stream_data.data_lost.store(true, SeqCst);
        let data = d.stream_data.clone();
        let revision = d.revision.clone();
        *d.datagram_access.change.lock().unwrap() = Some(Box::new(move || {
            data.data_lost.store(false, SeqCst);
            revision.fetch_add(1, SeqCst);
        }));
        let closing_peak = Rc::new(Cell::new(0));
        let closing_scratch = Rc::new(Cell::new(0));
        let registry = ConsumerNotificationRegistry::try_new_revisioned(d.providers(false), {
            let revision = d.revision.clone();
            let plans = plans.clone();
            let peak = closing_peak.clone();
            let scratch = closing_scratch.clone();
            move || {
                peak.set(
                    plans
                        .account(ResourceKind::AdmissionPeakLiveBytesGlobalMax)
                        .used(),
                );
                scratch.set(
                    plans
                        .account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
                        .used(),
                );
                Some(revision.load(SeqCst))
            }
        })
        .unwrap();
        assert!(matches!(
            select(&registry, &thing, pin(None, None), &plans, &mut work()),
            Err(CoreError::StaleHandle(_))
        ));
        assert!(closing_peak.get() >= d.datagram.preparation_bounds().retained_bytes);
        assert_eq!(
            closing_scratch.get(),
            d.datagram.preparation_bounds().temporary_bytes
        );
        clean(&plans);
        let stable = select(&registry, &thing, pin(None, None), &plans, &mut work());
        match (first_protocol, stable) {
            (Some("stream"), Ok(SelectedEvent::Stream(chosen))) => {
                assert_eq!(chosen.input().form, 0)
            }
            (
                None,
                Err(CoreError::Selection {
                    reason: SelectionFailureReason::AmbiguousBindingOwner,
                    ..
                }),
            ) => {}
            _ => panic!("stable decision disagreed with the new configuration"),
        }
        let original_services = d.services.clone();
        drop((registry, d));
        peers.quiet();
        clean(&plans);
        clean(&original_services);
    }
}

#[test]
fn decisive_competition_pays_for_both_mappings_and_refusals_release_all_capacity() {
    for reverse in [false, true] {
        let peers = Peers::new();
        let mut d = Deployment::new(peers.addr);
        let thing = td(peers.addr, &[(None, None)]);
        let registry = d.registry(reverse);
        let plans = bank();
        let mut credit = work().with_remaining(
            WorkClass::BindingPolls,
            2 * d.stream.preparation_bounds().work - 1,
        );
        assert!(matches!(
            select(&registry, &thing, pin(None, None), &plans, &mut credit),
            Err(CoreError::Backpressure(_))
        ));
        assert_eq!(
            d.stream.compilations.load(SeqCst) + d.datagram.compilations.get(),
            1
        );
        clean(&plans);
        let mut credit = work().with_remaining(
            WorkClass::BindingPolls,
            2 * d.stream.preparation_bounds().work,
        );
        assert!(matches!(
            select(&registry, &thing, pin(None, None), &plans, &mut credit),
            Err(CoreError::Selection {
                reason: SelectionFailureReason::AmbiguousBindingOwner,
                ..
            })
        ));
        clean(&plans);
        let mut limits = GatewayDefaultV1::LIMITS.clone();
        assert!(limits.set(ResourceKind::CleanupItemsMax, Some(1)));
        let one = Arc::new(HostResourceAccounts::new(&limits));
        assert!(matches!(
            select(&registry, &thing, pin(None, None), &one, &mut work()),
            Err(CoreError::LimitExceeded {
                resource: ResourceKind::CleanupItemsMax,
                ..
            })
        ));
        clean(&one);
        let SelectedEvent::Datagram(winner) =
            select(&registry, &thing, pin(None, Some(72)), &one, &mut work()).unwrap()
        else {
            panic!()
        };
        assert_eq!(one.account(ResourceKind::CleanupItemsMax).used(), 1);
        drop(winner);
        clean(&one);
        assert!(matches!(
            select(
                &registry,
                &td(peers.addr, &[(None, Some("unimplemented"))]),
                pin(None, None),
                &plans,
                &mut work()
            ),
            Err(CoreError::UnsupportedOperation(_))
        ));
        d.datagram_access.denied.store(true, SeqCst);
        assert!(matches!(
            select(&registry, &thing, pin(None, Some(72)), &plans, &mut work()),
            Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                ..
            })
        ));
        drop(registry);
        // Native support on this exact configured executor declines the plain
        // Form after compilation. A different registration can still execute it.
        d.datagram.config.require_hex = true;
        let registry = d.registry(reverse);
        assert!(matches!(
            select(&registry, &thing, pin(None, Some(72)), &plans, &mut work()),
            Err(CoreError::UnsupportedOperation(_))
        ));
        d.datagram_access.denied.store(false, SeqCst);
        assert!(matches!(
            select(&registry, &thing, pin(None, None), &plans, &mut work()),
            Ok(SelectedEvent::Stream(_))
        ));
        drop(registry);
        peers.quiet();
        clean(&plans);
        let original_services = d.services.clone();
        drop(d);
        clean(&original_services);
    }
}

#[test]
fn each_native_program_fits_alone_but_overlapping_competitors_need_peak_capacity() {
    use ResourceKind::{AdmissionPeakLiveBytesGlobalMax, PeakLiveBytesPerAdmissionMax};
    let peers = Peers::new();
    let deployment = Deployment::new(peers.addr);
    let registry = deployment.registry(false);
    let thing = td(peers.addr, &[(None, None)]);
    let one_mapping = [71, 72]
        .into_iter()
        .map(|id| {
            let original = bank();
            drop(
                select(
                    &registry,
                    &thing,
                    pin(None, Some(id)),
                    &original,
                    &mut work(),
                )
                .unwrap(),
            );
            clean(&original);
            original.account(AdmissionPeakLiveBytesGlobalMax).peak()
        })
        .max()
        .unwrap();

    for resource in [
        PeakLiveBytesPerAdmissionMax,
        AdmissionPeakLiveBytesGlobalMax,
    ] {
        let plans = bank();
        let mut limits = GatewayDefaultV1::LIMITS.clone();
        assert!(limits.set(resource, Some(one_mapping)));
        let result = registry.prepare_notification(
            &thing,
            &alarm(),
            NotificationKind::Event,
            ConsumerSelector::default(),
            &limits,
            plans.clone(),
            &mut work(),
        );
        assert!(
            matches!(result, Err(CoreError::LimitExceeded {resource: actual, ..}) if actual == resource)
        );
        clean(&plans);
    }
    peers.quiet();
}

#[test]
fn an_uncoded_occurrence_on_the_selected_coded_route_fences_delivery() {
    runtime().block_on(async {
        let peers = Peers::new();
        let plans = bank();
        let definition = {
            let deployment = Deployment::new(peers.addr);
            let registry = deployment.registry(false);
            let thing = td(peers.addr, &[(Some("datagram"), Some("x-test-hex"))]);
            let SelectedEvent::Datagram(definition) =
                select(&registry, &thing, pin(None, None), &plans, &mut work()).unwrap()
            else {
                panic!()
            };
            definition
        };
        let mut subscription = Conversation::start(definition, &plans);
        subscription.until(NotificationState::Active).await;
        let client = peers.packet(b"LISTEN");
        peers.datagram.send_to(b"7", client).unwrap();
        subscription.until(NotificationState::Closed).await;
        let report = subscription
            .work
            .take_settlement(subscription.cursor.identity())
            .unwrap();
        assert!(matches!(report.first_cause, Some(CoreError::Validation(_))));
        assert_eq!(report.delivered_items, 0);
        drop(UdpSocket::bind(client).unwrap());
        drop((report, subscription));
        peers.quiet();
        clean(&plans);
    });
}

#[test]
fn production_mqtt_and_downstream_udp_execute_from_one_registry() {
    use clinkz_wot_binding_mqtt::{
        MqttConsumerConfig, MqttConsumerService, MqttNotificationConsumer,
    };
    enum Route {
        Mqtt(ConsumerNotificationDefinition<MqttNotificationConsumer>),
        Datagram(ConsumerNotificationDefinition<Datagram>),
    }

    let peers = Peers::new();
    let plans = bank();
    let (mqtt, udp) = {
        let deployment = Deployment::new(peers.addr);
        let service = MqttConsumerService::new(
            MqttConsumerConfig::for_loopback(peers.addr, 8192).unwrap(),
            BindingId::new(8),
            BindingGeneration::INITIAL,
        )
        .unwrap();
        let registry = ConsumerNotificationRegistry::try_new_fixed(vec![
            RetainedConsumerNotificationRegistration {
                binding: deployment.datagram.clone(),
                services: deployment.datagram_services.clone(),
            }
            .into_provider(Route::Datagram),
            RetainedConsumerNotificationRegistration {
                binding: service.binding(),
                services: deployment.stream_services.clone(),
            }
            .into_provider(Route::Mqtt),
        ])
        .unwrap();
        let thing = serde_json::from_value(json!({
            "@context": ["https://www.w3.org/2022/wot/td/v1.1", {"mqv":"https://www.w3.org/2019/wot/mqtt#"}],
            "title": "MQTT and datagram alarm", "security": "n", "securityDefinitions": {"n": {"scheme": "nosec"}},
            "events": {"alarm": {"data": {"type": "integer"}, "forms": [
                {"href": format!("mqtt://{}", peers.addr), "mqv:filter": "alarm", "mqv:qos": "0", "mqv:retain": false},
                {"href": format!("testevent://{}/alarm", peers.addr), "subprotocol": "datagram", "contentCoding": "x-test-hex"}
            ]}}
        })).unwrap();
        let mut limits = GatewayDefaultV1::LIMITS.clone();
        assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
        let prepare = || {
            registry
                .prepare_notification(
                    &thing,
                    &alarm(),
                    NotificationKind::Event,
                    ConsumerSelector::default(),
                    &limits,
                    plans.clone(),
                    &mut work(),
                )
                .unwrap()
        };
        let Route::Mqtt(mqtt) = prepare() else {
            panic!("MQTT Form is preferred")
        };
        deployment.stream_access.missing.store(true, SeqCst);
        let Route::Datagram(udp) = prepare() else {
            panic!("the later UDP Form is viable")
        };
        assert_eq!((mqtt.input().form, udp.input().form), (0, 1));
        (mqtt, udp)
    };
    peers.quiet();
    let peer = mqtt_peer(peers.stream.try_clone().unwrap());
    runtime().block_on(async {
        let mut first = Conversation::start(mqtt, &plans);
        first.until(NotificationState::Active).await;
        first.receive(b"5").await;
        let mut second = Conversation::start(udp, &plans);
        second.until(NotificationState::Active).await;
        let client = peers.packet(b"LISTEN");
        assert_eq!(
            first.finish(true).await.remote_cancellation,
            RemoteCancellation::Confirmed
        );
        drop(peer.join().unwrap());
        peers.datagram.send_to(b"36", client).unwrap();
        second.receive(b"6").await;
        drop(second.finish(true).await);
        assert_eq!(peers.packet(b"UNLISTEN"), client);
        clean(&plans);
    });
}

// Independent MQTT 3.1.1 wire peer for the one QoS 0 exchange above. Every
// packet in this fixture fits a one-byte remaining length.
fn mqtt_peer(listener: TcpListener) -> std::thread::JoinHandle<std::net::TcpStream> {
    use std::net::TcpStream;
    fn packet(stream: &mut TcpStream, header: u8) -> Vec<u8> {
        let mut prefix = [0; 2];
        stream.read_exact(&mut prefix).unwrap();
        assert_eq!(prefix[0], header);
        assert!(prefix[1] < 128);
        let mut body = vec![0; usize::from(prefix[1])];
        stream.read_exact(&mut body).unwrap();
        body
    }
    std::thread::spawn(move || {
        listener.set_nonblocking(false).unwrap();
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        assert!(packet(&mut stream, 0x10).starts_with(b"\0\x04MQTT\x04"));
        stream.write_all(&[0x20, 2, 0, 0]).unwrap();
        let sub = packet(&mut stream, 0x82);
        assert_eq!(&sub[2..], b"\0\x05alarm\0");
        stream.write_all(&[0x90, 3, sub[0], sub[1], 0]).unwrap();
        stream.write_all(b"\x30\x08\0\x05alarm5").unwrap();
        let stop = packet(&mut stream, 0xa2);
        assert_eq!(&stop[2..], b"\0\x05alarm");
        stream.write_all(&[0xb0, 2, stop[0], stop[1]]).unwrap();
        stream
    })
}
