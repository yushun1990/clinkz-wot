#![cfg(feature = "http")]
mod support;
use clinkz_wot_binding_http::{HttpConsumerConfig, try_consumer_registration};
use clinkz_wot_core::{consumer::*, host_accounts::HostResourceAccounts, *};
use clinkz_wot_discovery::http::*;
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::{ConsumerRegistry, ConsumerSelector};
use clinkz_wot_servient_finite::consumer::{ConsumerAssociation, ConsumerCalls};
use serde_json::json;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
    time::Duration,
};
use support::*;
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, 1_000_000))
}
struct Permission {
    endpoint: String,
    instance: ConsumerInstanceRef,
    allowed: Arc<AtomicBool>,
}
impl ConsumerAccessService for Permission {
    fn authorize(
        &self,
        _: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        if self.allowed.load(Ordering::Acquire)
            && target == self.endpoint
            && operation == Operation::ReadProperty
        {
            Ok(())
        } else {
            Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Validate, RetryClass::Never),
            })
        }
    }
    fn authorize_instance(
        &self,
        instance: &ConsumerInstanceRef,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        if !instance.is_current() || instance != &self.instance {
            return Err(CoreError::Cancelled(ErrorContext::new(
                ErrorPhase::Validate,
                RetryClass::Never,
            )));
        }
        self.authorize(binding, target, operation)
    }
    fn retained_bytes(&self) -> u64 {
        1024
    }
    fn work(&self) -> u64 {
        4096
    }
}
#[allow(clippy::result_large_err)]
fn prepare(
    thing: &DiscoveredThing,
    target: &str,
    instance: &ConsumerInstance,
    allowed: Arc<AtomicBool>,
    bank: &Arc<HostResourceAccounts>,
) -> CoreResult<(ConsumerAssociation, ConsumerCalls)> {
    let mut registration = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(770),
        BindingGeneration::INITIAL,
        770,
    )
    .unwrap();
    registration.services.access = Arc::new(Permission {
        endpoint: target.into(),
        instance: instance.reference(),
        allowed,
    });
    let registry = ConsumerRegistry::try_new(vec![registration], bank)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let prepared = registry.prepare(
        thing.thing(),
        GatewayDefaultV1::LIMITS,
        bank.clone(),
        ConsumerSelector::default(),
        &mut work(),
        |job| {
            while !job.is_finished() {
                job.step(&mut work(), false);
            }
        },
    )?;
    let (definitions, funding) = prepared.into_execution();
    drop(registry);
    ConsumerAssociation::try_new(
        instance.reference(),
        definitions,
        funding,
        bank.clone(),
    )
    .map(|association| {
        (
            association,
            ConsumerCalls::try_new(bank.clone(), 1).unwrap(),
        )
    })
    .map_err(|e| e.error().error().clone())
}
fn now() -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(770), 0)
}
async fn fetch(
    peer: &Peer,
    trust: DiscoveryTransport,
    authenticated: bool,
) -> Vec<DiscoveredThing> {
    let endpoint = peer.endpoint("/things", trust);
    let credentials = authenticated
        .then(|| DiscoveryCredentials::basic(&endpoint, b"directory", b"secret").unwrap());
    let mut session = HttpDiscoverySession::directory(endpoint, credentials, limits()).unwrap();
    let page = session
        .next_page(&DiscoveryCancellation::default())
        .await
        .unwrap()
        .unwrap();
    session.stop();
    drop(session);
    page
}
#[tokio::test]
async fn application_selects_then_separately_authorizes_and_executes_engine_read() {
    let native = Peer::start(
        vec![Step::new("/level", reply("23", "application/json", ""))],
        None,
    )
    .await;
    let sentinel = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let forbidden = format!("http://{}/private", sentinel.local_addr().unwrap());
    let target = native.url("/level", false);
    let keys = Keys::new("127.0.0.1");
    let directory = Peer::start(
        vec![Step::new(
            "/things?limit=2",
            reply(
                &json!([
                    td("urn:a-hostile", &forbidden),
                    td("urn:b-selected", &target)
                ])
                .to_string(),
                "application/ld+json",
                "",
            ),
        )],
        Some(&keys),
    )
    .await;
    let discovered = fetch(&directory, keys.transport(), true).await;
    assert_eq!(discovered.len(), 2);
    assert!(
        directory.requests.lock().unwrap()[0]
            .contains("authorization: Basic ZGlyZWN0b3J5OnNlY3JldA==\r\n")
    );
    directory.finish(1).await;
    // Merely admitting a TD grants no networking permission to its Forms.
    assert!(
        tokio::time::timeout(Duration::from_millis(15), sentinel.accept())
            .await
            .is_err()
    );
    let selected = discovered
        .into_iter()
        .find(|t| t.thing().id.as_ref().unwrap().as_str() == "urn:b-selected")
        .unwrap();
    let accounts = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let instance = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL));
    let permission = Arc::new(AtomicBool::new(true));
    let runtime = prepare(&selected, &target, &instance, permission.clone(), &accounts).unwrap();
    drop(selected); // Execution retains no Directory TD/session/compiler.
    permission.store(false, Ordering::Release);
    assert!(
        runtime
            .0
            .read_property(&runtime.1, "level", Deadline::NONE, now(), &mut work())
            .is_err()
    );
    permission.store(true, Ordering::Release);
    let mut call = runtime
        .0
        .read_property(&runtime.1, "level", Deadline::NONE, now(), &mut work())
        .unwrap();
    let result = loop {
        if let Poll::Ready(result) =
            call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
        {
            break result;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    };
    assert_eq!(result.result().unwrap().data().unwrap().body, b"23");
    drop(call);
    drop(runtime);
    drop(instance);
    assert!(
        ResourceKind::ALL
            .into_iter()
            .any(|k| accounts.account(k).used() > 0),
        "result retains original account"
    );
    drop(result);
    assert!(
        ResourceKind::ALL
            .into_iter()
            .all(|k| accounts.account(k).used() == 0)
    );
    assert!(
        !native.requests.lock().unwrap()[0]
            .to_ascii_lowercase()
            .contains("authorization:")
    );
    native.finish(1).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(15), sentinel.accept())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn valid_admission_does_not_claim_unsupported_security_or_destination_permission() {
    let sentinel = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = format!("http://{}/level", sentinel.local_addr().unwrap());
    for mode in 0..3 {
        let mut description = td("urn:a", &target);
        if mode == 0 {
            description["securityDefinitions"]["n"] = json!({"scheme":"basic"});
        }
        if mode == 2 {
            description["@context"] = json!([
                "https://www.w3.org/2022/wot/td/v1.1",
                format!("http://{}/context", sentinel.local_addr().unwrap())
            ]);
            description["links"] =
                json!([{ "rel":"describedby", "type":"application/td+json", "href":target }]);
        }
        let directory = Peer::start(
            vec![Step::new(
                "/things?limit=2",
                reply(&json!([description]).to_string(), "application/json", ""),
            )],
            None,
        )
        .await;
        let selected = fetch(&directory, DiscoveryTransport::cleartext_loopback(), false)
            .await
            .pop()
            .unwrap();
        directory.finish(1).await;
        let accounts = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let instance =
            ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL));
        assert!(
            prepare(
                &selected,
                &target,
                &instance,
                Arc::new(AtomicBool::new(mode != 1)),
                &accounts
            )
            .is_err()
        );
        drop(instance);
        drop(selected);
        assert!(
            ResourceKind::ALL
                .into_iter()
                .all(|k| accounts.account(k).used() == 0)
        );
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(15), sentinel.accept())
            .await
            .is_err()
    );
}
