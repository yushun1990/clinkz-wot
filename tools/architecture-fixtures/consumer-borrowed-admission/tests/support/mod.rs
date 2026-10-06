use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    pub attempts: usize,
    pub allocations: usize,
    pub releases: usize,
    pub live: usize,
    pub peak: usize,
    pub largest: usize,
}
#[derive(Clone, Copy)]
struct State {
    on: bool,
    fail: usize,
    counts: Counts,
    pointers: [usize; 512],
    sizes: [usize; 512],
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
    };
}
thread_local! {static STATE:Cell<State>=const {Cell::new(State::EMPTY)};}
pub struct Allocator;
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let fail = STATE
            .try_with(|s| {
                let mut v = s.get();
                if !v.on {
                    return false;
                }
                v.counts.attempts += 1;
                let fail = v.counts.attempts == v.fail;
                s.set(v);
                fail
            })
            .unwrap_or(false);
        if fail {
            return std::ptr::null_mut();
        }
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            let _ = STATE.try_with(|s| {
                let mut v = s.get();
                if v.on {
                    let n = v
                        .pointers
                        .iter()
                        .position(|p| *p == 0)
                        .expect("test allocation catalog exceeded");
                    v.pointers[n] = p as usize;
                    v.sizes[n] = l.size();
                    v.counts.allocations += 1;
                    v.counts.live += l.size();
                    v.counts.peak = v.counts.peak.max(v.counts.live);
                    v.counts.largest = v.counts.largest.max(l.size());
                    s.set(v);
                }
            });
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        let _ = STATE.try_with(|s| {
            let mut v = s.get();
            if v.on
                && let Some(n) = v.pointers.iter().position(|v| *v == p as usize)
            {
                v.counts.live -= v.sizes[n];
                v.counts.releases += 1;
                v.pointers[n] = 0;
                s.set(v);
            }
        });
        unsafe { System.dealloc(p, l) };
    }
}
pub struct Guard;
pub fn start(fail: Option<usize>) -> Guard {
    STATE.with(|s| {
        assert!(!s.get().on);
        let mut v = State::EMPTY;
        v.on = true;
        v.fail = fail.unwrap_or(usize::MAX);
        s.set(v);
    });
    Guard
}
pub fn counts() -> Counts {
    STATE.with(|s| s.get().counts)
}
impl Drop for Guard {
    fn drop(&mut self) {
        STATE.with(|s| {
            let mut v = s.get();
            v.on = false;
            s.set(v);
        });
    }
}
