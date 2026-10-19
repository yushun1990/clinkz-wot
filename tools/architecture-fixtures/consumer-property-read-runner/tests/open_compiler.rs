//! No engine test hooks: external compiler, complete registration, real public
//! consume/read/shutdown roots, and the binding's finite loopback transport.
use clinkz_wot_consumer_property_read_binding_fixture::{Clock, StaticClient, Stats, native};
use clinkz_wot_consumer_property_read_runtime_runner as app;
use clinkz_wot_core::*;
use std::sync::{Arc, atomic::Ordering};

fn source(verb: &str, prefix: &str, namespace: &str) -> clinkz_wot_td::thing::Thing {
    let target = "a".repeat(96);
    serde_json::from_value(serde_json::json!({
        "@context": ["https://www.w3.org/2022/wot/td/v1.1", {prefix: namespace}],
        "title":"external native compiler", "id":"urn:external:native", "base":"wire://peer/",
        "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
        "properties":{
            "a-read":{"forms":[
                {"href":"wire://peer/skip", "op":[]},
                {"href":"wire://peer/first", format!("{prefix}:verb"):"READ"},
                {"href":"wire://peer/selected", format!("{prefix}:verb"):"FETCH"}]},
            "c-温度":{"forms":[
                {"href":"wire://peer/write", "op":["writeproperty"]},
                {"href":target, format!("{prefix}:verb"):verb}]}
        }
    }))
    .unwrap()
}
fn outputs() -> [Option<InteractionOutput>; 8] {
    core::array::from_fn(|_| {
        Some(InteractionOutput::with_data(Payload::new(
            &b"17"[..],
            "application/json",
        )))
    })
}
fn request(stats: &Stats) -> String {
    let n = stats.target_len.load(Ordering::SeqCst);
    String::from_utf8(
        stats.target[..n]
            .iter()
            .map(|v| v.load(Ordering::SeqCst))
            .collect(),
    )
    .unwrap()
}
#[test]
fn external_native_lowering_executes_after_source_destruction_in_static_runtime() {
    for (verb, prefix) in [("FETCH", "x"), ("READ", "other")] {
        let stats = Arc::new(Stats::default());
        let registration = native::static_registration(StaticClient::new(stats.clone(), outputs()));
        assert!(registration.support().artifact_layout().size() > 64);
        assert_eq!(
            app::application_registered(
                source(verb, prefix, "urn:external:wire:"),
                registration,
                &Clock::default()
            )
            .unwrap(),
            [599, 599]
        );
        assert_eq!(
            request(&stats),
            format!("{verb} wire://peer/{}", "a".repeat(96))
        );
        assert_eq!(stats.starts.load(Ordering::SeqCst), 2);
        assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 2);
        assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    }
}
#[test]
#[cfg(feature = "std")]
fn external_native_lowering_executes_after_source_destruction_in_host_runtime() {
    use clinkz_wot_consumer_property_read_binding_fixture::HostClient;
    for (verb, prefix) in [("FETCH", "x"), ("READ", "other")] {
        let stats = Arc::new(Stats::default());
        let registration = native::host_registration(HostClient::new(stats.clone(), outputs()));
        assert!(registration.support().host_slot_layout().unwrap().size() > 64);
        assert_eq!(
            app::host_application_registered(
                source(verb, prefix, "urn:external:wire:"),
                registration,
                Arc::new(Clock::default())
            )
            .unwrap(),
            [599, 599]
        );
        assert_eq!(
            request(&stats),
            format!("{verb} wire://peer/{}", "a".repeat(96))
        );
        assert_eq!(stats.starts.load(Ordering::SeqCst), 2);
        assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 2);
        assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn later_native_rejection_rolls_back_earlier_artifacts_without_invocation() {
    use clinkz_wot_foundation::*;
    use clinkz_wot_servient::consumer_property_read::*;
    use std::{
        cell::{Cell, RefCell},
        task::{Context, Waker},
    };
    for (verb, namespace) in [("UNKNOWN", "urn:external:wire:"), ("READ", "urn:wrong:")] {
        let stats = Arc::new(Stats::default());
        let registration = native::static_registration(StaticClient::new(stats.clone(), outputs()));
        let policy = app::policy(GatewayDefaultV1::LIMITS.clone());
        let parent = RefCell::new(policy.parent_accounts());
        let mut report = (
            Cell::new((None, [0; 4])),
            Cell::new(0),
            RefCell::new(ResourceAccount::new(
                SlotIndex::new(50),
                Generation::INITIAL,
                ResourceKind::CleanupBytesMax,
                4096,
            )),
        );
        let clock = Clock::default();
        let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
            registration,
            policy,
            &clock,
            &parent,
            &mut report,
        )
        .unwrap_or_else(|_| panic!("runtime"));
        let thing = source(verb, "x", namespace);
        let error = runtime
            .consume(&thing, &mut app::work(100000), |b| {
                for _ in 0..100000 {
                    // Repeated unpaid retries cannot enter the external compiler.
                    let polls = stats.compiler_polls.load(Ordering::SeqCst);
                    b.step(&mut app::work(0));
                    assert_eq!(stats.compiler_polls.load(Ordering::SeqCst), polls);
                    b.step(&mut app::work(4096));
                    if b.is_finished() {
                        return;
                    }
                }
                panic!("failed build stalled");
            })
            .err()
            .expect("unsupported native instruction");
        assert!(matches!(error, CoreError::UnsupportedOperation(_)));
        drop(thing);
        assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
        assert_eq!(
            stats.compilations.load(Ordering::SeqCst),
            if namespace == "urn:external:wire:" {
                2
            } else {
                0
            }
        );
        assert_eq!(stats.compiler_aborts.load(Ordering::SeqCst), 1);
        runtime.close();
        let mut cx = Context::from_waker(Waker::noop());
        let mut terminal = false;
        for _ in 0..100000 {
            if let StartStatus::Ready(outcome) = runtime.shutdown(&mut cx, &mut app::work(4096)) {
                assert!(outcome.is_complete());
                terminal = true;
                break;
            }
        }
        assert!(terminal);
        drop(runtime);
        assert!(observer().0.unwrap().is_complete());
        drop(observer);
        assert!(parent.borrow().iter().all(|a| a.used() == 0));
    }
}

#[cfg(feature = "std")]
#[test]
fn host_later_native_rejection_keeps_the_original_cleanup_root() {
    use clinkz_wot_consumer_property_read_binding_fixture::HostClient;
    use clinkz_wot_foundation::*;
    use clinkz_wot_servient::consumer_property_read::*;
    use std::{
        sync::Mutex,
        task::{Context, Waker},
    };
    let stats = Arc::new(Stats::default());
    let registration = native::host_registration(HostClient::new(stats.clone(), outputs()));
    let policy = app::policy(GatewayDefaultV1::LIMITS.clone());
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        Arc::new(Clock::default()),
        parent.clone(),
    )
    .unwrap_or_else(|_| panic!("runtime"));
    let thing = source("UNKNOWN", "x", "urn:external:wire:");
    let error = runtime
        .consume(&thing, &mut app::work(100000), |b| {
            for _ in 0..100000 {
                let polls = stats.compiler_polls.load(Ordering::SeqCst);
                b.step(&mut app::work(0));
                assert_eq!(stats.compiler_polls.load(Ordering::SeqCst), polls);
                b.step(&mut app::work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!("failed build stalled");
        })
        .err()
        .expect("unsupported native instruction");
    assert!(matches!(error, CoreError::UnsupportedOperation(_)));
    drop(thing);
    assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
    assert_eq!(stats.compilations.load(Ordering::SeqCst), 2);
    assert_eq!(stats.compiler_aborts.load(Ordering::SeqCst), 1);
    runtime.close();
    let mut cx = Context::from_waker(Waker::noop());
    let mut terminal = false;
    for _ in 0..100000 {
        if let StartStatus::Ready(outcome) = runtime.shutdown(&mut cx, &mut app::work(4096)) {
            assert!(outcome.is_complete());
            terminal = true;
            break;
        }
    }
    assert!(terminal);
    drop(runtime);
    assert!(observer().0.unwrap().is_complete());
    drop(observer);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
}
