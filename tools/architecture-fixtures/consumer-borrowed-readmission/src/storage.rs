//! Two actual allocation sites. Reservations precede allocator entry; frame
//! movement is one paid element and release never walks the borrowed source.
use super::Cause;
use alloc::alloc::{alloc, dealloc};
use clinkz_wot_foundation::{AdmissionLedger, ResourceKind as R};
use core::{alloc::Layout, marker::PhantomData, mem, ptr};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Site {
    Frames,
    Bytes,
}
struct Block<T> {
    pointer: *mut T,
    cap: usize,
    len: usize,
    _type: PhantomData<T>,
}
impl<T> Default for Block<T> {
    fn default() -> Self {
        Self {
            pointer: ptr::null_mut(),
            cap: 0,
            len: 0,
            _type: PhantomData,
        }
    }
}
impl<T> Block<T> {
    fn layout(cap: usize) -> Result<Layout, Cause> {
        Layout::array::<T>(cap).map_err(|_| Cause::Arithmetic)
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Footprint {
    pub reservation_peak_bytes: u64,
    pub largest_request_bytes: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocationEventKind {
    Vacant,
    Reserved,
    Allocated,
    AllocationFailed,
    BeforeRelease,
    Released,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AllocationEvent {
    pub kind: AllocationEventKind,
    pub bytes: u64,
    pub alignment: usize,
    pub local_live: u64,
}
impl AllocationEvent {
    const EMPTY: Self = Self {
        kind: AllocationEventKind::Vacant,
        bytes: 0,
        alignment: 0,
        local_live: 0,
    };
}
pub struct Storage<F> {
    ledger: AdmissionLedger,
    frames: Block<F>,
    bytes: Block<u8>,
    transfer: Option<Block<F>>,
    copied: usize,
    limits: [(R, u64); 6],
    attempts: u64,
    allocations: u64,
    releases: u64,
    largest: u64,
    actual_peak: u64,
    events: [AllocationEvent; 256],
    event_count: usize,
}
impl<F> Storage<F> {
    pub fn new(ledger: AdmissionLedger, limits: [(R, u64); 6]) -> Self {
        assert!(!mem::needs_drop::<F>());
        Self {
            ledger,
            frames: Block::default(),
            bytes: Block::default(),
            transfer: None,
            copied: 0,
            limits,
            attempts: 0,
            allocations: 0,
            releases: 0,
            largest: 0,
            actual_peak: 0,
            events: [AllocationEvent::EMPTY; 256],
            event_count: 0,
        }
    }
    pub fn len(&self, s: Site) -> usize {
        match s {
            Site::Frames => self.frames.len,
            Site::Bytes => self.bytes.len,
        }
    }
    pub fn capacity(&self, s: Site) -> usize {
        match s {
            Site::Frames => self.frames.cap,
            Site::Bytes => self.bytes.cap,
        }
    }
    pub fn check_grow(&self, s: Site, cap: usize) -> Result<(), Cause> {
        let n = match s {
            Site::Frames => Block::<F>::layout(cap)?,
            Site::Bytes => Block::<u8>::layout(cap)?,
        }
        .size() as u64;
        let live = self.live_bytes().checked_add(n).ok_or(Cause::Arithmetic)?;
        for &(kind, configured) in &self.limits {
            let observed = if kind == R::LargestContiguousAllocationBytesMax {
                n
            } else {
                live
            };
            if observed > configured {
                return Err(Cause::Resource {
                    kind,
                    configured,
                    observed,
                });
            }
        }
        Ok(())
    }
    fn allocate<T>(&mut self, cap: usize) -> Result<Block<T>, Cause> {
        let layout = Block::<T>::layout(cap)?;
        if layout.size() == 0 {
            return Ok(Block::default());
        }
        self.ledger
            .try_reserve_temporary(
                R::AdmissionTemporaryBytesPerOperationMax,
                layout.size() as u64,
            )
            .ok_or(Cause::Memory)?
            .commit();
        self.record(AllocationEventKind::Reserved, layout);
        self.attempts += 1;
        // SAFETY: the checked nonzero Layout has matching admitted capacity.
        // On null the uncommitted reservation rolls back before return.
        let pointer = unsafe { alloc(layout) } as *mut T;
        if pointer.is_null() {
            self.record(AllocationEventKind::AllocationFailed, layout);
            assert!(self.ledger.release_temporary(layout.size() as u64));
            self.record(AllocationEventKind::Released, layout);
            return Err(Cause::Allocation {
                requested_bytes: layout.size() as u64,
            });
        }
        self.allocations += 1;
        self.actual_peak = self.actual_peak.max(self.ledger.live_bytes());
        self.largest = self.largest.max(layout.size() as u64);
        self.record(AllocationEventKind::Allocated, layout);
        Ok(Block {
            pointer,
            cap,
            len: 0,
            _type: PhantomData,
        })
    }
    pub fn begin_grow(&mut self, s: Site, cap: usize) -> Result<(), Cause> {
        assert!(self.transfer.is_none());
        self.check_grow(s, cap)?;
        match s {
            Site::Frames => {
                self.transfer = Some(self.allocate(cap)?);
                self.copied = 0;
            }
            Site::Bytes => {
                assert_eq!(self.bytes.cap, 0);
                self.bytes = self.allocate(cap)?;
            }
        }
        Ok(())
    }
    pub fn transferring(&self) -> Option<Site> {
        self.transfer.as_ref().map(|_| Site::Frames)
    }
    pub fn copy_one(&mut self) {
        if self.copied < self.frames.len {
            // SAFETY: distinct live blocks, one initialized trivial element.
            // The old block is frozen and no element destructor is run.
            unsafe {
                ptr::copy_nonoverlapping(
                    self.frames.pointer.add(self.copied),
                    self.transfer.as_mut().unwrap().pointer.add(self.copied),
                    1,
                );
            }
            self.copied += 1;
        } else {
            let mut next = self.transfer.take().unwrap();
            next.len = self.frames.len;
            self.release_site(Site::Frames);
            self.frames = next;
        }
    }
    pub fn push_frame(&mut self, value: F) {
        assert!(self.transfer.is_none() && self.frames.len < self.frames.cap);
        unsafe {
            self.frames.pointer.add(self.frames.len).write(value);
        }
        self.frames.len += 1;
    }
    pub fn pop_frame(&mut self) -> F {
        assert!(self.transfer.is_none() && self.frames.len != 0);
        self.frames.len -= 1;
        unsafe { self.frames.pointer.add(self.frames.len).read() }
    }
    pub fn frame_mut(&mut self) -> &mut F {
        assert!(self.transfer.is_none() && self.frames.len != 0);
        unsafe { &mut *self.frames.pointer.add(self.frames.len - 1) }
    }
    pub fn push_byte(&mut self, byte: u8) {
        assert!(self.bytes.len < self.bytes.cap);
        unsafe {
            self.bytes.pointer.add(self.bytes.len).write(byte);
        }
        self.bytes.len += 1;
    }
    pub fn byte(&self, n: usize) -> u8 {
        assert!(n < self.bytes.len);
        unsafe { *self.bytes.pointer.add(n) }
    }
    pub fn set_byte(&mut self, n: usize, byte: u8) {
        assert!(n < self.bytes.len);
        unsafe {
            self.bytes.pointer.add(n).write(byte);
        }
    }
    pub fn truncate_bytes(&mut self, len: usize) {
        assert!(len <= self.bytes.len);
        self.bytes.len = len;
    }
    pub fn temporary_bytes(&self) -> &[u8] {
        if self.bytes.cap == 0 {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(self.bytes.pointer, self.bytes.len) }
        }
    }
    fn release<T>(&mut self, b: Block<T>) {
        if b.cap == 0 {
            return;
        }
        let layout = Block::<T>::layout(b.cap).unwrap();
        self.record(AllocationEventKind::BeforeRelease, layout);
        // Physical child destruction precedes release of its original charge.
        unsafe {
            dealloc(b.pointer as *mut u8, layout);
        }
        assert!(self.ledger.release_temporary(layout.size() as u64));
        self.record(AllocationEventKind::Released, layout);
        self.releases += 1;
    }
    pub fn release_site(&mut self, s: Site) {
        match s {
            Site::Frames => {
                let old = mem::take(&mut self.frames);
                self.release(old);
            }
            Site::Bytes => {
                let old = mem::take(&mut self.bytes);
                self.release(old);
            }
        }
    }
    pub fn clear(&mut self) {
        if let Some(block) = self.transfer.take() {
            self.release(block);
        }
        self.release_site(Site::Frames);
        self.release_site(Site::Bytes);
        assert_eq!(self.ledger.live_bytes(), 0);
    }
    pub fn live_bytes(&self) -> u64 {
        self.ledger.live_bytes()
    }
    pub fn allocations(&self) -> u64 {
        self.allocations
    }
    pub fn releases(&self) -> u64 {
        self.releases
    }
    pub fn attempts(&self) -> u64 {
        self.attempts
    }
    pub fn footprint(&self) -> Footprint {
        Footprint {
            reservation_peak_bytes: self.actual_peak,
            largest_request_bytes: self.largest,
        }
    }
    fn record(&mut self, kind: AllocationEventKind, layout: Layout) {
        // Job is larger than eight bytes. With the checked isize byte envelope
        // on supported 32/64-bit targets, growth from four frames has at most
        // 59 capacities (including the final non-power-of-two cap). Four events
        // per successful allocation plus one URI block fit these 256 slots.
        assert!(self.event_count < self.events.len());
        self.events[self.event_count] = AllocationEvent {
            kind,
            bytes: layout.size() as u64,
            alignment: layout.align(),
            local_live: self.ledger.live_bytes(),
        };
        self.event_count += 1;
    }
    pub fn events(&self) -> &[AllocationEvent] {
        &self.events[..self.event_count]
    }
}
impl<F> Drop for Storage<F> {
    fn drop(&mut self) {
        self.clear();
    }
}
