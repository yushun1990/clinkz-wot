//! Fixed caller-reserved allocator for the native no_std + alloc witness.
//! No request has a hidden malloc header, size-class rounding or dynamic
//! metadata. The whole aligned arena and fixed slot catalog are prepaid once.
use core::{alloc::Layout, cell::UnsafeCell, mem::MaybeUninit, ptr};

const BYTES: usize = 131_072;
const SLOTS: usize = 512;
#[repr(align(64))]
struct Arena([MaybeUninit<u8>; BYTES]);
#[derive(Clone, Copy)]
struct Slot {
    offset: usize,
    bytes: usize,
    alignment: usize,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Observations {
    pub allocations: usize,
    pub releases: usize,
    pub live: usize,
    pub peak: usize,
    pub occupied_span: usize,
    pub alignment_padding: usize,
}
struct Pool {
    arena: Arena,
    slots: [Option<Slot>; SLOTS],
    next: usize,
    observed: Observations,
}
thread_local! {static POOL: UnsafeCell<Pool> = const { UnsafeCell::new(Pool {
    arena: Arena([MaybeUninit::uninit(); BYTES]), slots: [None; SLOTS], next: 0,
    observed: Observations { allocations: 0, releases: 0, live: 0, peak: 0,
        occupied_span: 0, alignment_padding: 0 },
}) };}

pub fn owner_layout() -> Layout {
    Layout::new::<Pool>()
}
pub fn contains(pointer: *mut u8) -> bool {
    POOL.try_with(|cell| {
        // Only reads the arena address. No other allocator call occurs inside.
        let base = unsafe { (*cell.get()).arena.0.as_ptr() as usize };
        (base..base + BYTES).contains(&(pointer as usize))
    })
    .unwrap_or(false)
}
pub fn allocate(layout: Layout) -> *mut u8 {
    POOL.with(|cell| {
        // This thread owns the fixed allocator; methods neither allocate nor
        // call user code, so no nested alias of this Pool can be created.
        let pool = unsafe { &mut *cell.get() };
        let base = pool.arena.0.as_mut_ptr() as *mut u8;
        let Some(slot) = pool.slots.iter().position(Option::is_none) else {
            return ptr::null_mut();
        };
        let Some(aligned) = (base as usize)
            .checked_add(pool.next)
            .and_then(|v| v.checked_add(layout.align() - 1))
            .map(|v| v & !(layout.align() - 1))
        else {
            return ptr::null_mut();
        };
        let offset = aligned - base as usize;
        let Some(end) = offset
            .checked_add(layout.size())
            .filter(|&end| end <= BYTES)
        else {
            return ptr::null_mut();
        };
        pool.slots[slot] = Some(Slot {
            offset,
            bytes: layout.size(),
            alignment: layout.align(),
        });
        pool.observed.alignment_padding += offset - pool.next;
        pool.next = end;
        pool.observed.allocations += 1;
        pool.observed.live += layout.size();
        pool.observed.peak = pool.observed.peak.max(pool.observed.live);
        pool.observed.occupied_span = pool.observed.occupied_span.max(end);
        // Preserve provenance; alignment arithmetic does not manufacture ptrs.
        unsafe { base.add(offset) }
    })
}
pub fn release(pointer: *mut u8, layout: Layout) {
    POOL.with(|cell| {
        let pool = unsafe { &mut *cell.get() };
        let base = pool.arena.0.as_ptr() as usize;
        let offset = pointer as usize - base;
        let index = pool
            .slots
            .iter()
            .position(|s| s.is_some_and(|s| s.offset == offset))
            .unwrap();
        let slot = pool.slots[index].take().unwrap();
        assert_eq!(
            (slot.bytes, slot.alignment),
            (layout.size(), layout.align())
        );
        pool.observed.live -= slot.bytes;
        pool.observed.releases += 1;
        // Fixed scalar/slot release. Reset only when every child is dead.
        if pool.observed.live == 0 {
            pool.next = 0;
        }
    });
}
pub fn assert_empty() {
    POOL.with(|cell| {
        let pool = unsafe { &*cell.get() };
        assert_eq!(pool.observed.live, 0);
        assert!(pool.slots.iter().all(Option::is_none));
    });
}
pub fn reset() {
    assert_empty();
    POOL.with(|cell| unsafe {
        (*cell.get()).observed = Observations::default();
    });
}
pub fn observations() -> Observations {
    POOL.with(|cell| unsafe { (*cell.get()).observed })
}
