#![cfg(feature = "prepare")]
#![allow(dead_code)]
//! Real TLS, qualified TD Producer, Engine Consumer and an independent TLS peer.
include!("support/collection.rs");
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

const USER: &[u8] = b"device";
const PASSWORD: &[u8] = b"fixture-secret-not-a-principal";
struct Verifier {
    expected_password: &'static [u8],
    replacement_identity: AtomicBool,
    changed_work: AtomicBool,
    revoked: AtomicBool,
    calls: std::sync::atomic::AtomicUsize,
}
impl HttpBasicVerifier for Verifier {
    fn verify(&self, username: &[u8], password: &[u8]) -> CoreResult<Principal> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.revoked.load(Ordering::Acquire)
            || username != USER
            || password != self.expected_password
        {
            return Err(CoreError::Security {
                reason: SecurityFailureReason::InvalidCredentials,
                context: ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
                    .with_redacted_cause(1, "redacted verifier refusal"),
            });
        }
        Ok(Principal {
            id: PrincipalId::from(if self.replacement_identity.load(Ordering::Acquire) {
                "replacement-account"
            } else {
                "verified-account"
            }),
            scopes: vec!["read".into()],
        })
    }
    fn work(&self) -> u64 {
        if self.changed_work.load(Ordering::Acquire) {
            2048
        } else {
            1024
        }
    }
}
struct Keys {
    cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
}
impl Keys {
    fn for_name(name: &str) -> Self {
        let rcgen::CertifiedKey { cert, signing_key } =
            rcgen::generate_simple_self_signed(vec![name.into()]).unwrap();
        Self {
            cert: cert.der().clone(),
            key: PrivatePkcs8KeyDer::from(signing_key.serialize_der()).into(),
        }
    }
    fn client(&self) -> HttpClientTls {
        HttpClientTls::from_roots(vec![self.cert.clone()]).unwrap()
    }
    fn server(&self, verifier: Arc<Verifier>) -> HttpServerSecurity {
        HttpServerSecurity::try_new(vec![self.cert.clone()], self.key.clone_key(), verifier)
            .unwrap()
    }
    fn independent_client(&self) -> Arc<rustls::ClientConfig> {
        let mut roots = rustls::RootCertStore::empty();
        roots.add(self.cert.clone()).unwrap();
        Arc::new(
            rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth(),
        )
    }
}
fn secure_description(addr: SocketAddr) -> Value {
    json!({"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"secured temperature",
        "security":["auth"],"securityDefinitions":{"auth":{"scheme":"basic"}},"forms":[],
        "properties":{"on":{"type":"integer","readOnly":true,"forms":[{"href":format!("https://{addr}/on")}]}}})
}
#[derive(Default)]
struct SecureApp {
    revoke_in_permission: RefCell<Option<Arc<Verifier>>>,
    denied: Cell<bool>,
    reads: Cell<usize>,
    deny_after: Cell<bool>,
    principal: RefCell<Option<Principal>>,
}
struct SecureBehavior(Rc<SecureApp>);
impl RawFiniteHandler for SecureBehavior {
    fn authorize(&self, input: &InteractionInput) -> CoreResult<()> {
        if let Some(verifier) = self.0.revoke_in_permission.borrow().as_ref() {
            verifier.revoked.store(true, Ordering::Release);
        }
        if self.0.denied.get()
            || input
                .principal
                .as_ref()
                .is_none_or(|p| p.id.as_str() != "verified-account" || p.scopes != ["read"])
        {
            return Err(denied());
        }
        Ok(())
    }
    fn handle(
        &self,
        _: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.0.reads.set(self.0.reads.get() + 1);
        *self.0.principal.borrow_mut() = input.principal.clone();
        if self.0.deny_after.get() {
            self.0.denied.set(true);
        }
        Ok(InteractionOutput::with_data(payload(b"17")))
    }
}
type SecureServer = RawFiniteRuntime<HttpFiniteBinding>;
struct SecureFixture {
    owner: RawProducerServient,
    exposure: ComposedProducerExposure,
    app: Rc<SecureApp>,
    accounts: Arc<HostResourceAccounts>,
    addr: SocketAddr,
    td: Value,
}
impl SecureFixture {
    fn new(keys: &Keys, verifier: Arc<Verifier>) -> Self {
        let addr = address();
        let td = secure_description(addr);
        let accounts = bank();
        let mut owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
            instances: 1,
            behaviors_per_instance: 1,
            target_name_bytes: 256,
        })
        .unwrap();
        let app = Rc::new(SecureApp::default());
        let authority = authority(Generation::INITIAL);
        let key = authority.key();
        let instance = RawProducerInstance::new(
            authority,
            vec![RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("on")),
                Operation::ReadProperty,
                SecureBehavior(app.clone()),
            )],
        )
        .with_description(Arc::from(td.to_string()));
        let listen = HttpProducerConfig::for_https_loopback(addr).unwrap();
        let security = keys.server(verifier);
        let provider: Rc<dyn ProducerCapability> = Rc::new(
            HttpFiniteProducer::try_https_basic(
                listen,
                BindingId::new(197),
                BindingGeneration::INITIAL,
                1000,
                security.clone(),
            )
            .unwrap(),
        );
        let prepared = prepare_thing(
            instance.description().unwrap(),
            key,
            GatewayDefaultV1::LIMITS,
            &mut work(),
            &[provider.as_ref()],
            |scope, op| match scope {
                ValidatedProducerScope::Property(n) => instance.supports_property(n, op),
                _ => false,
            },
        )
        .unwrap();
        let program = prepared.operations()[0]
            .definition::<HttpProducerProgram>()
            .unwrap()
            .clone();
        let config = HttpFiniteConfig::try_new(
            listen,
            program,
            key,
            AffordanceTarget::Property(Arc::from("on")),
            1000,
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
        .with_security(security)
        .unwrap();
        let server = RawFiniteRuntime::try_new(
            HttpFiniteBinding::new(
                config.clone(),
                BindingId::new(197),
                BindingGeneration::INITIAL,
            ),
            config,
            &owner,
            RawFiniteLimits {
                candidates: 1,
                input_body_bytes: 4096,
                output_body_bytes: 4096,
                output_media_capacity: 64,
                output_retained_bytes: 8192,
            },
            FiniteValidation::Required,
            Deadline::NONE,
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let mut exposure = ComposedProducerExposure::try_new(
            &owner,
            ComposedProducerInput {
                instance,
                prepared,
                providers: vec![provider],
                participants: vec![Box::new(server)],
                expected: None,
                handler_backing_bytes: 4096,
            },
            Arc::new(GatewayDefaultV1::LIMITS.clone()),
            accounts.clone(),
            &mut work(),
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let server = exposure.participant_mut::<SecureServer>(0).unwrap();
        let _ = server.step(&mut cx(), &mut work(), now(0));
        assert_eq!(
            server.state(),
            RawFiniteState::InstalledClosed,
            "{:?}",
            server.first_cause()
        );
        exposure.publish(&mut owner, &mut work()).unwrap();
        Self {
            owner,
            exposure,
            app,
            accounts,
            addr,
            td,
        }
    }
    fn server(&mut self) -> &mut SecureServer {
        self.exposure.participant_mut::<SecureServer>(0).unwrap()
    }
    fn step(&mut self) {
        let _ = self
            .owner
            .step(
                self.exposure.participant_mut::<SecureServer>(0).unwrap(),
                &mut cx(),
                &mut work(),
                now(0),
            )
            .unwrap();
    }
    async fn exchange(&mut self, call: &mut ConsumerCall) -> ConsumerSettlement {
        let start = Instant::now();
        loop {
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "TLS exchange stalled: {:?}, {:?}",
                self.server().state(),
                self.server().first_cause()
            );
            self.step();
            if let Poll::Ready(result) = call.poll(&mut cx(), now(0), &mut work()) {
                return result;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    fn close(mut self) {
        self.exposure.begin_close();
        for _ in 0..100 {
            self.step();
            if self.server().state() == RawFiniteState::Closed {
                break;
            }
        }
        assert_eq!(self.server().state(), RawFiniteState::Closed);
        drop(self.server().take_settlement());
        self.exposure.retire(&mut self.owner, &mut work()).unwrap();
        drop(self.exposure);
        drop(self.owner);
        clean(&self.accounts);
    }
}
fn verifier() -> Arc<Verifier> {
    verifier_password(PASSWORD)
}
fn verifier_password(expected_password: &'static [u8]) -> Arc<Verifier> {
    Arc::new(Verifier {
        expected_password,
        replacement_identity: AtomicBool::new(false),
        changed_work: AtomicBool::new(false),
        revoked: AtomicBool::new(false),
        calls: std::sync::atomic::AtomicUsize::new(0),
    })
}
fn credentials() -> HttpBasicCredentials {
    HttpBasicCredentials::try_new(USER, PASSWORD).unwrap()
}
fn access(
    owner: &ConsumerInstance,
    target: &str,
    present: bool,
    deny: Arc<AtomicBool>,
) -> Arc<HttpBasicClientAccess> {
    Arc::new(
        HttpBasicClientAccess::try_new(
            owner.reference(),
            target,
            Arc::new(Access(deny)),
            present.then(credentials),
        )
        .unwrap(),
    )
}
fn secure_prepare(
    td: &Value,
    keys: &Keys,
    access: Arc<HttpBasicClientAccess>,
    accounts: &Arc<HostResourceAccounts>,
    reuse: Option<&Arc<PreparedConsumerContent>>,
) -> CoreResult<PreparedConsumer> {
    let registration = try_https_basic_consumer_registration(
        keys.client(),
        access,
        BindingId::new(198),
        BindingGeneration::INITIAL,
        0,
    )?;
    let registry = ConsumerRegistry::try_new(vec![registration], accounts)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let source: Thing = serde_json::from_value(td.clone()).unwrap();
    let drive = |job: &mut ConsumerCompilation<'_>| {
        while !job.is_finished() {
            job.step(&mut work(), false);
        }
    };
    if let Some(content) = reuse {
        registry.prepare_with_content(
            &source,
            GatewayDefaultV1::LIMITS,
            accounts.clone(),
            ConsumerSelector::default(),
            content,
            &mut work(),
            drive,
        )
    } else {
        registry.prepare(
            &source,
            GatewayDefaultV1::LIMITS,
            accounts.clone(),
            ConsumerSelector::default(),
            &mut work(),
            drive,
        )
    }
}
fn secure_client(
    td: &Value,
    keys: &Keys,
    owner: &ConsumerInstance,
    access: Arc<HttpBasicClientAccess>,
    accounts: &Arc<HostResourceAccounts>,
    reuse: Option<&Arc<PreparedConsumerContent>>,
) -> (ConsumerRuntime, Arc<PreparedConsumerContent>) {
    let prepared = secure_prepare(td, keys, access, accounts, reuse).unwrap();
    if reuse.is_some() {
        assert_eq!(prepared.content_constructions(), 0);
    }
    let (bindings, funding) = prepared.into_execution();
    let content = bindings[0].definition().content().clone();
    let runtime =
        ConsumerRuntime::try_new(owner.reference(), bindings, funding, accounts.clone(), 1)
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
    (runtime, content)
}
fn read_secure(client: &ConsumerRuntime) -> ConsumerCall {
    client
        .read_property("on", Deadline::NONE, now(0), &mut work())
        .unwrap()
}

#[test]
fn real_https_verifies_credentials_then_principal_and_reuses_only_immutable_meaning() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let verify_a = verifier();
        let verify_b = verifier_password(b"independent-instance-password");
        let mut a = SecureFixture::new(&keys, verify_a.clone());
        let mut b = SecureFixture::new(&keys, verify_b.clone());
        let owner_a = consumer_instance();
        let owner_b = consumer_instance();
        let bank_a = bank();
        let bank_b = bank();
        let access_a = access(
            &owner_a,
            a.td["properties"]["on"]["forms"][0]["href"]
                .as_str()
                .unwrap(),
            true,
            Arc::new(AtomicBool::new(false)),
        );
        let access_b = Arc::new(
            HttpBasicClientAccess::try_new(
                owner_b.reference(),
                b.td["properties"]["on"]["forms"][0]["href"]
                    .as_str()
                    .unwrap(),
                Arc::new(Access(Arc::new(AtomicBool::new(false)))),
                Some(
                    HttpBasicCredentials::try_new(USER, b"independent-instance-password").unwrap(),
                ),
            )
            .unwrap(),
        );
        let (client_a, content) =
            secure_client(&a.td, &keys, &owner_a, access_a.clone(), &bank_a, None);
        let (client_b, content_b) =
            secure_client(&b.td, &keys, &owner_b, access_b, &bank_b, Some(&content));
        assert!(Arc::ptr_eq(&content, &content_b));
        let mut call_a = read_secure(&client_a);
        let mut call_b = read_secure(&client_b);
        let result_a = a.exchange(&mut call_a).await;
        let result_b = b.exchange(&mut call_b).await;
        assert_eq!(result_a.result().unwrap().data().unwrap().body, b"17");
        assert!(result_b.result().is_ok());
        assert_eq!(
            a.app.principal.borrow().as_ref().unwrap().id.as_str(),
            "verified-account"
        );
        assert!(verify_a.calls.load(Ordering::Relaxed) >= 3);
        owner_a.revoke();
        assert!(
            client_a
                .read_property("on", Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        drop((call_b, result_b));
        drop(b.server().take_settlement());
        let mut next_b = read_secure(&client_b);
        let result_next_b = b.exchange(&mut next_b).await;
        assert!(
            result_next_b.result().is_ok(),
            "sibling credentials and authority remain independent"
        );
        drop((next_b, result_next_b));
        drop((
            call_a, client_a, client_b, owner_a, owner_b, content_b, access_a,
        ));
        assert!(
            bank_a
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        drop((result_a, content));
        clean(&bank_a);
        clean(&bank_b);
        a.close();
        b.close();
    });
}

async fn until(server: &mut SecureFixture, call: &mut ConsumerCall, state: RawFiniteState) {
    let start = Instant::now();
    loop {
        server.step();
        assert!(call.poll(&mut cx(), now(0), &mut work()).is_pending());
        if server.server().state() == state {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "state not reached: {:?}",
            server.server().state()
        );
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[test]
fn actual_support_refuses_complete_unsupported_expressions_and_incompatible_reuse() {
    let keys = Keys::for_name("127.0.0.1");
    let td = secure_description(address());
    let owner = consumer_instance();
    let accounts = bank();
    let target = td["properties"]["on"]["forms"][0]["href"].as_str().unwrap();
    let access = access(&owner, target, true, Arc::new(AtomicBool::new(false)));
    let prepared = secure_prepare(&td, &keys, access.clone(), &accounts, None).unwrap();
    let (bindings, funding) = prepared.into_execution();
    let definition = bindings[0].definition().clone();
    let content = definition.content().clone();
    let denied_service = ConsumerServiceOwner::retain_access(
        Arc::new(Access(Arc::new(AtomicBool::new(false)))),
        &accounts,
    )
    .unwrap();
    assert!(
        definition
            .clone()
            .bind(denied_service, accounts.clone(), &mut work())
            .is_err(),
        "NoSec-only service cannot bind Basic meaning"
    );
    let registration = keys.server(verifier());
    let address = target
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let provider = HttpFiniteProducer::try_https_basic(
        HttpProducerConfig::for_https_loopback(address).unwrap(),
        BindingId::new(197),
        BindingGeneration::INITIAL,
        1000,
        registration,
    )
    .unwrap();
    let supported = prepare_thing(
        &td.to_string(),
        authority(Generation::INITIAL).key(),
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[&provider],
        |_, _| true,
    )
    .unwrap();
    let program = supported.operations()[0]
        .definition::<HttpProducerProgram>()
        .unwrap()
        .clone();
    let wrong_service = keys.server(verifier());
    let config = HttpFiniteConfig::try_new(
        HttpProducerConfig::for_https_loopback(address).unwrap(),
        program.clone(),
        authority(Generation::INITIAL).key(),
        AffordanceTarget::Property(Arc::from("on")),
        1000,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert!(
        config.with_security(wrong_service).is_err(),
        "equal scheme tags cannot substitute the actual verifier"
    );
    use clinkz_wot_core::producer::ProducerExecutor;
    let legacy = producer_execution(
        HttpProducerConfig::for_loopback(address).unwrap(),
        BindingId::new(197),
        BindingGeneration::INITIAL,
        0,
    );
    assert!(
        legacy.check_program(&program).is_err(),
        "legacy cleartext executor must refuse a TLS program"
    );
    let mut changes = Vec::new();
    for scheme in [
        "nosec",
        "digest",
        "apikey",
        "bearer",
        "psk",
        "auto",
        "vendor:unknown",
    ] {
        let mut changed = td.clone();
        changed["securityDefinitions"]["auth"] = json!({"scheme":scheme});
        changes.push(changed);
    }
    for expression in [
        json!({"scheme":"combo","oneOf":["none","b"]}),
        json!({"scheme":"combo","allOf":["none","b"]}),
    ] {
        let mut changed = td.clone();
        changed["securityDefinitions"] =
            json!({"auth":expression,"none":{"scheme":"nosec"},"b":{"scheme":"basic"}});
        changes.push(changed);
    }
    for location in ["query", "body", "cookie", "uri"] {
        let mut changed = td.clone();
        changed["securityDefinitions"]["auth"]["in"] = json!(location);
        changes.push(changed);
    }
    for changed in [
        json!({"scheme":"basic","name":"X-Secret"}),
        json!({"scheme":"basic","proxy":"https://127.0.0.1/"}),
        json!({"scheme":"basic","extra":"unimplemented"}),
    ] {
        let mut change = td.clone();
        change["securityDefinitions"]["auth"] = changed;
        changes.push(change);
    }
    let mut simultaneous = td.clone();
    simultaneous["securityDefinitions"]["another"] = json!({"scheme":"basic"});
    simultaneous["security"] = json!(["auth", "another"]);
    changes.push(simultaneous);
    let mut empty = td.clone();
    empty["properties"]["on"]["forms"][0]["security"] = json!([]);
    changes.push(empty);
    let mut scope = td.clone();
    scope["properties"]["on"]["forms"][0]["scopes"] = json!(["admin"]);
    changes.push(scope);
    let mut clear = td.clone();
    clear["properties"]["on"]["forms"][0]["href"] = json!(target.replacen("https:", "http:", 1));
    changes.push(clear);
    let mut write = td.clone();
    write["properties"]["on"]["readOnly"] = json!(false);
    changes.push(write);
    for changed in changes {
        // A Consumer may keep a supported Read from a mixed operation TD, but
        // complete Producer qualification must refuse an advertised Write.
        if changed["properties"]["on"]["readOnly"] != false {
            assert!(
                secure_prepare(&changed, &keys, access.clone(), &accounts, Some(&content)).is_err()
            );
        }
        assert!(
            prepare_thing(
                &changed.to_string(),
                authority(Generation::INITIAL).key(),
                GatewayDefaultV1::LIMITS,
                &mut work(),
                &[&provider],
                |_, _| true
            )
            .is_err()
        );
    }
    // Form override replaces inherited NoSec, rather than combining or ignoring it.
    let mut override_td = td.clone();
    override_td["securityDefinitions"]["none"] = json!({"scheme":"nosec"});
    override_td["security"] = json!(["none"]);
    override_td["properties"]["on"]["forms"][0]["security"] = json!(["auth"]);
    let overridden =
        secure_prepare(&override_td, &keys, access, &accounts, Some(&content)).unwrap();
    assert_eq!(overridden.content_constructions(), 0);
    drop((overridden, bindings, funding, content, definition));
    clean(&accounts);
}

#[test]
fn missing_credentials_and_wrong_instance_refuse_before_network_and_rotation_fences_unread_result()
{
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let verify = verifier();
        let mut server = SecureFixture::new(&keys, verify.clone());
        let owner = consumer_instance();
        let accounts = bank();
        let target = server.td["properties"]["on"]["forms"][0]["href"]
            .as_str()
            .unwrap()
            .to_owned();
        let missing = access(&owner, &target, false, Arc::new(AtomicBool::new(false)));
        let (client, content) =
            secure_client(&server.td, &keys, &owner, missing.clone(), &accounts, None);
        assert!(
            client
                .read_property("on", Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        assert_eq!(verify.calls.load(Ordering::Relaxed), 0);
        drop(client);
        missing.replace_credentials(Some(credentials())).unwrap();
        let (client, _) = secure_client(
            &server.td,
            &keys,
            &owner,
            missing.clone(),
            &accounts,
            Some(&content),
        );
        let mut call = read_secure(&client);
        until(&mut server, &mut call, RawFiniteState::Delivering).await;
        // Let the native operation complete without taking the Consumer result.
        let start = Instant::now();
        loop {
            server.step();
            client.step(&mut cx(), now(0), &mut work());
            if server.server().take_settlement().is_some() {
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        missing.replace_credentials(None).unwrap();
        let result = loop {
            server.step();
            if let Poll::Ready(r) = call.poll(&mut cx(), now(0), &mut work()) {
                break r;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        assert!(result.result().is_err());
        drop((result, call, client));
        let replacement = ConsumerInstance::new(ThingSlotId::new(
            SlotIndex::new(0),
            Generation::new(2).unwrap(),
        ));
        missing.replace_credentials(Some(credentials())).unwrap();
        let (wrong, _) = secure_client(
            &server.td,
            &keys,
            &replacement,
            missing.clone(),
            &accounts,
            Some(&content),
        );
        assert!(
            wrong
                .read_property("on", Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        drop(wrong);
        let fresh = access(
            &replacement,
            &target,
            true,
            Arc::new(AtomicBool::new(false)),
        );
        let (client, _) = secure_client(
            &server.td,
            &keys,
            &replacement,
            fresh,
            &accounts,
            Some(&content),
        );
        let mut call = read_secure(&client);
        let result = server.exchange(&mut call).await;
        assert!(result.result().is_ok());
        drop((result, call, client, owner, replacement, missing, content));
        clean(&accounts);
        server.close();
    });
}

#[test]
fn changed_verifier_application_policy_and_instance_authority_fence_original_delivery() {
    io().block_on(async {
        for case in 0..9 {
            let keys = Keys::for_name("127.0.0.1");
            let verify = verifier();
            let mut server = SecureFixture::new(&keys, verify.clone());
            let owner = consumer_instance();
            let accounts = bank();
            let access = access(
                &owner,
                server.td["properties"]["on"]["forms"][0]["href"]
                    .as_str()
                    .unwrap(),
                true,
                Arc::new(AtomicBool::new(false)),
            );
            let (client, content) =
                secure_client(&server.td, &keys, &owner, access, &accounts, None);
            let mut call = read_secure(&client);
            let accepted = case != 0;
            until(
                &mut server,
                &mut call,
                if accepted {
                    RawFiniteState::Delivering
                } else {
                    RawFiniteState::Dispatching
                },
            )
            .await;
            match case {
                0 | 1 => verify.revoked.store(true, Ordering::Release),
                2 => server.app.denied.set(true),
                3 => {
                    assert!(server.owner.revoke(server.exposure.key()));
                }
                4 => {
                    let successor = RawProducerInstance::new(
                        authority(Generation::new(2).unwrap()),
                        vec![RawFiniteBehavior::new(
                            AffordanceTarget::Property(Arc::from("on")),
                            Operation::ReadProperty,
                            SecureBehavior(Rc::new(SecureApp::default())),
                        )],
                    );
                    server
                        .owner
                        .replace(server.exposure.key(), successor)
                        .unwrap_or_else(|e| panic!("{:?}", e.error()));
                }
                5 => server.exposure.begin_close(),
                6 => verify.replacement_identity.store(true, Ordering::Release),
                7 => verify.changed_work.store(true, Ordering::Release),
                8 => *server.app.revoke_in_permission.borrow_mut() = Some(verify.clone()),
                _ => unreachable!(),
            }
            let result = server.exchange(&mut call).await;
            assert!(result.result().is_err());
            assert_eq!(server.app.reads.get(), usize::from(accepted));
            let settlement = server.server().take_settlement().unwrap();
            if accepted {
                assert!(settlement.result.is_ok(), "retain original late output");
                assert_ne!(settlement.delivery, FiniteDelivery::Delivered);
            } else {
                assert_eq!(settlement.effect, FiniteEffect::NotInvoked);
            }
            drop((settlement, result, call, client, content, owner));
            clean(&accounts);
            server.close();
        }
    });
}

fn independent_request(
    addr: SocketAddr,
    config: Arc<rustls::ClientConfig>,
    authorization: Option<&str>,
) -> std::thread::JoinHandle<Vec<u8>> {
    let authorization = authorization.map(str::to_owned);
    std::thread::spawn(move || {
        let socket = TcpStream::connect(addr).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let connection = rustls::ClientConnection::new(
            config,
            rustls::pki_types::ServerName::IpAddress(addr.ip().into()),
        )
        .unwrap();
        let mut stream = rustls::StreamOwned::new(connection, socket);
        let mut request = format!("GET /on HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n");
        if let Some(header) = authorization {
            request.push_str("Authorization: ");
            request.push_str(&header);
            request.push_str("\r\n");
        }
        request.push_str("\r\n");
        stream.write_all(request.as_bytes()).unwrap();
        stream.flush().unwrap();
        let mut result = Vec::new();
        match stream.read_to_end(&mut result) {
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => (),
            Err(error) => panic!("independent TLS receive: {error:?}"),
        }
        result
    })
}

#[test]
fn independent_tls_peer_observes_401_403_and_verified_success_without_credential_diagnostics() {
    io().block_on(async {
        use base64::Engine;
        let keys = Keys::for_name("127.0.0.1");
        let valid = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode([USER, b":", PASSWORD].concat())
        );
        for case in 0..4 {
            let verify = verifier();
            let mut server = SecureFixture::new(&keys, verify);
            server.app.denied.set(case == 2);
            let peer = independent_request(
                server.addr,
                keys.independent_client(),
                match case {
                    0 => None,
                    1 => Some("Basic ZGV2aWNlOndyb25n"),
                    _ => Some(&valid),
                },
            );
            let start = Instant::now();
            let settlement = loop {
                server.step();
                if let Some(s) = server.server().take_settlement() {
                    break s;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            let bytes = peer.join().unwrap();
            let expected = match case {
                0 | 1 => 401,
                2 => 403,
                _ => 200,
            };
            assert!(
                std::str::from_utf8(&bytes)
                    .unwrap()
                    .starts_with(&format!("HTTP/1.1 {expected}"))
            );
            if expected == 401 {
                assert!(
                    std::str::from_utf8(&bytes)
                        .unwrap()
                        .to_lowercase()
                        .contains("www-authenticate: basic realm=\"wot\"")
                );
            }
            assert!(!bytes.windows(PASSWORD.len()).any(|w| w == PASSWORD));
            let diagnostic = format!(
                "{settlement:?} {:?} {:?}",
                credentials(),
                AuthMaterial::Basic {
                    username: "private-user".into(),
                    password: "private-password".into()
                }
            );
            for secret in [
                std::str::from_utf8(PASSWORD).unwrap(),
                "private-user",
                "private-password",
                "ZGV2aWNlOndyb25n",
            ] {
                assert!(!diagnostic.contains(secret));
            }
            assert_eq!(server.app.reads.get(), usize::from(case == 3));
            drop(settlement);
            server.close();
        }
    });
}

#[test]
fn destination_trust_failures_never_reach_verifier_or_behavior() {
    io().block_on(async {
        for case in 0..3 {
            let keys = if case == 1 {
                Keys::for_name("127.0.0.2")
            } else if case == 2 {
                let mut params = rcgen::CertificateParams::new(vec!["127.0.0.1".into()]).unwrap();
                params.not_before = rcgen::date_time_ymd(2000, 1, 1);
                params.not_after = rcgen::date_time_ymd(2001, 1, 1);
                let key = rcgen::KeyPair::generate().unwrap();
                let cert = params.self_signed(&key).unwrap();
                Keys {
                    cert: cert.der().clone(),
                    key: PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
                }
            } else {
                Keys::for_name("127.0.0.1")
            };
            let wrong_roots = Keys::for_name("127.0.0.1");
            let verify = verifier();
            let mut server = SecureFixture::new(&keys, verify.clone());
            let owner = consumer_instance();
            let accounts = bank();
            let access = access(
                &owner,
                server.td["properties"]["on"]["forms"][0]["href"]
                    .as_str()
                    .unwrap(),
                true,
                Arc::new(AtomicBool::new(false)),
            );
            let (client, content) = secure_client(
                &server.td,
                if case == 0 { &wrong_roots } else { &keys },
                &owner,
                access,
                &accounts,
                None,
            );
            let mut call = read_secure(&client);
            let result = server.exchange(&mut call).await;
            assert_eq!(
                result.result().unwrap_err().security_reason(),
                Some(SecurityFailureReason::ProviderFailure)
            );
            assert_eq!(verify.calls.load(Ordering::Relaxed), 0);
            assert_eq!(server.app.reads.get(), 0);
            drop((result, call, client, content, owner));
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn cancellation_policy_change_and_zero_verification_credit_retain_original_owners_until_cleanup() {
    io().block_on(async {
        for case in 0..5 {
            let keys = Keys::for_name("127.0.0.1");
            let verify = verifier();
            let mut server = SecureFixture::new(&keys, verify.clone());
            let owner = consumer_instance();
            let accounts = bank();
            let deny = Arc::new(AtomicBool::new(false));
            let access = access(
                &owner,
                server.td["properties"]["on"]["forms"][0]["href"]
                    .as_str()
                    .unwrap(),
                true,
                deny.clone(),
            );
            let (client, content) =
                secure_client(&server.td, &keys, &owner, access.clone(), &accounts, None);
            let mut call = read_secure(&client);
            if case != 0 {
                until(
                    &mut server,
                    &mut call,
                    if case == 1 {
                        RawFiniteState::Dispatching
                    } else {
                        RawFiniteState::Delivering
                    },
                )
                .await;
            }
            if case == 1 {
                let mut no_verification = work().with_remaining(WorkClass::HandlerSteps, 0);
                let _ = server
                    .owner
                    .step(
                        server.exposure.participant_mut::<SecureServer>(0).unwrap(),
                        &mut cx(),
                        &mut no_verification,
                        now(0),
                    )
                    .unwrap();
                assert_eq!(verify.calls.load(Ordering::Relaxed), 0);
                assert_eq!(server.app.reads.get(), 0);
            }
            match case {
                0..=2 => call.cancel(),
                3 => deny.store(true, Ordering::Release),
                4 => access
                    .replace_credentials(Some(
                        HttpBasicCredentials::try_new(USER, b"replacement-password").unwrap(),
                    ))
                    .unwrap(),
                _ => unreachable!(),
            }
            let result = server.exchange(&mut call).await;
            assert!(result.result().is_err());
            drop((call, client, owner, access, content));
            assert!(
                accounts
                    .account(ResourceKind::EngineLiveBytesGlobalMax)
                    .used()
                    > 0,
                "unread result retains original funding"
            );
            drop(result);
            clean(&accounts);
            server.close();
        }
    });
}
