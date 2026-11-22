use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Overlap {
    Neither,
    Left,
    Right,
}

struct Competitor {
    overlap: RefCell<PublicationState<Overlap>>,
    switch_after_left: Cell<bool>,
}
impl Competitor {
    fn new(overlap: Overlap) -> Rc<Self> {
        Rc::new(Self {
            overlap: RefCell::new(PublicationState::new(overlap)),
            switch_after_left: Cell::new(false),
        })
    }
    fn install(self: &Rc<Self>, registrations: &ProducerRegistrations) {
        let mut providers = registrations
            .snapshot(&mut PublicationFacts::default())
            .unwrap()
            .to_vec();
        providers.push(self.clone());
        registrations.replace(providers);
    }
}
impl ProducerCapability for Competitor {
    fn observe_configuration(&self, facts: &mut PublicationObserver<'_>) -> CoreResult<()> {
        self.overlap.borrow().observe(facts)
    }

    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(121), BindingGeneration::INITIAL)
    }
    fn definition_bytes(&self, _: &dyn Any) -> Option<usize> {
        Some(0)
    }
    fn final_check_temporary_bytes(&self) -> Option<usize> {
        Some(0)
    }
    fn prepare(
        &self,
        i: ProducerOperationInput<'_>,
        b: &mut WorkBudget,
    ) -> CoreResult<Option<Box<dyn Any>>> {
        Ok(self.supports(i, b)?.then(|| Box::new(()) as Box<dyn Any>))
    }
    fn supports(&self, i: ProducerOperationInput<'_>, _: &mut WorkBudget) -> CoreResult<bool> {
        let left = matches!(i.site, FormSite::Property(name) if name == "left");
        let supported = matches!(
            (left, *self.overlap.borrow().get()),
            (true, Overlap::Left) | (false, Overlap::Right)
        );
        if left && self.switch_after_left.replace(false) {
            self.overlap.borrow_mut().replace(Overlap::Left);
        }
        Ok(supported)
    }
    fn check_final(
        &self,
        i: ProducerOperationInput<'_>,
        _: &dyn Any,
        b: &mut WorkBudget,
    ) -> CoreResult<bool> {
        self.supports(i, b)
    }
}
#[test]
fn final_walk_refuses_both_fixed_overlaps_and_a_reload_between_forms() {
    // Use the caller-driven final walk as well as the source-free managed
    // installer. These are two real, separately bound and still closed sockets.
    let mut owner = owner();
    let addresses = [tcp_addr(), tcp_addr()];
    let counts = Rc::new(Counts::default());
    let mut input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        counts.clone(),
    );
    let competitor = Competitor::new(Overlap::Right);
    competitor.install(&input.providers);
    let registrations = input
        .providers
        .snapshot(&mut PublicationFacts::default())
        .unwrap();
    let providers: Vec<_> = registrations.iter().map(|p| p.as_ref()).collect();
    competitor.switch_after_left.set(true);
    let draft = prepare_thing(
        input.instance.description().unwrap(),
        input.instance.key(),
        GatewayDefaultV1::LIMITS,
        &mut budget(),
        &providers,
        |_, _| true,
    );
    assert!(matches!(draft, Err(CoreError::StaleHandle(_))));
    assert!(!competitor.switch_after_left.get());

    let mut publication = owner
        .prepare_publication(input.instance, None)
        .ok()
        .unwrap();
    let gates: Vec<_> = input
        .participants
        .iter()
        .map(|p| p.ingress_gate().clone())
        .collect();
    for p in &mut input.participants {
        let runtime = p.as_any_mut().downcast_mut::<HttpRuntime>().unwrap();
        assert_eq!(
            runtime.step(&mut cx(), &mut budget(), now()),
            Poll::Ready(RawFiniteState::InstalledClosed)
        );
    }
    let mut members: Vec<_> = input
        .participants
        .iter_mut()
        .map(|p| p.as_participation_mut())
        .collect();
    for (overlap, switch) in [
        (Overlap::Left, false),
        (Overlap::Right, false),
        (Overlap::Right, true),
    ] {
        competitor.overlap.borrow_mut().replace(overlap);
        competitor.switch_after_left.set(switch);
        let result = publish_prepared_thing(
            &mut owner,
            &mut publication,
            &input.prepared,
            &providers,
            &mut members,
            GatewayDefaultV1::LIMITS,
            &mut budget(),
        );
        if switch {
            assert!(matches!(result, Err(CoreError::StaleHandle(_))));
            assert!(
                !competitor.switch_after_left.get(),
                "reload actually ran during qualification"
            );
        } else {
            assert!(matches!(result, Err(CoreError::UnsupportedOperation(_))));
        }
        assert_eq!(owner.description(publication.key()), None);
        assert!(gates.iter().all(|gate| !gate.is_open()));
        assert_eq!(counts.property.get(), 0);
    }
    competitor.overlap.borrow_mut().replace(Overlap::Neither);
    publish_prepared_thing(
        &mut owner,
        &mut publication,
        &input.prepared,
        &providers,
        &mut members,
        GatewayDefaultV1::LIMITS,
        &mut budget(),
    )
    .unwrap();
    assert!(gates.iter().all(|gate| gate.is_open()));
    for (p, address) in input.participants.iter_mut().zip(addresses) {
        let runtime = p.as_any_mut().downcast_mut::<HttpRuntime>().unwrap();
        assert!(read_peer(address, &mut owner, runtime).starts_with(b"HTTP/1.1 200"));
        assert_eq!(
            runtime.take_settlement().unwrap().delivery,
            FiniteDelivery::Delivered
        );
    }
    assert_eq!(counts.property.get(), 2);
}

#[test]
fn replacement_of_the_registration_set_refuses_source_free_install_and_preserves_the_incumbent() {
    let mut owner = owner();
    let accounts = bank();
    let incumbent_addresses = [tcp_addr(), tcp_addr()];
    let old_counts = Rc::new(Counts::default());
    let input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        incumbent_addresses,
        old_counts.clone(),
    );
    let old_td = input.instance.description().unwrap().to_owned();
    let mut incumbent =
        ComposedProducerExposure::try_new(&owner, input, limits(), accounts.clone(), &mut budget())
            .ok()
            .unwrap();
    start_http(&mut incumbent);
    incumbent.publish(&mut owner, &mut budget()).unwrap();

    let addresses = [tcp_addr(), tcp_addr()];
    let counts = Rc::new(Counts::default());
    let mut input = http_input(
        &owner,
        key(1, Generation::new(2).unwrap()),
        addresses,
        counts.clone(),
    );
    input.expected = Some(incumbent.key());
    let registrations = input.providers.clone();
    let gates: Vec<_> = input
        .participants
        .iter()
        .map(|p| p.ingress_gate().clone())
        .collect();
    let mut candidate =
        ComposedProducerExposure::try_new(&owner, input, limits(), accounts.clone(), &mut budget())
            .ok()
            .unwrap();
    start_http(&mut candidate);
    Competitor::new(Overlap::Left).install(&registrations);
    let mut source_free = budget()
        .with_remaining(WorkClass::CodecInputBytes, 0)
        .with_remaining(WorkClass::PlanningItems, 0);
    assert!(matches!(
        candidate.publish(&mut owner, &mut source_free),
        Err(CoreError::StaleHandle(_))
    ));
    assert_eq!(owner.description(candidate.key()), None);
    assert!(gates.iter().all(|gate| !gate.is_open()));
    assert_eq!(counts.property.get(), 0);
    assert_eq!(owner.description(incumbent.key()), Some(old_td.as_str()));
    for (i, address) in incumbent_addresses.into_iter().enumerate() {
        let runtime = incumbent.participant_mut::<HttpRuntime>(i).unwrap();
        assert!(read_peer(address, &mut owner, runtime).starts_with(b"HTTP/1.1 200"));
        assert_eq!(
            runtime.take_settlement().unwrap().delivery,
            FiniteDelivery::Delivered
        );
    }
    assert_eq!(old_counts.property.get(), 2);
    close_http(&mut candidate);
    candidate.retire(&mut owner, &mut budget()).unwrap();
    for address in addresses {
        TcpListener::bind(address).unwrap();
    }
    close_http(&mut incumbent);
    incumbent.retire(&mut owner, &mut budget()).unwrap();
    drop((candidate, incumbent));
    assert_empty(&accounts);
}

type Hook = Rc<RefCell<Option<Box<dyn FnOnce()>>>>;
struct GateHook {
    native: HttpRuntime,
    hook: Hook,
    available: Rc<RefCell<PublicationState<bool>>>,
}
impl ClosedProducerParticipation for GateHook {
    fn qualified_operation_count(&self) -> Option<usize> {
        self.native.qualified_operation_count()
    }
    fn observe_installation(&self, facts: &mut PublicationObserver<'_>) -> CoreResult<()> {
        self.available.borrow().observe(facts)?;
        self.native.observe_installation(facts)
    }

    fn associated_with(&self, owner: &ProducerOwnerToken, key: ProducerInstanceKey) -> bool {
        self.native.associated_with(owner, key)
    }
    fn participation_id(&self) -> &ProducerParticipationId {
        self.native.participation_id()
    }
    fn retain_owner(&mut self, owner: Rc<dyn Any>) {
        self.native.retain_owner(owner)
    }
    fn admission_backing_bytes(
        &self,
        owner: &ProducerOwnerToken,
        key: ProducerInstanceKey,
    ) -> Option<usize> {
        self.native
            .admission_backing_bytes(owner, key)?
            .checked_add(128)
    }
    fn begin_retire(&mut self) {
        self.native.begin_retire()
    }
    fn ready_for(&self, owner: &ProducerOwnerToken, key: ProducerInstanceKey) -> bool {
        *self.available.borrow().get() && self.native.ready_for(owner, key)
    }
    fn covers(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: clinkz_wot_core::thing::AffordanceSite<'_>,
        form: u32,
        op: Operation,
        def: &dyn Any,
    ) -> bool {
        self.native.covers(key, binding, site, form, op, def)
    }
    fn ingress_gate(&self) -> &Rc<ProducerIngressGate> {
        if let Some(hook) = self.hook.borrow_mut().take() {
            hook();
        }
        self.native.ingress_gate()
    }
    fn retired(&self) -> bool {
        self.native.retired()
    }
}
#[test]
fn configuration_or_readiness_loss_in_the_last_gate_callback_prevents_commit() {
    for change_provider in [true, false] {
        let mut owner = owner();
        let accounts = bank();
        let addresses = [tcp_addr(), tcp_addr()];
        let counts = Rc::new(Counts::default());
        let mut input = http_input(
            &owner,
            key(1, Generation::INITIAL),
            addresses,
            counts.clone(),
        );
        let competitor = Competitor::new(Overlap::Neither);
        competitor.install(&input.providers);
        let op = &input.prepared.operations()[1];
        let hook: Hook = Rc::new(RefCell::new(None));
        let available = Rc::new(RefCell::new(PublicationState::new(true)));
        let native = http_read(
            &owner,
            addresses[1],
            op.definition::<HttpProducerProgram>().unwrap().clone(),
            input.instance.key(),
            "right",
            op.binding.0,
            FiniteValidation::Required,
        );
        input.participants[1] = OwnedProducerParticipation::new(
            GateHook {
                native,
                hook: hook.clone(),
                available: available.clone(),
            },
            |p, owner, cx, now, credit| {
                owner
                    .step(
                        &mut p.native,
                        cx,
                        &mut credit(ProducerWork::Participation),
                        now,
                    )
                    .map(|_| ())
            },
        );
        let gates: Vec<_> = input
            .participants
            .iter()
            .map(|p| p.ingress_gate().clone())
            .collect();
        let mut exposure = ComposedProducerExposure::try_new(
            &owner,
            input,
            limits(),
            accounts.clone(),
            &mut budget(),
        )
        .ok()
        .unwrap();
        exposure
            .step(&mut owner, &mut cx(), now(), |_, _| budget())
            .unwrap();
        *hook.borrow_mut() = Some(Box::new(move || {
            if change_provider {
                competitor.overlap.borrow_mut().replace(Overlap::Left);
            } else {
                available.borrow_mut().replace(false);
            }
        }));
        assert!(matches!(
            exposure.publish(&mut owner, &mut budget()),
            Err(CoreError::StaleHandle(_))
        ));
        assert!(hook.borrow().is_none());
        assert!(gates.iter().all(|gate| !gate.is_open()));
        assert_eq!(owner.description(exposure.key()), None);
        assert_eq!(counts.property.get(), 0);
        for _ in 0..4 {
            exposure
                .step(&mut owner, &mut cx(), now(), |_, _| budget())
                .unwrap();
        }
        exposure.retire(&mut owner, &mut budget()).unwrap();
        drop(exposure);
        assert_empty(&accounts);
        for address in addresses {
            TcpListener::bind(address).unwrap();
        }
    }
}

#[test]
fn an_empty_affordance_in_the_final_td_returns_the_staged_owners() {
    for family in ["properties", "actions", "events"] {
        let owner = owner();
        let accounts = bank();
        let addresses = [tcp_addr(), tcp_addr()];
        let mut input = http_input(
            &owner,
            key(1, Generation::INITIAL),
            addresses,
            Rc::new(Counts::default()),
        );
        let mut text: serde_json::Value =
            serde_json::from_str(input.instance.description().unwrap()).unwrap();
        text[family]["empty"] = serde_json::json!({"forms": []});
        input.instance = input.instance.with_description(Arc::from(text.to_string()));
        let key = input.instance.key();
        let first = input.participants[0].participation_id().downgrade();
        let refusal = ComposedProducerExposure::try_new(
            &owner,
            input,
            limits(),
            accounts.clone(),
            &mut budget(),
        )
        .err()
        .unwrap();
        assert!(matches!(refusal.error().error(), CoreError::Validation(_)));
        assert!(
            refusal.input().participants[0]
                .participation_id()
                .matches(&first)
        );
        assert_eq!(owner.description(key), None);
        for p in &refusal.input().participants {
            assert!(!p.ingress_gate().is_open());
        }
        for address in addresses {
            TcpListener::bind(address).unwrap();
        }
        assert_empty(&accounts);
    }
}

#[test]
fn closing_a_committed_member_fences_its_thing_and_preserves_an_unread_peer_result() {
    let mut owner = owner();
    let accounts = bank();
    let addresses = [tcp_addr(), tcp_addr()];
    let input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        Rc::new(Counts::default()),
    );
    let mut exposure =
        ComposedProducerExposure::try_new(&owner, input, limits(), accounts.clone(), &mut budget())
            .ok()
            .unwrap();
    start_http(&mut exposure);
    exposure.publish(&mut owner, &mut budget()).unwrap();
    let right = exposure.participant_mut::<HttpRuntime>(1).unwrap();
    assert!(read_peer(addresses[1], &mut owner, right).starts_with(b"HTTP/1.1 200"));
    let key = exposure.key();
    exposure
        .participant_mut::<HttpRuntime>(0)
        .unwrap()
        .begin_close();
    exposure
        .step(&mut owner, &mut cx(), now(), |_, _| budget())
        .unwrap();
    assert!(owner.description(key).is_none());
    for _ in 0..4 {
        exposure
            .step(&mut owner, &mut cx(), now(), |_, _| budget())
            .unwrap();
    }
    for i in 0..2 {
        assert_eq!(
            exposure.participant_mut::<HttpRuntime>(i).unwrap().state(),
            RawFiniteState::Closed
        );
    }
    assert!(exposure.retire(&mut owner, &mut budget()).is_err());
    let settlement = exposure
        .participant_mut::<HttpRuntime>(1)
        .unwrap()
        .take_settlement()
        .unwrap();
    assert_eq!(settlement.recipient, Some(key));
    assert_eq!(settlement.delivery, FiniteDelivery::Delivered);
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop((exposure, settlement));
    assert_empty(&accounts);
}

#[test]
fn an_extra_installed_http_route_for_the_same_behavior_refuses_coverage() {
    let owner = owner();
    let accounts = bank();
    let addresses = [tcp_addr(), tcp_addr()];
    let mut input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        Rc::new(Counts::default()),
    );
    let registrations = input
        .providers
        .snapshot(&mut PublicationFacts::default())
        .unwrap();
    let providers: Vec<_> = registrations.iter().map(|p| p.as_ref()).collect();
    let other = input
        .instance
        .description()
        .unwrap()
        .replace("/value", "/unadvertised");
    let other = prepare_thing(
        &other,
        input.instance.key(),
        GatewayDefaultV1::LIMITS,
        &mut budget(),
        &providers,
        |_, _| true,
    )
    .unwrap();
    let mappings = [&input.prepared, &other]
        .into_iter()
        .map(|prepared| {
            (
                AffordanceTarget::Property(Arc::from("left")),
                prepared.operations()[0]
                    .definition::<HttpProducerProgram>()
                    .unwrap()
                    .clone(),
            )
        })
        .collect();
    let config = HttpFiniteConfig::try_from_mappings(
        HttpProducerConfig::for_loopback(addresses[0]).unwrap(),
        mappings,
        input.instance.key(),
        100,
    )
    .ok()
    .unwrap();
    let runtime = HttpRuntime::try_new(
        HttpFiniteBinding::new(
            config.clone(),
            BindingId::new(101),
            BindingGeneration::INITIAL,
        ),
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
    .unwrap();
    input.participants[0] = OwnedProducerParticipation::finite(runtime);
    let result =
        ComposedProducerExposure::try_new(&owner, input, limits(), accounts.clone(), &mut budget())
            .err()
            .unwrap();
    assert!(matches!(
        result.error().error(),
        CoreError::UnsupportedOperation(_)
    ));
    assert_eq!(owner.description(result.input().instance.key()), None);
    assert_empty(&accounts);
    for address in addresses {
        TcpListener::bind(address).unwrap();
    }
}

#[test]
fn a_changed_action_service_during_handoff_returns_the_original_candidate() {
    struct ChangingWork(Cell<u64>);
    impl ActionValidator for ChangingWork {
        fn additional_validation_work(&self) -> u64 {
            let n = self.0.get();
            self.0.set(n + 1);
            n
        }
        fn validate_input(&self, input: &InteractionInput) -> CoreResult<()> {
            UdpJsonValidation.validate_input(input)
        }
        fn validate_output(&self, output: &InteractionOutput) -> CoreResult<()> {
            UdpJsonValidation.validate_output(output)
        }
    }
    let owner = owner();
    let accounts = bank();
    let native = ActionNative::new(udp_addr());
    let (mut input, _) = action_input(
        &owner,
        key(1, Generation::INITIAL),
        &native,
        Rc::new(Counts::default()),
        None,
    );
    let key = input.instance.key();
    let original = input.participants[0].participation_id().downgrade();
    input.participants[0]
        .as_any_mut()
        .downcast_mut::<ActionLease>()
        .unwrap()
        .validator = Rc::new(ChangingWork(Cell::new(0)));
    let refused =
        ComposedProducerExposure::try_new(&owner, input, limits(), accounts.clone(), &mut budget())
            .err()
            .expect("a changed service must return the candidate");
    assert!(matches!(refused.error().error(), CoreError::Validation(_)));
    assert_eq!(refused.input().instance.key(), key);
    assert!(
        refused.input().participants[0]
            .participation_id()
            .matches(&original)
    );
    assert!(native.borrow().socket.is_none());
    assert_empty(&accounts);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Withdrawal {
    BeforeRequest,
    DuringCredit,
    DuringAuthorization,
}

fn check_sibling_withdrawal(when: Withdrawal) {
    let mut owner = owner();
    let accounts = bank();
    let addresses = [tcp_addr(), tcp_addr()];
    let counts = Rc::new(Counts::default());
    let input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        counts.clone(),
    );
    let key = input.instance.key();
    let op = &input.prepared.operations()[0];
    let substitute = http_read(
        &owner,
        addresses[0],
        op.definition::<HttpProducerProgram>().unwrap().clone(),
        key,
        "left",
        op.binding.0,
        FiniteValidation::Required,
    );
    let mut exposure =
        ComposedProducerExposure::try_new(&owner, input, limits(), accounts.clone(), &mut budget())
            .ok()
            .unwrap();
    start_http(&mut exposure);
    exposure.publish(&mut owner, &mut budget()).unwrap();
    let left = exposure.participant_mut::<HttpRuntime>(0).unwrap();
    assert!(read_peer(addresses[0], &mut owner, left).starts_with(b"HTTP/1.1 200"));
    let mut original = std::mem::replace(left, substitute);
    match when {
        Withdrawal::BeforeRequest => original.begin_close(),
        Withdrawal::DuringAuthorization => {
            counts
                .withdraw_on_authorize
                .replace(Some(original.ingress_gate().clone()));
        }
        Withdrawal::DuringCredit => {}
    }
    let mut peer = TcpStream::connect(addresses[1]).unwrap();
    write!(
        peer,
        "GET /value HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        addresses[1]
    )
    .unwrap();
    let right = exposure.participant_mut::<HttpRuntime>(1).unwrap();
    let end = Instant::now() + Duration::from_secs(3);
    while right.state() != RawFiniteState::Dispatching {
        let _ = owner.step(right, &mut cx(), &mut budget(), now()).unwrap();
        assert!(Instant::now() < end);
    }
    if when == Withdrawal::DuringCredit {
        assert!(matches!(
            exposure.step(&mut owner, &mut cx(), now(), |i, _| {
                if i == 1 {
                    original.begin_close();
                }
                budget()
            }),
            Err(CoreError::StaleHandle(_))
        ));
    } else {
        let _ = owner.step(right, &mut cx(), &mut budget(), now()).unwrap();
    }
    assert_eq!(
        counts.property.get(),
        1,
        "withdrawn sibling entered the handler"
    );
    assert!(owner.description(key).is_none());
    assert_eq!(
        original.take_settlement().unwrap().delivery,
        FiniteDelivery::Delivered
    );
    *exposure.participant_mut::<HttpRuntime>(0).unwrap() = original;
    close_http(&mut exposure);
    let settlement = exposure
        .participant_mut::<HttpRuntime>(1)
        .unwrap()
        .take_settlement()
        .unwrap();
    assert_eq!(settlement.effect, FiniteEffect::NotInvoked);
    drop(settlement);
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    assert_empty(&accounts);
}

#[test]
fn direct_dispatch_refuses_a_request_after_an_original_member_closes() {
    check_sibling_withdrawal(Withdrawal::BeforeRequest);
}

#[test]
fn managed_dispatch_refuses_withdrawal_in_the_credit_callback() {
    check_sibling_withdrawal(Withdrawal::DuringCredit);
}

#[test]
fn dispatch_refuses_sibling_withdrawal_inside_authorization() {
    check_sibling_withdrawal(Withdrawal::DuringAuthorization);
}

fn check_staged_withdrawal(cause: Option<CoreError>) {
    let mut owner = owner();
    let accounts = bank();
    let addresses = [tcp_addr(), tcp_addr()];
    let input = http_input(
        &owner,
        key(1, Generation::INITIAL),
        addresses,
        Rc::new(Counts::default()),
    );
    let mut exposure =
        ComposedProducerExposure::try_new(&owner, input, limits(), accounts.clone(), &mut budget())
            .ok()
            .unwrap();
    start_http(&mut exposure);
    assert!(matches!(
        exposure.publish(&mut owner, &mut WorkBudget::new()),
        Err(CoreError::Backpressure(_))
    ));
    assert!(scratch(&accounts) > 0);
    assert!(exposure.publication_failure().is_none());
    let member = exposure.participant_mut::<HttpRuntime>(0).unwrap();
    if let Some(error) = &cause {
        member.fence(error.clone());
    } else {
        member.begin_close();
    }
    let failure = exposure.publish(&mut owner, &mut budget()).unwrap_err();
    match cause {
        Some(error) => assert_eq!(failure, error),
        None => assert!(matches!(failure, CoreError::Cancelled(_)), "{failure:?}"),
    }
    assert_eq!(exposure.publication_failure(), Some(&failure));
    assert_eq!(scratch(&accounts), 0);
    assert_eq!(
        exposure.participant_mut::<HttpRuntime>(1).unwrap().state(),
        RawFiniteState::Closing
    );
    assert!(owner.description(exposure.key()).is_none());
    for _ in 0..8 {
        exposure
            .step(&mut owner, &mut cx(), now(), |_, _| budget())
            .unwrap();
    }
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    for address in addresses {
        TcpListener::bind(address).unwrap();
    }
    assert_empty(&accounts);
}

#[test]
fn terminal_native_backpressure_rolls_back_funded_stage() {
    check_staged_withdrawal(Some(CoreError::Backpressure(
        ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
            .with_redacted_cause(77, "native participation fenced"),
    )));
}

#[test]
fn a_withdrawn_staged_member_rolls_back_before_physical_retirement() {
    check_staged_withdrawal(None);
}

#[test]
fn replacement_drains_original_members_with_or_without_a_td_lookup() {
    for (lookup, managed) in [(false, true), (true, true), (true, false)] {
        let mut owner = owner();
        let accounts = bank();
        let old_addresses = [tcp_addr(), tcp_addr()];
        let input = http_input(
            &owner,
            key(1, Generation::INITIAL),
            old_addresses,
            Rc::new(Counts::default()),
        );
        let mut old = ComposedProducerExposure::try_new(
            &owner,
            input,
            limits(),
            accounts.clone(),
            &mut budget(),
        )
        .ok()
        .unwrap();
        start_http(&mut old);
        old.publish(&mut owner, &mut budget()).unwrap();
        assert!(
            read_peer(
                old_addresses[0],
                &mut owner,
                old.participant_mut::<HttpRuntime>(0).unwrap()
            )
            .starts_with(b"HTTP/1.1 200")
        );

        let new_addresses = [tcp_addr(), tcp_addr()];
        let mut input = http_input(
            &owner,
            key(1, Generation::new(2).unwrap()),
            new_addresses,
            Rc::new(Counts::default()),
        );
        input.expected = Some(old.key());
        let mut successor = ComposedProducerExposure::try_new(
            &owner,
            input,
            limits(),
            accounts.clone(),
            &mut budget(),
        )
        .ok()
        .unwrap();
        start_http(&mut successor);
        // A healthy staged successor still owns its closed listeners while the
        // incumbent is serving in the same slot.
        successor
            .step(&mut owner, &mut cx(), now(), |_, _| budget())
            .unwrap();
        old.participant_mut::<HttpRuntime>(0).unwrap().begin_close();
        if lookup {
            assert!(owner.description(old.key()).is_none());
        }
        successor.publish(&mut owner, &mut budget()).unwrap();
        for _ in 0..8 {
            if managed {
                old.step(&mut owner, &mut cx(), now(), |_, _| budget())
                    .unwrap();
            } else {
                let key = old.key();
                for i in 0..2 {
                    let member = old.participant_mut::<HttpRuntime>(i).unwrap();
                    let _ = owner.step(member, &mut cx(), &mut budget(), now()).unwrap();
                    owner.reconcile_participation(key, member).unwrap();
                }
            }
        }

        for (i, addr) in new_addresses.into_iter().enumerate() {
            let runtime = successor.participant_mut::<HttpRuntime>(i).unwrap();
            assert!(read_peer(addr, &mut owner, runtime).starts_with(b"HTTP/1.1 200"));
            assert_eq!(
                runtime.take_settlement().unwrap().delivery,
                FiniteDelivery::Delivered
            );
        }
        for (i, addr) in old_addresses.into_iter().enumerate() {
            assert_eq!(
                old.participant_mut::<HttpRuntime>(i).unwrap().state(),
                RawFiniteState::Closed,
                "lookup={lookup}, member={i}"
            );
            TcpListener::bind(addr).unwrap();
        }
        assert!(old.retire(&mut owner, &mut budget()).is_err());
        let original = old
            .participant_mut::<HttpRuntime>(0)
            .unwrap()
            .take_settlement()
            .unwrap();
        assert_eq!(original.recipient, Some(old.key()));
        assert_eq!(original.delivery, FiniteDelivery::Delivered);
        old.retire(&mut owner, &mut budget()).unwrap();
        assert!(owner.description(successor.key()).is_some());
        close_http(&mut successor);
        successor.retire(&mut owner, &mut budget()).unwrap();
        drop((old, successor, original));
        assert_empty(&accounts);
    }
}
