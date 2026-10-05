//! Fixed thread-local allocator observation shared by construction fixtures.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
#[derive(Clone, Copy, Debug, Default)]
pub struct Observation {
    pub attempts: usize,
    pub allocations: usize,
    pub releases: usize,
    pub reallocations: usize,
    pub live: i64,
    pub peak: usize,
    pub largest: usize,
    pub fail_at: usize,
}
thread_local! { pub static OBSERVER: Cell<Option<Observation>> = const { Cell::new(None) }; }
struct Allocator;
// SAFETY: unchanged pointer/Layout pairs are forwarded. Fixed thread-local
// scalars track only this test thread; failed requests preserve live truth.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let fail = OBSERVER.with(|cell| {
            let Some(mut record) = cell.get() else {
                return false;
            };
            record.attempts += 1;
            record.largest = record.largest.max(layout.size());
            let fail = record.attempts == record.fail_at;
            cell.set(Some(record));
            fail
        });
        if fail {
            return std::ptr::null_mut();
        }
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            OBSERVER.with(|cell| {
                if let Some(mut record) = cell.get() {
                    record.allocations += 1;
                    record.live += layout.size() as i64;
                    record.peak = record.peak.max(record.live.max(0) as usize);
                    cell.set(Some(record));
                }
            });
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        OBSERVER.with(|cell| {
            if let Some(mut record) = cell.get() {
                record.releases += 1;
                record.live -= layout.size() as i64;
                cell.set(Some(record));
            }
        });
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        OBSERVER.with(|cell| {
            if let Some(mut record) = cell.get() {
                record.reallocations += 1;
                cell.set(Some(record));
            }
        });
        unsafe { System.realloc(pointer, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
pub fn observe<T>(fail_at: usize, operation: impl FnOnce() -> T) -> (T, Observation) {
    OBSERVER.with(|cell| {
        assert!(cell.get().is_none());
        cell.set(Some(Observation {
            fail_at,
            ..Observation::default()
        }));
    });
    let result = operation();
    (result, OBSERVER.with(|cell| cell.replace(None).unwrap()))
}
