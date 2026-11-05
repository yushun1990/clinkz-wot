// Requested allocator bytes on the test thread, including Arc/vector backing.
// This is neither RSS nor allocator metadata/fragmentation/stack measurement.
// Tests use current-thread HTTP runtimes; unrelated test threads have separate
// counters. No production allocator or allocation policy is changed.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
pub struct Allocator;
#[derive(Clone, Copy, Default)]
pub struct Reading {
    pub live: i64,
    pub allocations: u64,
    pub allocated_bytes: u64,
}
thread_local! { static METER: Cell<Reading> = const { Cell::new(Reading { live: 0, allocations: 0, allocated_bytes: 0 }) }; }
pub fn read() -> Reading {
    METER.with(Cell::get)
}
fn change(live: i64, allocations: u64, allocated_bytes: u64) {
    let _ = METER.try_with(|c| {
        let r = c.get();
        c.set(Reading {
            live: r.live + live,
            allocations: r.allocations + allocations,
            allocated_bytes: r.allocated_bytes + allocated_bytes,
        });
    });
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            change(layout.size() as i64, 1, layout.size() as u64);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe {
            System.dealloc(p, layout);
        }
        change(-(layout.size() as i64), 0, 0);
    }
    unsafe fn realloc(&self, p: *mut u8, old: Layout, size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(p, old, size) };
        if !next.is_null() {
            change(size as i64 - old.size() as i64, 1, size as u64);
        }
        next
    }
}
