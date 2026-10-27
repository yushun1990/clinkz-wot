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
                FiniteOutputValidation::Required,
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
    fn output_validator(&self) -> Option<&dyn ProducerOutputValidator> {
        self.inner.output_validator()
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
            FiniteOutputValidation::Required,
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
        FiniteOutputValidation::Required,
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
                FiniteOutputValidation::Raw
            } else {
                FiniteOutputValidation::Required
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
