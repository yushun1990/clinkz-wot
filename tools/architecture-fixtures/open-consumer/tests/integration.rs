#![cfg(feature = "prepare")]
#[path = "support/composition.rs"]
mod composition;
use clinkz_wot_core::{consumer::ConsumerInstance, host_accounts::HostResourceAccounts, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::consumer::*;
use independent_open_consumer::{prepare::*, *};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, UdpSocket},
    sync::{Arc, atomic::Ordering},
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000))
}
fn now() -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(99), 1)
}
fn source(forms: Value) -> clinkz_wot_td::thing::Thing {
    serde_json::from_value(json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"anonymous downstream Thing", "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}}, "properties":{"a":{"type":"integer","readOnly":true,"forms":forms}}})).unwrap()
}
fn registry(inputs: Vec<ConsumerRegistrationInput>) -> ConsumerRegistry {
    ConsumerRegistry::try_new(inputs).unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn drive(job: &mut ConsumerCompilation<'_>) {
    for _ in 0..600 {
        // Both zero and one-short credits must preserve the exact continuation.
        job.step(&mut WorkBudget::new(), false);
        job.step(
            &mut WorkBudget::new().with_remaining(WorkClass::BindingPolls, 2),
            false,
        );
        job.step(
            &mut WorkBudget::new().with_remaining(WorkClass::BindingPolls, 3),
            false,
        );
        if job.is_finished() {
            return;
        }
    }
    panic!("job exceeded declared lifetime");
}
fn prepare(
    registry: &ConsumerRegistry,
    source: &clinkz_wot_td::thing::Thing,
    bank: Arc<HostResourceAccounts>,
) -> PreparedConsumer {
    registry
        .prepare(
            source,
            &GatewayDefaultV1::LIMITS,
            bank,
            ConsumerSelector::default(),
            &mut work(),
            drive,
        )
        .unwrap()
}
fn instance() -> ConsumerInstance {
    ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL))
}
struct Runtime {
    inner: ConsumerRuntime,
    _instance: ConsumerInstance,
}
impl core::ops::Deref for Runtime {
    type Target = ConsumerRuntime;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
fn runtime(prepared: PreparedConsumer, bank: Arc<HostResourceAccounts>) -> Runtime {
    let (defs, funding) = prepared.into_execution();
    let owner = instance();
    let inner = ConsumerRuntime::try_new(owner.reference(), defs, funding, bank, 1)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    Runtime {
        inner,
        _instance: owner,
    }
}
fn poll(call: &mut ConsumerCall) -> ConsumerSettlement {
    let start = Instant::now();
    loop {
        if let Poll::Ready(settlement) =
            call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
        {
            return settlement;
        }
        assert!(start.elapsed() < Duration::from_secs(5), "call stalled");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn clean(bank: &HostResourceAccounts) {
    let used: Vec<_> = ResourceKind::ALL
        .iter()
        .filter_map(|&k| {
            let n = bank.account(k).used();
            (n != 0).then_some((k, n))
        })
        .collect();
    assert!(used.is_empty(), "leaked {used:?}");
}
fn udp() -> (UdpSocket, String) {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let target = format!("linkedudp://{}/temperature", socket.local_addr().unwrap());
    (socket, target)
}
fn exchange(socket: &UdpSocket, bytes: &[u8]) {
    let mut request = [0u8; 256];
    let (n, from) = socket.recv_from(&mut request).unwrap();
    assert_eq!(&request[..n], b"READ /temperature\n");
    socket.send_to(bytes, from).unwrap();
}
#[test]
fn downstream_owned_incremental_and_http_use_one_public_selection_path_after_sources_die() {
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let probe = Arc::new(Probe::default());
    let (udp, target) = udp();
    let http = TcpListener::bind("127.0.0.1:0").unwrap();
    let http_target = format!("http://{}/b", http.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        for _ in 0..2 {
            exchange(&udp, b"7");
        }
    });
    let http_peer = std::thread::spawn(move || {
        let (mut stream, _) = http.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = [0; 2048];
        let n = stream.read(&mut bytes).unwrap();
        assert!(
            std::str::from_utf8(&bytes[..n])
                .unwrap()
                .starts_with("GET /b HTTP/1.1\r\n")
        );
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n9",
            )
            .unwrap();
        assert_eq!(
            stream.read(&mut bytes).unwrap(),
            0,
            "connection physically released"
        );
    });
    let mut value = serde_json::to_value(source(
        json!([{"href":"unknown:alternative"},{"href":target}]),
    ))
    .unwrap();
    value["properties"]["a"]["forms"][1]["op"] = json!(["readproperty", "observeproperty"]);
    value["properties"]["a"]["observable"] = json!(true);
    value["properties"]["b"] = json!({"type":"integer","forms":[{"href":http_target}]});
    let thing = serde_json::from_value(value).unwrap();
    let registry = registry(vec![
        registration(1, probe.clone()),
        clinkz_wot_binding_http::try_consumer_registration(
            clinkz_wot_binding_http::HttpConsumerConfig::for_cleartext_ip().unwrap(),
            BindingId::new(2),
            BindingGeneration::INITIAL,
            2,
        )
        .unwrap(),
    ]);
    let prepared = prepare(&registry, &thing, bank.clone());
    assert!(
        prepared
            .support()
            .iter()
            .any(|s| s.operation == Operation::ObserveProperty && s.binding.is_none())
    );
    assert!(
        prepared
            .support()
            .iter()
            .any(|s| s.form == 0 && s.binding.is_none())
    );
    assert_eq!(probe.starts.load(Ordering::Relaxed), 1);
    assert_eq!(probe.steps.load(Ordering::Relaxed), "temperature".len() + 1);
    assert_eq!(probe.native.load(Ordering::Relaxed), 0, "pure preparation");
    drop(thing);
    drop(registry);
    assert_eq!(probe.compiler_drops.load(Ordering::Relaxed), 1);
    let rt = runtime(prepared, bank.clone());
    let io = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    io.block_on(async {
        for name in ["a", "a", "b"] {
            let mut call = rt
                .read_property(name, Deadline::NONE, now(), &mut work())
                .unwrap();
            let start = Instant::now();
            let settlement = loop {
                if let Poll::Ready(s) =
                    call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
                {
                    break s;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            assert!(settlement.first_cause().is_none());
            assert!(
                matches!(
                    settlement.native_outcome(),
                    BindingCallSettlement::Returned(Ok(_))
                ),
                "{name}: {:?}",
                settlement.native_outcome()
            );
            drop(settlement);
        }
    });
    assert_eq!(probe.starts.load(Ordering::Relaxed), 1, "no warm lowering");
    assert_eq!(probe.validates.load(Ordering::Relaxed), 2);
    drop(rt);
    assert_eq!(probe.programs.load(Ordering::Relaxed), 1);
    clean(&bank);
    peer.join().unwrap();
    http_peer.join().unwrap();
    for &port in probe.ports.lock().unwrap().iter() {
        drop(UdpSocket::bind(port).expect("final physical datagram socket release"));
    }
}
#[test]
fn ambiguous_actual_providers_refuse_and_selector_cannot_weaken_schema_or_security() {
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let p = Arc::new(Probe::default());
    let (_socket, target) = udp();
    let registry = registry(vec![registration(1, p.clone()), registration(2, p.clone())]);
    let thing = source(json!([{"href":target}]));
    assert!(
        registry
            .prepare(
                &thing,
                &GatewayDefaultV1::LIMITS,
                bank.clone(),
                ConsumerSelector::default(),
                &mut work(),
                drive
            )
            .is_err()
    );
    clean(&bank);
    let selected = ConsumerSelector {
        binding: Some(BindingId::new(2)),
        form: Some(0),
    };
    let prepared = registry
        .prepare(
            &thing,
            &GatewayDefaultV1::LIMITS,
            bank.clone(),
            selected,
            &mut work(),
            drive,
        )
        .unwrap();
    assert_eq!(prepared.support()[0].binding, Some(BindingId::new(2)));
    drop(prepared);
    clean(&bank);
    for edit in ["schema", "type", "security", "proxy", "format"] {
        let mut v = serde_json::to_value(&thing).unwrap();
        match edit {
            "schema" => v["properties"]["a"]["minimum"] = json!(0),
            "type" => v["properties"]["a"]["type"] = json!("string"),
            "security" => {
                v["securityDefinitions"]["none"] = json!({"scheme":"basic","in":"header"})
            }
            "proxy" => v["securityDefinitions"]["none"]["proxy"] = json!("http://127.0.0.1/"),
            _ => v["properties"]["a"]["forms"][0]["contentType"] = json!("application/cbor"),
        }
        let td = serde_json::from_value(v).unwrap();
        assert!(
            registry
                .prepare(
                    &td,
                    &GatewayDefaultV1::LIMITS,
                    bank.clone(),
                    selected,
                    &mut work(),
                    drive
                )
                .is_err(),
            "{edit}"
        );
        clean(&bank);
    }
    assert_eq!(p.native.load(Ordering::Relaxed), 0);
}
#[test]
fn exact_pair_refusal_recovers_compiler_and_executor_before_io() {
    use clinkz_wot_planning::*;
    let p = Arc::new(Probe::default());
    let other = ConsumerBindingConfiguration::for_semantic_host(
        256,
        BindingConfigurationDigest::new([179; 32]),
        configuration().native_compatibility().unwrap(),
        1024,
        16,
    )
    .unwrap();
    let compiler = HostBindingCompilerRegistration::try_new_consumer_extension(Compiler {
        probe: p.clone(),
        config: other,
        oversized_work: false,
    })
    .unwrap_or_else(|_| panic!());
    let rejected = HostConsumerBindingPreparation::try_pair(execution(1, p.clone()), compiler)
        .err()
        .unwrap();
    let (runtime, compiler) = rejected.into_input();
    assert_eq!(runtime.consumer_configuration(), Some(configuration()));
    assert_eq!(compiler.consumer_metadata().unwrap().configuration, other);
    assert_eq!(p.native.load(Ordering::Relaxed), 0);
    assert_eq!(p.starts.load(Ordering::Relaxed), 0);
}
#[test]
fn unpaid_cancelled_failed_and_capacity_refused_preparation_releases_every_owner() {
    let p = Arc::new(Probe::default());
    let (_peer, target) = udp();
    let thing = source(json!([{"href":target}]));
    let registry = registry(vec![registration(1, p.clone())]);
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    assert!(
        registry
            .prepare(
                &thing,
                &GatewayDefaultV1::LIMITS,
                bank.clone(),
                ConsumerSelector::default(),
                &mut WorkBudget::new(),
                drive
            )
            .is_err()
    );
    clean(&bank);
    assert_eq!(p.bounds.load(Ordering::Relaxed), 0);
    assert!(
        registry
            .prepare(
                &thing,
                &GatewayDefaultV1::LIMITS,
                bank.clone(),
                ConsumerSelector::default(),
                &mut work(),
                |job| {
                    job.step(&mut work(), false);
                    job.step(&mut WorkBudget::new(), true);
                }
            )
            .is_err()
    );
    assert_eq!(p.aborts.load(Ordering::Relaxed), 1);
    clean(&bank);
    let short = GatewayDefaultV1::LIMITS
        .clone()
        .with_limit(ResourceKind::BindingArtifactsGlobalMax, Some(0));
    let short = Arc::new(HostResourceAccounts::new(&short));
    let starts = p.starts.load(Ordering::Relaxed);
    assert!(
        registry
            .prepare(
                &thing,
                &GatewayDefaultV1::LIMITS,
                short.clone(),
                ConsumerSelector::default(),
                &mut work(),
                drive
            )
            .is_err()
    );
    assert_eq!(p.starts.load(Ordering::Relaxed), starts);
    clean(&short);
    let mut bad = registration(2, p.clone());
    let (executor, _) = bad.preparation.into_parts();
    let compiler =
        clinkz_wot_planning::HostBindingCompilerRegistration::try_new_consumer_extension(
            Compiler {
                probe: p.clone(),
                config: configuration(),
                oversized_work: true,
            },
        )
        .unwrap_or_else(|_| panic!());
    bad.preparation =
        clinkz_wot_planning::HostConsumerBindingPreparation::try_pair(executor, compiler)
            .unwrap_or_else(|_| panic!());
    let bad = ConsumerRegistry::try_new(vec![bad]).unwrap_or_else(|_| panic!());
    assert!(
        bad.prepare(
            &thing,
            &GatewayDefaultV1::LIMITS,
            bank.clone(),
            ConsumerSelector::default(),
            &mut work(),
            drive
        )
        .is_err()
    );
    clean(&bank);
    assert_eq!(p.native.load(Ordering::Relaxed), 0);
}
#[test]
fn validation_unread_results_configuration_loss_and_original_account_pressure() {
    let p = Arc::new(Probe::default());
    let (peer, target) = udp();
    let registry = registry(vec![registration(1, p.clone())]);
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let thing = source(json!([{"href":target}]));
    let rt = runtime(prepare(&registry, &thing, bank.clone()), bank.clone());
    drop(registry);
    drop(thing);
    let peer = std::thread::spawn(move || {
        exchange(&peer, b"\"wrong\"");
        exchange(&peer, b"7");
    });
    p.changed.store(true, Ordering::Relaxed);
    assert!(
        rt.read_property("a", Deadline::NONE, now(), &mut work())
            .is_err()
    );
    assert_eq!(p.native.load(Ordering::Relaxed), 0);
    p.changed.store(false, Ordering::Relaxed);
    p.denied.store(true, Ordering::Relaxed);
    assert!(
        rt.read_property("a", Deadline::NONE, now(), &mut work())
            .is_err()
    );
    p.denied.store(false, Ordering::Relaxed);
    let mut call = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    assert!(
        rt.read_property("a", Deadline::NONE, now(), &mut work())
            .is_err(),
        "occupied logical slot"
    );
    let invalid = poll(&mut call);
    assert!(matches!(invalid.result(), Err(CoreError::Validation(_))));
    assert!(invalid.rejected_output().is_some());
    assert!(
        bank.account(ResourceKind::PendingHandlerCallsGlobalMax)
            .used()
            > 0,
        "unread external result owns original reservation"
    );
    drop(invalid);
    drop(call);
    let amount = bank
        .account(ResourceKind::PendingHandlerCallsGlobalMax)
        .limit();
    let pressure = bank
        .reserve(ResourceKind::PendingHandlerCallsGlobalMax, amount)
        .unwrap();
    assert!(
        rt.read_property("a", Deadline::NONE, now(), &mut work())
            .is_err()
    );
    drop(pressure);
    let mut call = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    let settlement = poll(&mut call);
    drop(rt);
    assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
    drop(settlement);
    drop(call);
    peer.join().unwrap();
    clean(&bank);
}
#[test]
fn real_late_result_after_revocation_retains_first_cause_and_original_binding() {
    let p = Arc::new(Probe::default());
    p.late_on_cancel.store(true, Ordering::Relaxed);
    let (peer, target) = udp();
    let registry = registry(vec![registration(7, p.clone())]);
    let thing = source(json!([{"href":target}]));
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let rt = runtime(prepare(&registry, &thing, bank.clone()), bank.clone());
    let mut call = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    assert!(
        call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending()
    );
    rt.begin_close();
    assert!(
        rt.read_property("a", Deadline::NONE, now(), &mut work())
            .is_err()
    );
    exchange(&peer, b"11");
    let settlement = poll(&mut call);
    assert!(settlement.first_cause().is_some());
    assert_eq!(settlement.binding().binding_id(), BindingId::new(7));
    assert!(
        matches!(
            settlement.native_outcome(),
            BindingCallSettlement::Returned(Ok(_))
        ),
        "late original result preserved: {:?}",
        settlement.native_outcome()
    );
    assert_eq!(p.validates.load(Ordering::Relaxed), 1);
    drop(rt);
    drop(registry);
    drop(settlement);
    drop(call);
    drop(thing);
    clean(&bank);
}
#[test]
fn lost_handles_failed_and_pending_cleanup_stay_funded_until_final_release() {
    let p = Arc::new(Probe::default());
    let (_peer, target) = udp();
    let registry = registry(vec![registration(1, p.clone())]);
    let thing = source(json!([{"href":target}]));
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let rt = runtime(prepare(&registry, &thing, bank.clone()), bank.clone());
    drop(registry);
    drop(thing);
    let mut call = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    assert!(
        call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending()
    );
    p.fail_close.store(true, Ordering::Relaxed);
    drop(call);
    rt.step(&mut Context::from_waker(Waker::noop()), now(), &mut work());
    assert!(!rt.is_idle());
    assert_eq!(p.calls.load(Ordering::Relaxed), 0);
    p.fail_close.store(false, Ordering::Relaxed);
    p.pending_close.store(1, Ordering::Relaxed);
    rt.step(&mut Context::from_waker(Waker::noop()), now(), &mut work());
    assert!(!rt.is_idle());
    rt.step(
        &mut Context::from_waker(Waker::noop()),
        now(),
        &mut WorkBudget::new(),
    );
    assert!(!rt.is_idle());
    rt.step(&mut Context::from_waker(Waker::noop()), now(), &mut work());
    assert!(rt.is_idle());
    assert_eq!(p.calls.load(Ordering::Relaxed), 1);
    drop(rt);
    clean(&bank);
    assert_eq!(p.programs.load(Ordering::Relaxed), 1);
}

#[test]
fn compiler_failure_invalid_artifact_and_exhausted_lifetime_rollback_without_io() {
    let (_peer, target) = udp();
    let thing = source(json!([{"href":target}]));
    for failure in ["step", "artifact", "lifetime", "native_work"] {
        let p = Arc::new(Probe::default());
        match failure {
            "step" => p.step_failure.store(true, Ordering::Relaxed),
            "artifact" => p.invalid_artifact.store(true, Ordering::Relaxed),
            "native_work" => p.short_compile_budget.store(true, Ordering::Relaxed),
            _ => p.stall_compiler.store(true, Ordering::Relaxed),
        }
        let registry = registry(vec![registration(1, p.clone())]);
        let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
        assert!(
            registry
                .prepare(
                    &thing,
                    &GatewayDefaultV1::LIMITS,
                    bank.clone(),
                    ConsumerSelector::default(),
                    &mut work(),
                    drive
                )
                .is_err(),
            "{failure}"
        );
        assert_eq!(p.native.load(Ordering::Relaxed), 0);
        assert!(p.steps.load(Ordering::Relaxed) <= 256);
        clean(&bank);
        assert_eq!(
            p.aborts.load(Ordering::Relaxed),
            usize::from(failure != "artifact")
        );
        assert_eq!(
            p.programs.load(Ordering::Relaxed),
            usize::from(failure == "artifact")
        );
    }
}
#[test]
fn later_failure_reclaims_earlier_owned_output_and_runtime_rejects_a_foreign_bank() {
    let p = Arc::new(Probe::default());
    let (_peer, target) = udp();
    let registry = registry(vec![registration(1, p.clone())]);
    let mut v = serde_json::to_value(source(json!([{"href":target}]))).unwrap();
    v["properties"]["b"] = json!({"forms":[{"href":target.replace("temperature","failure")}]});
    let bad = serde_json::from_value(v).unwrap();
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    assert!(
        registry
            .prepare(
                &bad,
                &GatewayDefaultV1::LIMITS,
                bank.clone(),
                ConsumerSelector::default(),
                &mut work(),
                drive
            )
            .is_err()
    );
    assert_eq!(p.programs.load(Ordering::Relaxed), 1);
    clean(&bank);
    let thing = source(json!([{"href":target}]));
    let prepared = prepare(&registry, &thing, bank.clone());
    let (defs, funding) = prepared.into_execution();
    let foreign = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let owner = instance();
    let rejected = ConsumerRuntime::try_new(owner.reference(), defs, funding, foreign.clone(), 1)
        .err()
        .unwrap();
    clean(&foreign);
    let (defs, funding) = rejected.into_input();
    let rt = ConsumerRuntime::try_new(owner.reference(), defs, funding, bank.clone(), 1)
        .unwrap_or_else(|_| panic!());
    drop(rt);
    clean(&bank);
}
#[test]
fn selected_service_loss_deadline_clock_errors_and_root_loss_fence_real_calls() {
    let p = Arc::new(Probe::default());
    let (_peer, target) = udp();
    let registry = registry(vec![registration(1, p.clone())]);
    let thing = source(json!([{"href":target}]));
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let rt = runtime(prepare(&registry, &thing, bank.clone()), bank.clone());
    drop(registry);
    drop(thing);
    assert!(matches!(
        rt.read_property("a", Deadline::at(now()), now(), &mut work()),
        Err(CoreError::TimedOut(_))
    ));
    assert!(
        rt.read_property(
            "a",
            Deadline::at(MonotonicInstant::new(ClockId::new(100), 2)),
            now(),
            &mut work()
        )
        .is_err()
    );
    p.data_lost.store(true, Ordering::Relaxed);
    assert!(
        rt.read_property("a", Deadline::NONE, now(), &mut work())
            .is_err()
    );
    assert_eq!(p.native.load(Ordering::Relaxed), 0);
    p.data_lost.store(false, Ordering::Relaxed);
    let mut call = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    assert!(
        call.poll(
            &mut Context::from_waker(Waker::noop()),
            now(),
            &mut WorkBudget::new()
        )
        .is_pending()
    );
    assert_eq!(p.native.load(Ordering::Relaxed), 0);
    assert!(
        call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending()
    );
    let later = MonotonicInstant::new(ClockId::new(100), 2);
    let settlement = loop {
        if let Poll::Ready(s) =
            call.poll(&mut Context::from_waker(Waker::noop()), later, &mut work())
        {
            break s;
        }
    };
    assert!(matches!(
        settlement.first_cause(),
        Some(CoreError::InternalInvariant(_))
    ));
    drop(settlement);
    drop(call);
    let mut call = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    drop(rt);
    let settlement = poll(&mut call);
    assert!(settlement.result().is_err());
    drop(settlement);
    drop(call);
    clean(&bank);
    for &port in p.ports.lock().unwrap().iter() {
        drop(UdpSocket::bind(port).unwrap());
    }
}
#[test]
fn replacement_and_unread_results_never_gain_the_new_incarnations_authority() {
    let p = Arc::new(Probe::default());
    p.late_on_cancel.store(true, Ordering::Relaxed);
    let (peer, target) = udp();
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let thing = source(json!([{"href":target}]));
    let old_reg = registry(vec![registration(1, p.clone())]);
    let old = runtime(prepare(&old_reg, &thing, bank.clone()), bank.clone());
    let new_reg = registry(vec![registration(2, p.clone())]);
    let new = runtime(prepare(&new_reg, &thing, bank.clone()), bank.clone());
    let mut old_call = old
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    assert!(
        old_call
            .poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending()
    );
    old.begin_close();
    exchange(&peer, b"7");
    let old_result = poll(&mut old_call);
    assert_eq!(old_result.binding().binding_id(), BindingId::new(1));
    assert!(old_result.result().is_err());
    let mut new_call = new
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    assert!(
        new_call
            .poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending()
    );
    exchange(&peer, b"9");
    // Drive to an unread terminal slot, then revoke before handoff.
    new.step(&mut Context::from_waker(Waker::noop()), now(), &mut work());
    new.begin_close();
    let new_result = poll(&mut new_call);
    assert_eq!(new_result.binding().binding_id(), BindingId::new(2));
    assert!(new_result.result().is_err());
    assert!(matches!(
        new_result.native_outcome(),
        BindingCallSettlement::Returned(Ok(_))
    ));
    drop(old);
    drop(new);
    drop(old_call);
    drop(new_call);
    drop(old_result);
    drop(new_result);
    drop(old_reg);
    drop(new_reg);
    drop(thing);
    clean(&bank);
}

#[test]
fn independently_typed_static_pair_accepts_owned_compiler_without_name_admission() {
    use clinkz_wot_planning::*;
    let probe = Arc::new(Probe::default());
    let config = configuration();
    let (_peer, target) = udp();
    let host = execution(1, probe.clone());
    let id = host.identity();
    let resources = host.resources();
    let status = host.status();
    let raw = StaticConsumerBindingRegistration::new(StaticConsumerBindingRegistrationInput::new(
        id,
        BindingExecutionSupport::application_static(),
        StaticClient {
            probe: probe.clone(),
        },
        resources,
        status,
    ))
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let compiler = StaticBindingCompilerRegistration::new(Compiler {
        probe: probe.clone(),
        config,
        oversized_work: false,
    });
    let pair = StaticConsumerBindingPreparation::try_pair(raw, compiler)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(pair.consumer_contract().unwrap().max_steps, 256);
    let plan = LogicalInteractionPlan::try_property_read(
        PlanId::new(SlotIndex::new(0), Generation::INITIAL),
        ThingId::from("urn:adapter-metadata"),
        "a".into(),
        0,
        target.into(),
        Some("application/json".into()),
        None,
    )
    .unwrap();
    let input = BindingCompilerInput::new(
        &plan,
        BindingCandidate::new(
            id.binding_id(),
            id.binding_generation(),
            id.configuration(),
            id.artifact_compatibility(),
            0,
            0,
        ),
        BindingArtifactRole::ConsumerCall,
    );
    let compiler = pair.compiler().compiler();
    let mut cursor = compiler.start(&input).unwrap();
    let output = loop {
        match compiler.step(&input, cursor, &mut work()) {
            BindingCompilerStep::Pending(next) => cursor = next,
            BindingCompilerStep::Complete(out) => break out,
            BindingCompilerStep::Failed(_) => panic!(),
        }
    };
    drop(pair);
    drop(plan);
    drop(output);
    assert_eq!(probe.programs.load(Ordering::Relaxed), 1);
}

#[test]
fn unsupported_semantic_alternative_keeps_the_supported_form_executable() {
    let p = Arc::new(Probe::default());
    let (peer, target) = udp();
    let registry = registry(vec![registration(1, p.clone())]);
    let thing = source(
        json!([{"href":target,"@context":"https://unsupported.example/native-context"},{"href":target}]),
    );
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let prepared = prepare(&registry, &thing, bank.clone());
    assert_eq!(
        prepared.support()[0].unsupported,
        Some(ConsumerUnsupported::SemanticMeaning)
    );
    assert!(
        prepared
            .support()
            .iter()
            .any(|s| s.form == 1 && s.operation == Operation::ReadProperty && s.selected)
    );
    let rt = runtime(prepared, bank.clone());
    drop(thing);
    drop(registry);
    let mut call = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    assert!(
        call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending()
    );
    exchange(&peer, b"7");
    let result = poll(&mut call);
    assert!(result.result().is_ok());
    drop(result);
    drop(call);
    drop(rt);
    clean(&bank);
}

#[test]
fn unread_native_failure_wins_over_later_cancel_and_fabricated_transfer_stays_owned() {
    let p = Arc::new(Probe::default());
    let (peer, target) = udp();
    let registry = registry(vec![registration(1, p.clone())]);
    let thing = source(json!([{"href":target}]));
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let rt = runtime(prepare(&registry, &thing, bank.clone()), bank.clone());
    let mut call = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    assert!(
        call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending()
    );
    exchange(&peer, &[b'x'; 1025]);
    rt.step(&mut Context::from_waker(Waker::noop()), now(), &mut work());
    call.cancel();
    let outcome = poll(&mut call);
    assert!(
        matches!(outcome.first_cause(), Some(CoreError::Validation(_))),
        "the earlier native failure wins"
    );
    drop(outcome);
    drop(call);
    p.transfer_once.store(true, Ordering::Relaxed);
    let mut call = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    call.cancel();
    assert!(
        call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending(),
        "transfer was never acknowledged"
    );
    assert_eq!(
        bank.account(ResourceKind::PendingHandlerCallsGlobalMax)
            .used(),
        1
    );
    let result = poll(&mut call);
    assert!(result.refused_transfer().is_some());
    assert!(result.cleanup_error().is_some());
    assert!(result.result().is_err());
    drop(result);
    drop(call);
    drop(rt);
    drop(registry);
    drop(thing);
    clean(&bank);
}
#[test]
fn restricted_inline_registration_cannot_misstate_general_callback_costs() {
    let config = clinkz_wot_binding_http::HttpConsumerConfig::for_cleartext_ip().unwrap();
    let inline = clinkz_wot_binding_http::try_inline_consumer_registration(
        config,
        BindingId::new(1),
        BindingGeneration::INITIAL,
        1,
    )
    .unwrap();
    let input = ConsumerRegistrationInput {
        preparation: inline.into_registration(),
        services: config.consumer_services(),
    };
    assert!(ConsumerRegistry::try_new(vec![input]).is_err());
}

#[test]
fn unavailable_selected_codec_is_never_called_for_a_late_native_result() {
    let p = Arc::new(Probe::default());
    p.late_on_cancel.store(true, Ordering::Relaxed);
    let (peer, target) = udp();
    let registry = registry(vec![registration(1, p.clone())]);
    let thing = source(json!([{"href":target}]));
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let rt = runtime(prepare(&registry, &thing, bank.clone()), bank.clone());
    drop(registry);
    drop(thing);
    let mut call = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    assert!(
        call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending()
    );
    p.data_lost.store(true, Ordering::Relaxed);
    exchange(&peer, b"7");
    let result = poll(&mut call);
    assert!(result.result().is_err());
    assert!(result.rejected_output().is_some());
    assert_eq!(p.validates.load(Ordering::Relaxed), 0);
    drop(result);
    drop(call);
    drop(rt);
    clean(&bank);
}

#[test]
fn review_long_unselected_alternative_does_not_hide_supported_form() {
    let p = Arc::new(Probe::default());
    let (_peer, target) = udp();
    let registry = registry(vec![registration(1, p)]);
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let control = source(json!([{"href":format!("unknown:{}", "x".repeat(100))},{"href":target}]));
    drop(prepare(&registry, &control, bank.clone()));
    clean(&bank);
    let long = format!("unknown:{}", "x".repeat(300));
    let thing = source(json!([{"href":long},{"href":target}]));
    let prepared = registry.prepare(
        &thing,
        &GatewayDefaultV1::LIMITS,
        bank.clone(),
        ConsumerSelector {
            binding: None,
            form: Some(1),
        },
        &mut work(),
        drive,
    );
    assert!(
        prepared.is_ok(),
        "explicitly selected short Form was rejected: {:?}",
        prepared.err()
    );
}

#[test]
fn review_cleanup_progress_is_fair_with_one_native_credit() {
    let first = Arc::new(Probe::default());
    let second = Arc::new(Probe::default());
    first.fail_close.store(true, Ordering::Relaxed);
    let (_peer_a, target_a) = udp();
    let (_peer_b, target_b) = udp();
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let reg_a = registry(vec![registration(1, first.clone())]);
    let reg_b = registry(vec![registration(2, second.clone())]);
    let source_a = source(json!([{"href":target_a}]));
    let mut value_b = serde_json::to_value(source(json!([{"href":target_b}]))).unwrap();
    value_b["properties"]["b"] = value_b["properties"]["a"].take();
    value_b["properties"].as_object_mut().unwrap().remove("a");
    let source_b = serde_json::from_value(value_b).unwrap();
    let (mut defs_a, mut funding_a) = prepare(&reg_a, &source_a, bank.clone()).into_execution();
    let (mut defs_b, mut funding_b) = prepare(&reg_b, &source_b, bank.clone()).into_execution();
    defs_a.append(&mut defs_b);
    funding_a.append(&mut funding_b);
    let owner = instance();
    let rt = ConsumerRuntime::try_new(owner.reference(), defs_a, funding_a, bank.clone(), 2)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let mut a = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    let mut b = rt
        .read_property("b", Deadline::NONE, now(), &mut work())
        .unwrap();
    assert!(
        a.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending()
    );
    assert!(
        b.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending()
    );
    drop(a);
    drop(b);
    for _ in 0..64 {
        let mut budget = work();
        budget.set_remaining(WorkClass::BindingPolls, 1);
        rt.step(&mut Context::from_waker(Waker::noop()), now(), &mut budget);
    }
    let second_released = second.calls.load(Ordering::Relaxed);
    first.fail_close.store(false, Ordering::Relaxed);
    rt.step(&mut Context::from_waker(Waker::noop()), now(), &mut work());
    assert_eq!(
        second.calls.load(Ordering::Relaxed),
        1,
        "larger credit control"
    );
    drop(rt);
    drop(reg_a);
    drop(reg_b);
    clean(&bank);
    assert_eq!(
        second_released, 1,
        "second abandoned call never receives cleanup credit"
    );
}

#[test]
fn review_service_capacity_failure_keeps_artifact_charged_through_destructor() {
    let p = Arc::new(Probe::default());
    let (_peer, target) = udp();
    let registry = registry(vec![registration(1, p.clone())]);
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    *p.review_bank.lock().unwrap() = Some(bank.clone());
    let thing = source(json!([{"href":target}]));
    drop(prepare(&registry, &thing, bank.clone()));
    assert!(
        p.review_drop_charge.load(Ordering::Relaxed) > 0,
        "normal destruction control"
    );
    p.programs.store(0, Ordering::Relaxed);
    p.review_drop_charge.store(0, Ordering::Relaxed);
    let mut competing = None;
    let result = registry.prepare(
        &thing,
        &GatewayDefaultV1::LIMITS,
        bank.clone(),
        ConsumerSelector::default(),
        &mut work(),
        |job| {
            drive(job);
            let account = bank.account(ResourceKind::EngineLiveBytesGlobalMax);
            competing = bank.reserve(
                ResourceKind::EngineLiveBytesGlobalMax,
                account.limit() - account.used(),
            );
            assert!(competing.is_some());
        },
    );
    assert!(result.is_err());
    assert_eq!(p.programs.load(Ordering::Relaxed), 1);
    drop(competing);
    clean(&bank);
    assert!(
        p.review_drop_charge.load(Ordering::Relaxed) > 0,
        "artifact funding was released before its destructor ran"
    );
}

#[test]
fn service_accounting_arithmetic_and_definition_refusals_keep_original_artifact_funding() {
    use clinkz_wot_core::consumer::{ConsumerDataPolicy, ConsumerDataService};
    use clinkz_wot_core::value_type::TypeSchema;
    struct DeclaredBacking {
        inner: Arc<dyn ConsumerDataService>,
        bytes: u64,
    }
    impl ConsumerDataService for DeclaredBacking {
        fn content_type(&self) -> &str {
            self.inner.content_type()
        }
        fn validate(&self, schema: TypeSchema, output: &Payload) -> CoreResult<()> {
            self.inner.validate(schema, output)
        }
        fn policy(&self) -> ConsumerDataPolicy {
            ConsumerDataPolicy {
                retained_bytes: self.bytes,
                ..self.inner.policy()
            }
        }
        fn supports_schema(&self, schema: TypeSchema) -> bool {
            self.inner.supports_schema(schema)
        }
        fn available(&self) -> bool {
            self.inner.available()
        }
    }
    let (_peer, target) = udp();
    let thing = source(json!([{"href":target}]));
    for failure in ["service_sum", "owner_sum", "definition"] {
        let p = Arc::new(Probe::default());
        let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
        *p.review_bank.lock().unwrap() = Some(bank.clone());
        let mut input = registration(1, p.clone());
        if failure != "definition" {
            let bytes = if failure == "service_sum" {
                u64::MAX
            } else {
                u64::MAX - input.services.access.retained_bytes()
            };
            input.services.data = Arc::new(DeclaredBacking {
                inner: input.services.data,
                bytes,
            });
        }
        let registry = registry(vec![input]);
        let result = registry.prepare(
            &thing,
            &GatewayDefaultV1::LIMITS,
            bank.clone(),
            ConsumerSelector::default(),
            &mut work(),
            |job| {
                drive(job);
                if failure == "definition" {
                    p.changed.store(true, Ordering::Relaxed);
                }
            },
        );
        assert!(matches!(result, Err(CoreError::Validation(_))), "{failure}");
        assert_eq!(p.programs.load(Ordering::Relaxed), 1, "{failure}");
        assert!(
            p.review_drop_charge.load(Ordering::Relaxed) > 0,
            "{failure} refunded the original artifact before destruction"
        );
        assert_eq!(p.native.load(Ordering::Relaxed), 0);
        clean(&bank);
    }
}

#[test]
fn target_envelope_is_partial_support_and_selected_short_form_still_executes() {
    let p = Arc::new(Probe::default());
    let (peer, target) = udp();
    let registry = registry(vec![registration(1, p.clone())]);
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    for target_bytes in [256, 257, 308] {
        for form in [None, Some(1)] {
            let long = format!("unknown:{}", "x".repeat(target_bytes - "unknown:".len()));
            let thing = source(json!([{"href":long},{"href":target}]));
            let prepared = registry
                .prepare(
                    &thing,
                    &GatewayDefaultV1::LIMITS,
                    bank.clone(),
                    ConsumerSelector {
                        binding: None,
                        form,
                    },
                    &mut work(),
                    drive,
                )
                .unwrap();
            let excluded = prepared
                .support()
                .iter()
                .find(|s| s.form == 0 && s.operation == Operation::ReadProperty)
                .unwrap();
            assert_eq!(
                excluded.unsupported,
                Some(match form {
                    Some(_) => ConsumerUnsupported::Selector,
                    None if target_bytes > 256 => ConsumerUnsupported::Target,
                    None => ConsumerUnsupported::NoCompleteProvider,
                })
            );
            assert!(
                prepared
                    .support()
                    .iter()
                    .any(|s| s.form == 1 && s.operation == Operation::ReadProperty && s.selected)
            );
            drop(prepared);
            clean(&bank);
        }
    }
    let long = format!("unknown:{}", "x".repeat(300));
    let thing = source(json!([{"href":long,"op":"writeproperty"},{"href":target}]));
    let prepared = prepare(&registry, &thing, bank.clone());
    assert!(prepared.support().iter().any(|s| s.form == 0
        && s.operation == Operation::WriteProperty
        && s.unsupported == Some(ConsumerUnsupported::Target)));
    let rt = runtime(prepared, bank.clone());
    drop(thing);
    drop(registry);
    let mut call = rt
        .read_property("a", Deadline::NONE, now(), &mut work())
        .unwrap();
    assert!(
        call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
            .is_pending()
    );
    exchange(&peer, b"7");
    let result = poll(&mut call);
    assert!(result.result().is_ok());
    drop(result);
    drop(call);
    drop(rt);
    clean(&bank);
    for &port in p.ports.lock().unwrap().iter() {
        drop(UdpSocket::bind(port).unwrap());
    }
}

#[test]
fn pending_native_read_cannot_starve_an_independent_ready_result() {
    for minimal in [false, true] {
        let first = Arc::new(Probe::default());
        let second = Arc::new(Probe::default());
        let (_peer_a, target_a) = udp();
        let (peer_b, target_b) = udp();
        let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
        let reg_a = registry(vec![registration(1, first.clone())]);
        let reg_b = registry(vec![registration(2, second.clone())]);
        let source_a = source(json!([{"href":target_a}]));
        let mut value_b = serde_json::to_value(source(json!([{"href":target_b}]))).unwrap();
        value_b["properties"]["b"] = value_b["properties"]["a"].take();
        value_b["properties"].as_object_mut().unwrap().remove("a");
        let source_b = serde_json::from_value(value_b).unwrap();
        let (mut definitions, mut funding) =
            prepare(&reg_a, &source_a, bank.clone()).into_execution();
        let (mut second_definitions, mut second_funding) =
            prepare(&reg_b, &source_b, bank.clone()).into_execution();
        definitions.append(&mut second_definitions);
        funding.append(&mut second_funding);
        let owner = instance();
        let rt = ConsumerRuntime::try_new(owner.reference(), definitions, funding, bank.clone(), 2)
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        drop(reg_a);
        drop(reg_b);
        drop(source_a);
        drop(source_b);
        let mut a = rt
            .read_property("a", Deadline::NONE, now(), &mut work())
            .unwrap();
        let mut b = rt
            .read_property("b", Deadline::NONE, now(), &mut work())
            .unwrap();
        assert!(
            a.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
                .is_pending()
        );
        assert!(
            b.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
                .is_pending()
        );
        exchange(&peer_b, b"9");
        for _ in 0..2 {
            let mut budget = work();
            budget.set_remaining(WorkClass::BindingPolls, 1);
            if minimal {
                budget.set_remaining(WorkClass::ProviderProbes, 2);
                budget.set_remaining(WorkClass::HandlerSteps, 1);
            }
            rt.step(&mut Context::from_waker(Waker::noop()), now(), &mut budget);
        }
        assert_eq!(
            second.validates.load(Ordering::Relaxed),
            1,
            "minimal={minimal}: ready result starved behind the pending read"
        );
        let mut no_native = work();
        no_native.set_remaining(WorkClass::BindingPolls, 0);
        let Poll::Ready(result) = b.poll(
            &mut Context::from_waker(Waker::noop()),
            now(),
            &mut no_native,
        ) else {
            panic!("background progress did not retain the ready result");
        };
        assert!(result.result().is_ok());
        assert_eq!(result.binding().binding_id(), BindingId::new(2));
        drop(result);
        drop(b);
        drop(a);
        rt.step(&mut Context::from_waker(Waker::noop()), now(), &mut work());
        assert!(rt.is_idle());
        drop(rt);
        clean(&bank);
        for probe in [first, second] {
            for &port in probe.ports.lock().unwrap().iter() {
                drop(UdpSocket::bind(port).unwrap());
            }
        }
    }
}

#[test]
fn rereview_unread_success_cannot_bypass_changed_access_with_short_credit() {
    for short_credit in [false, true] {
        let p = Arc::new(Probe::default());
        let (peer, target) = udp();
        let registry = registry(vec![registration(1, p.clone())]);
        let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
        let thing = source(json!([{"href":target}]));
        let rt = runtime(prepare(&registry, &thing, bank.clone()), bank.clone());
        let mut call = rt
            .read_property("a", Deadline::NONE, now(), &mut work())
            .unwrap();
        assert!(
            call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
                .is_pending()
        );
        exchange(&peer, b"7");
        let start = Instant::now();
        while p.validates.load(Ordering::Relaxed) == 0 {
            rt.step(&mut Context::from_waker(Waker::noop()), now(), &mut work());
            assert!(start.elapsed() < Duration::from_secs(5));
        }
        p.denied.store(true, Ordering::Relaxed);
        let mut budget = work();
        if short_credit {
            budget.set_remaining(WorkClass::ProviderProbes, 0);
        }
        let accepted = match call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut budget)
        {
            Poll::Ready(result) => {
                if !short_credit {
                    assert!(
                        matches!(result.result(), Err(CoreError::Validation(_))),
                        "fully paid control must enforce the changed policy"
                    );
                }
                result.result().is_ok()
            }
            Poll::Pending => {
                assert!(short_credit, "fully paid terminal control must settle");
                false
            }
        };
        drop(call);
        drop(rt);
        drop(registry);
        clean(&bank);
        assert!(
            !accepted,
            "short_credit={short_credit}: delivered success without the current access policy check"
        );
    }
}

#[test]
fn rereview_inspection_scratch_must_obey_the_admission_limit() {
    use clinkz_wot_planning::*;
    const INSPECTION_BYTES: u64 = 8_388_609;
    struct LargeInspection(Compiler);
    impl ConsumerCompiler for LargeInspection {
        const CONTRACT: ConsumerCompilerContract = ConsumerCompilerContract {
            inspection_temporary_bytes: INSPECTION_BYTES,
            ..<Compiler as ConsumerCompiler>::CONTRACT
        };
        fn configuration(&self) -> ConsumerBindingConfiguration {
            ConsumerCompiler::configuration(&self.0)
        }
    }
    impl BindingCompilerExtension for LargeInspection {
        type Cursor = <Compiler as BindingCompilerExtension>::Cursor;
        type Artifact = <Compiler as BindingCompilerExtension>::Artifact;
        fn compatibility(&self) -> BindingArtifactCompatibility {
            self.0.compatibility()
        }
        fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
            let mut scratch = Vec::<u8>::new();
            scratch
                .try_reserve_exact(INSPECTION_BYTES as usize)
                .unwrap();
            std::hint::black_box(scratch.capacity());
            self.0.bounds(input)
        }
        fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
            self.0.start(input)
        }
        fn step(
            &self,
            input: &BindingCompilerInput<'_>,
            cursor: Self::Cursor,
            budget: &mut WorkBudget,
        ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
            self.0.step(input, cursor, budget)
        }
        fn abort(&self, cursor: Self::Cursor) {
            self.0.abort(cursor)
        }
    }
    let p = Arc::new(Probe::default());
    let (_peer, target) = udp();
    let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
    let thing = source(json!([{"href":target}]));
    let control = registry(vec![registration(1, p.clone())]);
    drop(prepare(&control, &thing, bank.clone()));
    drop(control);
    clean(&bank);
    p.bounds.store(0, Ordering::Relaxed);
    let compiler =
        HostBindingCompilerRegistration::try_new_consumer_extension(LargeInspection(Compiler {
            probe: p.clone(),
            config: configuration(),
            oversized_work: false,
        }))
        .unwrap_or_else(|_| panic!("compiler capture"));
    let preparation = HostConsumerBindingPreparation::try_pair(execution(1, p.clone()), compiler)
        .unwrap_or_else(|_| panic!("pairing"));
    let registry = registry(vec![ConsumerRegistrationInput {
        preparation,
        services: services(p.clone()),
    }]);
    let cap = GatewayDefaultV1::LIMITS
        .get(ResourceKind::AdmissionTemporaryBytesPerOperationMax)
        .unwrap();
    assert!(INSPECTION_BYTES > cap);
    let result = registry.prepare(
        &thing,
        &GatewayDefaultV1::LIMITS,
        bank.clone(),
        ConsumerSelector::default(),
        &mut work(),
        drive,
    );
    assert!(matches!(
        &result,
        Err(CoreError::LimitExceeded {
            resource: ResourceKind::AdmissionTemporaryBytesPerOperationMax,
            limit,
            requested: Some(requested),
            ..
        }) if *limit == cap && *requested == INSPECTION_BYTES
    ));
    drop(result);
    clean(&bank);
    assert_eq!(p.bounds.load(Ordering::Relaxed), 0, "no bounds callback");

    // At and below the local envelope, inspection still reserves its backing
    // together with the live common arena in both global accounts.
    for cap in [INSPECTION_BYTES, INSPECTION_BYTES + 1] {
        let limits = GatewayDefaultV1::LIMITS.clone().with_limit(
            ResourceKind::AdmissionTemporaryBytesPerOperationMax,
            Some(cap),
        );
        let bank = Arc::new(HostResourceAccounts::new(&limits));
        let result = registry
            .prepare(
                &thing,
                &limits,
                bank.clone(),
                ConsumerSelector::default(),
                &mut work(),
                drive,
            )
            .unwrap();
        assert!(
            bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
                .peak()
                >= cap + INSPECTION_BYTES
        );
        assert!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).peak() >= cap + INSPECTION_BYTES
        );
        drop(result);
        clean(&bank);
    }
    assert_eq!(p.bounds.load(Ordering::Relaxed), 2);

    // A locally valid callback cannot run if the overlapping common/inspection
    // backing would exceed the global temporary account.
    let limits = GatewayDefaultV1::LIMITS
        .clone()
        .with_limit(
            ResourceKind::AdmissionTemporaryBytesPerOperationMax,
            Some(INSPECTION_BYTES),
        )
        .with_limit(
            ResourceKind::AdmissionTemporaryBytesGlobalMax,
            Some(2 * INSPECTION_BYTES - 1),
        );
    let bank = Arc::new(HostResourceAccounts::new(&limits));
    assert!(
        registry
            .prepare(
                &thing,
                &limits,
                bank.clone(),
                ConsumerSelector::default(),
                &mut work(),
                drive,
            )
            .is_err()
    );
    assert_eq!(p.bounds.load(Ordering::Relaxed), 2, "no unpaid callback");
    assert_eq!(p.native.load(Ordering::Relaxed), 0, "pure preparation");
    drop(registry);
    clean(&bank);
}

#[test]
fn unread_result_retains_its_owner_until_fully_paid_current_checks_can_run() {
    for change in ["none", "access", "configuration", "data"] {
        let p = Arc::new(Probe::default());
        let (peer, target) = udp();
        let registry = registry(vec![registration(1, p.clone())]);
        let bank = Arc::new(HostResourceAccounts::new(&GatewayDefaultV1::LIMITS));
        let rt = runtime(
            prepare(&registry, &source(json!([{"href":target}])), bank.clone()),
            bank.clone(),
        );
        let mut call = rt
            .read_property("a", Deadline::NONE, now(), &mut work())
            .unwrap();
        assert!(
            call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut work())
                .is_pending()
        );
        exchange(&peer, b"7");
        let start = Instant::now();
        while p.validates.load(Ordering::Relaxed) == 0 {
            rt.step(&mut Context::from_waker(Waker::noop()), now(), &mut work());
            assert!(start.elapsed() < Duration::from_secs(5));
        }
        match change {
            "none" => {}
            "access" => p.denied.store(true, Ordering::Relaxed),
            "configuration" => p.changed.store(true, Ordering::Relaxed),
            "data" => p.data_lost.store(true, Ordering::Relaxed),
            _ => unreachable!(),
        }
        let charged: Vec<_> = ResourceKind::ALL
            .iter()
            .map(|&kind| bank.account(kind).used())
            .collect();
        for credit in [0, 1] {
            let mut budget = work();
            budget.set_remaining(WorkClass::ProviderProbes, credit);
            assert!(
                call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut budget)
                    .is_pending(),
                "{change}, credit={credit}: handoff must wait for the full current check"
            );
            assert!(!rt.is_idle());
            assert_eq!(p.calls.load(Ordering::Relaxed), 0);
            assert_eq!(
                charged,
                ResourceKind::ALL
                    .iter()
                    .map(|&kind| bank.account(kind).used())
                    .collect::<Vec<_>>()
            );
        }
        let mut budget = WorkBudget::new().with_remaining(WorkClass::ProviderProbes, 2);
        let Poll::Ready(settlement) =
            call.poll(&mut Context::from_waker(Waker::noop()), now(), &mut budget)
        else {
            panic!("{change}: a paid terminal check must allow handoff");
        };
        if change == "none" {
            assert!(settlement.result().is_ok());
            assert!(settlement.first_cause().is_none());
        } else {
            assert!(matches!(settlement.result(), Err(CoreError::Validation(_))));
            assert!(matches!(
                settlement.first_cause(),
                Some(CoreError::Validation(_))
            ));
        }
        assert!(matches!(
            settlement.native_outcome(),
            BindingCallSettlement::Returned(Ok(_))
        ));
        assert!(rt.is_idle());
        assert_eq!(p.calls.load(Ordering::Relaxed), 0);
        drop(settlement);
        assert_eq!(p.calls.load(Ordering::Relaxed), 1);
        drop(call);
        drop(rt);
        drop(registry);
        clean(&bank);
        for &port in p.ports.lock().unwrap().iter() {
            drop(UdpSocket::bind(port).unwrap());
        }
    }
}
