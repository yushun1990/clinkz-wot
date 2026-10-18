//! Run: cargo run -p clinkz-wot-binding-http --example read_property -- TD.json PROPERTY
use clinkz_wot_binding_http::{HttpConsumerConfig, try_consumer_registration};
use clinkz_wot_core::*;
use clinkz_wot_foundation::*;
use clinkz_wot_servient::consumer_property_read::*;
use std::{
    num::NonZeroU64,
    sync::{Arc, Mutex},
    task::{Context, Waker},
    time::{Duration, Instant},
};
struct Clock(Instant);
impl RuntimeClock for Clock {
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(ClockId::new(600), self.0.elapsed().as_millis() as u64)
    }
    fn ticks_per_second(&self) -> NonZeroU64 {
        NonZeroU64::new(1000).unwrap()
    }
}
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000))
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "provide TD.json and property name");
    // Typed application input, not bounded remote TD ingestion.
    let thing: clinkz_wot_td::thing::Thing =
        serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let io_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    io_runtime.block_on(async {
        let policy = ConsumerPropertyReadPolicy::try_new_owned(
            BindingResourceContract::ConsumerOwnedHostV2,
            ResourceProfileId::GATEWAY_DEFAULT_V1,
            GatewayDefaultV1::LIMITS.clone(),
            BindingLifetimeFootprint::new(8, 8192),
            BindingLifetimeFootprint::new(8, 4096),
            [1, 64, 64, 64],
            [4096, 16],
            [2, 2, 1, 1, 1, 1, 1],
            [30_000, 1000],
            65536,
        )
        .unwrap();
        let parent = Arc::new(Mutex::new(policy.parent_accounts()));
        let registration = try_consumer_registration(
            HttpConsumerConfig::for_cleartext_ip().unwrap(),
            BindingId::new(600),
            BindingGeneration::INITIAL,
            600,
        )
        .unwrap();
        let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
            registration,
            policy,
            Arc::new(Clock(Instant::now())),
            parent.clone(),
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let handle = match runtime
            .consume(&thing, &mut work(), |build| {
                for _ in 0..10000 {
                    build.step(&mut work());
                    if build.is_finished() {
                        return;
                    }
                }
                panic!("consume stalled");
            })
            .unwrap()
        {
            StartStatus::Ready(h) => h,
            _ => panic!("publication pending"),
        };
        drop(thing);
        let mut call = handle
            .read_property(&args[2], InteractionOptions::new(), &mut work())
            .unwrap();
        let result = loop {
            call.step(&mut Context::from_waker(Waker::noop()), &mut work());
            if let Some(result) = call.try_take(&mut work()).unwrap() {
                break result;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        match result {
            Ok(output) => println!("{}", String::from_utf8_lossy(&output.data().unwrap().body)),
            Err(error) => eprintln!("{error:?}"),
        }
        drop(call);
        drop(handle);
        loop {
            if let StartStatus::Ready(outcome) =
                runtime.shutdown(&mut Context::from_waker(Waker::noop()), &mut work())
            {
                assert!(outcome.is_complete());
                break;
            }
        }
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    });
}
