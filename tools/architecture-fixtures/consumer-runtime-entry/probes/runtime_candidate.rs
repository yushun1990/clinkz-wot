//! Admission discriminators, not an implementation of the Servient registry.
//! Real public output/root/reclaimer; a deliberately small publication model.
use super::*;
use std::sync::{Barrier, Mutex, atomic::AtomicU8};

type Output = ConsumerPropertyReadOwnedOutput<HostBindingArtifact>;

struct Slot {
    epoch: u64,
    cancelled: bool,
    draining: bool,
    output: Option<Output>,
}

#[derive(Debug, PartialEq)]
enum Install {
    Pending,
    Rejected,
    Published,
}

// The exact production final-check debit must be measured during implementation.
// Six is only this fixed model's declared envelope, not a new frozen work cost.
const FINAL_WORK: u64 = 6;
struct Permit;
fn install(
    slot: &Mutex<Slot>,
    epoch: u64,
    output: &mut Option<Output>,
    work: &mut WorkBudget,
) -> Install {
    if work.consume(W::PlanningItems, FINAL_WORK).is_err() {
        return Install::Pending;
    }
    let mut slot = slot.lock().unwrap();
    if slot.epoch != epoch || slot.cancelled || slot.draining || slot.output.is_some() {
        return Install::Rejected;
    }
    assert!(output.is_some());
    let _permit = Permit;
    slot.output = output.take();
    Install::Published
}

fn prepared() -> (Output, ResourceAccount) {
    let source = source();
    let stats = Arc::new(Stats::default());
    let registration =
        production::host_registration(ResolvedTargetCompiler::try_new(64).unwrap(), stats);
    let LocalOwner {
        draft,
        root,
        account,
    } = prepare(&source, &registration, usize::MAX).unwrap_or_else(|_| panic!("prepare"));
    disarm();
    drop(source);
    drop(registration);
    let output = without_heap(|| {
        ConsumerPropertyReadOwnedOutput::try_from_local(draft, root, &mut budget(1))
    })
    .unwrap_or_else(|_| panic!("detach"));
    (output, account)
}

#[test]
fn publication_model_short_credit_stale_slot_and_both_cancel_orders() {
    for cancel_first in [true, false] {
        let (output, account) = prepared();
        let mut output = Some(output);
        let slot = Mutex::new(Slot {
            epoch: 17,
            cancelled: false,
            draining: false,
            output: None,
        });
        for n in [0, FINAL_WORK - 1] {
            let mut work = budget(n);
            assert_eq!(
                without_heap(|| install(&slot, 17, &mut output, &mut work)),
                Install::Pending
            );
            assert_eq!(work.remaining(W::PlanningItems), n);
            assert!(output.is_some());
            assert!(slot.lock().unwrap().output.is_none());
        }
        assert_eq!(
            without_heap(|| install(&slot, 16, &mut output, &mut budget(FINAL_WORK))),
            Install::Rejected
        );
        assert!(output.is_some());
        let ordered = Barrier::new(2);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                if !cancel_first {
                    ordered.wait();
                }
                let mut slot = slot.lock().unwrap();
                slot.cancelled = true;
                // Cancellation after publication closes new pins; it cannot undo install.
                slot.draining = slot.output.is_some();
                drop(slot);
                if cancel_first {
                    ordered.wait();
                }
            });
            if cancel_first {
                ordered.wait();
            }
            let mut work = budget(FINAL_WORK);
            assert_eq!(
                without_heap(|| install(&slot, 17, &mut output, &mut work)),
                if cancel_first {
                    Install::Rejected
                } else {
                    Install::Published
                }
            );
            assert_eq!(work.remaining(W::PlanningItems), 0);
            if !cancel_first {
                ordered.wait();
            }
        });
        let mut slot = slot.into_inner().unwrap();
        assert_eq!(slot.draining, !cancel_first);
        let output = output.or_else(|| slot.output.take()).unwrap();
        let (draft, root) = output.into_local();
        reclaim(draft.into_rollback(), root, account);
        assert_eq!(trace().node_releases, 1);
    }
}

// One prepaid detach credit makes normal scope exit and caller-disinterest
// scope exit identical. The caller holds only the separate fixed header.
struct Checkout<'a> {
    parked: &'a mut Option<Output>,
    local: Option<(
        ConsumerPropertyReadDraft<HostBindingArtifact>,
        ConsumerPropertyReadCleanupSlot<HostBindingArtifact>,
    )>,
    return_work: WorkBudget,
}
impl<'a> Checkout<'a> {
    fn take(parked: &'a mut Option<Output>, work: &mut WorkBudget) -> Option<Self> {
        work.consume(W::PlanningItems, 1).ok()?;
        let local = parked.take()?.into_local();
        Some(Self {
            parked,
            local: Some(local),
            return_work: WorkBudget::new().with_remaining(W::PlanningItems, 1),
        })
    }
}
impl Drop for Checkout<'_> {
    fn drop(&mut self) {
        let (draft, root) = self.local.take().unwrap();
        *self.parked = Some(
            ConsumerPropertyReadOwnedOutput::try_from_local(draft, root, &mut self.return_work)
                .unwrap_or_else(|_| panic!("matching exclusive pair")),
        );
    }
}

#[test]
fn checked_out_public_graph_survives_caller_disinterest_without_callback_lock() {
    let (output, account) = prepared();
    let capacity = account.used();
    let mut parked = Some(output);
    assert!(Checkout::take(&mut parked, &mut budget(0)).is_none());
    assert!(parked.is_some());
    let header = Mutex::new(false);
    let checkout = without_heap(|| Checkout::take(&mut parked, &mut budget(1))).unwrap();
    let (draft, _) = checkout.local.as_ref().unwrap();
    let selected =
        select_consumer_property_read_aggregate(draft, "a-read", &InteractionOptions::default())
            .unwrap();
    let reference = selected.artifact_ref();
    let address = selected.artifact() as *const _ as usize;
    std::thread::scope(|scope| {
        scope
            .spawn(|| *header.lock().unwrap() = true)
            .join()
            .unwrap();
    });
    assert!(
        *header.try_lock().unwrap(),
        "no header lock spans selection/driver work"
    );
    assert_eq!(account.used(), capacity);
    without_heap(|| drop(checkout));
    let (draft, root) = parked.take().unwrap().into_local();
    let selected =
        select_consumer_property_read_aggregate(&draft, "a-read", &InteractionOptions::default())
            .unwrap();
    assert_eq!(selected.artifact_ref(), reference);
    assert_eq!(selected.artifact() as *const _ as usize, address);
    reclaim(draft.into_rollback(), root, account);
    assert_eq!(trace().node_releases, 1);
}

#[test]
fn terminal_local_discard_uses_one_finite_budget_and_releases_real_parent_last() {
    let (output, account) = prepared();
    let held = account.used();
    let mut parent = ResourceAccount::new(
        SlotIndex::new(91),
        Generation::INITIAL,
        K::CleanupBytesMax,
        held,
    );
    parent.try_reserve(held).unwrap().commit();
    assert!(parent.try_reserve(1).is_none());
    // Preserve the existing pointer/Layout records while observing the whole
    // discard interval. Expected child/root deallocations remain allowed.
    let before = trace();
    TRACE.with(|t| {
        t.set(Trace {
            armed: true,
            ..before
        })
    });
    let report = AtomicU8::new(0);
    let (draft, mut root) = output.into_local();
    let footprint = draft.footprint().retained_bytes();
    // Every scope, even empty text, has an accounted Box<str> table entry.
    // Retained bytes therefore conservatively bound the scope item count.
    let items = footprint
        .checked_add(draft.property_count() as u64)
        .and_then(|n| n.checked_add(draft.coordinate_count() as u64))
        .and_then(|n| n.checked_add(3)) // two tables and the terminal operation
        .unwrap();
    // Each progressing call pays at least a byte or an item. Production can
    // obtain tighter item counts from its retained paid preflight.
    let steps = items.checked_add(footprint).unwrap();
    let mut rollback = draft.into_rollback();
    let mut terminal_work = WorkBudget::new()
        .with_remaining(W::PlanningItems, items)
        .with_remaining(W::CleanupItems, items)
        .with_remaining(W::CodecInputBytes, footprint);
    let mut done = false;
    let mut iterations = 0;
    // Residual disposition is committed BEFORE any local object destruction.
    report.store(1, Ordering::SeqCst);
    for _ in 0..steps {
        iterations += 1;
        assert_eq!(parent.used(), held);
        assert_eq!(account.used(), held);
        match rollback.step(&mut terminal_work) {
            ConsumerPropertyReadRollbackStep::Pending(next) => rollback = next,
            ConsumerPropertyReadRollbackStep::Complete(_) => {
                done = true;
                break;
            }
        }
    }
    assert!(done, "finite terminal allowance exhausted; never refill it");
    assert_eq!(
        report.load(Ordering::SeqCst),
        1,
        "discard cannot report successful cleanup"
    );
    assert!(root.take().is_none());
    release_root(root, account);
    assert_eq!(trace().node_releases, 1);
    assert!(parent.release_committed(held));
    assert_eq!(parent.used(), 0);
    let mut after = trace();
    after.armed = before.armed;
    TRACE.with(|t| t.set(after));
    assert_eq!(
        after.requests, before.requests,
        "terminal discard allocated"
    );
    println!(
        "terminal witness: byte allowance={footprint}, item allowance={items}, step bound={steps}, actual steps={iterations}"
    );
    // Neither parent ceiling equality nor child metadata was used as a permit.
}
