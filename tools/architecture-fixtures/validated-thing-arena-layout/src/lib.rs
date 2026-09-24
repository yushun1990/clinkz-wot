//! Non-production allocation-layout witness for WP-100 pre-readmission item 2.
//! The scalar records and sample contents are illustrative, not a TD model.
#![no_std]

extern crate alloc;

use alloc::alloc::{alloc, dealloc};
use clinkz_wot_foundation::{AdmissionLedger, Generation, ResourceKind, SlotIndex};
use core::{
    alloc::Layout,
    mem,
    ptr::{self, NonNull},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetainedNode {
    pub kind: u32,
    pub first_edge: u32,
    pub edge_count: u32,
    pub first_byte: u32,
    pub byte_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetainedEdge {
    pub target: u32,
    pub original_index: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraversalFrame {
    pub node: u32,
    pub next_child: u32,
}

// None of these records recursively owns or drops data. The byte arena owns
// the content identified by scalar ranges in the other two retained arenas.
const _: () = assert!(!mem::needs_drop::<RetainedNode>());
const _: () = assert!(!mem::needs_drop::<RetainedEdge>());
const _: () = assert!(!mem::needs_drop::<TraversalFrame>());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Arithmetic,
    Limit,
    Allocation,
}

#[derive(Clone, Copy)]
enum Account {
    Source,
    Temporary,
}

struct Arena<T: Copy> {
    pointer: NonNull<T>,
    length: usize,
    capacity: usize,
    request_bytes: u64,
}

impl<T: Copy> Default for Arena<T> {
    fn default() -> Self {
        Self {
            pointer: NonNull::dangling(),
            length: 0,
            capacity: 0,
            request_bytes: 0,
        }
    }
}

impl<T: Copy> Arena<T> {
    fn push(&mut self, value: T) -> Result<(), Error> {
        if self.length == self.capacity {
            return Err(Error::Limit);
        }
        // SAFETY: length is below capacity, so this is within the live
        // allocation. Only initialized elements [0, length) are ever read.
        unsafe { self.pointer.as_ptr().add(self.length).write(value) };
        self.length += 1;
        Ok(())
    }

    pub fn as_slice(&self) -> &[T] {
        // SAFETY: the first length elements have been initialized by push or
        // copied from an initialized predecessor. A dangling pointer is valid
        // for an empty slice.
        unsafe { core::slice::from_raw_parts(self.pointer.as_ptr(), self.length) }
    }

    fn requested_layout(&self) -> Option<Layout> {
        (self.capacity != 0).then(|| Layout::array::<T>(self.capacity).unwrap())
    }
}

impl<T: Copy> Drop for Arena<T> {
    fn drop(&mut self) {
        if let Some(layout) = self.requested_layout() {
            // SAFETY: pointer came from alloc with this exact checked Layout;
            // T has no destructor and no element owns a nested allocation.
            unsafe { dealloc(self.pointer.as_ptr().cast(), layout) };
        }
    }
}

/// The seven arena slots are fixed: three build, one frame, three retained.
/// No arena handle contains an owning collection or nested allocation.
pub struct Prototype {
    accounting: Accounting,
    build_nodes: Arena<RetainedNode>,
    build_edges: Arena<RetainedEdge>,
    build_bytes: Arena<u8>,
    frames: Arena<TraversalFrame>,
    retained_nodes: Arena<RetainedNode>,
    retained_edges: Arena<RetainedEdge>,
    retained_bytes: Arena<u8>,
    sealed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Footprint {
    pub retained_requested_bytes: u64,
    pub retained_allocation_count: u64,
    pub largest_request_bytes: u64,
    pub temporary_peak_bytes: u64,
    pub conversion_peak_bytes: u64,
}

impl Prototype {
    pub fn new(
        source_limit: u64,
        temporary_limit: u64,
        peak_limit: u64,
        contiguous_limit: u64,
    ) -> Self {
        Self {
            accounting: Accounting::new(
                source_limit,
                temporary_limit,
                peak_limit,
                contiguous_limit,
            ),
            build_nodes: Arena::default(),
            build_edges: Arena::default(),
            build_bytes: Arena::default(),
            frames: Arena::default(),
            retained_nodes: Arena::default(),
            retained_edges: Arena::default(),
            retained_bytes: Arena::default(),
            sealed: false,
        }
    }

    pub fn grow_nodes(&mut self, capacity: usize) -> Result<(), Error> {
        assert!(!self.sealed);
        self.accounting
            .replace(&mut self.build_nodes, capacity, Account::Temporary)
    }

    pub fn grow_edges(&mut self, capacity: usize) -> Result<(), Error> {
        assert!(!self.sealed);
        self.accounting
            .replace(&mut self.build_edges, capacity, Account::Temporary)
    }

    pub fn grow_bytes(&mut self, capacity: usize) -> Result<(), Error> {
        assert!(!self.sealed);
        self.accounting
            .replace(&mut self.build_bytes, capacity, Account::Temporary)
    }

    pub fn grow_frames(&mut self, capacity: usize) -> Result<(), Error> {
        assert!(!self.sealed);
        self.accounting
            .replace(&mut self.frames, capacity, Account::Temporary)
    }

    pub fn push_node(&mut self, value: RetainedNode) -> Result<(), Error> {
        self.build_nodes.push(value)
    }
    pub fn push_edge(&mut self, value: RetainedEdge) -> Result<(), Error> {
        self.build_edges.push(value)
    }
    pub fn push_byte(&mut self, value: u8) -> Result<(), Error> {
        self.build_bytes.push(value)
    }
    pub fn push_frame(&mut self, value: TraversalFrame) -> Result<(), Error> {
        self.frames.push(value)
    }

    /// Consuming failure drops every still-live build and partial sealed site.
    /// Success keeps only exact-length source arenas; even an empty build arena
    /// with spare capacity is released before the owner becomes visible.
    pub fn seal(mut self) -> Result<Self, Error> {
        self.accounting
            .seal_one(&mut self.build_nodes, &mut self.retained_nodes)?;
        self.accounting
            .seal_one(&mut self.build_edges, &mut self.retained_edges)?;
        self.accounting
            .seal_one(&mut self.build_bytes, &mut self.retained_bytes)?;
        self.accounting
            .release(&mut self.frames, Account::Temporary);
        self.sealed = true;
        Ok(self)
    }

    pub fn footprint(&self) -> Footprint {
        assert!(self.sealed);
        let sites = [
            self.retained_nodes.request_bytes,
            self.retained_edges.request_bytes,
            self.retained_bytes.request_bytes,
        ];
        Footprint {
            retained_requested_bytes: self.accounting.source_live,
            retained_allocation_count: sites.iter().filter(|&&bytes| bytes != 0).count() as u64,
            largest_request_bytes: self.accounting.ledger.largest_contiguous_allocation(),
            temporary_peak_bytes: self.accounting.temporary_peak,
            conversion_peak_bytes: self.accounting.ledger.peak_live_bytes(),
        }
    }

    pub fn ledger(&self) -> &AdmissionLedger {
        &self.accounting.ledger
    }
    pub fn nodes(&self) -> &[RetainedNode] {
        assert!(self.sealed);
        self.retained_nodes.as_slice()
    }
    pub fn edges(&self) -> &[RetainedEdge] {
        assert!(self.sealed);
        self.retained_edges.as_slice()
    }
    pub fn bytes(&self) -> &[u8] {
        assert!(self.sealed);
        self.retained_bytes.as_slice()
    }
}

impl Drop for Prototype {
    fn drop(&mut self) {
        self.accounting
            .release(&mut self.build_nodes, Account::Temporary);
        self.accounting
            .release(&mut self.build_edges, Account::Temporary);
        self.accounting
            .release(&mut self.build_bytes, Account::Temporary);
        self.accounting
            .release(&mut self.frames, Account::Temporary);
        self.accounting
            .release(&mut self.retained_nodes, Account::Source);
        self.accounting
            .release(&mut self.retained_edges, Account::Source);
        self.accounting
            .release(&mut self.retained_bytes, Account::Source);
        debug_assert_eq!(self.accounting.ledger.live_bytes(), 0);
    }
}

// All limits and Foundation reservations are localized here. Each Arena owns
// only a pointer and checked Layout; Accounting owns all live-byte charges.
struct Accounting {
    ledger: AdmissionLedger,
    source_limit: u64,
    temporary_limit: u64,
    peak_limit: u64,
    contiguous_limit: u64,
    source_live: u64,
    temporary_live: u64,
    temporary_peak: u64,
}

impl Accounting {
    fn new(
        source_limit: u64,
        temporary_limit: u64,
        peak_limit: u64,
        contiguous_limit: u64,
    ) -> Self {
        Self {
            ledger: AdmissionLedger::new(
                SlotIndex::new(0),
                Generation::INITIAL,
                source_limit,
                temporary_limit,
                source_limit,
                0,
                0,
                0,
            ),
            source_limit,
            temporary_limit,
            peak_limit,
            contiguous_limit,
            source_live: 0,
            temporary_live: 0,
            temporary_peak: 0,
        }
    }

    fn replace<T: Copy>(
        &mut self,
        arena: &mut Arena<T>,
        capacity: usize,
        account: Account,
    ) -> Result<(), Error> {
        if capacity <= arena.capacity {
            return Ok(());
        }
        let layout = Layout::array::<T>(capacity).map_err(|_| Error::Arithmetic)?;
        let bytes = u64::try_from(layout.size()).map_err(|_| Error::Arithmetic)?;
        if bytes > self.contiguous_limit
            || self
                .ledger
                .live_bytes()
                .checked_add(bytes)
                .ok_or(Error::Arithmetic)?
                > self.peak_limit
            || match account {
                Account::Source => {
                    self.source_live
                        .checked_add(bytes)
                        .ok_or(Error::Arithmetic)?
                        > self.source_limit
                }
                Account::Temporary => {
                    self.temporary_live
                        .checked_add(bytes)
                        .ok_or(Error::Arithmetic)?
                        > self.temporary_limit
                }
            }
        {
            return Err(Error::Limit);
        }
        // Exactly one reservation for exactly this allocation's checked Layout.
        let reservation = match account {
            Account::Source => self
                .ledger
                .try_reserve_source(ResourceKind::RetainedSourceBytesPerOwnerMax, bytes),
            Account::Temporary => self
                .ledger
                .try_reserve_temporary(ResourceKind::AdmissionTemporaryBytesPerOperationMax, bytes),
        }
        .ok_or(Error::Limit)?;
        // SAFETY: Layout is checked, nonzero, and aligned for T. The pointer
        // is not dereferenced unless the allocator succeeds.
        let pointer = unsafe { alloc(layout) };
        let Some(pointer) = NonNull::new(pointer.cast::<T>()) else {
            return Err(Error::Allocation);
        };
        reservation.commit();
        // SAFETY: blocks are distinct; the source initialized prefix fits
        // both allocations and T is Copy.
        unsafe { ptr::copy_nonoverlapping(arena.pointer.as_ptr(), pointer.as_ptr(), arena.length) };
        let old = mem::replace(
            arena,
            Arena {
                pointer,
                length: arena.length,
                capacity,
                request_bytes: bytes,
            },
        );
        match account {
            Account::Source => self.source_live += bytes,
            Account::Temporary => {
                self.temporary_live += bytes;
                self.temporary_peak = self.temporary_peak.max(self.temporary_live);
            }
        }
        // Keep both blocks charged until the old allocation is deallocated.
        let old_bytes = old.request_bytes;
        drop(old);
        if old_bytes != 0 {
            let released = match account {
                Account::Source => {
                    self.source_live -= old_bytes;
                    self.ledger.release_source(old_bytes)
                }
                Account::Temporary => {
                    self.temporary_live -= old_bytes;
                    self.ledger.release_temporary(old_bytes)
                }
            };
            assert!(released);
        }
        Ok(())
    }

    fn seal_one<T: Copy>(
        &mut self,
        build: &mut Arena<T>,
        retained: &mut Arena<T>,
    ) -> Result<(), Error> {
        if build.length != 0 {
            self.replace(retained, build.length, Account::Source)?;
            // SAFETY: retained was allocated at the exact initialized length
            // while the build allocation remains live.
            unsafe {
                ptr::copy_nonoverlapping(
                    build.pointer.as_ptr(),
                    retained.pointer.as_ptr(),
                    build.length,
                )
            };
            retained.length = build.length;
        }
        self.release(build, Account::Temporary);
        Ok(())
    }

    fn release<T: Copy>(&mut self, arena: &mut Arena<T>, account: Account) {
        let old = mem::take(arena);
        let bytes = old.request_bytes;
        drop(old);
        if bytes == 0 {
            return;
        }
        let released = match account {
            Account::Source => {
                self.source_live -= bytes;
                self.ledger.release_source(bytes)
            }
            Account::Temporary => {
                self.temporary_live -= bytes;
                self.ledger.release_temporary(bytes)
            }
        };
        assert!(released);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NODE: RetainedNode = RetainedNode {
        kind: 1,
        first_edge: 0,
        edge_count: 2,
        first_byte: 0,
        byte_count: 3,
    };
    const EDGES: [RetainedEdge; 2] = [
        RetainedEdge {
            target: 7,
            original_index: 0,
        },
        RetainedEdge {
            target: 9,
            original_index: 1,
        },
    ];

    #[test]
    fn grow_and_seal_charge_actual_layouts_and_preserve_content() {
        assert_eq!(mem::size_of::<RetainedNode>(), 20);
        assert_eq!(mem::size_of::<RetainedEdge>(), 8);
        let mut owner = Prototype::new(10_000, 10_000, 20_000, 10_000);
        owner.grow_nodes(2).unwrap();
        owner.push_node(NODE).unwrap();
        owner.grow_nodes(4).unwrap(); // 40 + 80 overlap, then old 40 released.
        owner.grow_edges(2).unwrap();
        owner.push_edge(EDGES[0]).unwrap();
        owner.push_edge(EDGES[1]).unwrap();
        owner.grow_bytes(5).unwrap();
        for byte in b"abc" {
            owner.push_byte(*byte).unwrap();
        }
        owner.grow_frames(2).unwrap();
        owner
            .push_frame(TraversalFrame {
                node: 0,
                next_child: 0,
            })
            .unwrap();
        assert_eq!(owner.ledger().live_bytes(), 80 + 16 + 5 + 16);
        assert_eq!(owner.ledger().largest_contiguous_allocation(), 80);

        let owner = owner.seal().unwrap();
        assert_eq!(owner.nodes(), &[NODE]);
        assert_eq!(owner.edges(), &EDGES);
        assert_eq!(owner.bytes(), b"abc");
        assert_eq!(owner.ledger().live_bytes(), 20 + 16 + 3);
        assert_eq!(
            owner.footprint(),
            Footprint {
                retained_requested_bytes: 39,
                retained_allocation_count: 3,
                largest_request_bytes: 80,
                temporary_peak_bytes: 120,
                conversion_peak_bytes: 137,
            }
        );
    }

    #[test]
    fn empty_and_spare_capacity_have_no_retained_allocation() {
        let empty = Prototype::new(1_000, 1_000, 2_000, 1_000).seal().unwrap();
        assert_eq!(empty.footprint().retained_requested_bytes, 0);
        assert_eq!(empty.footprint().retained_allocation_count, 0);
        assert_eq!(empty.ledger().live_bytes(), 0);

        let mut previously_allocated = Prototype::new(1_000, 1_000, 2_000, 1_000);
        previously_allocated.grow_nodes(4).unwrap();
        previously_allocated.grow_edges(4).unwrap();
        previously_allocated.grow_bytes(20).unwrap();
        previously_allocated.grow_frames(2).unwrap();
        let previously_allocated = previously_allocated.seal().unwrap();
        assert_eq!(previously_allocated.footprint().retained_requested_bytes, 0);
        assert_eq!(
            previously_allocated.footprint().retained_allocation_count,
            0
        );
        assert_eq!(previously_allocated.ledger().live_bytes(), 0);
    }

    #[test]
    fn rejected_grow_preserves_old_allocation_and_contents() {
        let mut owner = Prototype::new(1_000, 119, 119, 1_000);
        owner.grow_nodes(2).unwrap();
        owner.push_node(NODE).unwrap();
        assert_eq!(owner.grow_nodes(4), Err(Error::Limit)); // 40 + 80 > 119.
        assert_eq!(owner.ledger().live_bytes(), 40);
        assert_eq!(owner.ledger().largest_contiguous_allocation(), 40);
        let owner = owner.seal().unwrap();
        assert_eq!(owner.nodes(), &[NODE]);
        assert_eq!(owner.footprint().retained_requested_bytes, 20);

        let mut contiguous = Prototype::new(1_000, 1_000, 2_000, 79);
        contiguous.grow_nodes(2).unwrap();
        assert_eq!(contiguous.grow_nodes(4), Err(Error::Limit));
        assert_eq!(contiguous.ledger().live_bytes(), 40);
    }

    #[test]
    fn seal_overlap_is_bounded_before_source_allocation() {
        let mut source = Prototype::new(19, 1_000, 2_000, 1_000);
        source.grow_nodes(1).unwrap();
        source.push_node(NODE).unwrap();
        assert!(matches!(source.seal(), Err(Error::Limit)));

        let mut peak = Prototype::new(1_000, 1_000, 39, 1_000);
        peak.grow_nodes(1).unwrap();
        peak.push_node(NODE).unwrap();
        assert!(matches!(peak.seal(), Err(Error::Limit))); // 20 + 20 > 39.

        // The node source request succeeds, then the edge source request
        // fails. Consuming seal drops both the partial source and the still
        // live edge build block; Prototype::drop asserts a zero ledger.
        let mut partial = Prototype::new(20, 1_000, 2_000, 1_000);
        partial.grow_nodes(1).unwrap();
        partial.push_node(NODE).unwrap();
        partial.grow_edges(1).unwrap();
        partial.push_edge(EDGES[0]).unwrap();
        assert!(matches!(partial.seal(), Err(Error::Limit)));
    }

    #[test]
    fn layout_arithmetic_fails_before_reservation_or_allocation() {
        let mut owner = Prototype::new(u64::MAX, u64::MAX, u64::MAX, u64::MAX);
        assert_eq!(owner.grow_nodes(usize::MAX), Err(Error::Arithmetic));
        assert_eq!(owner.ledger().live_bytes(), 0);
        assert_eq!(owner.ledger().largest_contiguous_allocation(), 0);
    }
}
