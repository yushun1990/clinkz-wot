//! cargo run -p clinkz-wot-binding-http --example read_property -- TD.json PROPERTY
use clinkz_wot_binding_http::{HttpConsumerConfig, try_consumer_registration};
use clinkz_wot_core::{host_accounts::HostResourceAccounts, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::{ConsumerRegistry, ConsumerSelector};
use clinkz_wot_servient_finite::consumer::ConsumerRuntime;
use std::{
    sync::Arc,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000))
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "provide TD.json and property name");
    let thing: clinkz_wot_td::thing::Thing =
        serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let registration = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(600),
        BindingGeneration::INITIAL,
        600,
    )
    .unwrap();
    let registry =
        ConsumerRegistry::try_new(vec![registration]).unwrap_or_else(|e| panic!("{:?}", e.error()));
    let prepared = registry
        .prepare(
            &thing,
            &GatewayDefaultV1::LIMITS,
            bank.clone(),
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
    drop(thing);
    drop(registry);
    let runtime = ConsumerRuntime::try_new(definitions, funding, bank.clone(), 1)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let clock = Instant::now();
    let now = || MonotonicInstant::new(ClockId::new(600), clock.elapsed().as_millis() as u64);
    let io = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    io.block_on(async {
        let mut call = runtime
            .read_property(&args[2], Deadline::NONE, now(), &mut work())
            .unwrap();
        let result = loop {
            if let Poll::Ready(result) =
                call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            {
                break result;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        match result.result() {
            Ok(output) => println!("{}", String::from_utf8_lossy(&output.data().unwrap().body)),
            Err(error) => eprintln!("{error:?}"),
        }
    });
    drop(runtime);
    assert!(
        ResourceKind::ALL
            .iter()
            .all(|&kind| bank.account(kind).used() == 0)
    );
}
