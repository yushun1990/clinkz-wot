include!("collection.rs");
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

const USER: &[u8] = b"device";
const PASSWORD: &[u8] = b"fixture-secret-not-a-principal";
type VerificationHook = (usize, Box<dyn FnOnce()>);
thread_local! { static VERIFICATION_HOOK: RefCell<Option<VerificationHook>> = const { RefCell::new(None) }; }
fn verification_hook(call: usize) {
    let hook = VERIFICATION_HOOK.with(|pending| {
        let mut pending = pending.borrow_mut();
        if pending.as_ref().is_some_and(|(at, _)| *at == call) {
            pending.take()
        } else {
            None
        }
    });
    if let Some((_, hook)) = hook {
        hook();
    }
}
struct Verifier {
    revision: std::sync::atomic::AtomicU64,
    expected_password: &'static [u8],
    replacement_identity: AtomicBool,
    changed_work: AtomicBool,
    revoked: AtomicBool,
    calls: std::sync::atomic::AtomicUsize,
}
impl HttpBasicVerifier for Verifier {
    fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
    fn verify(&self, username: &[u8], password: &[u8]) -> CoreResult<Principal> {
        let call = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        verification_hook(call);
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
            scopes: vec!["property-access".into()],
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
    authority_in_permission: RefCell<Option<Box<dyn FnOnce()>>>,
    revoke_in_permission: RefCell<Option<Arc<Verifier>>>,
    denied: Cell<bool>,
    allow_anonymous: Cell<bool>,
    revision: Cell<u64>,
    reads: Cell<usize>,
    writes: Cell<usize>,
    value: RefCell<Vec<u8>>,
    write_fault: Cell<u8>,
    identity: RefCell<
        Option<(
            ProducerInstanceKey,
            Operation,
            (BindingId, BindingGeneration),
        )>,
    >,
    deny_after: Cell<bool>,
    principal: RefCell<Option<Principal>>,
}
impl SecureApp {
    fn deny(&self) {
        self.denied.set(true);
        self.revision
            .set(self.revision.get().checked_add(1).unwrap());
    }
}
struct SecureBehavior(Rc<SecureApp>);
impl RawFiniteHandler for SecureBehavior {
    fn authorization_revision(&self) -> u64 {
        self.0.revision.get()
    }
    fn authorize(&self, input: &InteractionInput) -> CoreResult<()> {
        let hook = self.0.authority_in_permission.borrow_mut().take();
        if let Some(hook) = hook {
            hook();
        }
        if let Some(verifier) = self.0.revoke_in_permission.borrow().as_ref() {
            verifier.revoked.store(true, Ordering::Release);
        }
        if self.0.denied.get()
            || input.principal.as_ref().is_none_or(|p| {
                !(self.0.allow_anonymous.get() && p == &Principal::anonymous())
                    && (p.id.as_str() != "verified-account" || p.scopes != ["property-access"])
            })
        {
            return Err(denied());
        }
        Ok(())
    }
    fn handle(
        &self,
        ctx: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        *self.0.principal.borrow_mut() = input.principal.clone();
        *self.0.identity.borrow_mut() = Some((
            ctx.producer_instance().unwrap(),
            ctx.operation(),
            ctx.binding().unwrap(),
        ));
        if self.0.deny_after.get() {
            self.0.deny();
        }
        if ctx.operation() == Operation::WriteProperty {
            self.0.writes.set(self.0.writes.get() + 1);
            *self.0.value.borrow_mut() = input.data.as_ref().unwrap().body.to_vec();
            match self.0.write_fault.get() {
                1 => Err(denied()), // failure after actual effects
                2 => Ok(InteractionOutput::with_data(payload(b"17"))),
                _ => Ok(InteractionOutput::empty()),
            }
        } else {
            self.0.reads.set(self.0.reads.get() + 1);
            let value = self.0.value.borrow();
            Ok(InteractionOutput::with_data(payload(if value.is_empty() {
                b"17"
            } else {
                &value
            })))
        }
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
        Self::from_td(keys, verifier, addr, td, &[Operation::ReadProperty])
    }
    fn from_td(
        keys: &Keys,
        verifier: Arc<Verifier>,
        addr: SocketAddr,
        td: Value,
        operations: &[Operation],
    ) -> Self {
        Self::from_security(keys.server(verifier), addr, td, operations)
    }
    fn from_security(
        security: HttpServerSecurity,
        addr: SocketAddr,
        td: Value,
        operations: &[Operation],
    ) -> Self {
        let accounts = bank();
        let mut owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
            instances: 1,
            behaviors_per_instance: operations.len(),
            target_name_bytes: 256,
        })
        .unwrap();
        let app = Rc::new(SecureApp::default());
        let authority = authority(Generation::INITIAL);
        let key = authority.key();
        let instance = RawProducerInstance::new(
            authority,
            operations
                .iter()
                .map(|&operation| {
                    RawFiniteBehavior::new(
                        AffordanceTarget::Property(Arc::from("on")),
                        operation,
                        SecureBehavior(app.clone()),
                    )
                })
                .collect(),
        )
        .with_description(Arc::from(td.to_string()));
        let listen = HttpProducerConfig::for_https_loopback(addr).unwrap();
        let provider: Rc<dyn ProducerCapability> = Rc::new(
            HttpFiniteProducer::try_https(
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
        let mappings = prepared
            .operations()
            .iter()
            .map(|op| {
                (
                    AffordanceTarget::Property(Arc::from("on")),
                    op.definition::<HttpProducerProgram>().unwrap().clone(),
                )
            })
            .collect();
        let config = HttpFiniteConfig::try_from_mappings(listen, mappings, key, 1000)
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
        revision: std::sync::atomic::AtomicU64::new(0),
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
) -> Arc<HttpClientAccess> {
    Arc::new(
        HttpClientAccess::try_new(
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
    access: Arc<HttpClientAccess>,
    accounts: &Arc<HostResourceAccounts>,
    reuse: Option<&Arc<PreparedConsumerContent>>,
) -> CoreResult<PreparedConsumer> {
    let registration = try_https_consumer_registration(
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
    access: Arc<HttpClientAccess>,
    accounts: &Arc<HostResourceAccounts>,
    reuse: Option<&Arc<PreparedConsumerContent>>,
) -> ((StandaloneConsumerAssociation, ConsumerCalls), Arc<PreparedConsumerContent>) {
    let prepared = secure_prepare(td, keys, access, accounts, reuse).unwrap();
    if reuse.is_some() {
        assert_eq!(prepared.content_constructions(), 0);
    }
    let (bindings, funding) = prepared.into_execution();
    let content = bindings[0].definition().content().clone();
    let runtime =
        StandaloneConsumerAssociation::try_new(owner.reference(), bindings, funding, accounts.clone()).map(|association| (association, ConsumerCalls::try_new(accounts.clone(), 1).unwrap()))
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
    (runtime, content)
}
fn read_secure(client: &(StandaloneConsumerAssociation, ConsumerCalls)) -> ConsumerCall {
    client.0.read_property(&client.1, "on", Deadline::NONE, now(0), &mut work())
        .unwrap()
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
