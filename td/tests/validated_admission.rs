//! Exercise the frozen production entry as a normal public caller, including
//! actual allocator requests, failed allocation, abandonment and cancellation.
#![cfg(feature = "validated-thing")]
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, ResourceKind as R, SlotIndex,
    StaticResourceProfile, WorkBudget, WorkClass,
};
use clinkz_wot_td::{
    ValidatedThingAdmissionConfig, ValidatedThingCause as Cause, ValidatedThingCursor,
    ValidatedThingFailureKind, ValidatedThingPhase, ValidatedThingProgress as Progress,
    thing::Thing, validate::Validate,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
#[derive(Clone, Copy)]
struct Observation {
    active: bool,
    fail: usize,
    attempts: usize,
    live: usize,
    peak: usize,
    freed: usize,
    sizes: [usize; 16],
    aligns: [usize; 16],
    requests: usize,
}
const EMPTY: Observation = Observation {
    active: false,
    fail: 0,
    attempts: 0,
    live: 0,
    peak: 0,
    freed: 0,
    sizes: [0; 16],
    aligns: [0; 16],
    requests: 0,
};
std::thread_local! {static OBS:Cell<Observation>=const {Cell::new(EMPTY)};}
struct Allocator;
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let fail = OBS
            .try_with(|slot| {
                let mut o = slot.get();
                if !o.active {
                    return false;
                }
                o.attempts += 1;
                let fail = o.fail == o.attempts;
                if !fail {
                    o.sizes[o.requests] = layout.size();
                    o.aligns[o.requests] = layout.align();
                    o.requests += 1;
                    o.live += layout.size();
                    o.peak = o.peak.max(o.live);
                }
                slot.set(o);
                fail
            })
            .unwrap_or(false);
        if fail {
            std::ptr::null_mut()
        } else {
            unsafe { System.alloc(layout) }
        }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // Admission releases the physical child while its original charge is
        // still live; the observer sees only allocation Layouts, never estimates.
        unsafe {
            System.dealloc(pointer, layout);
        }
        let _ = OBS.try_with(|slot| {
            let mut o = slot.get();
            if o.active {
                o.live -= layout.size();
                o.freed += 1;
                slot.set(o);
            }
        });
    }
}
fn observe(fail: usize) {
    OBS.with(|o| {
        o.set(Observation {
            active: true,
            fail,
            ..EMPTY
        })
    });
}
fn stop() -> Observation {
    OBS.with(|slot| {
        let mut o = slot.get();
        o.active = false;
        slot.set(o);
        o
    })
}
fn budget() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 10_000))
}
fn ledger() -> AdmissionLedger {
    AdmissionLedger::new(
        SlotIndex::new(1),
        Generation::new(1).unwrap(),
        0,
        8 << 20,
        0,
        0,
        0,
        0,
    )
}
fn thing() -> Thing {
    serde_json::from_str(r#"{
    "@context":"https://www.w3.org/2022/wot/td/v1.1","title":"public caller",
    "security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},
    "properties":{"p":{"type":"object","properties":{"inner":{"type":"array","items":{"type":"string"}}},"forms":[{"href":"/p"}]}},
    "arbitrary":{"array":[1,{"more":[true,null,"original opaque leaf"]}]}
}"#).unwrap()
}
fn checked() -> ValidatedThingAdmissionConfig {
    ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap()
}
fn advance<'a>(mut c: ValidatedThingCursor<'a>, count: usize) -> Option<ValidatedThingCursor<'a>> {
    for _ in 0..count {
        match c.step(&mut budget(), false) {
            Progress::Pending(next) => c = next,
            Progress::Complete(proof) => {
                drop(proof);
                return None;
            }
            Progress::Failed(_) => panic!("valid input failed"),
        }
    }
    Some(c)
}
#[test]
fn actual_requests_growth_overlap_and_preallocation_rejection() {
    let t = thing();
    let config = checked();
    observe(0);
    let mut cursor = ValidatedThingCursor::from_thing(&t, &config, ledger());
    let constructed = OBS.with(Cell::get);
    assert_eq!(constructed.attempts, 0);
    let mut steps = 0;
    loop {
        steps += 1;
        match cursor.step(&mut budget(), false) {
            Progress::Pending(c) => cursor = c,
            Progress::Complete(proof) => {
                drop(proof);
                break;
            }
            Progress::Failed(_) => panic!("valid typed input failed"),
        }
    }
    let o = stop();
    assert_eq!(o.live, 0);
    assert_eq!(o.freed, o.requests);
    assert!(o.requests >= 3);
    assert!(steps > 100);
    assert!(o.peak > *o.sizes[..o.requests].iter().max().unwrap());
    for i in 1..o.requests {
        assert_eq!(o.sizes[i], 2 * o.sizes[i - 1]);
        assert_eq!(o.aligns[i], o.aligns[0]);
    }
    for (resource, value) in [
        (R::LargestContiguousAllocationBytesMax, 0),
        (R::AdmissionTemporaryBytesPerOperationMax, 0),
        (R::EngineLiveBytesGlobalMax, 0),
    ] {
        let limits = GatewayDefaultV1::LIMITS
            .clone()
            .with_limit(resource, Some(value));
        let config = ValidatedThingAdmissionConfig::try_from_limits(&limits).unwrap();
        observe(0);
        let c = ValidatedThingCursor::from_thing(&t, &config, ledger());
        assert!(matches!(
            c.step(&mut budget(), false),
            Progress::Failed(Cause::Limit(_))
        ));
        let o = stop();
        assert_eq!(o.attempts, 0);
        assert_eq!(o.live, 0);
    }
}
#[test]
fn every_allocation_failure_finishes_fixed_cleanup_and_leaves_source_owned_by_caller() {
    let t = thing();
    let config = checked();
    observe(0);
    let mut c = ValidatedThingCursor::from_thing(&t, &config, ledger());
    loop {
        match c.step(&mut budget(), false) {
            Progress::Pending(next) => c = next,
            Progress::Complete(proof) => {
                drop(proof);
                break;
            }
            _ => panic!(),
        }
    }
    let requests = stop().requests;
    for fail in 1..=requests {
        observe(fail);
        let mut c = ValidatedThingCursor::from_thing(&t, &config, ledger());
        let failure = loop {
            match c.step(&mut budget(), false) {
                Progress::Pending(next) => c = next,
                Progress::Failed(Cause::Failed(failure)) => break failure,
                _ => panic!("allocation failure was hidden"),
            }
        };
        let o = stop();
        assert_eq!(failure.kind(), ValidatedThingFailureKind::AllocationFailed);
        assert!(failure.requested_bytes() > 0);
        assert_eq!(o.attempts, fail);
        assert_eq!(o.live, 0);
        assert_eq!(o.requests, o.freed);
        assert_eq!(t.properties.as_ref().unwrap().len(), 1);
    }
}
#[test]
fn every_suspension_can_cancel_or_drop_with_zero_additional_credit() {
    let t = thing();
    let config = checked();
    let mut position = 0;
    loop {
        observe(0);
        let cursor = ValidatedThingCursor::from_thing(&t, &config, ledger());
        let Some(cursor) = advance(cursor, position) else {
            let o = stop();
            assert_eq!(o.live, 0);
            break;
        };
        let snapshot = OBS.with(Cell::get);
        drop(cursor);
        let o = stop();
        assert_eq!(o.live, 0);
        assert_eq!(o.attempts, snapshot.attempts);
        assert_eq!(o.freed, o.requests);
        observe(0);
        let cursor = ValidatedThingCursor::from_thing(&t, &config, ledger());
        let cursor = advance(cursor, position).unwrap();
        let attempts = OBS.with(Cell::get).attempts;
        let cause = match cursor.step(&mut WorkBudget::new(), true) {
            Progress::Failed(cause) => cause,
            _ => panic!("cancel did not consume the input owner"),
        };
        let o = stop();
        assert!(matches!(cause, Cause::Cancelled { .. }));
        assert_eq!(o.live, 0);
        assert_eq!(o.attempts, attempts);
        assert_eq!(o.freed, o.requests);
        position += 1;
    }
    assert!(position > 100);
}
#[test]
fn zero_budget_entry_does_not_allocate_or_traverse() {
    let t = thing();
    let config = checked();
    observe(0);
    let mut c = ValidatedThingCursor::from_thing(&t, &config, ledger());
    for _ in 0..16 {
        c = match c.step(&mut WorkBudget::new(), false) {
            Progress::Pending(next) => next,
            _ => panic!(),
        };
    }
    drop(c);
    let o = stop();
    assert_eq!(o.attempts, 0);
    assert_eq!(o.live, 0);
}

#[test]
fn map_limits_terminate_with_the_configured_maximum_iterator_debit() {
    const MEMBERS: u64 = 32;
    let limits = GatewayDefaultV1::LIMITS
        .clone()
        .with_limit(R::JsonMembersPerObjectMax, Some(MEMBERS));
    let config = ValidatedThingAdmissionConfig::try_from_limits(&limits).unwrap();
    for opaque in [false, true] {
        for members in [MEMBERS - 1, MEMBERS, MEMBERS + 1, 2 * MEMBERS + 1] {
            let mut t = thing();
            let entries = (0..members).map(|i| (format!("field{i:02}"), serde_json::Value::Null));
            t._extra_fields.clear();
            if opaque {
                t._extra_fields.insert(
                    "opaque".into(),
                    serde_json::Value::Object(entries.collect()),
                );
            } else {
                t._extra_fields.extend(entries);
            }
            assert!(t.validate().is_ok());
            observe(0);
            let mut cursor = ValidatedThingCursor::from_thing(&t, &config, ledger());
            let mut steps = 0;
            let (terminal, last_debit) = loop {
                if steps == 20_000 {
                    drop(cursor);
                    break (None, 0);
                }
                steps += 1;
                let mut allowance = budget();
                allowance.set_remaining(WorkClass::DocumentNodes, MEMBERS + 1);
                let progress = cursor.step(&mut allowance, false);
                let debit = MEMBERS + 1 - allowance.remaining(WorkClass::DocumentNodes);
                match progress {
                    Progress::Pending(next) => cursor = next,
                    Progress::Complete(proof) => {
                        drop(proof);
                        break (Some(Ok(())), debit);
                    }
                    Progress::Failed(cause) => break (Some(Err(cause)), debit),
                }
            };
            let o = stop();
            assert_eq!(o.live, 0);
            assert_eq!(o.requests, o.freed);
            if members <= MEMBERS {
                assert_eq!(terminal, Some(Ok(())), "opaque={opaque}, members={members}");
            } else {
                match terminal {
                    Some(Err(Cause::Limit(limit))) => {
                        assert_eq!(limit.kind(), R::JsonMembersPerObjectMax);
                        assert_eq!(limit.configured(), MEMBERS);
                        assert_eq!(limit.observed(), members);
                        assert_eq!(limit.phase(), ValidatedThingPhase::Inspect);
                        assert_eq!(last_debit, 1);
                    }
                    _ => panic!("no map rejection: opaque={opaque}, members={members}"),
                }
            }
        }
    }
}
