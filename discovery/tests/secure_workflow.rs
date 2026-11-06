#![cfg(feature = "http")]
//! Exercise the runnable application's public API composition. Neither peer
//! uses a ClinkZ Directory or Producer implementation.
#[path = "../examples/secure_discover_read.rs"]
#[allow(dead_code)]
mod application;
mod support;
use application::*;
use clinkz_wot_binding_http::{
    HttpBasicClientAccess, HttpBasicCredentials, HttpClientTls,
    try_https_basic_consumer_registration,
};
use clinkz_wot_core::{consumer::*, host_accounts::HostResourceAccounts, *};
use clinkz_wot_discovery::{DiscoveryError, http::*};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::ConsumerRegistrationInput;
use clinkz_wot_servient_finite::consumer::{ConsumerCall, ConsumerNativeOutcome, ConsumerRuntime};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Waker},
    time::Duration,
};
use support::*;
use tokio::net::TcpListener;

const ID: &str = "urn:secure:selected";
const BODY: &str = r#"{"temperature":23,"label":"office"}"#;
const DIRECTORY_USER: &str = "directory-reader";
const DIRECTORY_PASSWORD: &str = "directory-only";
const THING_USER: &str = "sensor-reader";
const THING_PASSWORD: &str = "thing-only";

fn description(target: &str, other: &str) -> Value {
    let property = json!({"type":"object", "readOnly":true,
        "required":["temperature","label"],
        "properties": {
            "temperature":{"type":"integer","minimum":0,"maximum":50},
            "label":{"type":"string","minLength":1,"maxLength":8}
        },
        "forms":[{"href":other, "op":"readproperty"},
                 {"href":target, "op":"readproperty"}]});
    json!({"@context":"https://www.w3.org/2022/wot/td/v1.1",
        "id":ID,"title":"secure sensor", "security":"basic",
        "securityDefinitions":{"basic":{"scheme":"basic"}},
        // A second, supported Property tests exact application selection after
        // whole-description preparation. It must never replace `level`.
        "properties":{"level":property.clone(), "mirror":property},
        "links":[{"href":other,"rel":"describedby","type":"application/td+json"}]})
}
fn credentials(user: &str, password: &str) -> HttpBasicCredentials {
    HttpBasicCredentials::try_new(user.as_bytes(), password.as_bytes()).unwrap()
}
async fn thing_peer(keys: &Keys, response: String) -> Peer {
    Peer::start(
        vec![Step::new("/level", response).authenticated(THING_USER, THING_PASSWORD)],
        Some(keys),
    )
    .await
}
fn session(
    peer: &Peer,
    trust: DiscoveryTransport,
    user: &str,
    password: &str,
) -> HttpDiscoverySession {
    let endpoint = peer.endpoint("/things", trust);
    let credentials =
        DiscoveryCredentials::basic(&endpoint, user.as_bytes(), password.as_bytes()).unwrap();
    HttpDiscoverySession::directory(endpoint, Some(credentials), limits()).unwrap()
}
async fn discover(description: Value, keys: &Keys) -> DiscoveredThing {
    let mut decoy = description.clone();
    decoy["id"] = json!("urn:secure:decoy");
    decoy["properties"] = json!({});
    let directory = Peer::start(
        vec![
            Step::new(
                "/things?limit=2",
                reply(
                    &json!([decoy, description]).to_string(),
                    "application/ld+json",
                    "",
                ),
            )
            .authenticated(DIRECTORY_USER, DIRECTORY_PASSWORD),
        ],
        Some(keys),
    )
    .await;
    let mut retrieval = session(
        &directory,
        keys.transport(),
        DIRECTORY_USER,
        DIRECTORY_PASSWORD,
    );
    let page = retrieval
        .next_page(&DiscoveryCancellation::default())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(page.len(), 2);
    let selected = page
        .into_iter()
        .find(|t| t.thing().id.as_ref().unwrap().as_str() == ID)
        .unwrap();
    retrieval.stop();
    drop(retrieval);
    assert_credentials(&directory, DIRECTORY_USER, DIRECTORY_PASSWORD);
    directory.finish(1).await; // Directory socket/credentials retire before execution.
    selected
}
fn assert_credentials(peer: &Peer, user: &str, password: &str) {
    use base64::Engine;
    let expected = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}"))
    );
    let requests = peer.requests.lock().unwrap();
    assert_eq!(requests.len(), 1, "no replay or fallback");
    let headers: Vec<_> = requests[0]
        .lines()
        .filter_map(|l| l.split_once(':'))
        .filter(|(k, _)| k.eq_ignore_ascii_case("authorization"))
        .map(|(_, v)| v.trim())
        .collect();
    assert_eq!(headers, [expected.as_str()]);
}
fn clean(bank: &Arc<HostResourceAccounts>) {
    for kind in ResourceKind::ALL {
        assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
    }
}
async fn untouched(listener: &TcpListener) {
    assert!(
        tokio::time::timeout(Duration::from_millis(20), listener.accept())
            .await
            .is_err(),
        "unselected destination contacted"
    );
}
struct App {
    instance: ConsumerInstance,
    bank: Arc<HostResourceAccounts>,
    policy: Arc<Permission>,
    access: Arc<HttpBasicClientAccess>,
    tls: HttpClientTls,
}
impl App {
    fn new(target: &str, keys: &Keys, credentials: Option<HttpBasicCredentials>) -> Self {
        let instance =
            ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL));
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let policy = Arc::new(Permission::new(target.into()));
        let access = Arc::new(
            HttpBasicClientAccess::try_new(
                instance.reference(),
                target,
                policy.clone(),
                credentials,
            )
            .unwrap(),
        );
        Self {
            instance,
            bank,
            policy,
            access,
            tls: HttpClientTls::from_roots(vec![keys.cert.clone()]).unwrap(),
        }
    }
    #[allow(clippy::result_large_err)]
    fn prepare(
        &self,
        thing: &DiscoveredThing,
        form: u32,
        instance: &ConsumerInstance,
    ) -> CoreResult<ConsumerRuntime> {
        prepare_read(
            thing,
            "level",
            form,
            self.registration()?,
            instance,
            &self.bank,
        )
    }
    #[allow(clippy::result_large_err)]
    fn registration(&self) -> CoreResult<ConsumerRegistrationInput> {
        try_https_basic_consumer_registration(
            self.tls.clone(),
            self.access.clone(),
            BINDING,
            BindingGeneration::INITIAL,
            771,
        )
    }
}
// Test-only observation of the real validator, to synchronize an unread-result
// race without relying on sleeps or replacing validation with a stub.
struct ObservedData {
    actual: Arc<dyn ConsumerDataService>,
    validations: Arc<AtomicUsize>,
}
impl ConsumerDataService for ObservedData {
    fn content_type(&self) -> &str {
        self.actual.content_type()
    }
    fn policy(&self) -> ConsumerDataPolicy {
        let mut policy = self.actual.policy();
        policy.retained_bytes += 64;
        policy
    }
    fn supports_schema(&self, schema: &data_schema::PreparedSchema) -> bool {
        self.actual.supports_schema(schema)
    }
    fn validate(&self, schema: &data_schema::PreparedSchema, payload: &Payload) -> CoreResult<()> {
        self.actual.validate(schema, payload)?;
        self.validations.fetch_add(1, Ordering::Release);
        Ok(())
    }
}
fn read(runtime: &ConsumerRuntime) -> ConsumerCall {
    runtime
        .read_property(
            "level",
            Deadline::at(now().checked_add_ticks(4_000).unwrap()),
            now(),
            &mut work(),
        )
        .unwrap()
}
async fn until_request(runtime: &ConsumerRuntime, peer: &Peer) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            runtime.step(&mut Context::from_waker(Waker::noop()), now(), &mut work());
            if !peer.requests.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn authenticated_directory_to_exact_basic_https_form_validates_structured_result() {
    let directory_keys = Keys::new("127.0.0.1");
    let thing_keys = Keys::new("127.0.0.1");
    assert_ne!(directory_keys.cert, thing_keys.cert);
    let thing = thing_peer(&thing_keys, reply(BODY, "application/json", "")).await;
    let sentinel = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let other = format!("https://{}/unselected", sentinel.local_addr().unwrap());
    let target = thing.url("/level", true);
    let selected = discover(description(&target, &other), &directory_keys).await;
    assert!(thing.requests.lock().unwrap().is_empty());
    untouched(&sentinel).await;
    let app = App::new(
        &target,
        &thing_keys,
        Some(credentials(THING_USER, THING_PASSWORD)),
    );
    let runtime = app.prepare(&selected, 1, &app.instance).unwrap();
    drop(selected); // No Discovery session/source/compiler is needed to execute.
    assert!(
        runtime
            .read_property("mirror", Deadline::NONE, now(), &mut work())
            .is_err()
    );
    let mut call = read(&runtime);
    let result = settle(&mut call).await;
    assert_eq!(
        result.result().unwrap().data().unwrap().body,
        BODY.as_bytes()
    );
    assert_eq!(result.consumer(), &app.instance.reference());
    assert_eq!(result.operation(), Operation::ReadProperty);
    assert_eq!(result.binding().binding_id(), BINDING);
    assert_credentials(&thing, THING_USER, THING_PASSWORD);
    let bank = app.bank.clone();
    let access_owner = Arc::downgrade(&app.access);
    let policy_owner = Arc::downgrade(&app.policy);
    drop((call, runtime, app));
    assert!(
        ResourceKind::ALL
            .into_iter()
            .any(|k| bank.account(k).used() > 0),
        "settlement owns original funding"
    );
    drop(result);
    clean(&bank);
    assert!(access_owner.upgrade().is_none());
    assert!(policy_owner.upgrade().is_none());
    thing.finish(1).await;
    untouched(&sentinel).await;
}

#[tokio::test]
async fn explicit_form_never_falls_back_or_uses_an_unrelated_supported_property() {
    let directory_keys = Keys::new("127.0.0.1");
    let keys = Keys::new("127.0.0.1");
    let sentinel = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = format!("https://{}/level", sentinel.local_addr().unwrap());
    for case in 0..9 {
        let mut td = description(&target, &target);
        // Form zero and mirror remain executable alternatives at the allowed
        // destination. Explicit Form one must fail on its complete requirement.
        match case {
            0 => {
                td["securityDefinitions"]["other"] = json!({"scheme":"bearer"});
                td["properties"]["level"]["forms"][1]["security"] = json!("other");
            }
            1 => {
                td["securityDefinitions"]["other"] = json!({"scheme":"nosec"});
                td["properties"]["level"]["forms"][1]["security"] = json!(["basic", "other"]);
            }
            2 => td["properties"]["level"]["forms"][1]["scopes"] = json!(["read"]),
            3 => td["properties"]["level"]["forms"][1]["contentCoding"] = json!("gzip"),
            4 => td["properties"]["level"]["forms"][1]["contentType"] = json!("application/cbor"),
            5 => {
                td["properties"]["level"]["forms"][1]["href"] =
                    json!(target.replacen("https:", "http:", 1))
            }
            6 => td["properties"]["level"]["properties"]["label"]["pattern"] = json!("^[a-z]+$"),
            7 => {
                td["@context"] = json!([
                    "https://www.w3.org/2022/wot/td/v1.1",
                    format!("{target}/untrusted-context")
                ])
            }
            8 => td["properties"]["level"]["forms"][1]["op"] = json!("writeproperty"),
            _ => unreachable!(),
        }
        let selected = discover(td, &directory_keys).await;
        let app = App::new(
            &target,
            &keys,
            Some(credentials(THING_USER, THING_PASSWORD)),
        );
        assert!(
            app.prepare(&selected, 1, &app.instance).is_err(),
            "case {case}"
        );
        clean(&app.bank);
    }
    // An absent Form cannot default to an existing one either.
    let selected = discover(description(&target, &target), &directory_keys).await;
    let app = App::new(
        &target,
        &keys,
        Some(credentials(THING_USER, THING_PASSWORD)),
    );
    assert!(app.prepare(&selected, 9, &app.instance).is_err());
    clean(&app.bank);
    untouched(&sentinel).await;
}

#[tokio::test]
async fn destination_mismatch_never_rewrites_the_td_to_the_allowed_url() {
    let directory_keys = Keys::new("127.0.0.1");
    let keys = Keys::new("127.0.0.1");
    let allowed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let denied = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = format!("https://{}/level", allowed.local_addr().unwrap());
    for wrong in [
        format!("{target}/different-path"),
        format!("https://{}/level", denied.local_addr().unwrap()),
    ] {
        let mut td = description(&target, &target);
        td["properties"]["level"]["forms"][1]["href"] = json!(wrong);
        let selected = discover(td, &directory_keys).await;
        let app = App::new(
            &target,
            &keys,
            Some(credentials(THING_USER, THING_PASSWORD)),
        );
        assert!(app.prepare(&selected, 1, &app.instance).is_err());
        clean(&app.bank);
    }
    untouched(&allowed).await;
    untouched(&denied).await;
}

#[tokio::test]
async fn discovery_does_not_supply_missing_credentials_policy_or_instance_authority() {
    let directory_keys = Keys::new("127.0.0.1");
    let keys = Keys::new("127.0.0.1");
    let sentinel = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = format!("https://{}/level", sentinel.local_addr().unwrap());
    let selected = discover(description(&target, &target), &directory_keys).await;
    for case in 0..4 {
        let app = App::new(
            &target,
            &keys,
            (case != 0).then(|| credentials(THING_USER, THING_PASSWORD)),
        );
        // Same slot/generation is still a different authority allocation.
        let impostor =
            ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL));
        let runtime = app
            .prepare(
                &selected,
                1,
                if case == 3 { &impostor } else { &app.instance },
            )
            .unwrap();
        if case == 1 {
            app.policy.allowed.store(false, Ordering::Release);
        }
        if case == 2 {
            app.instance.revoke();
        }
        assert!(
            runtime
                .read_property("level", Deadline::NONE, now(), &mut work())
                .is_err(),
            "case {case}"
        );
        drop(runtime);
        clean(&app.bank);
    }
    untouched(&sentinel).await;
}

#[tokio::test]
async fn tls_domains_are_independent_in_both_directions() {
    let directory_keys = Keys::new("127.0.0.1");
    let thing_keys = Keys::new("127.0.0.1");
    let thing = thing_peer(&thing_keys, reply(BODY, "application/json", "")).await;
    let target = thing.url("/level", true);
    let td = description(&target, &target);
    let selected = discover(td.clone(), &directory_keys).await;
    let app = App::new(
        &target,
        &directory_keys,
        Some(credentials(THING_USER, THING_PASSWORD)),
    );
    let runtime = app.prepare(&selected, 1, &app.instance).unwrap();
    let mut call = read(&runtime);
    let result = settle(&mut call).await;
    assert!(result.result().is_err());
    assert!(
        thing.requests.lock().unwrap().is_empty(),
        "failed trust must precede credentials"
    );
    drop((result, call, runtime));
    clean(&app.bank);
    thing.finish(1).await;
    let directory = Peer::start(
        vec![
            Step::new(
                "/things?limit=2",
                reply(&json!([td]).to_string(), "application/ld+json", ""),
            )
            .authenticated(DIRECTORY_USER, DIRECTORY_PASSWORD),
        ],
        Some(&directory_keys),
    )
    .await;
    let mut retrieval = session(
        &directory,
        thing_keys.transport(),
        DIRECTORY_USER,
        DIRECTORY_PASSWORD,
    );
    assert_eq!(
        retrieval
            .next_page(&DiscoveryCancellation::default())
            .await
            .unwrap_err(),
        DiscoveryError::Trust
    );
    assert!(directory.requests.lock().unwrap().is_empty());
    directory.finish(1).await;
}

#[tokio::test]
async fn wrong_credentials_forbidden_and_redirect_responses_do_not_retry_or_navigate() {
    let directory_keys = Keys::new("127.0.0.1");
    let keys = Keys::new("127.0.0.1");
    let sentinel = TcpListener::bind("127.0.0.1:0").await.unwrap();
    for case in 0..3 {
        let response = match case {
            0 => reply(BODY, "application/json", ""), // peer returns 401 for Directory credentials
            1 => "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n".into(),
            _ => format!(
                "HTTP/1.1 302 Found\r\nLocation: https://{}/stolen\r\nContent-Length: 0\r\n\r\n",
                sentinel.local_addr().unwrap()
            ),
        };
        let thing = thing_peer(&keys, response).await;
        let target = thing.url("/level", true);
        let selected = discover(description(&target, &target), &directory_keys).await;
        let (user, password) = if case == 0 {
            (DIRECTORY_USER, DIRECTORY_PASSWORD)
        } else {
            (THING_USER, THING_PASSWORD)
        };
        let app = App::new(&target, &keys, Some(credentials(user, password)));
        let runtime = app.prepare(&selected, 1, &app.instance).unwrap();
        let mut call = read(&runtime);
        let result = settle(&mut call).await;
        assert!(result.result().is_err(), "case {case}");
        assert_credentials(&thing, user, password);
        drop((result, call, runtime));
        clean(&app.bank);
        thing.finish(1).await;
    }
    untouched(&sentinel).await;
}

#[tokio::test]
async fn authenticated_success_still_requires_selected_dataschema_validation() {
    let directory_keys = Keys::new("127.0.0.1");
    let keys = Keys::new("127.0.0.1");
    for body in [
        r#"{"temperature":51,"label":"office"}"#,
        r#"{"temperature":23}"#,
        r#"{"temperature":23,"label":"too-long-label"}"#,
        r#"{"temperature":"23","label":"office"}"#,
    ] {
        let thing = thing_peer(&keys, reply(body, "application/json", "")).await;
        let target = thing.url("/level", true);
        let selected = discover(description(&target, &target), &directory_keys).await;
        let app = App::new(
            &target,
            &keys,
            Some(credentials(THING_USER, THING_PASSWORD)),
        );
        let runtime = app.prepare(&selected, 1, &app.instance).unwrap();
        let mut call = read(&runtime);
        let result = settle(&mut call).await;
        assert!(result.result().is_err(), "{body}");
        assert_credentials(&thing, THING_USER, THING_PASSWORD);
        drop((result, call, runtime));
        clean(&app.bank);
        thing.finish(1).await;
    }
}

#[tokio::test]
async fn in_flight_cancellation_revocation_rotation_and_abandonment_release_the_tls_socket() {
    let directory_keys = Keys::new("127.0.0.1");
    let keys = Keys::new("127.0.0.1");
    for case in 0..5 {
        let thing = thing_peer(
            &keys,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\n\r\n{"
                .into(),
        )
        .await;
        let target = thing.url("/level", true);
        let selected = discover(description(&target, &target), &directory_keys).await;
        let app = App::new(
            &target,
            &keys,
            Some(credentials(THING_USER, THING_PASSWORD)),
        );
        let runtime = app.prepare(&selected, 1, &app.instance).unwrap();
        let mut call = read(&runtime);
        until_request(&runtime, &thing).await;
        match case {
            0 => call.cancel(),
            1 => app.policy.allowed.store(false, Ordering::Release),
            2 => app.instance.revoke(),
            3 => app.access.replace_credentials(None).unwrap(),
            4 => {} // Drop the accepted call and its runtime with native I/O pending.
            _ => unreachable!(),
        }
        if case != 4 {
            let result = settle(&mut call).await;
            assert!(result.result().is_err(), "case {case}");
            assert_eq!(result.consumer(), &app.instance.reference());
            drop(result);
        }
        drop((call, runtime));
        clean(&app.bank);
        assert_credentials(&thing, THING_USER, THING_PASSWORD);
        thing.finish(1).await;
    }
}

#[tokio::test]
async fn validated_unread_success_is_fenced_by_changed_policy_or_original_instance() {
    let directory_keys = Keys::new("127.0.0.1");
    let keys = Keys::new("127.0.0.1");
    for case in 0..3 {
        let thing = thing_peer(&keys, reply(BODY, "application/json", "")).await;
        let target = thing.url("/level", true);
        let selected = discover(description(&target, &target), &directory_keys).await;
        let app = App::new(
            &target,
            &keys,
            Some(credentials(THING_USER, THING_PASSWORD)),
        );
        let validations = Arc::new(AtomicUsize::new(0));
        let mut registration = app.registration().unwrap();
        registration.services.data = Arc::new(ObservedData {
            actual: registration.services.data,
            validations: validations.clone(),
        });
        let runtime = prepare_read(
            &selected,
            "level",
            1,
            registration,
            &app.instance,
            &app.bank,
        )
        .unwrap();
        let mut call = read(&runtime);
        // Drive native completion/validation without handing the result to the application.
        tokio::time::timeout(Duration::from_secs(3), async {
            while validations.load(Ordering::Acquire) == 0 {
                runtime.step(&mut Context::from_waker(Waker::noop()), now(), &mut work());
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        match case {
            0 => app.policy.allowed.store(false, Ordering::Release),
            1 => {
                app.policy.revision.fetch_add(1, Ordering::AcqRel);
            }
            _ => app.instance.revoke(),
        }
        let result = settle(&mut call).await;
        assert!(result.result().is_err());
        assert!(
            matches!(
                result.native_outcome(),
                ConsumerNativeOutcome::Returned(Ok(_))
            ),
            "retain the actual late native success"
        );
        let bank = app.bank.clone();
        drop((call, runtime, app));
        assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
        drop(result);
        clean(&bank);
        thing.finish(1).await;
    }
}

#[tokio::test]
async fn authenticated_directory_cancellation_or_bad_td_cannot_create_execution_authority() {
    let keys = Keys::new("127.0.0.1");
    for abandon in [false, true] {
        let directory = Peer::start(vec![Step::new("/things?limit=2", "HTTP/1.1 200 OK\r\nContent-Type: application/ld+json\r\nContent-Length: 1000\r\n\r\n[".into())
            .authenticated(DIRECTORY_USER, DIRECTORY_PASSWORD)], Some(&keys)).await;
        let mut retrieval = session(
            &directory,
            keys.transport(),
            DIRECTORY_USER,
            DIRECTORY_PASSWORD,
        );
        let cancel = DiscoveryCancellation::default();
        {
            let mut pending = Box::pin(retrieval.next_page(&cancel));
            tokio::select! { r = &mut pending => panic!("incomplete page escaped: {r:?}"), _ = directory.wait_request() => {} }
            if !abandon {
                cancel.cancel();
                assert_eq!(pending.await.unwrap_err(), DiscoveryError::Cancelled);
            }
        }
        assert_eq!(
            retrieval.error(),
            Some(if abandon {
                DiscoveryError::Interrupted
            } else {
                DiscoveryError::Cancelled
            })
        );
        assert!(retrieval.next_page(&cancel).await.unwrap().is_none());
        directory.finish(1).await;
    }
    let directory = Peer::start(
        vec![
            Step::new(
                "/things?limit=2",
                reply(
                    r#"[{"id":"urn:bad","title":"missing required TD meaning"}]"#,
                    "application/ld+json",
                    "",
                ),
            )
            .authenticated(DIRECTORY_USER, DIRECTORY_PASSWORD),
        ],
        Some(&keys),
    )
    .await;
    let mut retrieval = session(
        &directory,
        keys.transport(),
        DIRECTORY_USER,
        DIRECTORY_PASSWORD,
    );
    assert_eq!(
        retrieval
            .next_page(&DiscoveryCancellation::default())
            .await
            .unwrap_err(),
        DiscoveryError::InvalidThingDescription
    );
    directory.finish(1).await;
}
