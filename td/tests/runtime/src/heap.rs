//! Caller-supplied fixed backing, including metadata, padding and redzones.
//! Provisioning and TD use separate regions so an immutable caller source can
//! stay live while every TD suspension is rerun. No TD algorithm lives here.
use crate::access::Region;
#[cfg(target_os = "none")]
use core::sync::atomic::AtomicUsize;
use core::{
    alloc::{GlobalAlloc, Layout},
    cell::{Cell, UnsafeCell},
    mem::MaybeUninit,
    ptr,
};

pub const REGION_BYTES: usize = 131_072;
const BYTES: usize = REGION_BYTES;
const SLOTS: usize = 256;
const REQUESTS: usize = 32;
const GUARD: usize = 8;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Request {
    pub bytes: usize,
    pub align: usize,
    pub live_before: usize,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Trace {
    pub attempts: usize,
    pub allocations: usize,
    pub releases: usize,
    pub live: usize,
    pub peak: usize,
    pub largest: usize,
    pub span: usize,
    pub requests: [Request; REQUESTS],
}
const EMPTY: Trace = Trace {
    attempts: 0,
    allocations: 0,
    releases: 0,
    live: 0,
    peak: 0,
    largest: 0,
    span: 0,
    requests: [Request {
        bytes: 0,
        align: 0,
        live_before: 0,
    }; REQUESTS],
};
#[derive(Clone, Copy)]
struct Slot {
    offset: usize,
    bytes: usize,
    align: usize,
    isolated: usize,
}
#[repr(C, align(4096))]
struct Arena([MaybeUninit<u8>; BYTES]);
struct Metadata {
    slots: [Option<Slot>; SLOTS],
    next: usize,
    fail: usize,
    trace: Trace,
}
struct Pool {
    arena: UnsafeCell<Arena>,
    metadata: UnsafeCell<Metadata>,
}
impl Pool {
    const fn new() -> Self {
        Self {
            arena: UnsafeCell::new(Arena([MaybeUninit::uninit(); BYTES])),
            metadata: UnsafeCell::new(Metadata {
                slots: [None; SLOTS],
                next: 0,
                fail: 0,
                trace: EMPTY,
            }),
        }
    }
    fn base(&self) -> *mut u8 {
        // No reference to the arena (or the entire mutable heap state) is
        // formed while live suballocations have outstanding raw pointers.
        self.arena.get().cast()
    }
    fn allocate(&self, layout: Layout, record: bool, isolate: bool) -> *mut u8 {
        // Metadata is disjoint from all live allocation ranges. The sole
        // caller holds this loan only until the allocator method returns.
        let m = unsafe { &mut *self.metadata.get() };
        m.trace.attempts += 1;
        if record {
            assert!(
                m.trace.attempts <= REQUESTS,
                "increase observed Layout catalog"
            );
            m.trace.requests[m.trace.attempts - 1] = Request {
                bytes: layout.size(),
                align: layout.align(),
                live_before: m.trace.live,
            };
        }
        if m.fail == m.trace.attempts {
            return ptr::null_mut();
        }
        let Some(index) = m.slots.iter().position(Option::is_none) else {
            return ptr::null_mut();
        };
        let base = self.base();
        let Some(payload) = layout.size().checked_add(GUARD).filter(|&n| n <= BYTES) else {
            return ptr::null_mut();
        };
        // An isolated allocation owns its entire power-of-two page region.
        // Permission changes never cover a sibling, metadata or live frame.
        // Extra alignment/unused page capacity stays in the startup backing,
        // not in the production request Layout or child byte count.
        let isolated = if isolate {
            payload.next_power_of_two().max(4096)
        } else {
            0
        };
        let align = layout.align().max(isolated);
        let Some(aligned) = (base as usize)
            .checked_add(m.next)
            .and_then(|n| n.checked_add(align - 1))
            .map(|n| n & !(align - 1))
        else {
            return ptr::null_mut();
        };
        let offset = aligned - base as usize;
        let Some(end) = offset
            .checked_add(if isolate { isolated } else { payload })
            .filter(|&n| n <= BYTES)
        else {
            return ptr::null_mut();
        };
        m.slots[index] = Some(Slot {
            offset,
            bytes: layout.size(),
            align: layout.align(),
            isolated,
        });
        m.next = end;
        m.trace.allocations += 1;
        m.trace.live += layout.size();
        m.trace.peak = m.trace.peak.max(m.trace.live);
        m.trace.largest = m.trace.largest.max(layout.size());
        m.trace.span = m.trace.span.max(end);
        unsafe {
            base.add(offset + layout.size()).write_bytes(0xa5, GUARD);
            base.add(offset)
        }
    }
    fn release(&self, pointer: *mut u8, layout: Layout) {
        let m = unsafe { &mut *self.metadata.get() };
        let base = self.base();
        let offset = pointer as usize - base as usize;
        let index = m
            .slots
            .iter()
            .position(|s| s.is_some_and(|s| s.offset == offset))
            .expect("release must match a live physical child");
        let slot = m.slots[index].take().unwrap();
        assert_eq!((slot.bytes, slot.align), (layout.size(), layout.align()));
        for i in 0..GUARD {
            assert_eq!(
                unsafe { base.add(offset + slot.bytes + i).read() },
                0xa5,
                "TD wrote past its actual Layout"
            );
        }
        m.trace.live -= slot.bytes;
        m.trace.releases += 1;
        if m.trace.live == 0 {
            m.next = 0;
        }
    }
    fn empty(&self) {
        let m = unsafe { &*self.metadata.get() };
        assert_eq!(m.trace.live, 0);
        assert!(m.slots.iter().all(Option::is_none));
    }
    fn regions(&self) -> impl Iterator<Item = Region> + '_ {
        let m = unsafe { &*self.metadata.get() };
        m.slots.iter().flatten().filter_map(|s| {
            (s.isolated != 0).then_some(Region {
                base: self.base() as usize + s.offset,
                bytes: s.isolated,
            })
        })
    }
}
struct State {
    source: Pool,
    td: Pool,
    observing: Cell<bool>,
    isolate_source: Cell<bool>,
    #[cfg(target_os = "none")]
    controls: AccessControls,
}
#[cfg(target_os = "none")]
pub struct AccessControls {
    pub expected: AtomicUsize,
    pub observed: AtomicUsize,
}
// This binary never starts threads; ARM interrupts are disabled before entry.
// Allocator methods neither allocate, invoke callbacks nor retain a state loan.
unsafe impl Sync for State {}
static STATE: State = State {
    source: Pool::new(),
    td: Pool::new(),
    observing: Cell::new(false),
    isolate_source: Cell::new(false),
    #[cfg(target_os = "none")]
    controls: AccessControls {
        expected: AtomicUsize::new(0),
        observed: AtomicUsize::new(0),
    },
};

pub struct Allocator;
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if STATE.observing.get() {
            STATE.td.allocate(layout, true, layout.align() == 1)
        } else {
            STATE
                .source
                .allocate(layout, false, STATE.isolate_source.get())
        }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        let base = STATE.td.base() as usize;
        if (base..base + BYTES).contains(&(pointer as usize)) {
            STATE.td.release(pointer, layout);
        } else {
            STATE.source.release(pointer, layout);
        }
    }
}
pub fn owner_layout() -> Layout {
    Layout::new::<State>()
}
pub fn source_live() -> usize {
    unsafe { (*STATE.source.metadata.get()).trace.live }
}
pub fn trace() -> Trace {
    unsafe { (*STATE.td.metadata.get()).trace }
}
pub fn begin(fail: usize) {
    assert!(!STATE.observing.get());
    STATE.td.empty();
    let m = unsafe { &mut *STATE.td.metadata.get() };
    m.trace = EMPTY;
    m.fail = fail;
    STATE.observing.set(true);
}
pub fn end() -> Trace {
    assert!(STATE.observing.get());
    STATE.td.empty();
    let trace = trace();
    assert_eq!(trace.allocations, trace.releases);
    STATE.observing.set(false);
    trace
}
pub fn assert_empty() {
    assert!(!STATE.observing.get());
    STATE.source.empty();
    STATE.td.empty();
}
pub fn isolated_source<T>(make: impl FnOnce() -> T) -> T {
    assert!(!STATE.observing.get() && !STATE.isolate_source.replace(true));
    let value = make();
    STATE.isolate_source.set(false);
    value
}
pub fn byte_regions(include_source: bool) -> impl Iterator<Item = Region> {
    STATE
        .td
        .regions()
        .chain(STATE.source.regions().filter(move |_| include_source))
}
#[cfg(target_os = "none")]
pub fn access_controls() -> &'static AccessControls {
    &STATE.controls
}
