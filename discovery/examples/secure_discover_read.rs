//! Explicit application composition of authenticated Discovery and Thing access.
//! See docs/evidence/secure-discovery-consumer.md for invocation and limitations.
//! This example is also exercised by tests/secure_workflow.rs against real peers.
use clinkz_wot_binding_http::{
    HttpBasicCredentials, HttpClientAccess, HttpClientTls, try_https_consumer_registration,
};
use clinkz_wot_core::{consumer::*, host_accounts::HostResourceAccounts, *};
use clinkz_wot_discovery::http::*;
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::{
    ConsumerRegistrationInput, ConsumerRegistry, ConsumerSelector,
};
use clinkz_wot_servient_finite::consumer::{ConsumerCall, ConsumerRuntime, ConsumerSettlement};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

pub(crate) const BINDING: BindingId = BindingId::new(771);
pub(crate) fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, 1_000_000))
}
pub(crate) fn now() -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(771), 0)
}

/// Provisioned application policy: never derived from the Directory's TD.
pub(crate) struct Permission {
    pub target: String,
    pub allowed: AtomicBool,
    pub revision: AtomicU64,
}
impl Permission {
    pub fn new(target: String) -> Self {
        Self {
            target,
            allowed: AtomicBool::new(true),
            revision: AtomicU64::new(0),
        }
    }
}
impl ConsumerAccessService for Permission {
    fn authorize(
        &self,
        binding: BindingRegistrationIdentity,
        target: &str,
        op: Operation,
    ) -> CoreResult<()> {
        if self.allowed.load(Ordering::Acquire)
            && binding.binding_id() == BINDING
            && target == self.target
            && op == Operation::ReadProperty
        {
            Ok(())
        } else {
            Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Validate, RetryClass::Never),
            })
        }
    }
    fn security_revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
    fn authorize_instance(
        &self,
        instance: &ConsumerInstanceRef,
        binding: BindingRegistrationIdentity,
        target: &str,
        op: Operation,
    ) -> CoreResult<()> {
        // HttpClientAccess also checks exact original-instance identity.
        if !instance.is_current() {
            return Err(CoreError::Cancelled(ErrorContext::new(
                ErrorPhase::Validate,
                RetryClass::Never,
            )));
        }
        self.authorize(binding, target, op)
    }
    fn retained_bytes(&self) -> u64 {
        1024
    }
    fn work(&self) -> u64 {
        4096
    }
}

/// Application selection, using only public preparation and execution APIs.
/// The selected TD is passed intact: no synthetic replacement TD or href rewrite.
#[allow(clippy::result_large_err)]
pub(crate) fn prepare_read(
    thing: &DiscoveredThing,
    property: &str,
    form: u32,
    registration: ConsumerRegistrationInput,
    instance: &ConsumerInstance,
    accounts: &Arc<HostResourceAccounts>,
) -> CoreResult<ConsumerRuntime> {
    let registry = ConsumerRegistry::try_new(vec![registration], accounts)
        .map_err(|e| e.error().error().clone())?;
    let prepared = registry.prepare(
        thing.thing(),
        GatewayDefaultV1::LIMITS,
        accounts.clone(),
        ConsumerSelector {
            binding: Some(BINDING),
            form: Some(form),
        },
        &mut work(),
        |job| {
            while !job.is_finished() {
                job.step(&mut work(), false);
            }
        },
    )?;
    let (bindings, funding) = prepared.into_execution();
    // The finite registry prepares a whole description. Select the exact
    // Property/operation from its owned bindings; an unrelated supported
    // Property does not prove that the requested one is supported.
    let selected = bindings
        .into_iter()
        .find(|b| {
            matches!(b.affordance(), AffordanceTarget::Property(name) if name.as_ref() == property)
                && b.operation() == Operation::ReadProperty
                && b.form() == form
        })
        .ok_or_else(|| {
            CoreError::UnsupportedOperation(ErrorContext::new(
                ErrorPhase::Prepare,
                RetryClass::Never,
            ))
        })?;
    drop(registry);
    ConsumerRuntime::try_new(
        instance.reference(),
        vec![selected],
        funding,
        accounts.clone(),
        1,
    )
    .map_err(|e| e.error().error().clone())
}

pub(crate) async fn settle(call: &mut ConsumerCall) -> ConsumerSettlement {
    tokio::time::timeout(Duration::from_secs(5), async {
        let start = Instant::now();
        loop {
            let instant =
                MonotonicInstant::new(ClockId::new(771), start.elapsed().as_millis() as u64);
            if let Poll::Ready(result) = call.poll(
                &mut Context::from_waker(Waker::noop()),
                instant,
                &mut work(),
            ) {
                return result;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("bounded interaction did not settle")
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(
        args.len(),
        9,
        "DIRECTORY_URL PINNED_SOCKET THING_ID PROPERTY FORM_INDEX ALLOWED_THING_URL DIRECTORY_ROOT.der THING_ROOT.der"
    );
    // Four separately provisioned values, no password arguments or shared
    // credential fallback. Production applications supply their own secret store.
    let directory_user = zeroize::Zeroizing::new(std::env::var("CLINKZ_DIRECTORY_USER").unwrap());
    let directory_password =
        zeroize::Zeroizing::new(std::env::var("CLINKZ_DIRECTORY_PASSWORD").unwrap());
    let thing_user = zeroize::Zeroizing::new(std::env::var("CLINKZ_THING_USER").unwrap());
    let thing_password = zeroize::Zeroizing::new(std::env::var("CLINKZ_THING_PASSWORD").unwrap());
    let endpoint = TrustedDiscoveryEndpoint::new(
        &args[1],
        args[2].parse().unwrap(),
        DiscoveryTransport::tls(vec![std::fs::read(&args[7]).unwrap().into()]).unwrap(),
    )
    .unwrap();
    let credentials = DiscoveryCredentials::basic(
        &endpoint,
        directory_user.as_bytes(),
        directory_password.as_bytes(),
    )
    .unwrap();
    drop((directory_user, directory_password));
    let io = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    io.block_on(async {
        let mut directory = HttpDiscoverySession::directory(
            endpoint,
            Some(credentials),
            DiscoveryLimits::default(),
        )
        .unwrap();
        let cancel = DiscoveryCancellation::default();
        let mut selected = None;
        while let Some(page) = directory.next_page(&cancel).await.unwrap() {
            selected = page.into_iter().find(|t| {
                t.thing()
                    .id
                    .as_ref()
                    .is_some_and(|id| id.as_str() == args[3])
            });
            if selected.is_some() {
                break;
            }
        }
        directory.stop();
        drop(directory);
        let selected = selected.expect("requested Thing was not discovered");
        let instance =
            ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL));
        let accounts = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let policy = Arc::new(Permission::new(args[6].clone()));
        let access = Arc::new(
            HttpClientAccess::try_new(
                instance.reference(),
                &args[6],
                policy,
                Some(
                    HttpBasicCredentials::try_new(thing_user.as_bytes(), thing_password.as_bytes())
                        .unwrap(),
                ),
            )
            .unwrap(),
        );
        drop((thing_user, thing_password));
        let registration = try_https_consumer_registration(
            HttpClientTls::from_roots(vec![std::fs::read(&args[8]).unwrap().into()]).unwrap(),
            access,
            BINDING,
            BindingGeneration::INITIAL,
            771,
        )
        .unwrap();
        let runtime = prepare_read(
            &selected,
            &args[4],
            args[5].parse().unwrap(),
            registration,
            &instance,
            &accounts,
        )
        .unwrap();
        drop(selected);
        let mut call = runtime
            .read_property(
                &args[4],
                Deadline::at(now().checked_add_ticks(4_000).unwrap()),
                now(),
                &mut work(),
            )
            .unwrap();
        let result = settle(&mut call).await;
        let output = result.result().expect("Thing read refused");
        println!(
            "{}",
            std::str::from_utf8(output.data().unwrap().body).unwrap()
        );
        drop((result, call, runtime, instance));
        assert!(
            ResourceKind::ALL
                .into_iter()
                .all(|k| accounts.account(k).used() == 0)
        );
    });
}
