//! Observe actual allocations, with one-shot failure injection. Caller-owned
//! Thing/registration provisioning precedes observation and is not a TD charge.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::RefCell,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    pub attempts: usize,
    pub allocations: usize,
    pub releases: usize,
    pub live: usize,
    pub peak: usize,
}
struct State {
    active: bool,
    fail: usize,
    counts: Counts,
    pointers: [usize; 256],
    layouts: [(usize, usize); 256],
}
thread_local! {
    static STATE: RefCell<State> = const { RefCell::new(State {
        active: false, fail: usize::MAX,
        counts: Counts { attempts: 0, allocations: 0, releases: 0, live: 0, peak: 0 },
        pointers: [0; 256], layouts: [(0,0); 256],
    }) };
}
pub struct Allocator;
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let fail = STATE
            .try_with(|state| {
                let mut s = state.borrow_mut();
                if !s.active {
                    return false;
                }
                s.counts.attempts += 1;
                s.counts.attempts == s.fail
            })
            .unwrap_or(false);
        if fail {
            return core::ptr::null_mut();
        }
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            let _ = STATE.try_with(|state| {
                let mut s = state.borrow_mut();
                if !s.active {
                    return;
                }
                let i = s
                    .pointers
                    .iter()
                    .position(|&p| p == 0)
                    .expect("allocation catalog");
                s.pointers[i] = pointer as usize;
                s.layouts[i] = (layout.size(), layout.align());
                s.counts.allocations += 1;
                s.counts.live += layout.size();
                s.counts.peak = s.counts.peak.max(s.counts.live);
            });
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        let _ = STATE.try_with(|state| {
            let mut s = state.borrow_mut();
            if !s.active {
                return;
            }
            if let Some(i) = s.pointers.iter().position(|&p| p == pointer as usize) {
                assert_eq!(s.layouts[i], (layout.size(), layout.align()));
                s.pointers[i] = 0;
                s.counts.releases += 1;
                s.counts.live -= layout.size();
            }
        });
    }
}
pub struct Guard;
pub fn start(fail: Option<usize>) -> Guard {
    STATE.with(|state| {
        let mut s = state.borrow_mut();
        assert!(!s.active);
        assert!(s.pointers.iter().all(|&p| p == 0));
        s.active = true;
        s.fail = fail.unwrap_or(usize::MAX);
        s.counts = Counts::default();
    });
    Guard
}
pub fn fail_next() {
    STATE.with(|state| {
        let mut s = state.borrow_mut();
        s.fail = s.counts.attempts + 1;
    });
}
pub fn counts() -> Counts {
    STATE.with(|s| s.borrow().counts)
}
impl Drop for Guard {
    fn drop(&mut self) {
        STATE.with(|state| {
            let mut s = state.borrow_mut();
            s.active = false;
            assert_eq!(s.counts.live, 0);
            assert_eq!(s.counts.allocations, s.counts.releases);
        });
    }
}
