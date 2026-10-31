use super::*;
use clinkz_wot_servient::{
    host_accounts::HostResourceAccounts,
    producer_composition::{
        ComposedProducerExposure, ComposedProducerInput, OwnedProducerParticipation,
    },
};

type HttpRuntime = RawFiniteRuntime<HttpFiniteBinding>;

fn bank() -> Arc<HostResourceAccounts> {
    Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS))
}
fn limits() -> Arc<ResourceLimits> {
    Arc::new(GatewayDefaultV1::LIMITS.clone())
}
fn live(bank: &HostResourceAccounts) -> u64 {
    bank.account(ResourceKind::EngineLiveBytesGlobalMax).used()
}
fn scratch(bank: &HostResourceAccounts) -> u64 {
    bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
        .used()
}
fn assert_empty(bank: &HostResourceAccounts) {
    for kind in [
        ResourceKind::EngineLiveBytesGlobalMax,
        ResourceKind::AdmissionTemporaryBytesGlobalMax,
        ResourceKind::CleanupItemsMax,
        ResourceKind::CleanupBytesMax,
        ResourceKind::ThingsGlobalMax,
    ] {
        assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
    }
}

fn http_input(
    owner: &RawProducerServient,
    authority: ProducerInstanceAuthority,
    addresses: [SocketAddr; 2],
    counts: Rc<Counts>,
) -> ComposedProducerInput {
    let key = authority.key();
    let text = http_properties(addresses[0], addresses[1]);
    let instance = RawProducerInstance::new(
        authority,
        ["left", "right"]
            .into_iter()
            .map(|name| {
                RawFiniteBehavior::new(
                    AffordanceTarget::Property(Arc::from(name)),
                    Operation::ReadProperty,
                    Property(Rc::clone(&counts)),
                )
            })
            .collect(),
    )
    .with_description(text);
    let providers: Vec<Rc<dyn ProducerCapability>> = addresses
        .into_iter()
        .enumerate()
        .map(|(i, addr)| {
            Rc::new(
                HttpFiniteProducer::try_new(
                    HttpProducerConfig::for_loopback(addr).unwrap(),
                    BindingId::new(101 + i as u32),
                    BindingGeneration::INITIAL,
                    100,
                )
                .unwrap(),
            ) as Rc<dyn ProducerCapability>
        })
        .collect();
    let refs: Vec<_> = providers.iter().map(|p| p.as_ref()).collect();
    let prepared = prepare_thing(
        instance.description().unwrap(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut budget(),
        &refs,
        |scope, op| match scope {
            ValidatedProducerScope::Property(n) => instance.supports_property(n, op),
            _ => false,
        },
    )
    .unwrap();
    let participants: Vec<Box<dyn OwnedProducerParticipation>> = prepared
        .operations()
        .iter()
        .zip(addresses)
        .map(|(operation, address)| {
            let FormSite::Property(name) = &operation.site else {
                panic!("property")
            };
            Box::new(http_read(
                owner,
                address,
                operation
                    .definition::<HttpProducerProgram>()
                    .unwrap()
                    .clone(),
                key,
                name,
                operation.binding.0,
                FiniteValidation::Required,
            )) as Box<dyn OwnedProducerParticipation>
        })
        .collect();
    ComposedProducerInput {
        instance,
        prepared,
        providers,
        participants,
        expected: None,
        handler_backing_bytes: std::mem::size_of::<Counts>(),
    }
}

fn action_input(
    owner: &RawProducerServient,
    authority: ProducerInstanceAuthority,
    native: &Rc<RefCell<ActionNative>>,
    counts: Rc<Counts>,
    expected: Option<ProducerInstanceKey>,
) -> (ComposedProducerInput, [ActionRoute; 3]) {
    let key = authority.key();
    let provider: Rc<dyn ProducerCapability> =
        Rc::new(UdpActionProvider::new(native.borrow().address));
    let instance =
        action_instance(authority, counts).with_description(action_only(native.borrow().address));
    let prepared = prepare_thing(
        instance.description().unwrap(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut budget(),
        &[provider.as_ref()],
        |scope, op| match scope {
            ValidatedProducerScope::Action(n) => instance.supports_action(n, op),
            _ => false,
        },
    )
    .unwrap();
    let route = routes(&prepared);
    let _ = owner; // the native Action lease carries no logical dispatch handle
    (
        ComposedProducerInput {
            instance,
            prepared,
            providers: vec![provider],
            participants: vec![Box::new(ActionNative::lease(native, key))],
            expected,
            handler_backing_bytes: std::mem::size_of::<Counts>(),
        },
        route,
    )
}

#[test]
fn action_without_captured_validation_service_refuses_managed_handoff_before_native_setup() {
    let owner = owner();
    let address = udp_addr();
    let native = ActionNative::new(address);
    let accounts = bank();
    let (mut input, _) = action_input(
        &owner,
        key(1, Generation::INITIAL),
        &native,
        Rc::new(Counts::default()),
        None,
    );
    let key = input.instance.key();
    input.participants[0]
        .as_mut()
        .as_any_mut()
        .downcast_mut::<ActionLease>()
        .unwrap()
        .validator_enabled = false;
    let rejection = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&accounts),
        &mut budget(),
    )
    .err()
    .expect("a ready Action without a service cannot qualify");
    assert_eq!(owner.description(key), None);
    assert!(native.borrow().socket.is_none());
    drop(rejection);
    assert_empty(&accounts);
    UdpSocket::bind(address).unwrap();
}

#[test]
fn action_without_prepared_control_route_refuses_managed_handoff_before_native_setup() {
    let owner = owner();
    let address = udp_addr();
    let native = ActionNative::new(address);
    let accounts = bank();
    let (mut input, _) = action_input(
        &owner,
        key(1, Generation::INITIAL),
        &native,
        Rc::new(Counts::default()),
        None,
    );
    let key = input.instance.key();
    input.participants[0]
        .as_mut()
        .as_any_mut()
        .downcast_mut::<ActionLease>()
        .unwrap()
        .control_routes_enabled = false;
    let rejection = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&accounts),
        &mut budget(),
    )
    .err()
    .expect("a described control without its actual route cannot qualify");
    assert_eq!(owner.description(key), None);
    assert!(native.borrow().socket.is_none());
    drop(rejection);
    assert_empty(&accounts);
    UdpSocket::bind(address).unwrap();
}

fn start_http(exposure: &mut ComposedProducerExposure) {
    for i in 0..2 {
        assert_eq!(
            exposure.participant_mut::<HttpRuntime>(i).unwrap().step(
                &mut cx(),
                &mut budget(),
                now()
            ),
            Poll::Ready(RawFiniteState::InstalledClosed)
        );
    }
}
fn close_http(exposure: &mut ComposedProducerExposure) {
    exposure.begin_close();
    for i in 0..2 {
        let runtime = exposure.participant_mut::<HttpRuntime>(i).unwrap();
        while runtime.state() != RawFiniteState::Closed {
            let _ = runtime.step(&mut cx(), &mut budget(), now());
        }
    }
}

#[test]
fn aggregate_handoff_refuses_saturation_and_retains_unread_http_results() {
    let mut owner = owner();
    let accounts = bank();
    let addresses = [tcp_addr(), tcp_addr()];
    let counts = Rc::new(Counts::default());
    let input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        Rc::clone(&counts),
    );
    let mut exposure = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&accounts),
        &mut budget(),
    )
    .ok()
    .unwrap();
    assert!(live(&accounts) > 0 && scratch(&accounts) > 0);
    assert!(
        exposure
            .publish(&mut owner, &mut WorkBudget::new())
            .is_err()
    );
    assert!(exposure.publish(&mut owner, &mut budget()).is_err());
    start_http(&mut exposure);
    exposure.publish(&mut owner, &mut budget()).unwrap();
    assert_eq!(scratch(&accounts), 0);
    let retained = live(&accounts);
    let snapshot = accounts.account(ResourceKind::ThingsGlobalMax);
    let saturated = accounts
        .reserve(
            ResourceKind::ThingsGlobalMax,
            snapshot.limit() - snapshot.used(),
        )
        .unwrap();
    let rejected_addresses = [tcp_addr(), tcp_addr()];
    let input = http_input(
        &owner,
        key(2, Generation::INITIAL),
        rejected_addresses,
        Rc::new(Counts::default()),
    );
    let rejection = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&accounts),
        &mut budget(),
    )
    .err()
    .unwrap();
    let mut returned = rejection.into_input();
    assert_eq!(live(&accounts), retained);
    assert_eq!(
        returned.participants[0]
            .as_mut()
            .as_any_mut()
            .downcast_mut::<HttpRuntime>()
            .unwrap()
            .state(),
        RawFiniteState::Starting
    );
    for address in rejected_addresses {
        TcpListener::bind(address).unwrap();
    }
    drop(returned);
    drop(saturated);
    for (i, address) in addresses.into_iter().enumerate() {
        assert!(
            read_peer(
                address,
                &mut owner,
                exposure.participant_mut::<HttpRuntime>(i).unwrap()
            )
            .starts_with(b"HTTP/1.1 200")
        );
    }
    assert_eq!(counts.property.get(), 2);
    close_http(&mut exposure);
    assert!(exposure.retire(&mut owner, &mut budget()).is_err());
    assert_eq!(live(&accounts), retained);
    for i in 0..2 {
        let settlement = exposure
            .participant_mut::<HttpRuntime>(i)
            .unwrap()
            .take_settlement()
            .unwrap();
        assert_eq!(settlement.recipient, Some(exposure.key()));
        assert_eq!(settlement.delivery, FiniteDelivery::Delivered);
    }
    assert!(exposure.retire(&mut owner, &mut WorkBudget::new()).is_err());
    exposure.retire(&mut owner, &mut budget()).unwrap();
    assert_eq!(live(&accounts), retained); // actual layouts still belong to exposure
    drop(exposure);
    assert_empty(&accounts);
    for address in addresses {
        TcpListener::bind(address).unwrap();
    }
}

#[test]
fn shared_udp_replacement_keeps_original_bank_until_action_acknowledgement() {
    let mut owner = owner();
    let address = udp_addr();
    let native = ActionNative::new(address);
    let old_bank = bank();
    let next_bank = bank();
    let old = Rc::new(Counts::default());
    let sibling = Rc::new(Counts::default());
    let successor = Rc::new(Counts::default());
    let (input, route) = action_input(
        &owner,
        key(1, Generation::INITIAL),
        &native,
        Rc::clone(&old),
        None,
    );
    let mut first = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&old_bank),
        &mut budget(),
    )
    .ok()
    .unwrap();
    let k1 = first.key();
    let (input, _) = action_input(
        &owner,
        key(2, Generation::INITIAL),
        &native,
        Rc::clone(&sibling),
        None,
    );
    let mut second = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&next_bank),
        &mut budget(),
    )
    .ok()
    .unwrap();
    let k2 = second.key();
    ActionNative::start(&native).unwrap();
    first.publish(&mut owner, &mut budget()).unwrap();
    second.publish(&mut owner, &mut budget()).unwrap();
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    for key in [k1, k2] {
        assert_eq!(
            udp_call(&peer, &native, &mut owner, route, key, "invoke"),
            "accepted"
        );
    }
    let h1 = native.borrow().handles[0].1.clone();
    let h2 = native.borrow().handles[1].1.clone();
    let (input, _) = action_input(
        &owner,
        key(1, Generation::INITIAL.checked_next().unwrap()),
        &native,
        Rc::clone(&successor),
        Some(k1),
    );
    let mut third = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&next_bank),
        &mut budget(),
    )
    .ok()
    .unwrap();
    third.publish(&mut owner, &mut budget()).unwrap();
    drop(first);
    assert!(live(&old_bank) > 0);
    assert_eq!(scratch(&old_bank), 0);
    assert!(owner.retire_revoked_publication(k2, &mut budget()).is_err());
    assert_eq!(
        owner.step_raw_action(&h1, &mut WorkBudget::new()).unwrap(),
        Poll::Pending
    );
    assert_eq!(old.cancels.get(), 0);
    assert_eq!(
        owner.step_raw_action(&h1, &mut budget()).unwrap(),
        Poll::Pending
    );
    assert_eq!(old.cancels.get(), 1);
    assert_eq!(sibling.cancels.get(), 0);
    assert_eq!(
        udp_call(&peer, &native, &mut owner, route, k2, "query"),
        "running"
    );
    old.complete.set(true);
    assert_eq!(
        owner.step_raw_action(&h1, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    assert!(live(&old_bank) > 0); // unread late result still owns original backing
    let result = owner.take_raw_action_settlement(&h1).unwrap().unwrap();
    assert_eq!(result.instance, k1);
    assert!(result.cancel_requested);
    assert_eq!(result.effect, ActionEffect::PossiblyApplied);
    assert_eq!(
        result.result.unwrap().data().unwrap().body.as_ref(),
        b"\"done\""
    );
    assert_empty(&old_bank);
    second.begin_close();
    assert!(second.retire(&mut owner, &mut budget()).is_err()); // native close is not Action retirement
    sibling.complete.set(true);
    assert_eq!(
        owner.step_raw_action(&h2, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    assert!(second.retire(&mut owner, &mut budget()).is_err()); // even terminal work needs acknowledgement
    assert_eq!(
        owner
            .take_raw_action_settlement(&h2)
            .unwrap()
            .unwrap()
            .instance,
        k2
    );
    second.retire(&mut owner, &mut budget()).unwrap();
    third.begin_close();
    third.retire(&mut owner, &mut budget()).unwrap();
    drop(second);
    drop(third);
    assert_empty(&next_bank);
    assert!(UdpSocket::bind(address).is_err()); // native owner deliberately retains its idle socket
    drop(native);
    UdpSocket::bind(address).unwrap();
}

#[test]
fn lost_exposure_and_root_release_only_after_actual_member_disposal() {
    for root_first in [false, true] {
        let mut owner = owner();
        let accounts = bank();
        let addresses = [tcp_addr(), tcp_addr()];
        let input = http_input(
            &owner,
            key(1, Generation::INITIAL),
            addresses,
            Rc::new(Counts::default()),
        );
        let mut exposure = ComposedProducerExposure::try_new(
            &owner,
            input,
            limits(),
            Arc::clone(&accounts),
            &mut budget(),
        )
        .ok()
        .unwrap();
        let k = exposure.key();
        start_http(&mut exposure);
        exposure.publish(&mut owner, &mut budget()).unwrap();
        assert!(owner.retire_revoked_publication(k, &mut budget()).is_err());
        if root_first {
            drop(owner);
            assert!(live(&accounts) > 0);
            close_http(&mut exposure); // explicit native cleanup works without the logical root
            drop(exposure);
        } else {
            drop(exposure); // member Drop aborts local native work; registry still owns backing
            assert!(live(&accounts) > 0);
            for address in addresses {
                TcpListener::bind(address).unwrap();
            }
            assert!(
                owner
                    .retire_revoked_publication(k, &mut WorkBudget::new())
                    .is_err()
            );
            assert!(live(&accounts) > 0);
            owner.retire_revoked_publication(k, &mut budget()).unwrap();
        }
        assert_empty(&accounts);
        for address in addresses {
            TcpListener::bind(address).unwrap();
        }
    }
}

struct CloseControl {
    pending: Cell<bool>,
    failures: Cell<usize>,
    polls: Cell<usize>,
}
struct ControlledClose {
    inner: HttpFiniteBinding,
    control: Rc<CloseControl>,
}
impl FiniteIngressBinding for ControlledClose {
    type Reply = HttpFiniteReply;
    type Configuration = HttpFiniteConfig;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        self.inner.identity()
    }
    fn configuration(&self) -> &Self::Configuration {
        self.inner.configuration()
    }
    fn engine_backing_bytes(&self) -> Option<usize> {
        self.inner.engine_backing_bytes()
    }
    fn prepared_for_instance(&self, key: ProducerInstanceKey) -> bool {
        self.inner.prepared_for_instance(key)
    }
    fn covers_definition(
        &self,
        key: ProducerInstanceKey,
        site: clinkz_wot_core::thing::AffordanceSite<'_>,
        form: u32,
        op: Operation,
        definition: &dyn Any,
    ) -> bool {
        self.inner
            .covers_definition(key, site, form, op, definition)
    }
    fn validator(&self) -> Option<&dyn FiniteValidator> {
        self.inner.validator()
    }
    fn request_deadline(&self) -> Deadline {
        self.inner.request_deadline()
    }
    fn poll_start(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
        self.inner.poll_start(cx)
    }
    fn poll_receive(
        &mut self,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> Poll<CoreResult<FiniteIngress<Self::Reply>>> {
        self.inner.poll_receive(cx, now)
    }
    fn poll_send(
        &mut self,
        cx: &mut Context<'_>,
        reply: &RawFiniteReply<Self::Reply>,
    ) -> Poll<CoreResult<()>> {
        self.inner.poll_send(cx, reply)
    }
    fn poll_close(
        &mut self,
        cx: &mut Context<'_>,
        ingress: Option<&FiniteIngress<Self::Reply>>,
        reply: Option<&RawFiniteReply<Self::Reply>>,
        cause: Option<&CoreError>,
    ) -> Poll<CoreResult<FiniteDelivery>> {
        self.control.polls.set(self.control.polls.get() + 1);
        if self.control.failures.get() > 0 {
            self.control.failures.set(self.control.failures.get() - 1);
            return Poll::Ready(Err(CoreError::Binding(ErrorContext::new(
                ErrorPhase::Cleanup,
                RetryClass::Never,
            ))));
        }
        if self.control.pending.get() {
            return Poll::Pending;
        }
        self.inner.poll_close(cx, ingress, reply, cause)
    }
}

#[test]
fn partial_native_setup_and_failed_cleanup_keep_original_capacity_and_sockets() {
    type ControlledRuntime = RawFiniteRuntime<ControlledClose>;
    let mut owner = owner();
    let accounts = bank();
    let addresses = [tcp_addr(), tcp_addr()];
    let occupied = TcpListener::bind(addresses[1]).unwrap();
    let counts = Rc::new(Counts::default());
    let mut input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        Rc::clone(&counts),
    );
    let control = Rc::new(CloseControl {
        pending: Cell::new(true),
        failures: Cell::new(1),
        polls: Cell::new(0),
    });
    let program = input.prepared.operations()[0]
        .definition::<HttpProducerProgram>()
        .unwrap()
        .clone();
    let config = HttpFiniteConfig::try_new(
        HttpProducerConfig::for_loopback(addresses[0]).unwrap(),
        program,
        input.instance.key(),
        Arc::from("left"),
        100,
    )
    .unwrap();
    input.participants[0] = Box::new(
        RawFiniteRuntime::try_new(
            ControlledClose {
                inner: HttpFiniteBinding::new(
                    config.clone(),
                    BindingId::new(101),
                    BindingGeneration::INITIAL,
                ),
                control: Rc::clone(&control),
            },
            config,
            &owner,
            RawFiniteLimits {
                candidates: 1,
                input_body_bytes: 0,
                output_body_bytes: 4096,
                output_media_capacity: 64,
                output_retained_bytes: 8192,
            },
            FiniteValidation::Required,
            Deadline::NONE,
        )
        .ok()
        .unwrap(),
    );
    let mut exposure = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&accounts),
        &mut budget(),
    )
    .ok()
    .unwrap();
    assert_eq!(
        exposure
            .participant_mut::<ControlledRuntime>(0)
            .unwrap()
            .step(&mut cx(), &mut budget(), now()),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    assert_eq!(
        exposure
            .participant_mut::<HttpRuntime>(1)
            .unwrap()
            .step(&mut cx(), &mut budget(), now()),
        Poll::Ready(RawFiniteState::Closing)
    );
    assert!(exposure.publish(&mut owner, &mut budget()).is_err());
    assert!(!exposure.is_committed());
    assert_eq!(counts.property.get(), 0);
    exposure.begin_close();
    assert_eq!(scratch(&accounts), 0);
    let retained = live(&accounts);
    let first = exposure.participant_mut::<ControlledRuntime>(0).unwrap();
    assert_eq!(
        first.step(&mut cx(), &mut WorkBudget::new(), now()),
        Poll::Pending
    );
    assert_eq!(control.polls.get(), 0);
    assert_eq!(
        first.step(&mut cx(), &mut budget(), now()),
        Poll::Ready(RawFiniteState::Closing)
    ); // actual close failure
    assert!(matches!(first.first_cause(), Some(CoreError::Binding(_))));
    assert_eq!(first.step(&mut cx(), &mut budget(), now()), Poll::Pending); // retained Pending cleanup
    assert_eq!(control.polls.get(), 2);
    assert!(TcpListener::bind(addresses[0]).is_err());
    assert_eq!(live(&accounts), retained);
    assert!(exposure.retire(&mut owner, &mut budget()).is_err());
    control.pending.set(false);
    for i in 0..2 {
        if i == 0 {
            let runtime = exposure.participant_mut::<ControlledRuntime>(i).unwrap();
            assert_eq!(
                runtime.step(&mut cx(), &mut budget(), now()),
                Poll::Ready(RawFiniteState::Closed)
            );
            assert!(matches!(runtime.first_cause(), Some(CoreError::Binding(_))));
        } else {
            let runtime = exposure.participant_mut::<HttpRuntime>(i).unwrap();
            while runtime.state() != RawFiniteState::Closed {
                let _ = runtime.step(&mut cx(), &mut budget(), now());
            }
            assert!(matches!(runtime.first_cause(), Some(CoreError::Binding(_))));
        }
    }
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    assert_empty(&accounts);
    drop(occupied);
    for address in addresses {
        TcpListener::bind(address).unwrap();
    }
}

#[test]
fn moved_native_owner_keeps_funding_and_a_substitute_cannot_publish_or_retire() {
    let mut owner = owner();
    let accounts = bank();
    let addresses = [tcp_addr(), tcp_addr()];
    let input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        Rc::new(Counts::default()),
    );
    let program = input.prepared.operations()[0]
        .definition::<HttpProducerProgram>()
        .unwrap()
        .clone();
    let replacement = http_read(
        &owner,
        addresses[0],
        program,
        input.instance.key(),
        "left",
        BindingId::new(101),
        FiniteValidation::Required,
    );
    let mut exposure = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&accounts),
        &mut budget(),
    )
    .ok()
    .unwrap();
    let original = std::mem::replace(
        exposure.participant_mut::<HttpRuntime>(0).unwrap(),
        replacement,
    );
    start_http(&mut exposure);
    assert!(exposure.publish(&mut owner, &mut budget()).is_err());
    assert!(exposure.publication_failure().is_some());
    assert!(!exposure.is_committed());
    close_http(&mut exposure);
    assert!(exposure.retire(&mut owner, &mut budget()).is_err());
    drop(exposure);
    assert!(live(&accounts) > 0); // the extracted actual owner retained the same bank
    drop(original);
    assert_empty(&accounts);
    for address in addresses {
        TcpListener::bind(address).unwrap();
    }
}

#[test]
fn raw_or_foreign_native_pairs_are_returned_before_managed_setup() {
    let owner = owner();
    let foreign = super::owner();
    let accounts = bank();
    for raw in [false, true] {
        let addresses = [tcp_addr(), tcp_addr()];
        let mut input = http_input(
            &owner,
            key(1, Generation::INITIAL),
            addresses,
            Rc::new(Counts::default()),
        );
        let program = input.prepared.operations()[0]
            .definition::<HttpProducerProgram>()
            .unwrap()
            .clone();
        input.participants[0] = Box::new(http_read(
            if raw { &owner } else { &foreign },
            addresses[0],
            program,
            input.instance.key(),
            "left",
            BindingId::new(101),
            if raw {
                FiniteValidation::Raw
            } else {
                FiniteValidation::Required
            },
        ));
        let rejection = ComposedProducerExposure::try_new(
            &owner,
            input,
            limits(),
            Arc::clone(&accounts),
            &mut budget(),
        )
        .err()
        .unwrap();
        let mut input = rejection.into_input();
        assert_eq!(
            input.participants[0]
                .as_mut()
                .as_any_mut()
                .downcast_mut::<HttpRuntime>()
                .unwrap()
                .state(),
            RawFiniteState::Starting
        );
        assert_empty(&accounts);
        for address in addresses {
            TcpListener::bind(address).unwrap();
        }
    }
}

#[test]
fn review_retiring_an_unpublished_exposure_releases_qualification_scratch() {
    let mut owner = owner();
    let accounts = bank();
    let addresses = [tcp_addr(), tcp_addr()];
    let input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        Rc::new(Counts::default()),
    );
    let mut exposure = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&accounts),
        &mut budget(),
    )
    .ok()
    .unwrap();
    let phase_bytes = scratch(&accounts);
    let total_before_retirement = live(&accounts);
    let remaining = accounts
        .account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
        .limit()
        - phase_bytes;
    let other_setup = accounts
        .reserve(ResourceKind::AdmissionTemporaryBytesGlobalMax, remaining)
        .unwrap();
    start_http(&mut exposure);
    // Native progress is explicitly caller-driven; close each original owner.
    for i in 0..2 {
        let runtime = exposure.participant_mut::<HttpRuntime>(i).unwrap();
        runtime.begin_close();
        while runtime.state() != RawFiniteState::Closed {
            let _ = runtime.step(&mut cx(), &mut budget(), now());
        }
    }
    assert!(exposure.retire(&mut owner, &mut WorkBudget::new()).is_err());
    assert_eq!(scratch(&accounts), phase_bytes + remaining);
    exposure.retire(&mut owner, &mut budget()).unwrap();
    assert!(!exposure.is_committed());
    assert!(exposure.publish(&mut owner, &mut budget()).is_err());
    assert_eq!(
        scratch(&accounts),
        remaining,
        "successfully retired exposure cannot use final qualification scratch"
    );
    assert_eq!(live(&accounts), total_before_retirement - phase_bytes);
    assert!(live(&accounts) > 0); // lifetime/layout/cleanup owners are still alive
    let input = http_input(
        &owner,
        key(2, Generation::INITIAL),
        [tcp_addr(), tcp_addr()],
        Rc::new(Counts::default()),
    );
    let unrelated = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&accounts),
        &mut budget(),
    )
    .ok()
    .unwrap();
    drop(unrelated); // the retired exposure no longer blocks unrelated setup
    drop(other_setup);
    assert_eq!(scratch(&accounts), 0);
    drop(exposure);
    assert_empty(&accounts);
}

#[test]
fn review_lost_action_exposure_waits_for_original_action_acknowledgement() {
    let mut owner = owner();
    let accounts = bank();
    let native = ActionNative::new(udp_addr());
    let counts = Rc::new(Counts::default());
    let (input, route) = action_input(
        &owner,
        key(1, Generation::INITIAL),
        &native,
        Rc::clone(&counts),
        None,
    );
    let mut exposure = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&accounts),
        &mut budget(),
    )
    .ok()
    .unwrap();
    let k = exposure.key();
    ActionNative::start(&native).unwrap();
    exposure.publish(&mut owner, &mut budget()).unwrap();
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    assert_eq!(
        udp_call(&peer, &native, &mut owner, route, k, "invoke"),
        "accepted"
    );
    let handle = native.borrow().handles[0].1.clone();
    drop(exposure);
    assert!(owner.retire_revoked_publication(k, &mut budget()).is_err());
    counts.complete.set(true);
    assert_eq!(
        owner.step_raw_action(&handle, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    assert!(owner.retire_revoked_publication(k, &mut budget()).is_err());
    assert!(live(&accounts) > 0);
    assert_eq!(
        owner
            .take_raw_action_settlement(&handle)
            .unwrap()
            .unwrap()
            .instance,
        k
    );
    owner.retire_revoked_publication(k, &mut budget()).unwrap();
    assert_empty(&accounts);
}

struct ReviewDefinitionMemory {
    bank: Arc<HostResourceAccounts>,
    recording: Cell<bool>,
    retained: Cell<usize>,
    peak: Cell<usize>,
    unfunded_peak: Cell<usize>,
    charge_at_peak: Cell<u64>,
    definition_allocations: Cell<usize>,
    checks: Cell<usize>,
    check_temporary: Cell<usize>,
}
impl ReviewDefinitionMemory {
    fn observe(&self) {
        if !self.recording.get() {
            return;
        }
        let retained = self.retained.get();
        let charge = live(&self.bank);
        if retained >= self.peak.get() {
            self.peak.set(retained);
            self.charge_at_peak.set(charge);
        }
        if retained as u64 > charge {
            self.unfunded_peak
                .set(self.unfunded_peak.get().max(retained));
        }
    }
}

struct ReviewLargeDefinition {
    program: HttpProducerProgram,
    bytes: Vec<u8>,
    memory: Rc<ReviewDefinitionMemory>,
}
impl Drop for ReviewLargeDefinition {
    fn drop(&mut self) {
        self.memory
            .retained
            .set(self.memory.retained.get() - self.bytes.capacity());
    }
}
struct ReviewLargeProvider {
    inner: Rc<dyn ProducerCapability>,
    memory: Rc<ReviewDefinitionMemory>,
}
struct ReviewCheckTemporary {
    bytes: Vec<u8>,
    memory: Rc<ReviewDefinitionMemory>,
}
impl Drop for ReviewCheckTemporary {
    fn drop(&mut self) {
        self.memory
            .retained
            .set(self.memory.retained.get() - self.bytes.capacity());
    }
}
impl ProducerCapability for ReviewLargeProvider {
    fn supports(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<bool> {
        self.inner.supports(input, budget)
    }
    fn final_check_temporary_bytes(&self) -> Option<usize> {
        self.inner
            .final_check_temporary_bytes()?
            .checked_add(self.memory.check_temporary.get())
    }
    fn identity(&self) -> (BindingId, BindingGeneration) {
        self.inner.identity()
    }
    fn definition_bytes(&self, definition: &dyn Any) -> Option<usize> {
        let definition = definition.downcast_ref::<ReviewLargeDefinition>()?;
        Some(std::mem::size_of::<ReviewLargeDefinition>() + definition.bytes.capacity())
    }
    fn prepare(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<Box<dyn Any>>> {
        let Some(definition) = self.inner.prepare(input, budget)? else {
            return Ok(None);
        };
        let program = *definition.downcast::<HttpProducerProgram>().unwrap();
        budget
            .consume(WorkClass::HandlerSteps, 12 * 1024 * 1024)
            .map_err(|_| {
                CoreError::Backpressure(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
            })?;
        let bytes = vec![0u8; 12 * 1024 * 1024];
        let retained = self.memory.retained.get() + bytes.capacity();
        self.memory.retained.set(retained);
        self.memory
            .definition_allocations
            .set(self.memory.definition_allocations.get() + 1);
        self.memory.observe();
        Ok(Some(Box::new(ReviewLargeDefinition {
            program,
            bytes,
            memory: Rc::clone(&self.memory),
        })))
    }
    fn check_final(
        &self,
        input: ProducerOperationInput<'_>,
        definition: &dyn Any,
        budget: &mut WorkBudget,
    ) -> CoreResult<bool> {
        self.memory.checks.set(self.memory.checks.get() + 1);
        budget
            .consume(
                WorkClass::HandlerSteps,
                self.memory.check_temporary.get() as u64,
            )
            .map_err(|_| {
                CoreError::Backpressure(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
            })?;
        let bytes = vec![0u8; self.memory.check_temporary.get()];
        self.memory
            .retained
            .set(self.memory.retained.get() + bytes.capacity());
        let _temporary = ReviewCheckTemporary {
            bytes,
            memory: Rc::clone(&self.memory),
        };
        self.memory.observe();
        let Some(definition) = definition.downcast_ref::<ReviewLargeDefinition>() else {
            return Ok(false);
        };
        self.inner.check_final(input, &definition.program, budget)
    }
}
struct ReviewLargeParticipant(HttpRuntime);
impl ClosedProducerParticipation for ReviewLargeParticipant {
    fn participation_id(&self) -> &ProducerParticipationId {
        self.0.participation_id()
    }
    fn retain_owner(&mut self, owner: Rc<dyn Any>) {
        self.0.retain_owner(owner)
    }
    fn admission_backing_bytes(
        &self,
        owner: &RawProducerServient,
        key: ProducerInstanceKey,
    ) -> Option<usize> {
        self.0.admission_backing_bytes(owner, key)
    }
    fn begin_retire(&mut self) {
        self.0.begin_retire()
    }
    fn ready_for(&self, owner: &RawProducerServient, key: ProducerInstanceKey) -> bool {
        self.0.ready_for(owner, key)
    }
    fn covers(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: clinkz_wot_core::thing::AffordanceSite<'_>,
        form: u32,
        operation: Operation,
        definition: &dyn Any,
    ) -> bool {
        definition
            .downcast_ref::<ReviewLargeDefinition>()
            .is_some_and(|definition| {
                self.0
                    .covers(key, binding, site, form, operation, &definition.program)
            })
    }
    fn ingress_gate(&self) -> &Rc<Cell<bool>> {
        self.0.ingress_gate()
    }
    fn retired(&self) -> bool {
        self.0.retired()
    }
}

fn review_large_input(
    owner: &RawProducerServient,
    policy: &ResourceLimits,
    accounts: &Arc<HostResourceAccounts>,
) -> (
    ComposedProducerInput,
    Rc<ReviewDefinitionMemory>,
    [SocketAddr; 2],
) {
    let memory = Rc::new(ReviewDefinitionMemory {
        bank: Arc::clone(&accounts),
        recording: Cell::new(false),
        retained: Cell::new(0),
        peak: Cell::new(0),
        unfunded_peak: Cell::new(0),
        charge_at_peak: Cell::new(0),
        definition_allocations: Cell::new(0),
        checks: Cell::new(0),
        check_temporary: Cell::new(0),
    });
    let addresses = [tcp_addr(), tcp_addr()];
    let mut input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        Rc::new(Counts::default()),
    );
    input.providers = std::mem::take(&mut input.providers)
        .into_iter()
        .map(|inner| {
            Rc::new(ReviewLargeProvider {
                inner,
                memory: Rc::clone(&memory),
            }) as Rc<dyn ProducerCapability>
        })
        .collect();
    let providers: Vec<_> = input
        .providers
        .iter()
        .map(|provider| provider.as_ref())
        .collect();
    input.prepared = prepare_thing(
        input.instance.description().unwrap(),
        input.instance.key(),
        &policy,
        &mut budget().with_remaining(WorkClass::HandlerSteps, 100_000_000),
        &providers,
        |scope, op| match scope {
            ValidatedProducerScope::Property(name) => input.instance.supports_property(name, op),
            _ => false,
        },
    )
    .unwrap();
    input.participants = input
        .prepared
        .operations()
        .iter()
        .zip(addresses)
        .map(|(operation, address)| {
            let FormSite::Property(name) = &operation.site else {
                panic!("property")
            };
            Box::new(ReviewLargeParticipant(http_read(
                &owner,
                address,
                operation
                    .definition::<ReviewLargeDefinition>()
                    .unwrap()
                    .program
                    .clone(),
                input.instance.key(),
                name,
                operation.binding.0,
                FiniteValidation::Required,
            ))) as Box<dyn OwnedProducerParticipation>
        })
        .collect();
    memory.recording.set(true);
    (input, memory, addresses)
}

fn review_large_policy(bytes: u64) -> ResourceLimits {
    let mut policy = GatewayDefaultV1::LIMITS.clone();
    assert!(policy.set(ResourceKind::EngineLiveBytesGlobalMax, Some(bytes)));
    assert!(policy.set(ResourceKind::CleanupBytesMax, Some(bytes)));
    policy
}

fn review_start_large(exposure: &mut ComposedProducerExposure) {
    for i in 0..2 {
        assert_eq!(
            exposure
                .participant_mut::<ReviewLargeParticipant>(i)
                .unwrap()
                .0
                .step(&mut cx(), &mut budget(), now()),
            Poll::Ready(RawFiniteState::InstalledClosed)
        );
    }
}
fn review_close_large(exposure: &mut ComposedProducerExposure, owner: &mut RawProducerServient) {
    exposure.begin_close();
    for i in 0..2 {
        let runtime = &mut exposure
            .participant_mut::<ReviewLargeParticipant>(i)
            .unwrap()
            .0;
        while runtime.state() != RawFiniteState::Closed {
            let _ = runtime.step(&mut cx(), &mut budget(), now());
        }
    }
    exposure.retire(owner, &mut budget()).unwrap();
}

#[test]
fn review_requalification_cannot_regenerate_unreserved_definition_backing() {
    let mut owner = owner();
    let policy = review_large_policy(44 * 1024 * 1024);
    let accounts = Arc::new(HostResourceAccounts::new(&policy));
    let (input, memory, addresses) = review_large_input(&owner, &policy, &accounts);
    let result = ComposedProducerExposure::try_new(
        &owner,
        input,
        Arc::new(policy),
        Arc::clone(&accounts),
        &mut budget().with_remaining(WorkClass::HandlerSteps, 100_000_000),
    );
    let mut exposure = result
        .ok()
        .expect("within-limit original definitions should be admissible");
    assert_eq!(memory.definition_allocations.get(), 2);
    assert_eq!(memory.checks.get(), 2);
    review_start_large(&mut exposure);
    let phase_bytes = scratch(&accounts);
    assert!(matches!(
        exposure.publish(
            &mut owner,
            &mut budget().with_remaining(WorkClass::BindingPolls, 0)
        ),
        Err(CoreError::Backpressure(_))
    ));
    assert_eq!(scratch(&accounts), phase_bytes);
    let before_publish_checks = memory.checks.get();
    exposure.publish(&mut owner, &mut budget()).unwrap();
    assert_eq!(memory.checks.get(), before_publish_checks + 2);
    assert_eq!(memory.definition_allocations.get(), 2);
    assert_eq!(scratch(&accounts), 0);
    for (i, address) in addresses.into_iter().enumerate() {
        let runtime = &mut exposure
            .participant_mut::<ReviewLargeParticipant>(i)
            .unwrap()
            .0;
        assert!(read_peer(address, &mut owner, runtime).starts_with(b"HTTP/1.1 200"));
        assert_eq!(
            runtime.take_settlement().unwrap().delivery,
            FiniteDelivery::Delivered
        );
    }
    eprintln!(
        "definition_bytes_peak={} account_used_at_peak={} account_limit={}",
        memory.peak.get(),
        memory.charge_at_peak.get(),
        accounts
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .limit()
    );
    assert_eq!(
        memory.unfunded_peak.get(),
        0,
        "managed qualification allocated overlapping definitions beyond its actual shared bank reservation"
    );
    assert_eq!(memory.peak.get(), 24 * 1024 * 1024);
    review_close_large(&mut exposure, &mut owner);
    drop(exposure);
    assert_eq!(memory.retained.get(), 0);
    assert_empty(&accounts);
}

#[test]
fn review_callback_temporaries_are_reserved_before_any_check_or_native_setup() {
    for (capacity, temporary) in [
        (44 * 1024 * 1024, 8 * 1024 * 1024),
        (64 * 1024 * 1024, 8 * 1024 * 1024),
        (44 * 1024 * 1024, usize::MAX),
    ] {
        let mut owner = owner();
        let policy = review_large_policy(capacity);
        let accounts = Arc::new(HostResourceAccounts::new(&policy));
        let (input, memory, addresses) = review_large_input(&owner, &policy, &accounts);
        let original = input.prepared.operations()[0]
            .definition::<ReviewLargeDefinition>()
            .unwrap() as *const _;
        memory.check_temporary.set(temporary);
        let result = ComposedProducerExposure::try_new(
            &owner,
            input,
            Arc::new(policy),
            Arc::clone(&accounts),
            &mut budget().with_remaining(WorkClass::HandlerSteps, 100_000_000),
        );
        if capacity == 44 * 1024 * 1024 {
            let input = result
                .err()
                .expect("temporary allowance cannot fit")
                .into_input();
            assert_eq!(memory.checks.get(), 0);
            assert_eq!(memory.definition_allocations.get(), 2);
            assert_eq!(memory.retained.get(), 24 * 1024 * 1024);
            assert_eq!(
                input.prepared.operations()[0]
                    .definition::<ReviewLargeDefinition>()
                    .unwrap() as *const _,
                original
            );
            assert_empty(&accounts);
            for address in addresses {
                TcpListener::bind(address).unwrap();
            }
            drop(input);
        } else {
            let mut exposure = result.ok().expect("declared temporary allowance fits");
            assert_eq!(memory.peak.get(), 32 * 1024 * 1024);
            assert_eq!(memory.unfunded_peak.get(), 0);
            let checks = memory.checks.get();
            review_start_large(&mut exposure);
            exposure
                .publish(
                    &mut owner,
                    &mut budget().with_remaining(WorkClass::HandlerSteps, 100_000_000),
                )
                .unwrap();
            assert_eq!(memory.checks.get(), checks + 2);
            assert_eq!(memory.definition_allocations.get(), 2);
            assert_eq!(memory.unfunded_peak.get(), 0);
            assert_eq!(scratch(&accounts), 0);
            review_close_large(&mut exposure, &mut owner);
            drop(exposure);
            assert_empty(&accounts);
        }
        assert_eq!(memory.retained.get(), 0);
    }
}

#[test]
fn review_changed_callback_allowance_fences_before_requalification() {
    let mut owner = owner();
    let policy = review_large_policy(64 * 1024 * 1024);
    let accounts = Arc::new(HostResourceAccounts::new(&policy));
    let (input, memory, _) = review_large_input(&owner, &policy, &accounts);
    let mut exposure = ComposedProducerExposure::try_new(
        &owner,
        input,
        Arc::new(policy),
        Arc::clone(&accounts),
        &mut budget().with_remaining(WorkClass::HandlerSteps, 100_000_000),
    )
    .ok()
    .unwrap();
    review_start_large(&mut exposure);
    let checks = memory.checks.get();
    memory.check_temporary.set(8 * 1024 * 1024);
    assert!(exposure.publish(&mut owner, &mut budget()).is_err());
    assert!(!exposure.is_committed());
    assert!(exposure.publication_failure().is_some());
    assert_eq!(memory.checks.get(), checks);
    assert_eq!(memory.definition_allocations.get(), 2);
    assert_eq!(scratch(&accounts), 0);
    assert!(live(&accounts) > 0);
    review_close_large(&mut exposure, &mut owner);
    drop(exposure);
    assert_eq!(memory.retained.get(), 0);
    assert_empty(&accounts);
}
