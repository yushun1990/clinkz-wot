//! Reproducible finite Consumer workload; see tools/multi-instance/README.md.
#[path = "multi_instance/meter.rs"]
mod meter;
#[path = "multi_instance/peer.rs"]
mod peer;

use clinkz_wot_binding_http::*;
use clinkz_wot_core::{consumer::*, host_accounts::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::consumer::*;
use clinkz_wot_td::thing::Thing;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    future::poll_fn,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
    task::{Context, Poll},
    time::{Duration, Instant},
};

#[global_allocator]
static ALLOCATOR: meter::Allocator = meter::Allocator;
const CREDIT: u64 = 1_000_000;
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, CREDIT))
}
fn now() -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(97), 0)
}
fn bank() -> Arc<HostResourceAccounts> {
    Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS))
}
fn clean(bank: &HostResourceAccounts) {
    for k in ResourceKind::ALL {
        assert_eq!(bank.account(k).used(), 0, "{k:?}");
    }
}
fn used(bank: &HostResourceAccounts) -> u64 {
    bank.account(ResourceKind::EngineLiveBytesGlobalMax).used()
}
fn denied() -> CoreError {
    CoreError::Security {
        reason: SecurityFailureReason::AuthorizationDenied,
        context: ErrorContext::new(ErrorPhase::Admission, RetryClass::Never),
    }
}
fn emit(value: Value) {
    println!("{value}");
}
fn report(
    mut value: Value,
    banks: &[Arc<HostResourceAccounts>],
    physical: &[Arc<HostResourceAccounts>],
) {
    let mut reservations = serde_json::Map::new();
    for k in [
        ResourceKind::EngineLiveBytesGlobalMax,
        ResourceKind::BindingPollTemporaryBytesGlobalMax,
        ResourceKind::BindingResponseBufferBytesGlobalMax,
        ResourceKind::CleanupItemsMax,
        ResourceKind::CleanupBytesMax,
        ResourceKind::PendingHandlerCallsGlobalMax,
    ] {
        let sum = |which: &[Arc<HostResourceAccounts>]| {
            which.iter().map(|b| b.account(k).used()).sum::<u64>()
        };
        reservations.insert(
            format!("{k:?}"),
            json!({"logical_and_mapping": sum(banks), "physical": sum(physical)}),
        );
    }
    value["reservations"] = Value::Object(reservations);
    emit(value);
}
#[derive(Debug)]
struct Options {
    population: usize,
    active: usize,
    lanes: usize,
    slots: usize,
    members: usize,
    rounds: usize,
    mode: String,
    driver: String,
    accounts: String,
    delay_us: u64,
}
impl Options {
    fn parse(args: &[String]) -> Self {
        if args.iter().any(|a| a == "--help") {
            println!(
                "multi_instance --population 1000 --active 0.01 --lanes 8 --slots 1 --members 16 --rounds 5 --mode shared|targeted|cold --driver engine|binding --accounts independent|shared --delay-us 0"
            );
            std::process::exit(0);
        }
        let mut values = BTreeMap::new();
        assert!(
            args.len().is_multiple_of(2),
            "options require values; use --help"
        );
        for a in args.chunks_exact(2) {
            assert!(
                [
                    "--population",
                    "--active",
                    "--lanes",
                    "--slots",
                    "--members",
                    "--rounds",
                    "--mode",
                    "--driver",
                    "--accounts",
                    "--delay-us"
                ]
                .contains(&a[0].as_str()),
                "unknown option {}",
                a[0]
            );
            assert!(
                values.insert(a[0].as_str(), a[1].as_str()).is_none(),
                "duplicate option"
            );
        }
        let get = |key, default| *values.get(key).unwrap_or(&default);
        let population = get("--population", "1000").parse::<usize>().unwrap();
        let fraction = get("--active", "0.01").parse::<f64>().unwrap();
        let out = Self {
            population,
            active: (population as f64 * fraction).ceil() as usize,
            lanes: get("--lanes", "8").parse().unwrap(),
            slots: get("--slots", "1").parse().unwrap(),
            members: get("--members", "16").parse().unwrap(),
            rounds: get("--rounds", "5").parse().unwrap(),
            mode: get("--mode", "shared").into(),
            driver: get("--driver", "engine").into(),
            accounts: get("--accounts", "independent").into(),
            delay_us: get("--delay-us", "0").parse().unwrap(),
        };
        assert!(
            (1..=100_000).contains(&population)
                && fraction.is_finite()
                && (0.0..=1.0).contains(&fraction)
        );
        assert!((1..=32).contains(&out.lanes) && out.lanes <= population);
        assert!((1..=32).contains(&out.slots) && (1..=32).contains(&out.members) && out.rounds > 0);
        assert!(["shared", "targeted", "cold"].contains(&out.mode.as_str()));
        assert!(["engine", "binding"].contains(&out.driver.as_str()));
        assert!(["independent", "shared"].contains(&out.accounts.as_str()));
        out
    }
    fn mappings(&self) -> usize {
        if self.mode == "shared" {
            self.lanes
        } else {
            self.population
        }
    }
    fn mapping(&self, i: usize) -> usize {
        if self.mode == "shared" {
            i % self.lanes
        } else {
            i
        }
    }
}
fn description(address: &str, id: usize, members: usize) -> Vec<u8> {
    let mut td = json!({"@context": "https://www.w3.org/2022/wot/td/v1.1", "title": "multi-instance meter",
        "security": ["n"], "securityDefinitions": {"n": {"scheme": "nosec"}},
        "forms": [{"op": "readallproperties", "href": format!("http://{address}/t/{id}"),
            "htv:headers": [{"htv:fieldName": "X-Target", "htv:fieldValue": id.to_string()}]}],
        "properties": {"id": {"type": "integer", "readOnly": true, "forms": []}}});
    for n in 1..members {
        td["properties"][format!("calibrated_measurement_{n:02}")] =
            json!({"type": "integer", "readOnly": true, "forms": []});
    }
    serde_json::to_vec(&td).unwrap()
}
fn registration(id: usize) -> ConsumerRegistrationInput {
    try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(id as u32 + 1),
        BindingGeneration::INITIAL,
        0,
    )
    .unwrap()
}
#[derive(Default)]
struct Preparation {
    parse: meter::Cost,
    qualify: meter::Cost,
    native: meter::Cost,
    jobs: usize,
    contents: usize,
    work: [u64; WorkClass::ALL.len()],
    native_work: [u64; WorkClass::ALL.len()],
}
fn add_work(total: &mut [u64; WorkClass::ALL.len()], budget: &WorkBudget) {
    for (v, k) in total.iter_mut().zip(WorkClass::ALL) {
        *v += CREDIT - budget.remaining(k);
    }
}
fn work_json(total: &[u64; WorkClass::ALL.len()]) -> Value {
    WorkClass::ALL
        .into_iter()
        .zip(total)
        .map(|(k, n)| (format!("{k:?}"), json!(n)))
        .collect()
}
impl Preparation {
    fn prepare(
        &mut self,
        registry: &ConsumerRegistry,
        td: &[u8],
        bank: &Arc<HostResourceAccounts>,
        lane: usize,
        reuse: Option<&Arc<PreparedConsumerContent>>,
    ) -> CoreResult<Arc<ConsumerDefinition>> {
        let before = meter::read();
        let start = Instant::now();
        let thing: Thing = serde_json::from_slice(td).unwrap();
        self.parse.add(before, start.elapsed().as_nanos());
        let before = meter::read();
        let start = Instant::now();
        let mut budget = work();
        let mut drive = |job: &mut ConsumerCompilation<'_>| {
            self.jobs += 1;
            let before = meter::read();
            let start = Instant::now();
            while !job.is_finished() {
                let mut budget = work();
                job.step(&mut budget, false);
                add_work(&mut self.native_work, &budget);
            }
            self.native.add(before, start.elapsed().as_nanos());
        };
        let selector = ConsumerSelector {
            binding: Some(BindingId::new(lane as u32 + 1)),
            form: None,
        };
        let p = if let Some(content) = reuse {
            registry.prepare_with_content(
                &thing,
                GatewayDefaultV1::LIMITS,
                bank.clone(),
                selector,
                content,
                &mut budget,
                &mut drive,
            )
        } else {
            registry.prepare(
                &thing,
                GatewayDefaultV1::LIMITS,
                bank.clone(),
                selector,
                &mut budget,
                &mut drive,
            )
        };
        add_work(&mut self.work, &budget);
        let result = p.map(|p| {
            self.contents += p.content_constructions();
            let (bindings, funding) = p.into_execution();
            assert_eq!(bindings.len(), 1);
            let definition = bindings[0].definition().clone();
            drop(bindings);
            drop(funding);
            definition
        });
        self.qualify.add(before, start.elapsed().as_nanos());
        result
    }
    fn measurement(&self, mut value: Value) -> Value {
        value["parse"] = self.parse.json();
        value["qualification_including_native"] = self.qualify.json();
        value["native_job_steps"] = self.native.json();
        value["native_jobs"] = json!(self.jobs);
        value["content_constructions"] = json!(self.contents);
        value["work"] = work_json(&self.work);
        value["native_work"] = work_json(&self.native_work);
        value
    }
}
struct Policy {
    expected: ConsumerInstanceRef,
    target: String,
    denied: Arc<AtomicBool>,
}
impl ConsumerAccessService for Policy {
    fn authorize(
        &self,
        _: BindingRegistrationIdentity,
        target: &str,
        _: Operation,
    ) -> CoreResult<()> {
        if target != self.target || self.denied.load(Relaxed) {
            Err(denied())
        } else {
            Ok(())
        }
    }
    fn authorize_instance(
        &self,
        who: &ConsumerInstanceRef,
        b: BindingRegistrationIdentity,
        target: &str,
        op: Operation,
    ) -> CoreResult<()> {
        if who != &self.expected {
            return Err(denied());
        }
        self.authorize(b, target, op)
    }
    fn retained_bytes(&self) -> u64 {
        (size_of::<Self>() + self.target.capacity() + 2 * size_of::<usize>()) as u64
    }
    fn work(&self) -> u64 {
        2
    }
}
struct Instance {
    owner: Option<ConsumerInstance>,
    runtime: Option<ConsumerRuntime>,
    denied: Arc<AtomicBool>,
}
fn bound(
    definition: &Arc<ConsumerDefinition>,
    owner: &ConsumerInstance,
    account: &Arc<HostResourceAccounts>,
    denied: Arc<AtomicBool>,
    budget: &mut WorkBudget,
) -> ConsumerBinding {
    let access = ConsumerServiceOwner::retain_access(
        Arc::new(Policy {
            expected: owner.reference(),
            target: definition.target().into(),
            denied,
        }),
        account,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    definition
        .clone()
        .bind(access, account.clone(), budget)
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
impl Instance {
    fn new(
        definition: &Arc<ConsumerDefinition>,
        account: &Arc<HostResourceAccounts>,
        slots: usize,
        index: usize,
        bind_work: &mut [u64; WorkClass::ALL.len()],
    ) -> Self {
        let owner = ConsumerInstance::new(ThingSlotId::new(
            SlotIndex::new(index as u32),
            Generation::INITIAL,
        ));
        let denied = Arc::new(AtomicBool::new(false));
        let mut budget = work();
        let binding = bound(definition, &owner, account, denied.clone(), &mut budget);
        add_work(bind_work, &budget);
        let runtime = ConsumerRuntime::try_new(
            owner.reference(),
            vec![binding],
            vec![],
            account.clone(),
            slots,
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        Self {
            owner: Some(owner),
            runtime: Some(runtime),
            denied,
        }
    }
    fn start(&self, budget: &mut WorkBudget) -> CoreResult<ConsumerCall> {
        self.runtime
            .as_ref()
            .unwrap()
            .read_all_properties(Deadline::NONE, now(), budget)
    }
}
fn hard_limits(definition: &Arc<ConsumerDefinition>) {
    let account = bank();
    for registrations in [32, 33] {
        let result =
            ConsumerRegistry::try_new((0..registrations).map(registration).collect(), &account);
        assert_eq!(result.is_ok(), registrations == 32);
        drop(result);
        clean(&account);
    }
    for slots in [0, 32, 33] {
        let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL));
        let binding = bound(
            definition,
            &owner,
            &account,
            Arc::new(AtomicBool::new(false)),
            &mut work(),
        );
        let result = ConsumerRuntime::try_new(
            owner.reference(),
            vec![binding],
            vec![],
            account.clone(),
            slots,
        );
        assert_eq!(result.is_ok(), slots == 32);
        if let Err(ref rejection) = result {
            assert_eq!(rejection.input().0.len(), 1);
        }
        drop(result);
        drop(owner);
        clean(&account);
    }
    emit(
        json!({"phase": "hard_limits", "registry_32_accepted_33_refused": true,
        "runtime_slots_32_accepted_0_and_33_refused": true, "refusal_returns_binding": true}),
    );
}
enum Handle {
    Engine(ConsumerCall),
    Binding(HostBindingCallBox<CoreResult<InteractionOutput>>),
}
// Keep the control's return value inline; boxing it would add artificial heap
// traffic to the Engine-versus-Binding comparison.
#[allow(clippy::large_enum_variant)]
enum ResultOwner {
    Engine(ConsumerSettlement),
    Binding(InteractionOutput),
}
impl ResultOwner {
    fn check(&self, expected: &[u8]) {
        match self {
            Self::Engine(s) => assert_eq!(s.result().unwrap().data().unwrap().body, expected),
            Self::Binding(o) => assert_eq!(o.data().unwrap().body.as_ref(), expected),
        }
    }
}
impl Handle {
    fn start(
        o: &Options,
        instance: &Instance,
        definition: &ConsumerDefinition,
        budget: &mut WorkBudget,
    ) -> Self {
        if o.driver == "engine" {
            Self::Engine(instance.start(budget).unwrap())
        } else {
            let request = OutboundRequest::finite(
                BindingArtifactRef::new(definition.artifact().identity(), SlotIndex::new(0)),
                Operation::ReadAllProperties,
                None,
                BTreeMap::new(),
                None,
            )
            .unwrap();
            Self::Binding(
                definition
                    .execution()
                    .start_finite(request, definition.artifact())
                    .unwrap(),
            )
        }
    }
    fn poll(&mut self, cx: &mut Context<'_>, budget: &mut WorkBudget) -> Poll<ResultOwner> {
        match self {
            Self::Engine(c) => c.poll(cx, now(), budget).map(ResultOwner::Engine),
            Self::Binding(c) => c
                .as_pin_mut()
                .poll_result(cx, budget)
                .map(|r| ResultOwner::Binding(r.unwrap())),
        }
    }
}
struct Flight {
    handle: Handle,
    index: usize,
    started: Instant,
}
async fn one(instance: &Instance) -> ConsumerSettlement {
    let mut call = instance.start(&mut work()).unwrap();
    poll_fn(|cx| call.poll(cx, now(), &mut work())).await
}
fn summary(values: &mut [u64]) -> Value {
    values.sort_unstable();
    let p = |fraction: f64| {
        values
            .get(((values.len() as f64 * fraction).ceil() as usize).saturating_sub(1))
            .copied()
    };
    json!({"p50_ns": p(0.5), "p95_ns": p(0.95), "p99_ns": p(0.99), "max_ns": values.last()})
}
async fn load(
    o: &Options,
    instances: &[Instance],
    definitions: &[Arc<ConsumerDefinition>],
    expected: &[Vec<u8>],
    banks: &[Arc<HostResourceAccounts>],
    physical: &[Arc<HostResourceAccounts>],
) {
    let storage = meter::Phase::begin();
    let mut flights = Vec::<Flight>::with_capacity(o.lanes);
    let mut service = Vec::with_capacity(o.active * o.rounds);
    let mut queued = Vec::with_capacity(o.active * o.rounds);
    let mut busy = vec![false; o.lanes];
    let mut credits = [0; WorkClass::ALL.len()];
    let (mut admission_ns, mut progress_ns, mut polls) = (0u128, 0u128, 0u64);
    report(storage.finish("load_driver_storage"), banks, physical);
    let phase = meter::Phase::begin();
    let started = Instant::now();
    for round in 0..o.rounds {
        let cohort = Instant::now();
        let mut next = 0;
        poll_fn(|cx| {
            while next < o.active {
                let i = (round * o.active + next) % o.population;
                let lane = i % o.lanes;
                if busy[lane] {
                    break;
                }
                let mut budget = work();
                let start = Instant::now();
                let handle =
                    Handle::start(o, &instances[i], &definitions[o.mapping(i)], &mut budget);
                admission_ns += start.elapsed().as_nanos();
                add_work(&mut credits, &budget);
                flights.push(Flight {
                    handle,
                    index: i,
                    started: start,
                });
                busy[lane] = true;
                next += 1;
            }
            let mut completed = false;
            let mut j = 0;
            while j < flights.len() {
                let mut budget = work();
                let start = Instant::now();
                polls += 1;
                let result = flights[j].handle.poll(cx, &mut budget);
                progress_ns += start.elapsed().as_nanos();
                add_work(&mut credits, &budget);
                if let Poll::Ready(result) = result {
                    let f = flights.swap_remove(j);
                    service.push(f.started.elapsed().as_nanos() as u64);
                    queued.push(cohort.elapsed().as_nanos() as u64);
                    result.check(&expected[o.mapping(f.index)]);
                    busy[f.index % o.lanes] = false;
                    drop(result);
                    drop(f);
                    completed = true;
                } else {
                    j += 1;
                }
            }
            if flights.is_empty() && next == o.active {
                Poll::Ready(())
            } else {
                if completed {
                    cx.waker().wake_by_ref();
                }
                Poll::Pending
            }
        })
        .await;
    }
    let elapsed = started.elapsed();
    let mut measured = phase.finish("load");
    measured["completed"] = json!(service.len());
    measured["throughput_per_s"] = json!(service.len() as f64 / elapsed.as_secs_f64());
    measured["admitted_latency"] = summary(&mut service);
    measured["cohort_latency"] = summary(&mut queued);
    measured["admission_cpu_ns"] = json!(admission_ns);
    measured["progress_cpu_ns"] = json!(progress_ns);
    measured["polls"] = json!(polls);
    measured["work"] = work_json(&credits);
    report(measured, banks, physical);
}
async fn lifecycle(
    o: &Options,
    instances: &mut [Instance],
    definitions: &[Arc<ConsumerDefinition>],
    banks: &[Arc<HostResourceAccounts>],
    peer: &peer::Peer,
) {
    let account = &banks[0];
    let mut credits = [0; WorkClass::ALL.len()];
    let sibling = Instance::new(&definitions[0], account, 1, o.population, &mut credits);
    instances[0].denied.store(true, Relaxed);
    let requests = peer.stats()["requests"].as_u64().unwrap();
    assert!(instances[0].start(&mut work()).is_err());
    let sibling_result = one(&sibling).await;
    sibling_result.result().unwrap();
    drop(sibling_result);
    assert_eq!(peer.stats()["requests"].as_u64().unwrap(), requests + 1);
    instances[0].denied.store(false, Relaxed);

    let retained = one(&instances[0]).await;
    retained.result().unwrap();
    let native_before = peer.stats();
    let charge_before = used(account);
    let started = Instant::now();
    let attempt = sibling.start(&mut work());
    let held_result_blocks_sibling = attempt.is_err();
    if let Ok(mut c) = attempt {
        let result = poll_fn(|cx| c.poll(cx, now(), &mut work())).await;
        result.result().unwrap();
        drop(result);
        drop(c);
    }
    let hold_ns = started.elapsed().as_nanos();
    assert!(used(account) >= charge_before);
    drop(retained);

    // Cancellation after actual request receipt, with the original settlement.
    let mut cancelled = instances[0].start(&mut work()).unwrap();
    let before = peer.stats()["requests"].as_u64().unwrap();
    while peer.stats()["requests"].as_u64().unwrap() == before {
        assert!(
            poll_fn(|cx| Poll::Ready(cancelled.poll(
                cx,
                now(),
                &mut work().with_remaining(WorkClass::CodecInputBytes, 0)
            )))
            .await
            .is_pending()
        );
        tokio::time::sleep(Duration::from_micros(100)).await;
    }
    let start = Instant::now();
    cancelled.cancel();
    let result = poll_fn(|cx| cancelled.poll(cx, now(), &mut work())).await;
    let cancellation_ns = start.elapsed().as_nanos();
    assert!(matches!(
        result.first_cause(),
        Some(CoreError::Cancelled(_))
    ));
    assert!(result.result().is_err());
    drop(result);
    drop(cancelled);
    let other = one(&sibling).await;
    other.result().unwrap();
    drop(other);

    // Let real native output arrive, but withhold validation credit. Replacement
    // must preserve its original identity/funding, never deliver it to the new one.
    let mut late = instances[0].start(&mut work()).unwrap();
    let before = peer.stats()["requests"].as_u64().unwrap();
    for _ in 0..1000 {
        assert!(
            poll_fn(|cx| Poll::Ready(late.poll(
                cx,
                now(),
                &mut work().with_remaining(WorkClass::CodecInputBytes, 0)
            )))
            .await
            .is_pending()
        );
        let stats = peer.stats();
        if stats["requests"].as_u64().unwrap() > before && stats["active"] == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_micros(100)).await;
    }
    // Drive the completed socket through Hyper under zero codec credit.
    for _ in 0..4 {
        assert!(
            poll_fn(|cx| Poll::Ready(late.poll(
                cx,
                now(),
                &mut work().with_remaining(WorkClass::CodecInputBytes, 0)
            )))
            .await
            .is_pending()
        );
        tokio::task::yield_now().await;
    }
    let original = instances[0].owner.as_ref().unwrap().reference();
    drop(instances[0].owner.take());
    drop(instances[0].runtime.take());
    let start = Instant::now();
    instances[0] = Instance::new(&definitions[0], account, o.slots, 0, &mut credits);
    assert_ne!(original, instances[0].owner.as_ref().unwrap().reference());
    let result = poll_fn(|cx| late.poll(cx, now(), &mut work())).await;
    assert_eq!(result.consumer(), &original);
    assert!(result.result().is_err());
    assert!(matches!(
        result.native_outcome(),
        ConsumerNativeOutcome::Returned(Ok(_))
    ));
    drop(result);
    drop(late);
    let replacement_ns = start.elapsed().as_nanos();
    let result = one(&instances[0]).await;
    result.result().unwrap();
    drop(result);
    emit(
        json!({"phase": "lifecycle", "held_result_blocks_sibling": held_result_blocks_sibling,
        "peer_while_result_held": native_before, "sibling_attempt_ns": hold_ns,
        "cancellation_ns": cancellation_ns, "replacement_ns": replacement_ns,
        "independent_permission": true, "late_original_result": true}),
    );
}

async fn run(o: Options) {
    let peer = peer::Peer::start(o.members, o.delay_us);
    emit(
        json!({"phase": "configuration", "population": o.population, "active": o.active, "lanes": o.lanes,
        "slots": o.slots, "members": o.members, "rounds": o.rounds, "mode": o.mode,
        "driver": o.driver, "accounts": o.accounts, "delay_us": o.delay_us,
        "native_mappings": o.mappings(), "physical_peer_processes": 1,
        "account_size": size_of::<HostResourceAccounts>() }),
    );
    let phase = meter::Phase::begin();
    let banks: Vec<_> = (0..if o.accounts == "shared" {
        1
    } else {
        o.population
    })
        .map(|_| bank())
        .collect();
    let physical = [bank(), bank()]; // actual services, then registration/registry records
    report(phase.finish("account_provisioning"), &banks, &physical);
    let at = |i: usize| &banks[i % banks.len()];
    let phase = meter::Phase::begin();
    let inputs: Vec<_> = (0..o.mappings())
        .map(|i| description(&peer.address, i, o.members))
        .collect();
    let expected: Vec<_> = (0..o.mappings())
        .map(|i| peer::body(i, o.members))
        .collect();
    let mut definitions: Vec<Arc<ConsumerDefinition>> = Vec::with_capacity(o.mappings());
    let mut instances = Vec::with_capacity(o.population);
    let mut retained = Vec::with_capacity(o.lanes);
    report(phase.finish("fixture_storage"), &banks, &physical);

    let phase = meter::Phase::begin();
    let first = registration(0);
    let data = ConsumerServiceOwner::retain_data(first.services.data, &physical[0])
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let access = ConsumerServiceOwner::retain_access(first.services.access, &physical[0])
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let weak_data = Arc::downgrade(&data);
    let mut registrations = vec![RetainedConsumerRegistrationInput {
        preparation: first.preparation,
        data: data.clone(),
        access: access.clone(),
    }];
    for lane in 1..o.lanes {
        registrations.push(RetainedConsumerRegistrationInput {
            preparation: registration(lane).preparation,
            data: data.clone(),
            access: access.clone(),
        });
    }
    let registry = ConsumerRegistry::try_from_retained(registrations, &physical[1])
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    drop(data);
    drop(access);
    report(
        phase.finish("physical_services_and_registrations"),
        &banks,
        &physical,
    );
    let phase = meter::Phase::begin();
    let mut cost = Preparation::default();
    for (i, td) in inputs.iter().enumerate() {
        let reuse = if i == 0 || o.mode == "cold" {
            None
        } else {
            Some(definitions[0].content())
        };
        let definition = match cost.prepare(&registry, td, at(i), i % o.lanes, reuse) {
            Ok(definition) => definition,
            Err(error) => {
                let mut measured = cost.measurement(phase.finish("preparation_refused"));
                measured["error"] = json!(format!("{error:?}"));
                measured["mapping_index"] = json!(i);
                report(measured, &banks, &physical);
                let phase = meter::Phase::begin();
                drop(definitions);
                drop(registry);
                drop(instances);
                drop(retained);
                drop(inputs);
                drop(expected);
                assert!(weak_data.upgrade().is_none());
                drop(weak_data);
                let measured = phase.finish("refusal_release");
                for b in banks.iter().chain(&physical) {
                    clean(b);
                }
                report(measured, &banks, &physical);
                emit(json!({"phase": "peer_final", "stats": peer.stats()}));
                let phase = meter::Phase::begin();
                drop(banks);
                drop(physical);
                emit(phase.finish("release_accounts"));
                return;
            }
        };
        if let Some(content) = reuse {
            assert!(Arc::ptr_eq(content, definition.content()));
        }
        definitions.push(definition);
    }
    assert_eq!(cost.jobs, o.mappings());
    assert_eq!(
        cost.contents,
        if o.mode == "cold" { o.mappings() } else { 1 }
    );
    let weak_content = Arc::downgrade(definitions[0].content());
    let measured = cost.measurement(phase.finish("preparation"));
    report(measured, &banks, &physical);
    let source_release = meter::Phase::begin();
    drop(inputs); // warm binding and execution below have no source TD
    report(source_release.finish("source_release"), &banks, &physical);
    let phase = meter::Phase::begin();
    let mut credits = [0; WorkClass::ALL.len()];
    for i in 0..o.population {
        instances.push(Instance::new(
            &definitions[o.mapping(i)],
            at(i),
            o.slots,
            i,
            &mut credits,
        ));
    }
    let mut measured = phase.finish("idle_instances");
    measured["work"] = work_json(&credits);
    report(measured, &banks, &physical);

    hard_limits(&definitions[0]);
    if o.driver == "engine" {
        lifecycle(&o, &mut instances, &definitions, &banks, &peer).await;
    }
    // Warm the same real lanes for both drivers before measuring sustained work.
    for i in 0..o.lanes {
        let mut handle = Handle::start(&o, &instances[i], &definitions[o.mapping(i)], &mut work());
        let result = poll_fn(|cx| handle.poll(cx, &mut work())).await;
        result.check(&expected[o.mapping(i)]);
        drop(result);
        drop(handle);
    }
    load(&o, &instances, &definitions, &expected, &banks, &physical).await;

    if o.driver == "engine" {
        let phase = meter::Phase::begin();
        let mut calls: Vec<_> = (0..o.lanes)
            .map(|i| instances[i].start(&mut work()).unwrap())
            .collect();
        report(phase.finish("admitted_calls"), &banks, &physical);
        let phase = meter::Phase::begin();
        for (i, call) in calls.iter_mut().enumerate() {
            let result = poll_fn(|cx| call.poll(cx, now(), &mut work())).await;
            assert_eq!(
                result.result().unwrap().data().unwrap().body,
                &expected[o.mapping(i)]
            );
            retained.push(result);
        }
        drop(calls);
        report(phase.finish("retained_results"), &banks, &physical);
    }
    let phase = meter::Phase::begin();
    drop(expected);
    report(phase.finish("fixture_release"), &banks, &physical);
    let phase = meter::Phase::begin();
    drop(instances);
    drop(definitions);
    drop(registry);
    report(phase.finish("retire_roots"), &banks, &physical);
    if !retained.is_empty() {
        assert!(weak_data.upgrade().is_some() && weak_content.upgrade().is_some());
    }
    let phase = meter::Phase::begin();
    drop(retained);
    assert!(weak_data.upgrade().is_none() && weak_content.upgrade().is_none());
    drop(weak_data);
    drop(weak_content);
    let measured = phase.finish("release_results");
    for b in banks.iter().chain(&physical) {
        clean(b);
    }
    report(measured, &banks, &physical);
    emit(json!({"phase": "peer_final", "stats": peer.stats()}));
    let phase = meter::Phase::begin();
    drop(banks);
    drop(physical);
    emit(phase.finish("release_accounts"));
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    if args.first().is_some_and(|s| s == "--peer") {
        runtime.block_on(peer::serve(
            args[1].parse().unwrap(),
            args[2].parse().unwrap(),
        ));
    } else {
        let options = Options::parse(&args);
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(180), run(options))
                .await
                .expect("workload timed out");
        });
    }
}
