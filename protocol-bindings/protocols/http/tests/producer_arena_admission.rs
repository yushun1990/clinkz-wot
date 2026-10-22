#![cfg(feature = "prepare")]
//! Public Producer preparation under independent source/scratch/block limits.
use clinkz_wot_binding_http::HttpProducerConfig;
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, ResourceKind as R, ResourceLimits, SlotIndex,
    StaticResourceProfile, WorkBudget, WorkClass,
};
use clinkz_wot_planning::{ProducerPropertyReadAdmission, ProducerPropertyReadStep};
use clinkz_wot_td::{
    ValidatedProducerStep, ValidatedSemanticError, ValidatedThing, ValidatedThingAdmissionConfig,
    ValidatedThingCause, ValidatedThingCursor, ValidatedThingProgress,
    semantic::ResolvedSemanticDocument, thing::Thing,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

const SOURCE: usize = 131_072;
const TEMPORARY: u64 = 65_536;
const JSON: &[u8] = br#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"Producer arena",
    "securityDefinitions":{"none":{"scheme":"nosec"}},"security":["none"],
    "properties":{"level":{"type":"number","readOnly":true,
    "forms":[{"href":"http://127.0.0.1:38080/level"}]}}}"#;

fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000))
}
fn ledger(source: u64) -> AdmissionLedger {
    let mut ledger = AdmissionLedger::new(
        SlotIndex::new(1),
        Generation::INITIAL,
        SOURCE as u64,
        TEMPORARY,
        0,
        0,
        0,
        0,
    );
    ledger
        .try_reserve_source(R::RetainedSourceBytesPerOwnerMax, source)
        .unwrap()
        .commit();
    assert_eq!(ledger.live_bytes(), source);
    ledger
}
fn proof<'a>(
    thing: &'a Thing,
    limits: &ResourceLimits,
    ledger: AdmissionLedger,
) -> ValidatedThing<'a> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(limits).unwrap();
    let mut cursor = ValidatedThingCursor::from_thing(thing, &config, ledger);
    for _ in 0..100_000 {
        cursor = match cursor.step(&mut work(), false) {
            ValidatedThingProgress::Pending(next) => next,
            ValidatedThingProgress::Complete(proof) => return proof,
            ValidatedThingProgress::Failed(cause) => panic!("Basic admission: {cause:?}"),
        };
    }
    panic!("Basic admission stalled")
}

#[test]
fn retained_source_does_not_consume_producer_temporary_capacity() {
    let mut source = vec![b' '; SOURCE].into_boxed_slice();
    source[..JSON.len()].copy_from_slice(JSON);
    let thing: Thing = serde_json::from_slice(&source).unwrap();
    let limits = GatewayDefaultV1::LIMITS
        .clone()
        .with_limit(R::AdmissionTemporaryBytesPerOperationMax, Some(TEMPORARY))
        .with_limit(R::AdmissionTemporaryBytesGlobalMax, Some(TEMPORARY))
        .with_limit(R::PeakLiveBytesPerAdmissionMax, Some(TEMPORARY));
    let config = HttpProducerConfig::for_loopback("127.0.0.1:38080".parse().unwrap()).unwrap();
    for charged in [0, SOURCE as u64] {
        let mut build =
            ProducerPropertyReadAdmission::new(proof(&thing, &limits, ledger(charged)), "level");
        let mut covered = false;
        for _ in 0..100_000 {
            if build.step(&mut work(), false).unwrap() == ProducerPropertyReadStep::Covered {
                covered = true;
                break;
            }
        }
        assert!(covered);
        let program = build
            .with_input(&mut work(), |input, budget| {
                config.compile_property_read(input, budget)
            })
            .unwrap_or_else(|e| panic!("retained source {charged}: {e:?}"))
            .unwrap();
        drop(build);
        assert_eq!(program.target(), "http://127.0.0.1:38080/level");
    }
    assert_eq!(&source[..JSON.len()], JSON);
}

#[derive(Clone, Copy, Default)]
struct Observation {
    sizes: [usize; 4],
    requests: usize,
    releases: usize,
}
std::thread_local! { static OBSERVATION: Cell<Option<Observation>> = const { Cell::new(None) }; }
struct Allocator;
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        let _ = OBSERVATION.try_with(|cell| {
            if let Some(mut observation) = cell.get() {
                if observation.requests < observation.sizes.len() {
                    observation.sizes[observation.requests] = layout.size();
                }
                observation.requests += 1;
                cell.set(Some(observation));
            }
        });
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe {
            System.dealloc(pointer, layout);
        }
        let _ = OBSERVATION.try_with(|cell| {
            if let Some(mut observation) = cell.get() {
                observation.releases += 1;
                cell.set(Some(observation));
            }
        });
    }
}
struct Observe;
impl Observe {
    fn start() -> Self {
        OBSERVATION.with(|cell| {
            assert!(cell.get().is_none());
            cell.set(Some(Observation::default()));
        });
        Self
    }
    fn finish(self) -> Observation {
        OBSERVATION.with(|cell| cell.take().unwrap())
    }
}
impl Drop for Observe {
    fn drop(&mut self) {
        OBSERVATION.with(|cell| cell.set(None));
    }
}

fn arena(limits: &ResourceLimits) -> (Result<Option<()>, ValidatedSemanticError>, Observation) {
    let thing: Thing = serde_json::from_slice(JSON).unwrap();
    let mut cursor = proof(&thing, limits, ledger(0)).into_producer();
    let mut done = false;
    for _ in 0..100_000 {
        match cursor.step(&mut work(), false).unwrap() {
            ValidatedProducerStep::Done => {
                done = true;
                break;
            }
            ValidatedProducerStep::Ready(_) => cursor.acknowledge(),
            ValidatedProducerStep::Pending => {}
        }
    }
    assert!(done);
    let mut budget = work();
    let observation = Observe::start();
    let result = cursor.with_property_semantics("level", 0, &mut budget, |_, _, _| ());
    (result, observation.finish())
}

#[test]
fn contiguous_ceiling_checks_physical_arena_blocks_at_exact_and_one_below() {
    let limits = GatewayDefaultV1::LIMITS.clone();
    let (result, observation) = arena(&limits);
    result.unwrap().unwrap();
    assert_eq!(observation.requests, 2);
    assert_eq!(observation.releases, 2);
    let sizes = &observation.sizes[..observation.requests];
    let largest = *sizes.iter().max().unwrap() as u64;
    let total = sizes.iter().sum::<usize>() as u64;
    assert_eq!(total, ResolvedSemanticDocument::capacity_bytes());
    assert!(largest < total);
    for ceiling in [total - 1, largest, largest - 1] {
        let (result, observation) = arena(
            &limits
                .clone()
                .with_limit(R::LargestContiguousAllocationBytesMax, Some(ceiling)),
        );
        if ceiling < largest {
            assert!(
                matches!(result, Err(ValidatedSemanticError::Admission(ValidatedThingCause::Limit(limit)))
                if limit.kind() == R::LargestContiguousAllocationBytesMax && limit.configured() == ceiling && limit.observed() == largest)
            );
            assert_eq!(observation.requests, 0);
            assert_eq!(observation.releases, 0);
        } else {
            result.unwrap().unwrap();
            assert_eq!(observation.requests, 2);
            assert_eq!(observation.releases, 2);
        }
    }
}
