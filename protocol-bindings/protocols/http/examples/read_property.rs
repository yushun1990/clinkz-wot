//! cargo run -p clinkz-wot-binding-http --example read_property -- TD.json PROPERTY
use clinkz_wot_binding_http::{HttpConsumerConfig, try_consumer_registration};
use clinkz_wot_core::{host_accounts::HostResourceAccounts, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::{ConsumerRegistry, ConsumerSelector};
use clinkz_wot_servient_finite::consumer::{
    ConsumerCalls, ConsumerOwner, StandaloneConsumerAssociation,
};
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
    let registry = ConsumerRegistry::try_new(vec![registration], &bank)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
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
    drop(registry);
    let owner = ConsumerOwner::try_new(bank.clone(), 1, 1).unwrap();
    let registered = clinkz_wot_servient::consumer::register_thing(&owner, &thing, &mut work())
        .expect("ID-addressed registration requires a unique TD id");
    let thing_id = registered.thing_id().to_owned();
    drop((registered, thing));
    // This move-only startup input is transferred into the managed owner. It
    // is not an application-owned registered lifetime or a retained handle table.
    let standalone = StandaloneConsumerAssociation::try_new(
        owner.instance(&thing_id, &mut work()).unwrap().consumer(),
        definitions,
        funding,
        bank.clone(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    drop(
        owner
            .set_configuration(&thing_id, "default", standalone, &mut work())
            .unwrap_or_else(|e| panic!("{:?}", e.error())),
    );
    let calls = ConsumerCalls::try_new(bank.clone(), 1).unwrap();
    let clock = Instant::now();
    let now = || MonotonicInstant::new(ClockId::new(600), clock.elapsed().as_millis() as u64);
    let io = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    io.block_on(async {
        let mut call = owner
            .read_property(
                &thing_id,
                "default",
                &calls,
                &args[2],
                Deadline::NONE,
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
        match result.result() {
            Ok(output) => println!("{}", String::from_utf8_lossy(&output.data().unwrap().body)),
            Err(error) => eprintln!("{error:?}"),
        }
    });
    owner.remove_thing(&thing_id, &mut work()).unwrap();
    drop((owner, calls));
    assert!(
        ResourceKind::ALL
            .iter()
            .all(|&kind| bank.account(kind).used() == 0)
    );
}
