//! Application composition, not an implicit Servient discovery operation.
//! cargo run -p clinkz-wot-discovery --example discover_read -- \
//!   COLLECTION_URL PINNED_SOCKET THING_ID PROPERTY ALLOWED_INTERACTION_URL [ROOT.der]
//! The selected interaction is the current NoSec cleartext-IP Property Read
//! subset; retrieval can independently use verified TLS. No credentials are
//! accepted through process arguments. Other Engine registrations can be composed
//! by the application without changing Discovery.
use clinkz_wot_binding_http::{HttpConsumerConfig, try_consumer_registration};
use clinkz_wot_core::{consumer::*, host_accounts::HostResourceAccounts, *};
use clinkz_wot_discovery::http::*;
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::{ConsumerRegistry, ConsumerSelector};
use clinkz_wot_servient_finite::consumer::{ConsumerAssociation, ConsumerCalls};
use std::{
    sync::Arc,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, 1_000_000))
}
/// Explicit deployment decision, supplied separately from all Directory data.
struct Permission {
    target: String,
    instance: ConsumerInstanceRef,
    binding: BindingId,
}
impl ConsumerAccessService for Permission {
    fn authorize(
        &self,
        binding: BindingRegistrationIdentity,
        target: &str,
        op: Operation,
    ) -> CoreResult<()> {
        if binding.binding_id() == self.binding
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
    fn authorize_instance(
        &self,
        instance: &ConsumerInstanceRef,
        binding: BindingRegistrationIdentity,
        target: &str,
        op: Operation,
    ) -> CoreResult<()> {
        if !instance.is_current() || instance != &self.instance {
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
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(
        (6..=7).contains(&args.len()),
        "COLLECTION_URL PINNED_SOCKET THING_ID PROPERTY ALLOWED_INTERACTION_URL [ROOT.der]"
    );
    let transport = if let Some(root) = args.get(6) {
        DiscoveryTransport::tls(vec![std::fs::read(root).unwrap().into()]).unwrap()
    } else {
        DiscoveryTransport::cleartext_loopback()
    };
    let endpoint =
        TrustedDiscoveryEndpoint::new(&args[1], args[2].parse().unwrap(), transport).unwrap();
    let io = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    io.block_on(async {
        let mut session =
            HttpDiscoverySession::directory(endpoint, None, DiscoveryLimits::default()).unwrap();
        let cancel = DiscoveryCancellation::default();
        let mut selected = None;
        while let Some(page) = session.next_page(&cancel).await.unwrap() {
            for candidate in page {
                if candidate
                    .thing()
                    .id
                    .as_ref()
                    .is_some_and(|id| id.as_str() == args[3])
                {
                    selected = Some(candidate);
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
        }
        session.stop();
        let selected = selected.expect("explicitly selected Thing was not discovered");
        let accounts = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let instance =
            ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL));
        let binding = BindingId::new(610);
        let mut registration = try_consumer_registration(
            HttpConsumerConfig::for_cleartext_ip().unwrap(),
            binding,
            BindingGeneration::INITIAL,
            610,
        )
        .unwrap();
        registration.services.access = Arc::new(Permission {
            target: args[5].clone(),
            instance: instance.reference(),
            binding,
        });
        let registry = ConsumerRegistry::try_new(vec![registration], &accounts)
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let prepared = registry
            .prepare(
                selected.thing(),
                GatewayDefaultV1::LIMITS,
                accounts.clone(),
                ConsumerSelector::default(),
                &mut work(),
                |job| {
                    while !job.is_finished() {
                        job.step(&mut work(), false);
                    }
                },
            )
            .unwrap();
        let (definitions, funding) = prepared.into_execution();
        drop(selected);
        drop(registry);
        drop(session);
        let runtime = ConsumerAssociation::try_new(
            instance.reference(),
            definitions,
            funding,
            accounts.clone(),
        )
        .map(|association| {
            (
                association,
                ConsumerCalls::try_new(accounts.clone(), 1).unwrap(),
            )
        })
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let clock = Instant::now();
        let now = || MonotonicInstant::new(ClockId::new(610), clock.elapsed().as_millis() as u64);
        let mut call = runtime
            .0
            .read_property(
                &runtime.1,
                &args[4],
                Deadline::at(now().checked_add_ticks(10_000).unwrap()),
                now(),
                &mut work(),
            )
            .unwrap();
        let result = loop {
            if let Poll::Ready(result) =
                call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            {
                break result;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        println!(
            "{:?}",
            result
                .result()
                .map(|o| o.data().map(|p| String::from_utf8_lossy(p.body)))
        );
        drop(result);
        drop(call);
        drop(runtime);
        drop(instance);
        assert!(
            ResourceKind::ALL
                .into_iter()
                .all(|k| accounts.account(k).used() == 0)
        );
    });
}
