use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::RefCell,
};
mod pool;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    pub attempts: usize,
    pub allocations: usize,
    pub releases: usize,
    pub live: usize,
    pub peak: usize,
    pub largest: usize,
}
struct State {
    on: bool,
    fail: usize,
    counts: Counts,
    pointers: [usize; 512],
    sizes: [usize; 512],
    aligns: [usize; 512],
    requests: [(usize, usize); 1024],
    request_count: usize,
    source_ledger: *mut clinkz_wot_foundation::AdmissionLedger,
    parent: *mut clinkz_wot_foundation::ResourceAccount,
    source_mode: bool,
    source_flags: [bool; 512],
    pool_mode: bool,
}
impl State {
    const EMPTY: Self = Self {
        on: false,
        fail: usize::MAX,
        counts: Counts {
            attempts: 0,
            allocations: 0,
            releases: 0,
            live: 0,
            peak: 0,
            largest: 0,
        },
        pointers: [0; 512],
        sizes: [0; 512],
        aligns: [0; 512],
        requests: [(0, 0); 1024],
        request_count: 0,
        source_ledger: core::ptr::null_mut(),
        parent: core::ptr::null_mut(),
        source_mode: false,
        source_flags: [false; 512],
        pool_mode: false,
    };
}
// Borrow fixed metadata in place; allocator callbacks must not copy the entire
// pointer/request catalog into unaccounted temporary owners.
thread_local! {static STATE:RefCell<State>=const {RefCell::new(State::EMPTY)};}
pub struct Allocator;
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let fail = STATE
            .try_with(|s| {
                let mut v = s.borrow_mut();
                if !v.on {
                    return false;
                }
                let index = v.request_count;
                v.requests[index] = (l.size(), l.align());
                v.request_count += 1;
                v.counts.attempts += 1;
                let fail = v.counts.attempts == v.fail;
                fail
            })
            .unwrap_or(false);
        if fail {
            return std::ptr::null_mut();
        }
        let source = STATE
            .try_with(|s| {
                let v = s.borrow();
                v.on && v.source_mode
            })
            .unwrap_or(false);
        if source {
            STATE.with(|s| {
                let v = s.borrow();
                unsafe {
                    (&mut *v.source_ledger)
                        .try_reserve_source(
                            clinkz_wot_foundation::ResourceKind::RetainedSourceBytesGlobalMax,
                            l.size() as u64,
                        )
                        .unwrap()
                        .commit();
                    (&mut *v.parent)
                        .try_reserve(l.size() as u64)
                        .unwrap()
                        .commit();
                }
            });
        }
        let use_pool = STATE
            .try_with(|s| {
                let v = s.borrow();
                v.on && v.pool_mode
            })
            .unwrap_or(false);
        let p = if use_pool {
            pool::allocate(l)
        } else {
            unsafe { System.alloc(l) }
        };
        if source && p.is_null() {
            STATE.with(|s| {
                let v = s.borrow();
                unsafe {
                    assert!((&mut *v.source_ledger).release_source(l.size() as u64));
                    assert!((&mut *v.parent).release_committed(l.size() as u64));
                }
            });
        }
        if !p.is_null() {
            let _ = STATE.try_with(|s| {
                let mut v = s.borrow_mut();
                if v.on {
                    let n = v
                        .pointers
                        .iter()
                        .position(|p| *p == 0)
                        .expect("test allocation catalog exceeded");
                    v.pointers[n] = p as usize;
                    v.sizes[n] = l.size();
                    v.aligns[n] = l.align();
                    v.source_flags[n] = source;
                    v.counts.allocations += 1;
                    v.counts.live += l.size();
                    v.counts.peak = v.counts.peak.max(v.counts.live);
                    v.counts.largest = v.counts.largest.max(l.size());
                }
            });
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // Release the actual storage before either the observer or a parent
        // account reports the child dead. Pool metadata/backing is prepaid.
        if pool::contains(p) {
            pool::release(p, l);
        } else {
            unsafe {
                System.dealloc(p, l);
            }
        }
        let _ = STATE.try_with(|s| {
            let mut v = s.borrow_mut();
            if v.on
                && let Some(n) = v.pointers.iter().position(|v| *v == p as usize)
            {
                assert_eq!((v.sizes[n], v.aligns[n]), (l.size(), l.align()));
                if v.source_flags[n] {
                    unsafe {
                        assert!((&mut *v.source_ledger).release_source(l.size() as u64));
                        assert!((&mut *v.parent).release_committed(l.size() as u64));
                    }
                }
                let bytes = v.sizes[n];
                v.counts.live -= bytes;
                v.counts.releases += 1;
                v.pointers[n] = 0;
            }
        });
    }
}
pub struct Guard;
pub fn start(fail: Option<usize>) -> Guard {
    STATE.with(|s| {
        assert!(!s.borrow().on);
        let mut v = State::EMPTY;
        v.on = true;
        v.fail = fail.unwrap_or(usize::MAX);
        s.replace(v);
    });
    Guard
}
pub fn counts() -> Counts {
    STATE.with(|s| s.borrow().counts)
}
impl Drop for Guard {
    fn drop(&mut self) {
        STATE.with(|s| {
            let mut v = s.borrow_mut();
            if v.pool_mode {
                assert_eq!(v.counts.live, 0);
                pool::assert_empty();
            }
            v.on = false;
        });
    }
}

pub fn start_pool() -> Guard {
    let guard = start(None);
    pool::reset();
    STATE.with(|s| {
        let mut v = s.borrow_mut();
        v.pool_mode = true;
    });
    guard
}
/// Caller-reserved TLS storage: all arena bytes, alignment gaps, placement and
/// observer metadata, including unused capacity. Never an allocator request.
pub fn pool_owner_layout() -> Layout {
    Layout::new::<RefCell<State>>()
        .extend(pool::owner_layout())
        .unwrap()
        .0
        .pad_to_align()
}
pub fn pool_observations() -> pool::Observations {
    pool::observations()
}

pub fn requests() -> ([(usize, usize); 1024], usize) {
    STATE.with(|s| {
        let v = s.borrow();
        (v.requests, v.request_count)
    })
}

/// Owns exclusive access to the real Foundation accounts while the allocator
/// funds actual upstream layouts. Ordinary serde provisioning remains outside
/// bounded admission; this only observes original source ownership/charges.
pub struct SourceGuard<'a> {
    ledger: *mut clinkz_wot_foundation::AdmissionLedger,
    parent: *mut clinkz_wot_foundation::ResourceAccount,
    _borrow: core::marker::PhantomData<(
        &'a mut clinkz_wot_foundation::AdmissionLedger,
        &'a mut clinkz_wot_foundation::ResourceAccount,
    )>,
    _guard: Guard,
}
pub fn source_start<'a>(
    ledger: &'a mut clinkz_wot_foundation::AdmissionLedger,
    parent: &'a mut clinkz_wot_foundation::ResourceAccount,
) -> SourceGuard<'a> {
    let guard = start(None);
    STATE.with(|s| {
        let mut v = s.borrow_mut();
        v.source_ledger = ledger;
        v.parent = parent;
        v.source_mode = true;
    });
    SourceGuard {
        ledger,
        parent,
        _borrow: core::marker::PhantomData,
        _guard: guard,
    }
}
impl SourceGuard<'_> {
    pub fn lend(&self) {
        STATE.with(|s| {
            let mut v = s.borrow_mut();
            v.source_mode = false;
        });
    }
    pub fn source_live(&self) -> u64 {
        unsafe { (*self.ledger).live_bytes() }
    }
    pub fn parent_live(&self) -> u64 {
        unsafe { (*self.parent).used() }
    }
    pub fn reserve_children(&mut self, n: u64) -> bool {
        unsafe {
            match (&mut *self.parent).try_reserve(n) {
                Some(v) => {
                    v.commit();
                    true
                }
                None => false,
            }
        }
    }
    pub fn release_children(&mut self, n: u64) {
        assert_eq!(counts().live as u64, self.source_live());
        unsafe {
            assert!((&mut *self.parent).release_committed(n));
        }
    }
}
impl Drop for SourceGuard<'_> {
    fn drop(&mut self) {
        assert_eq!(self.source_live(), 0);
        assert_eq!(self.parent_live(), 0);
    }
}
