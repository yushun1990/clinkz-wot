//! Real whole-description publication using one explicitly compiled HTTP
//! definition across independent names, native routes and logical authorities.
use super::*;

#[path = "shared_listener.rs"]
mod shared_listener;

#[derive(Default)]
struct App {
    denied: Cell<bool>,
    problem: Cell<bool>,
    calls: Cell<usize>,
}
struct InstanceProperty {
    app: Rc<App>,
    key: ProducerInstanceKey,
    name: &'static str,
    value: u8,
}
impl RawFiniteHandler for InstanceProperty {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        if self.app.denied.get() {
            return Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Admission, RetryClass::Never),
            });
        }
        Ok(())
    }
    fn handle(
        &self,
        ctx: HandlerContext<'_>,
        _: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        assert_eq!(ctx.producer_instance(), Some(self.key));
        assert_eq!(ctx.target().name(), Some(self.name));
        self.app.calls.set(self.app.calls.get() + 1);
        if self.app.problem.get() {
            return Ok(InteractionOutput::with_data(Payload::new(
                b"{\"reason\":\"busy\"}".to_vec(),
                "application/json",
            ))
            .with_status(InteractionStatus::ApplicationError)
            .try_with_metadata(
                InteractionOutputMetadata::default()
                    .with_response_selection(ResponseSelection::Additional(0)),
            )
            .unwrap());
        }
        Ok(InteractionOutput::with_data(Payload::new(
            format!("{{\"value\":{}}}", self.value).into_bytes(),
            "application/json",
        )))
    }
}
fn td(address: SocketAddr, name: &str, paths: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "@context": ["https://www.w3.org/2022/wot/td/v1.1", {"htv":"http://www.w3.org/2011/http#"}],
        "id": format!("urn:example:{name}"), "title": name,
        "security": ["n"], "securityDefinitions": {"n":{"scheme":"nosec"}},
        "schemaDefinitions": {"problem":{"type":"object", "required":["reason"],
            "properties":{"reason":{"type":"string", "minLength":1}}}},
        "properties": {name: {"type":"object", "readOnly":true, "required":["value"],
            "properties":{"value":{"type":"integer", "minimum":0, "maximum":100}},
            "forms": paths.iter().map(|path| serde_json::json!({
                "href":format!("http://{address}{path}"), "op":"readproperty",
                "additionalResponses":[{"success":false, "schema":"problem", "htv:statusCodeValue":422}]
            })).collect::<Vec<_>>()}}
    })
}
fn prepare(
    td: &serde_json::Value,
    key: ProducerInstanceKey,
    provider: &HttpFiniteProducer,
) -> CoreResult<PreparedThing> {
    prepare_thing(
        &td.to_string(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut budget(),
        &[provider],
        |scope, op| {
            matches!(scope, ValidatedProducerScope::Property(_)) && op == Operation::ReadProperty
        },
    )
}
fn input(
    owner: &RawProducerServient,
    authority: ProducerInstanceAuthority,
    td: &serde_json::Value,
    provider: Rc<HttpFiniteProducer>,
    name: &'static str,
    value: u8,
    app: Rc<App>,
) -> ComposedProducerInput {
    input_with_listener(owner, authority, td, provider, name, value, app, None)
}
#[allow(clippy::too_many_arguments)] // Each independent publication operand is explicit in the fixture.
fn input_with_listener(
    owner: &RawProducerServient,
    authority: ProducerInstanceAuthority,
    td: &serde_json::Value,
    provider: Rc<HttpFiniteProducer>,
    name: &'static str,
    value: u8,
    app: Rc<App>,
    listener: Option<&HttpFiniteListener>,
) -> ComposedProducerInput {
    input_with_capacity(
        owner, authority, td, provider, name, value, app, listener, None,
    )
}
#[allow(clippy::too_many_arguments)]
fn input_with_capacity(
    owner: &RawProducerServient,
    authority: ProducerInstanceAuthority,
    td: &serde_json::Value,
    provider: Rc<HttpFiniteProducer>,
    name: &'static str,
    value: u8,
    app: Rc<App>,
    listener: Option<&HttpFiniteListener>,
    capacity: Option<&ProducerFiniteCapacity>,
) -> ComposedProducerInput {
    let key = authority.key();
    let instance = RawProducerInstance::new(
        authority,
        vec![RawFiniteBehavior::new(
            AffordanceTarget::Property(Arc::from(name)),
            Operation::ReadProperty,
            InstanceProperty {
                app,
                key,
                name,
                value,
            },
        )],
    )
    .with_description(Arc::from(td.to_string()));
    let prepared = prepare_thing(
        instance.description().unwrap(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut budget(),
        &[provider.as_ref()],
        |scope, op| match scope {
            ValidatedProducerScope::Property(name) => instance.supports_property(name, op),
            _ => false,
        },
    )
    .unwrap();
    let mappings: Vec<_> = prepared
        .operations()
        .iter()
        .map(|operation| {
            (
                AffordanceTarget::Property(Arc::from(name)),
                operation
                    .definition::<HttpProducerProgram>()
                    .unwrap()
                    .clone(),
            )
        })
        .collect();
    let listen = HttpProducerConfig::for_loopback(mappings[0].1.listen_address()).unwrap();
    let config = HttpFiniteConfig::try_from_mappings(listen, mappings, key, 100).unwrap();
    let config = match listener {
        Some(listener) => config.with_listener(listener).unwrap(),
        None => config,
    };
    let runtime = RawFiniteRuntime::try_new(
        HttpFiniteBinding::new(config.clone(), provider.identity().0, provider.identity().1),
        config,
        owner,
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
    let runtime = match capacity {
        Some(capacity) => runtime.with_capacity(capacity.clone()).ok().unwrap(),
        None => runtime,
    };
    ComposedProducerInput {
        instance,
        prepared,
        providers: vec![provider],
        participants: vec![OwnedProducerParticipation::finite(runtime)],
        expected: None,
        handler_backing_bytes: std::mem::size_of::<App>(),
    }
}
fn admit(
    owner: &RawProducerServient,
    input: ComposedProducerInput,
    accounts: &Arc<HostResourceAccounts>,
) -> ComposedProducerExposure {
    ComposedProducerExposure::try_new(owner, input, limits(), accounts.clone(), &mut budget())
        .ok()
        .unwrap()
}
fn start(exposure: &mut ComposedProducerExposure) {
    assert_eq!(
        exposure
            .participant_mut::<HttpRuntime>(0)
            .unwrap()
            .step(&mut cx(), &mut budget(), now()),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
}
fn close(exposure: &mut ComposedProducerExposure) {
    exposure.begin_close();
    let runtime = exposure.participant_mut::<HttpRuntime>(0).unwrap();
    for _ in 0..100 {
        if runtime.state() == RawFiniteState::Closed {
            return;
        }
        let _ = runtime.step(&mut cx(), &mut budget(), now());
    }
    panic!("native close stalled");
}
fn request(
    address: SocketAddr,
    path: &str,
    owner: &mut RawProducerServient,
    exposure: &mut ComposedProducerExposure,
) -> Vec<u8> {
    let mut peer = TcpStream::connect(address).unwrap();
    peer.write_all(
        format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .unwrap();
    peer.set_nonblocking(true).unwrap();
    receive(peer, owner, exposure)
}
fn receive(
    mut peer: TcpStream,
    owner: &mut RawProducerServient,
    exposure: &mut ComposedProducerExposure,
) -> Vec<u8> {
    let mut output = Vec::new();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let _ = owner
            .step(
                exposure.participant_mut::<HttpRuntime>(0).unwrap(),
                &mut cx(),
                &mut budget(),
                now(),
            )
            .unwrap();
        let mut buf = [0; 1024];
        match peer.read(&mut buf) {
            Ok(0) => return output,
            Ok(n) => output.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("{e}"),
        }
        assert!(Instant::now() < end, "HTTP request stalled");
    }
}
fn acknowledge(exposure: &mut ComposedProducerExposure) -> RawFiniteSettlement {
    let report = exposure
        .participant_mut::<HttpRuntime>(0)
        .unwrap()
        .take_settlement()
        .unwrap();
    assert_eq!(report.recipient, Some(exposure.key()));
    report
}
fn reuse(
    address: SocketAddr,
    id: u32,
    definition: &HttpProducerDefinition,
) -> Rc<HttpFiniteProducer> {
    Rc::new(
        HttpFiniteProducer::reusing(
            HttpProducerConfig::for_loopback(address).unwrap(),
            BindingId::new(id),
            BindingGeneration::INITIAL,
            100,
            definition,
        )
        .unwrap(),
    )
}
fn definition(prepared: &PreparedThing) -> HttpProducerDefinition {
    HttpProducerDefinition::from_program(
        prepared.operations()[0]
            .definition::<HttpProducerProgram>()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn one_lowering_survives_independent_publication_failure_permission_retirement_and_original_bank_release()
 {
    let mut owner = owner();
    let original = bank();
    let second_bank = bank();
    let failed_bank = bank();
    let addresses = [tcp_addr(), tcp_addr(), tcp_addr()];
    let compiler = Rc::new(
        HttpFiniteProducer::compiling(
            HttpProducerConfig::for_loopback(addresses[0]).unwrap(),
            BindingId::new(301),
            BindingGeneration::INITIAL,
            100,
            original.clone(),
        )
        .unwrap(),
    );
    let apps = [
        Rc::new(App::default()),
        Rc::new(App::default()),
        Rc::new(App::default()),
    ];
    let first_input = input(
        &owner,
        key(1, Generation::INITIAL),
        &td(addresses[0], "alpha", &["/one"]),
        compiler.clone(),
        "alpha",
        11,
        apps[0].clone(),
    );
    let shared = definition(&first_input.prepared);
    let shared_bytes = shared.retained_bytes() as u64;
    assert_eq!(compiler.native_compilations(), 1);
    assert_eq!(live(&original), shared_bytes);
    let reuser = reuse(addresses[1], 302, &shared);
    let second_input = input(
        &owner,
        key(2, Generation::INITIAL),
        &td(addresses[1], "beta", &["/two", "/alias"]),
        reuser.clone(),
        "beta",
        22,
        apps[1].clone(),
    );
    for op in second_input.prepared.operations() {
        let bound = op.definition::<HttpProducerProgram>().unwrap();
        assert_eq!(bound.listen_address(), addresses[1]);
        assert!(
            shared.shares_compilation_with(&HttpProducerDefinition::from_program(bound).unwrap())
        );
    }
    let last_program = second_input.prepared.operations()[1]
        .definition::<HttpProducerProgram>()
        .unwrap()
        .clone();
    assert_eq!(reuser.native_compilations(), 0);
    assert_eq!(reuser.mapping_bindings(), 2); // two Forms, one independent Thing
    // A third staged publication conflicts with the first logical slot only
    // after both have completed pure admission and closed native installation.
    let failed_provider = reuse(addresses[2], 303, &shared);
    let failed_input = input(
        &owner,
        key(1, Generation::INITIAL),
        &td(addresses[2], "gamma", &["/three"]),
        failed_provider.clone(),
        "gamma",
        33,
        apps[2].clone(),
    );
    let mut first = admit(&owner, first_input, &original);
    let mut second = admit(&owner, second_input, &second_bank);
    let mut failed = admit(&owner, failed_input, &failed_bank);
    for exposure in [&mut first, &mut second, &mut failed] {
        start(exposure);
    }
    let mut early_peer = TcpStream::connect(addresses[0]).unwrap();
    early_peer
        .write_all(
            format!(
                "GET /one HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                addresses[0]
            )
            .as_bytes(),
        )
        .unwrap();
    early_peer.set_nonblocking(true).unwrap();
    // Actual queued ingress cannot enter behavior before publication.
    for exposure in [&mut first, &mut second, &mut failed] {
        let _ = exposure.participant_mut::<HttpRuntime>(0).unwrap().step(
            &mut cx(),
            &mut budget(),
            now(),
        );
    }
    assert!(apps.iter().all(|app| app.calls.get() == 0));
    first.publish(&mut owner, &mut budget()).unwrap();
    second.publish(&mut owner, &mut budget()).unwrap();
    assert!(failed.publish(&mut owner, &mut budget()).is_err());
    assert!(failed.publication_failure().is_some());
    assert!(!failed.is_committed());
    assert!(first.is_committed() && second.is_committed());
    assert_eq!(compiler.native_compilations(), 1); // admission and final check did not lower
    assert_eq!(failed_provider.native_compilations(), 0);
    close(&mut failed);
    failed.retire(&mut owner, &mut budget()).unwrap();
    drop(failed);
    drop(failed_provider);
    assert_empty(&failed_bank);
    TcpListener::bind(addresses[2]).unwrap();

    let a = receive(early_peer, &mut owner, &mut first);
    assert!(a.starts_with(b"HTTP/1.1 200"));
    assert!(a.ends_with(b"{\"value\":11}"));
    apps[1].denied.set(true);
    request(addresses[1], "/two", &mut owner, &mut second);
    let denied = acknowledge(&mut second);
    assert!(matches!(
        denied.result,
        Err(CoreError::Security {
            reason: SecurityFailureReason::AuthorizationDenied,
            ..
        })
    ));
    assert_eq!(denied.effect, FiniteEffect::NotInvoked);
    assert_eq!(apps[0].calls.get(), 1);
    assert_eq!(apps[1].calls.get(), 0);
    close(&mut first);
    assert!(first.retire(&mut owner, &mut budget()).is_err()); // original unread settlement
    assert_eq!(acknowledge(&mut first).delivery, FiniteDelivery::Delivered);
    first.retire(&mut owner, &mut budget()).unwrap();
    drop(first);
    drop(compiler);
    assert_eq!(live(&original), shared_bytes);
    TcpListener::bind(addresses[0]).unwrap();
    // Same physical definition, fresh current policy, actual independent handler.
    apps[1].denied.set(false);
    let b = request(addresses[1], "/alias", &mut owner, &mut second);
    assert!(b.starts_with(b"HTTP/1.1 200"));
    assert!(b.ends_with(b"{\"value\":22}"));
    assert_eq!(acknowledge(&mut second).delivery, FiniteDelivery::Delivered);
    assert_eq!(apps[1].calls.get(), 1);
    assert_eq!(apps[2].calls.get(), 0);
    apps[1].problem.set(true);
    let problem = request(addresses[1], "/two", &mut owner, &mut second);
    assert!(problem.starts_with(b"HTTP/1.1 422"));
    assert!(problem.ends_with(b"{\"reason\":\"busy\"}"));
    let report = acknowledge(&mut second);
    assert_eq!(report.delivery, FiniteDelivery::Delivered);
    assert_eq!(
        report.result.unwrap().metadata().response_selection(),
        ResponseSelection::Additional(0)
    );
    assert_eq!(reuser.native_compilations(), 0);
    println!(
        "one Producer lowering; shared definition {shared_bytes} bytes; source and sibling final checks preserved original instructions"
    );
    close(&mut second);
    second.retire(&mut owner, &mut budget()).unwrap();
    drop(second);
    drop(reuser);
    assert_empty(&second_bank);
    assert_eq!(live(&original), shared_bytes); // explicit definition still owns it
    drop(shared);
    assert_eq!(live(&original), shared_bytes); // even a detached native snapshot retains original funding
    drop(last_program);
    assert_empty(&original);
    assert_eq!(
        original
            .account(ResourceKind::BindingArtifactsGlobalMax)
            .used(),
        0
    );
    assert_eq!(
        original
            .account(ResourceKind::BindingArtifactBytesGlobalMax)
            .used(),
        0
    );
    TcpListener::bind(addresses[1]).unwrap();
}

#[test]
fn changed_dependencies_and_complete_final_description_refuse_without_lowering_or_native_setup() {
    let owner = owner();
    let accounts = bank();
    let source_addr = tcp_addr();
    let provider = HttpFiniteProducer::compiling(
        HttpProducerConfig::for_loopback(source_addr).unwrap(),
        BindingId::new(310),
        BindingGeneration::INITIAL,
        100,
        accounts.clone(),
    )
    .unwrap();
    let source = prepare(
        &td(source_addr, "source", &["/source"]),
        key(1, Generation::INITIAL).key(),
        &provider,
    )
    .unwrap();
    let shared = definition(&source);
    let address = tcp_addr();
    let reuser = reuse(address, 311, &shared);
    let base = td(address, "other", &["/other"]);
    let mut cases = Vec::new();
    let mut changed = base.clone();
    changed["properties"]["other"]["properties"]["value"]["maximum"] = 99.into();
    cases.push(changed);
    let mut changed = base.clone();
    changed["properties"]["other"]["required"] = serde_json::json!([]);
    cases.push(changed);
    let mut changed = base.clone();
    changed["schemaDefinitions"]["problem"]["properties"]["reason"]["minLength"] = 2.into();
    cases.push(changed);
    let mut changed = base.clone();
    changed["properties"]["other"]["forms"][0]["additionalResponses"][0]["htv:statusCodeValue"] =
        409.into();
    cases.push(changed);
    let mut changed = base.clone();
    changed["properties"]["other"]["forms"][0]["additionalResponses"][0]["success"] = true.into();
    cases.push(changed);
    let mut changed = base.clone();
    changed["properties"]["other"]["forms"][0]["response"] =
        serde_json::json!({"contentType":"application/json; charset=utf-8"});
    cases.push(changed);
    let mut changed = base.clone();
    changed["properties"]["other"]["forms"][0]["htv:methodName"] = "POST".into();
    cases.push(changed);
    let mut changed = base.clone();
    changed["properties"]["other"]["forms"][0]["contentCoding"] = "gzip".into();
    cases.push(changed);
    let mut changed = base.clone();
    changed["securityDefinitions"]["n"]["scheme"] = "basic".into();
    cases.push(changed);
    let mut changed = base.clone();
    changed["properties"]["other"]["forms"][0]["urn:unknown:meaning"] = true.into();
    cases.push(changed);
    let mut changed = base.clone();
    changed["properties"]["other"]["forms"].as_array_mut().unwrap().push(serde_json::json!({"href":format!("http://{address}/unsupported"),"op":"observeproperty"}));
    cases.push(changed);
    for (i, changed) in cases.iter().enumerate() {
        assert!(
            prepare(changed, key(2, Generation::INITIAL).key(), &reuser).is_err(),
            "case {i}"
        );
        assert_eq!(reuser.native_compilations(), 0);
        TcpListener::bind(address).unwrap();
    }
    // Changing the final TD after preparation must also refuse managed handoff.
    let app = Rc::new(App::default());
    let mut candidate = input(
        &owner,
        key(2, Generation::INITIAL),
        &base,
        reuser.clone(),
        "other",
        2,
        app.clone(),
    );
    let original_bytes = live(&accounts);
    candidate.instance = candidate
        .instance
        .with_description(Arc::from(cases[0].to_string()));
    let refused_bank = bank();
    assert!(
        ComposedProducerExposure::try_new(
            &owner,
            candidate,
            limits(),
            refused_bank.clone(),
            &mut budget()
        )
        .is_err()
    );
    assert_empty(&refused_bank);
    assert_eq!(live(&accounts), original_bytes);
    assert_eq!(app.calls.get(), 0);
    assert_eq!(provider.native_compilations(), 1);
    assert_eq!(reuser.native_compilations(), 0);
    drop(reuser);
    drop(shared);
    drop(source);
    drop(provider);
    assert_empty(&accounts);
}

#[test]
fn reused_forms_still_collide_natively_and_ambiguous_registrations_cannot_select_by_order() {
    let address = tcp_addr();
    let accounts = bank();
    let provider = HttpFiniteProducer::compiling(
        HttpProducerConfig::for_loopback(address).unwrap(),
        BindingId::new(320),
        BindingGeneration::INITIAL,
        100,
        accounts.clone(),
    )
    .unwrap();
    let source = prepare(
        &td(address, "a", &["/a"]),
        key(1, Generation::INITIAL).key(),
        &provider,
    )
    .unwrap();
    let shared = definition(&source);
    let reuser = reuse(address, 321, &shared);
    let authority = key(2, Generation::INITIAL);
    let collided = prepare(
        &td(address, "b", &["/same", "/same"]),
        authority.key(),
        &reuser,
    )
    .unwrap();
    let mappings = collided
        .operations()
        .iter()
        .map(|op| {
            (
                AffordanceTarget::Property(Arc::from("b")),
                op.definition::<HttpProducerProgram>().unwrap().clone(),
            )
        })
        .collect();
    assert!(
        HttpFiniteConfig::try_from_mappings(
            HttpProducerConfig::for_loopback(address).unwrap(),
            mappings,
            authority.key(),
            100
        )
        .is_err()
    );
    let other = reuse(address, 322, &shared);
    assert!(
        prepare_thing(
            &td(address, "b", &["/b"]).to_string(),
            authority.key(),
            GatewayDefaultV1::LIMITS,
            &mut budget(),
            &[reuser.as_ref(), other.as_ref()],
            |_, _| true
        )
        .is_err()
    );
    assert_eq!(provider.native_compilations(), 1);
    assert_eq!(
        reuser.native_compilations() + other.native_compilations(),
        0
    );
    TcpListener::bind(address).unwrap();
    drop(collided);
    drop(source);
    drop(shared);
    drop(provider);
    drop(reuser);
    drop(other);
    assert_empty(&accounts);
}

#[test]
fn explicit_compilation_reserves_before_lowering_and_rolls_back_each_failed_account() {
    let address = tcp_addr();
    for kind in [
        ResourceKind::EngineLiveBytesGlobalMax,
        ResourceKind::BindingArtifactsGlobalMax,
        ResourceKind::BindingArtifactBytesGlobalMax,
        ResourceKind::CleanupBytesMax,
        ResourceKind::CleanupItemsMax,
    ] {
        let accounts = bank();
        let blocker = accounts
            .reserve(kind, accounts.account(kind).limit())
            .unwrap();
        let provider = HttpFiniteProducer::compiling(
            HttpProducerConfig::for_loopback(address).unwrap(),
            BindingId::new(330),
            BindingGeneration::INITIAL,
            100,
            accounts.clone(),
        )
        .unwrap();
        assert!(matches!(
            prepare(
                &td(address, "a", &["/a"]),
                key(1, Generation::INITIAL).key(),
                &provider
            ),
            Err(CoreError::Backpressure(_))
        ));
        assert_eq!(provider.native_compilations(), 0);
        drop(blocker);
        assert_empty(&accounts);
        assert_eq!(
            accounts
                .account(ResourceKind::BindingArtifactsGlobalMax)
                .used(),
            0
        );
        assert_eq!(
            accounts
                .account(ResourceKind::BindingArtifactBytesGlobalMax)
                .used(),
            0
        );
    }
    TcpListener::bind(address).unwrap();
}

#[test]
fn final_check_qualifies_the_actual_supplied_definition_as_well_as_the_original_mapping() {
    let accounts = bank();
    let addresses = [tcp_addr(), tcp_addr()];
    let compiler = HttpFiniteProducer::compiling(
        HttpProducerConfig::for_loopback(addresses[0]).unwrap(),
        BindingId::new(340),
        BindingGeneration::INITIAL,
        100,
        accounts.clone(),
    )
    .unwrap();
    let authority = key(1, Generation::INITIAL);
    let original_td = td(addresses[0], "a", &["/a"]);
    let original = prepare(&original_td, authority.key(), &compiler).unwrap();
    let other_compiler = HttpFiniteProducer::compiling(
        HttpProducerConfig::for_loopback(addresses[1]).unwrap(),
        BindingId::new(341),
        BindingGeneration::INITIAL,
        100,
        accounts.clone(),
    )
    .unwrap();
    let mut changed = td(addresses[1], "b", &["/b"]);
    changed["properties"]["b"]["properties"]["value"]["maximum"] = 99.into();
    let other = prepare(&changed, key(2, Generation::INITIAL).key(), &other_compiler).unwrap();
    let different_definition = definition(&other);
    let substituted = reuse(addresses[0], 340, &different_definition);
    // Same native endpoint, Binding identity and an old program that agrees
    // with the input cannot certify this actual registration's different schema.
    assert!(
        check_final_thing(
            &original_td.to_string(),
            authority.key(),
            GatewayDefaultV1::LIMITS,
            &mut budget(),
            &[substituted.as_ref()],
            &original,
            |_, _| true
        )
        .is_err()
    );
    assert!(prepare(&original_td, authority.key(), &substituted).is_err());
    assert_eq!(substituted.native_compilations(), 0);
    for address in addresses {
        TcpListener::bind(address).unwrap();
    }
    drop(substituted);
    drop(different_definition);
    drop(other);
    drop(original);
    drop(compiler);
    drop(other_compiler);
    assert_empty(&accounts);
}
