use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::RefCell,
    ptr,
};
use validated_thing_arena_layout_probe::{
    Error,
    staged::{Site, Storage},
};
use validated_thing_value_construction_probe::{
    Cause, Cursor, Failure, Limits, OwnedValue, Phase, Progress,
};

#[allow(dead_code)]
mod support;
use support::{budget, drive};

#[derive(Clone, Copy, Default)]
struct Block {
    pointer: usize,
    bytes: usize,
    source: bool,
}

#[derive(Clone, Copy, Default)]
struct Observation {
    active: bool,
    fail_request: usize,
    requests: usize,
    allocations: usize,
    releases: usize,
    live: usize,
    peak: usize,
    largest: usize,
    live_blocks: usize,
    max_live_blocks: usize,
    source_request: bool,
    temporary: usize,
    temporary_peak: usize,
    blocks: [Block; 16],
}

thread_local! { static OBSERVATION: RefCell<Observation> = const { RefCell::new(Observation { active: false, fail_request: 0, requests: 0, allocations: 0, releases: 0, live: 0, peak: 0, largest: 0, live_blocks: 0, max_live_blocks: 0, source_request: false, temporary: 0, temporary_peak: 0, blocks: [Block { pointer: 0, bytes: 0, source: false }; 16] }) }; }

struct ObservedAllocator;
// SAFETY: forwards all blocks to System with their original Layout. The
// thread-local observer owns only fixed scalar records, never allocates, and
// ignores allocations/deallocations belonging to the caller baseline.
unsafe impl GlobalAlloc for ObservedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let deny = OBSERVATION
            .try_with(|state| {
                let mut state = state.borrow_mut();
                if !state.active {
                    return false;
                }
                state.requests += 1;
                state.fail_request != 0 && state.fail_request == state.requests
            })
            .unwrap_or(false);
        if deny {
            return ptr::null_mut();
        }
        // SAFETY: same checked caller Layout forwarded to System.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            let _ = OBSERVATION.try_with(|state| {
                let mut state = state.borrow_mut();
                if !state.active {
                    return;
                }
                let slot = state
                    .blocks
                    .iter()
                    .position(|block| block.pointer == 0)
                    .expect("owning allocation catalog exceeded observer capacity");
                state.blocks[slot] = Block {
                    pointer: pointer as usize,
                    bytes: layout.size(),
                    source: state.source_request,
                };
                if !state.source_request {
                    state.temporary += layout.size();
                    state.temporary_peak = state.temporary_peak.max(state.temporary);
                }
                state.live += layout.size();
                state.allocations += 1;
                state.live_blocks += 1;
                state.peak = state.peak.max(state.live);
                state.largest = state.largest.max(layout.size());
                state.max_live_blocks = state.max_live_blocks.max(state.live_blocks);
            });
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        let _ = OBSERVATION.try_with(|state| {
            let mut state = state.borrow_mut();
            if !state.active {
                return;
            }
            if let Some(slot) = state
                .blocks
                .iter()
                .position(|block| block.pointer == pointer as usize)
            {
                assert_eq!(state.blocks[slot].bytes, layout.size());
                if !state.blocks[slot].source {
                    state.temporary -= layout.size();
                }
                state.live -= layout.size();
                state.live_blocks -= 1;
                state.releases += 1;
                state.blocks[slot] = Block::default();
            }
        });
        // SAFETY: the original pointer/Layout pair is unchanged.
        unsafe { System.dealloc(pointer, layout) };
    }
}
#[global_allocator]
static ALLOCATOR: ObservedAllocator = ObservedAllocator;

fn start(fail_request: usize) {
    OBSERVATION.with(|state| {
        assert_eq!(state.borrow().live, 0);
        *state.borrow_mut() = Observation {
            active: true,
            fail_request,
            ..Observation::default()
        };
    });
}
fn snapshot() -> Observation {
    OBSERVATION.with(|state| *state.borrow())
}
fn end() -> Observation {
    OBSERVATION.with(|state| {
        let mut state = state.borrow_mut();
        state.active = false;
        *state
    })
}

#[test]
fn failed_replacement_reservation_is_not_a_physical_conversion_peak() {
    start(2);
    let mut storage = Storage::<u64>::new(10_000, 10_000, 10_000, 10_000);
    storage.begin_grow(Site::Nodes, 2).unwrap();
    storage.copy_one();
    assert_eq!(storage.begin_grow(Site::Nodes, 4), Err(Error::Allocation));
    let report = storage.footprint();
    let observed = snapshot();
    assert_eq!(observed.requests, 2);
    assert_eq!(report.conversion_peak_bytes, observed.peak as u64);
    assert_eq!(report.reservation_peak_bytes, 120);
    assert_eq!(report.largest_request_bytes, 80);
    assert_eq!(storage.live_bytes(), observed.live as u64);
    drop(storage);
    assert_eq!(end().live, 0);
}

// Keep failure resource observations inline inside the allocator interval.
#[allow(clippy::result_large_err)]
fn tracked_drive(mut cursor: Cursor<'_>, step: u64) -> Result<OwnedValue, Failure> {
    loop {
        // With one structural unit, a phase transition and a following
        // allocation cannot occur in the same step. This independently labels
        // actual allocator requests for the temporary-peak assertion at step=1.
        OBSERVATION.with(|state| state.borrow_mut().source_request = cursor.phase() == Phase::Seal);
        match cursor.step(&mut budget(step), false) {
            Progress::Pending(next) => cursor = next,
            Progress::Complete(value) => return Ok(value),
            Progress::Failed(failure) => return Err(failure),
        }
    }
}

#[test]
fn observed_layouts_match_the_ledger_and_each_actual_request_can_fail_safely() {
    let text = r#"{"z":{"discard":[true,"old"]},"a":[1e0,"long Unicode 中文😀",false],"z":{"b":null,"a":true},"other":"retained"}"#.as_bytes();
    let typed: serde_json::Value = serde_json::from_slice(text).unwrap();
    for from_typed in [false, true] {
        for step in [1, 7, 128] {
            start(0);
            let cursor = if from_typed {
                Cursor::from_value(&typed, Limits::default())
            } else {
                Cursor::from_json(text, Limits::default())
            };
            let value = tracked_drive(cursor, step).unwrap();
            let observed = snapshot();
            let report = value.footprint();
            assert_eq!(observed.live as u64, report.retained_requested_bytes);
            assert_eq!(observed.peak as u64, report.conversion_peak_bytes);
            if step == 1 {
                assert_eq!(observed.temporary_peak as u64, report.temporary_peak_bytes);
            }
            assert_eq!(observed.largest as u64, report.largest_request_bytes);
            assert_eq!(observed.allocations as u64, value.allocations());
            assert_eq!(observed.releases as u64, value.releases());
            assert_eq!(
                observed.live_blocks as u64,
                report.retained_allocation_count
            );
            assert_eq!(value.trace().work[3], observed.allocations as u64);
            // The catalog allows seven arenas and one replacement overlap. This
            // implementation observes at most four build sites plus one overlap.
            assert!(observed.max_live_blocks <= 5);
            let requests = observed.requests;
            let query_before = observed.allocations;
            assert_eq!(value.view().get("z").unwrap().member(0).unwrap().0, "a");
            assert_eq!(snapshot().allocations, query_before);
            drop(value);
            let released = end();
            assert_eq!(released.live, 0);
            assert_eq!(released.allocations, released.releases);
            if step == 1 {
                eprintln!(
                    "entry={} source={} temporary_peak={} total_peak={} largest_request={} requests={} cursor_inline={} owner_inline={} progress_inline={}",
                    if from_typed { "typed" } else { "strict" },
                    report.retained_requested_bytes,
                    report.temporary_peak_bytes,
                    report.conversion_peak_bytes,
                    report.largest_request_bytes,
                    requests,
                    std::mem::size_of::<Cursor<'_>>(),
                    std::mem::size_of::<OwnedValue>(),
                    std::mem::size_of::<Progress<'_>>()
                );
            }

            for fail_request in 1..=requests {
                start(fail_request);
                let cursor = if from_typed {
                    Cursor::from_value(&typed, Limits::default())
                } else {
                    Cursor::from_json(text, Limits::default())
                };
                let failure = tracked_drive(cursor, step).err().unwrap();
                let observed = end();
                assert_eq!(failure.cause, Cause::Allocation);
                assert_eq!(observed.requests, fail_request);
                assert_eq!(observed.live, 0);
                assert_eq!(observed.allocations, observed.releases);
                assert_eq!(failure.allocations as usize, observed.allocations);
                assert_eq!(failure.releases as usize, observed.releases);
                assert_eq!(
                    failure.resources.conversion_peak_bytes,
                    observed.peak as u64
                );
                assert_eq!(
                    failure.resources.temporary_peak_bytes,
                    observed.temporary_peak as u64
                );
                assert!(
                    failure.resources.reservation_peak_bytes
                        >= failure.resources.conversion_peak_bytes
                );
                // The rejected allocator request is prepaid too; its reservation
                // is abandoned before every earlier committed arena is released.
                assert_eq!(failure.trace.work[3] as usize, observed.requests);
            }
        }
    }
}

#[test]
fn impossible_literal_grow_or_seal_does_not_prepay_cleanup_even_with_no_lifetime() {
    let input = br#"{}"#;
    let typed = serde_json::json!({});
    for from_typed in [false, true] {
        for cleanup in [0, 1] {
            let limits = Limits {
                contiguous: 0,
                lifetime: if from_typed { 3 } else { 4 },
                ..Limits::default()
            };
            let cursor = if from_typed {
                Cursor::from_value(&typed, limits)
            } else {
                Cursor::from_json(input, limits)
            };
            start(0);
            let Progress::Failed(failure) = cursor.step(
                &mut budget(4096)
                    .with_remaining(clinkz_wot_foundation::WorkClass::CleanupItems, cleanup),
                false,
            ) else {
                panic!("impossible request before Pending");
            };
            let observed = end();
            assert_eq!(failure.cause, Cause::Memory);
            assert_eq!(failure.trace.work[3], 0);
            assert_eq!(observed.requests, 0);
        }
        let limits = Limits {
            source: 0,
            ..Limits::default()
        };
        let cursor = if from_typed {
            Cursor::from_value(&typed, limits)
        } else {
            Cursor::from_json(input, limits)
        };
        let prefix = drive(cursor, 4096)
            .err()
            .unwrap()
            .trace
            .work
            .into_iter()
            .sum();
        for cleanup in [0, 1] {
            let limits = Limits {
                lifetime: prefix,
                ..limits
            };
            let mut cursor = if from_typed {
                Cursor::from_value(&typed, limits)
            } else {
                Cursor::from_json(input, limits)
            };
            start(0);
            loop {
                let mut work = budget(1);
                if cursor.phase() == Phase::Seal {
                    work = work
                        .with_remaining(clinkz_wot_foundation::WorkClass::CleanupItems, cleanup);
                }
                match cursor.step(&mut work, false) {
                    Progress::Pending(next) => cursor = next,
                    Progress::Failed(failure) => {
                        assert_eq!(failure.cause, Cause::Memory);
                        assert_eq!(failure.trace.work.into_iter().sum::<u64>(), prefix);
                        assert_eq!(failure.trace.work[3] as usize, snapshot().requests);
                        break;
                    }
                    Progress::Complete(_) => panic!("source cannot fit"),
                }
            }
            assert_eq!(end().live, 0);
        }
    }
}

#[test]
fn every_step_boundary_can_abandon_both_entries_without_recursive_cleanup() {
    let text = br#"{"z":["old"],"z":[1e0,{"long-key":"value"}],"a":null}"#;
    let typed: serde_json::Value = serde_json::from_slice(text).unwrap();
    for from_typed in [false, true] {
        'stops: for stop in 0..20_000 {
            start(0);
            let mut cursor = if from_typed {
                Cursor::from_value(&typed, Limits::default())
            } else {
                Cursor::from_json(text, Limits::default())
            };
            for _ in 0..stop {
                match cursor.step(&mut budget(1), false) {
                    Progress::Pending(next) => {
                        assert_eq!(next.live_bytes() as usize, snapshot().live);
                        cursor = next;
                    }
                    Progress::Complete(value) => {
                        drop(value);
                        let observed = end();
                        assert_eq!(observed.live, 0);
                        assert_eq!(observed.allocations, observed.releases);
                        break 'stops;
                    }
                    Progress::Failed(failure) => panic!("{failure:?}"),
                }
            }
            drop(cursor);
            let observed = end();
            assert_eq!(observed.live, 0);
            assert_eq!(observed.allocations, observed.releases);
        }
    }
}

#[test]
fn arithmetic_and_rejected_layouts_never_reach_the_allocator() {
    start(0);
    let mut storage = Storage::<u64>::new(u64::MAX, u64::MAX, u64::MAX, u64::MAX);
    assert_eq!(
        storage.begin_grow(Site::Nodes, usize::MAX),
        Err(Error::Arithmetic)
    );
    assert_eq!(storage.live_bytes(), 0);
    drop(storage);
    let observed = end();
    assert_eq!(observed.requests, 0);
    start(0);
    let error = drive(
        Cursor::from_json(
            br#"{"x":true}"#,
            Limits {
                contiguous: 0,
                ..Limits::default()
            },
        ),
        1,
    )
    .err()
    .unwrap();
    let observed = end();
    assert_eq!(error.cause, Cause::Memory);
    assert_eq!(observed.requests, 0);
    // Unit frames are legal only in the completed owner, never an allocation.
    let mut storage = Storage::<()>::new(100, 100, 100, 100);
    assert_eq!(storage.begin_grow(Site::Frames, 2), Err(Error::Arithmetic));
}
