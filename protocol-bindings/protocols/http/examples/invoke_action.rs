//! cargo run -p clinkz-wot-binding-http --example invoke_action -- TD.json run 12 [--cancel]
//! Use the qualified TD printed by serve_action. Controls come only from Forms.
use clinkz_wot_binding_http::{HttpActionConsumer, HttpConsumerConfig};
use clinkz_wot_core::{consumer::*, host_accounts::HostResourceAccounts, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::consumer_action::*;
use std::{
    rc::Rc,
    sync::Arc,
    task::{Context, Waker},
    time::{Duration, Instant},
};
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000))
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(
        (4..=5).contains(&args.len()),
        "provide TD.json, Action name, JSON input (- for no input), and optional --cancel"
    );
    let thing = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    let bank = Arc::new(HostResourceAccounts::new(&limits));
    let config = HttpConsumerConfig::for_cleartext_ip().unwrap();
    let services = config.consumer_services();
    let services = RetainedConsumerServices {
        data: ConsumerServiceOwner::retain_data(services.data, &bank)
            .ok()
            .unwrap(),
        access: ConsumerServiceOwner::retain_access(services.access, &bank)
            .ok()
            .unwrap(),
    };
    let registry = ConsumerActionRegistry::try_new_fixed(vec![ConsumerActionRegistration {
        preparation: Box::new(HttpActionConsumer::new(
            config,
            BindingId::new(601),
            BindingGeneration::INITIAL,
            601,
        )),
        services: services.clone(),
    }])
    .unwrap();
    let definition = registry
        .prepare_action(
            &thing,
            &args[2],
            ConsumerActionSelectors::default(),
            &limits,
            bank.clone(),
            &mut work(),
        )
        .unwrap();
    drop(thing);
    drop(registry);
    let consumer = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL));
    let access = ConsumerActionAccess {
        invoke: services.access.clone(),
        query: definition
            .input()
            .query
            .as_ref()
            .map(|_| services.access.clone()),
        cancel: definition
            .input()
            .cancel
            .as_ref()
            .map(|_| services.access.clone()),
    };
    let association = ConsumerActionAssociation::try_new(
        consumer.reference(),
        Rc::new(definition),
        access,
        bank.clone(),
        &mut work(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    drop(services);
    let runtime = ConsumerActions::try_new(1, bank.clone()).unwrap();
    let clock = Instant::now();
    let now = || MonotonicInstant::new(ClockId::new(601), clock.elapsed().as_millis() as u64);
    let io = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    io.block_on(async {
        let input = (args[3] != "-").then(|| Payload::new(args[3].as_bytes(), "application/json"));
        let handle = runtime
            .invoke_action(
                &association,
                input,
                Deadline::at(now().checked_add_ticks(10_000).unwrap()),
                now(),
                &mut work(),
            )
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let mut cancel_sent = false;
        loop {
            match handle
                .progress(&mut Context::from_waker(Waker::noop()), now(), &mut work())
                .unwrap()
            {
                ConsumerActionState::Pending => {
                    if args.get(4).is_some_and(|arg| arg == "--cancel") && !cancel_sent {
                        handle.request_cancel(now(), &mut work()).unwrap();
                        cancel_sent = true;
                    } else {
                        handle.query(now(), &mut work()).unwrap();
                    }
                }
                ConsumerActionState::Completed
                | ConsumerActionState::Failed
                | ConsumerActionState::Closed => break,
                _ => {}
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let settlement = handle.take_settlement(now(), &mut work()).unwrap();
        eprintln!(
            "accepted={}, cancellation={:?}, effect={:?}",
            settlement.accepted(),
            settlement.remote_cancellation(),
            settlement.effect()
        );
        match settlement.result() {
            Ok(output) => println!(
                "{}",
                output
                    .data()
                    .map_or_else(|| "completed".into(), |p| String::from_utf8_lossy(p.body))
            ),
            Err(error) => eprintln!("{error:?}"),
        }
    });
    drop(runtime);
    drop(association);
    drop(consumer);
    assert!(
        ResourceKind::ALL
            .into_iter()
            .all(|kind| bank.account(kind).used() == 0)
    );
}
