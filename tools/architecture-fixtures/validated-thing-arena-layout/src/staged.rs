//! Budget-driver-facing version of the established seven-site layout witness.
//! Only an empty replacement is allocated synchronously. The driver pays for
//! each transferred element before `copy_one`; no input-sized memcpy occurs.

use super::{Account, Accounting, Arena, Error, Footprint, RetainedEdge, RetainedNode};
use core::{mem, ptr};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Site {
    Nodes,
    Edges,
    Bytes,
    Frames,
}

enum Transfer<F> {
    Nodes(Arena<RetainedNode>),
    Edges(Arena<RetainedEdge>),
    Bytes(Arena<u8>),
    Frames(Arena<F>),
}

#[derive(Default)]
struct Retained {
    nodes: Arena<RetainedNode>,
    edges: Arena<RetainedEdge>,
    bytes: Arena<u8>,
}

/// Compact immutable borrow, disjoint from the owner's accounting. This is
/// fixture plumbing; it grants no mutation or unchecked construction.
#[derive(Clone, Copy)]
pub struct Sealed<'a>(&'a Retained);
impl<'a> Sealed<'a> {
    pub fn nodes(self) -> &'a [RetainedNode] {
        self.0.nodes.as_slice()
    }
    pub fn edges(self) -> &'a [RetainedEdge] {
        self.0.edges.as_slice()
    }
    pub fn bytes(self) -> &'a [u8] {
        self.0.bytes.as_slice()
    }
}

/// Fixture scaffolding, not a public TD storage API. F must be non-dropping;
/// it can contain borrowed public iterators, never an owning collection.
/// All build arenas are frozen during transfer, including frame extraction.
/// Only transfer completion or whole-storage cleanup can release either copy.
pub struct Storage<F> {
    accounting: Accounting,
    nodes: Arena<RetainedNode>,
    edges: Arena<RetainedEdge>,
    bytes: Arena<u8>,
    frames: Arena<F>,
    retained: Retained,
    transfer: Option<Transfer<F>>,
    copied: usize,
    sealing: bool,
    sealed: bool,
    allocations: u64,
    releases: u64,
}

impl<F> Storage<F> {
    pub fn new(source: u64, temporary: u64, peak: u64, contiguous: u64) -> Self {
        Self {
            accounting: Accounting::new(source, temporary, peak, contiguous),
            nodes: Arena::default(),
            edges: Arena::default(),
            bytes: Arena::default(),
            frames: Arena::default(),
            retained: Retained::default(),
            transfer: None,
            copied: 0,
            sealing: false,
            sealed: false,
            allocations: 0,
            releases: 0,
        }
    }

    pub fn capacity(&self, site: Site) -> usize {
        match site {
            Site::Nodes => self.nodes.capacity,
            Site::Edges => self.edges.capacity,
            Site::Bytes => self.bytes.capacity,
            Site::Frames => self.frames.capacity,
        }
    }

    pub fn len(&self, site: Site) -> usize {
        match site {
            Site::Nodes => self.nodes.length,
            Site::Edges => self.edges.length,
            Site::Bytes => self.bytes.length,
            Site::Frames => self.frames.length,
        }
    }

    /// Same checked request as begin_grow, without any accounting mutation.
    pub fn check_grow(&self, site: Site, capacity: usize) -> Result<(), Error> {
        assert!(self.transfer.is_none() && !self.sealed);
        assert!(capacity > self.capacity(site));
        self.check_request(site, capacity, Account::Temporary)
    }
    pub fn check_seal(&self, site: Site) -> Result<(), Error> {
        assert!(self.transfer.is_none() && !self.sealed && site != Site::Frames);
        self.check_request(site, self.len(site), Account::Source)
    }
    fn check_request(&self, site: Site, capacity: usize, account: Account) -> Result<(), Error> {
        match site {
            Site::Nodes => self.accounting.request::<RetainedNode>(capacity, account),
            Site::Edges => self.accounting.request::<RetainedEdge>(capacity, account),
            Site::Bytes => self.accounting.request::<u8>(capacity, account),
            Site::Frames => self.accounting.request::<F>(capacity, account),
        }
        .map(|_| ())
    }

    /// Caller prepays one CleanupItems/lifetime unit for this actual request.
    /// Arithmetic and byte limits are checked by the original Accounting body.
    pub fn begin_grow(&mut self, site: Site, capacity: usize) -> Result<(), Error> {
        assert!(self.transfer.is_none() && !self.sealed);
        assert!(capacity > self.capacity(site));
        let transfer = match site {
            Site::Nodes => Transfer::Nodes(self.allocate(capacity, Account::Temporary)?),
            Site::Edges => Transfer::Edges(self.allocate(capacity, Account::Temporary)?),
            Site::Bytes => Transfer::Bytes(self.allocate(capacity, Account::Temporary)?),
            Site::Frames => Transfer::Frames(self.allocate(capacity, Account::Temporary)?),
        };
        self.transfer = Some(transfer);
        self.copied = 0;
        self.sealing = false;
        Ok(())
    }

    pub fn begin_seal(&mut self, site: Site) -> Result<(), Error> {
        assert!(self.transfer.is_none() && !self.sealed && site != Site::Frames);
        let transfer = match site {
            Site::Nodes => Transfer::Nodes(self.allocate(self.nodes.length, Account::Source)?),
            Site::Edges => Transfer::Edges(self.allocate(self.edges.length, Account::Source)?),
            Site::Bytes => Transfer::Bytes(self.allocate(self.bytes.length, Account::Source)?),
            Site::Frames => unreachable!(),
        };
        self.transfer = Some(transfer);
        self.copied = 0;
        self.sealing = true;
        Ok(())
    }

    fn allocate<T>(&mut self, capacity: usize, account: Account) -> Result<Arena<T>, Error> {
        let mut arena = Arena::default();
        // Replacing an empty arena copies zero elements. Reuse the checked
        // Layout/reservation/allocator-failure/peak body from #94.
        self.accounting.replace(&mut arena, capacity, account)?;
        if capacity != 0 {
            self.allocations += 1;
        }
        Ok(arena)
    }

    pub fn transferring(&self) -> Option<Site> {
        self.transfer.as_ref().map(|transfer| match transfer {
            Transfer::Nodes(_) => Site::Nodes,
            Transfer::Edges(_) => Site::Edges,
            Transfer::Bytes(_) => Site::Bytes,
            Transfer::Frames(_) => Site::Frames,
        })
    }
    pub fn copy_pending(&self) -> bool {
        self.transferring()
            .is_some_and(|site| self.copied < self.len(site))
    }

    /// One already-paid element move, or a constant-size completion/release.
    /// An in-flight replacement and its old block are both charged throughout.
    pub fn copy_one(&mut self) {
        let site = self.transferring().unwrap();
        if self.copied < self.len(site) {
            let index = self.copied;
            // SAFETY: initialized old prefix, distinct checked replacement,
            // exact one-element range. Non-Copy frame state is frozen during
            // transfer, and the old representation is deallocated without
            // running element destructors when the move completes.
            unsafe {
                match self.transfer.as_mut().unwrap() {
                    Transfer::Nodes(new) => copy_element(&self.nodes, new, index),
                    Transfer::Edges(new) => copy_element(&self.edges, new, index),
                    Transfer::Bytes(new) => copy_element(&self.bytes, new, index),
                    Transfer::Frames(new) => copy_element(&self.frames, new, index),
                }
            }
            self.copied += 1;
            return;
        }
        let transfer = self.transfer.take().unwrap();
        match transfer {
            Transfer::Nodes(new) => {
                self.release_site(Site::Nodes);
                if self.sealing {
                    self.release_retained(Site::Nodes);
                    self.retained.nodes = new;
                } else {
                    self.nodes = new;
                }
            }
            Transfer::Edges(new) => {
                self.release_site(Site::Edges);
                if self.sealing {
                    self.release_retained(Site::Edges);
                    self.retained.edges = new;
                } else {
                    self.edges = new;
                }
            }
            Transfer::Bytes(new) => {
                self.release_site(Site::Bytes);
                if self.sealing {
                    self.release_retained(Site::Bytes);
                    self.retained.bytes = new;
                } else {
                    self.bytes = new;
                }
            }
            Transfer::Frames(new) => {
                self.release_site(Site::Frames);
                self.frames = new;
            }
        }
    }

    fn assert_not_transferring(&self) {
        assert!(
            self.transfer.is_none(),
            "arena mutation is forbidden during transfer"
        );
    }

    fn release_retained(&mut self, site: Site) {
        let bytes = match site {
            Site::Nodes => self.retained.nodes.request_bytes,
            Site::Edges => self.retained.edges.request_bytes,
            Site::Bytes => self.retained.bytes.request_bytes,
            Site::Frames => unreachable!(),
        };
        self.releases += u64::from(bytes != 0);
        match site {
            Site::Nodes => self
                .accounting
                .release(&mut self.retained.nodes, Account::Source),
            Site::Edges => self
                .accounting
                .release(&mut self.retained.edges, Account::Source),
            Site::Bytes => self
                .accounting
                .release(&mut self.retained.bytes, Account::Source),
            Site::Frames => unreachable!(),
        }
    }

    pub fn release_site(&mut self, site: Site) {
        self.assert_not_transferring();
        let bytes = match site {
            Site::Nodes => self.nodes.request_bytes,
            Site::Edges => self.edges.request_bytes,
            Site::Bytes => self.bytes.request_bytes,
            Site::Frames => self.frames.request_bytes,
        };
        if bytes != 0 {
            self.releases += 1;
        }
        match site {
            Site::Nodes => self.accounting.release(&mut self.nodes, Account::Temporary),
            Site::Edges => self.accounting.release(&mut self.edges, Account::Temporary),
            Site::Bytes => self.accounting.release(&mut self.bytes, Account::Temporary),
            Site::Frames => self
                .accounting
                .release(&mut self.frames, Account::Temporary),
        }
    }

    pub fn push_node(&mut self, node: RetainedNode) {
        self.assert_not_transferring();
        self.nodes.push(node).unwrap();
    }
    pub fn push_edge(&mut self, edge: RetainedEdge) {
        self.assert_not_transferring();
        self.edges.push(edge).unwrap();
    }
    pub fn push_byte(&mut self, byte: u8) {
        self.assert_not_transferring();
        self.bytes.push(byte).unwrap();
    }
    pub fn push_frame(&mut self, frame: F) {
        self.assert_not_transferring();
        self.frames.push(frame).unwrap();
    }

    pub fn pop_frame(&mut self) -> F {
        self.assert_not_transferring();
        assert!(self.frames.length != 0);
        self.frames.length -= 1;
        // SAFETY: move out the initialized last element. The shortened prefix
        // no longer contains this non-Copy frame.
        unsafe { self.frames.pointer.as_ptr().add(self.frames.length).read() }
    }

    pub fn frame_mut(&mut self) -> &mut F {
        self.assert_not_transferring();
        assert!(self.frames.length != 0);
        // SAFETY: unique borrow of the initialized last element.
        unsafe { &mut *self.frames.pointer.as_ptr().add(self.frames.length - 1) }
    }

    pub fn node(&self, index: usize) -> RetainedNode {
        self.nodes.as_slice()[index]
    }
    pub fn edge(&self, index: usize) -> RetainedEdge {
        self.edges.as_slice()[index]
    }
    pub fn byte(&self, index: usize) -> u8 {
        self.bytes.as_slice()[index]
    }

    pub fn set_node(&mut self, index: usize, node: RetainedNode) {
        self.assert_not_transferring();
        assert!(index < self.nodes.length);
        // SAFETY: initialized index, unique owner, non-dropping scalar value.
        unsafe { self.nodes.pointer.as_ptr().add(index).write(node) };
    }
    pub fn set_edge(&mut self, index: usize, edge: RetainedEdge) {
        self.assert_not_transferring();
        assert!(index < self.edges.length);
        // SAFETY: initialized index, unique owner, non-dropping scalar value.
        unsafe { self.edges.pointer.as_ptr().add(index).write(edge) };
    }
    pub fn set_byte(&mut self, index: usize, byte: u8) {
        self.assert_not_transferring();
        assert!(index < self.bytes.length);
        // SAFETY: initialized index and unique owner.
        unsafe { self.bytes.pointer.as_ptr().add(index).write(byte) };
    }
    pub fn truncate(&mut self, site: Site, length: usize) {
        self.assert_not_transferring();
        match site {
            Site::Nodes => self.nodes.truncate(length),
            Site::Edges => self.edges.truncate(length),
            Site::Bytes => self.bytes.truncate(length),
            Site::Frames => self.frames.truncate(length),
        }
    }

    pub fn finish(&mut self) {
        assert!(self.transfer.is_none());
        assert_eq!(self.accounting.temporary_live, 0);
        self.sealed = true;
    }
    /// Empty traversal state cannot leave an input lifetime in the result.
    pub fn into_owned(mut self) -> Storage<()> {
        assert!(self.sealed && self.transfer.is_none());
        assert_eq!(self.frames.capacity, 0);
        let mut owned = Storage::new(0, 0, 0, 0);
        owned.accounting = mem::replace(&mut self.accounting, Accounting::new(0, 0, 0, 0));
        owned.retained.nodes = mem::take(&mut self.retained.nodes);
        owned.retained.edges = mem::take(&mut self.retained.edges);
        owned.retained.bytes = mem::take(&mut self.retained.bytes);
        owned.allocations = self.allocations;
        owned.releases = self.releases;
        owned.sealed = true;
        owned
    }
    pub fn nodes(&self) -> &[RetainedNode] {
        assert!(self.sealed);
        self.retained.nodes.as_slice()
    }
    pub fn edges(&self) -> &[RetainedEdge] {
        assert!(self.sealed);
        self.retained.edges.as_slice()
    }
    pub fn bytes(&self) -> &[u8] {
        assert!(self.sealed);
        self.retained.bytes.as_slice()
    }
    pub fn sealed(&self) -> Sealed<'_> {
        assert!(self.sealed);
        Sealed(&self.retained)
    }
    /// Reuse the already released traversal site for subsequent semantic work.
    /// Its requests share the actual source owner's Accounting/ledger, including
    /// live-source overlap. The source arrays remain immutably borrowed.
    pub fn inspection_parts<G>(&mut self) -> (Sealed<'_>, Frames<'_, G>) {
        assert!(self.sealed && self.transfer.is_none());
        assert_eq!(self.frames.capacity, 0);
        (
            Sealed(&self.retained),
            Frames {
                accounting: &mut self.accounting,
                allocations: &mut self.allocations,
                releases: &mut self.releases,
                arena: Arena::default(),
                replacement: None,
                copied: 0,
            },
        )
    }
    /// Lend the sealed input and the original three now-empty build sites.
    /// Both graphs share Accounting; dropping the borrower releases traversal
    /// state but leaves every output block owned by this rollback owner.
    pub fn rebuild_parts<G>(&mut self) -> (Sealed<'_>, Rebuild<'_, G>) {
        assert!(self.sealed && self.transfer.is_none());
        assert_eq!(self.frames.capacity, 0);
        assert_eq!(
            self.nodes.capacity + self.edges.capacity + self.bytes.capacity,
            0
        );
        assert!(!mem::needs_drop::<G>());
        self.sealed = false;
        (
            Sealed(&self.retained),
            Rebuild {
                accounting: &mut self.accounting,
                allocations: &mut self.allocations,
                releases: &mut self.releases,
                nodes: &mut self.nodes,
                edges: &mut self.edges,
                bytes: &mut self.bytes,
                frames: Arena::default(),
                transfer: None,
                copied: 0,
            },
        )
    }
    pub fn live_bytes(&self) -> u64 {
        self.accounting.ledger.live_bytes()
    }
    pub fn allocations(&self) -> u64 {
        self.allocations
    }
    pub fn releases(&self) -> u64 {
        self.releases
    }
    pub fn footprint(&self) -> Footprint {
        Footprint {
            retained_requested_bytes: self.accounting.source_live,
            retained_allocation_count: [
                self.retained.nodes.request_bytes,
                self.retained.edges.request_bytes,
                self.retained.bytes.request_bytes,
            ]
            .iter()
            .filter(|&&n| n != 0)
            .count() as u64,
            largest_request_bytes: self.accounting.largest_request,
            temporary_peak_bytes: self.accounting.temporary_peak,
            conversion_peak_bytes: self.accounting.conversion_peak,
            reservation_peak_bytes: self.accounting.ledger.peak_live_bytes(),
        }
    }

    /// The same bounded, prepaid release path serves failure and abandonment.
    pub fn clear(&mut self) {
        if let Some(transfer) = self.transfer.take() {
            let account = if self.sealing {
                Account::Source
            } else {
                Account::Temporary
            };
            match transfer {
                Transfer::Nodes(mut arena) => self.release_pending(&mut arena, account),
                Transfer::Edges(mut arena) => self.release_pending(&mut arena, account),
                Transfer::Bytes(mut arena) => self.release_pending(&mut arena, account),
                Transfer::Frames(mut arena) => self.release_pending(&mut arena, account),
            }
        }
        for site in [Site::Nodes, Site::Edges, Site::Bytes, Site::Frames] {
            self.release_site(site);
        }
        if self.retained.nodes.request_bytes != 0 {
            self.releases += 1;
        }
        if self.retained.edges.request_bytes != 0 {
            self.releases += 1;
        }
        if self.retained.bytes.request_bytes != 0 {
            self.releases += 1;
        }
        self.accounting
            .release(&mut self.retained.nodes, Account::Source);
        self.accounting
            .release(&mut self.retained.edges, Account::Source);
        self.accounting
            .release(&mut self.retained.bytes, Account::Source);
        assert_eq!(self.live_bytes(), 0);
    }

    fn release_pending<T>(&mut self, arena: &mut Arena<T>, account: Account) {
        if arena.request_bytes != 0 {
            self.releases += 1;
        }
        self.accounting.release(arena, account);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameResources {
    pub source_live: u64,
    pub temporary_live: u64,
    pub live: u64,
    pub temporary_peak: u64,
    pub conversion_peak: u64,
    pub reservation_peak: u64,
    pub largest_request: u64,
}

/// One traversal arena, with at most its old/replacement overlap. G cannot
/// own a destructor. No element is dropped during failure or abandonment.
pub struct Frames<'a, G> {
    accounting: &'a mut Accounting,
    allocations: &'a mut u64,
    releases: &'a mut u64,
    arena: Arena<G>,
    replacement: Option<Arena<G>>,
    copied: usize,
}
impl<G> Frames<'_, G> {
    pub fn len(&self) -> usize {
        self.arena.length
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn capacity(&self) -> usize {
        self.arena.capacity
    }
    pub fn check_grow(&self, capacity: usize) -> Result<(), Error> {
        assert!(self.replacement.is_none() && capacity > self.capacity());
        self.accounting
            .request::<G>(capacity, Account::Temporary)
            .map(|_| ())
    }
    /// Caller prepays one CleanupItems/lifetime unit. Allocate an EMPTY
    /// replacement through the same checked Layout and reservation body.
    pub fn begin_grow(&mut self, capacity: usize) -> Result<(), Error> {
        assert!(self.replacement.is_none() && capacity > self.capacity());
        let mut new = Arena::default();
        self.accounting
            .replace(&mut new, capacity, Account::Temporary)?;
        *self.allocations += 1;
        self.replacement = Some(new);
        self.copied = 0;
        Ok(())
    }
    pub fn transferring(&self) -> bool {
        self.replacement.is_some()
    }
    pub fn copy_pending(&self) -> bool {
        self.transferring() && self.copied < self.len()
    }
    /// One already-paid move, or fixed completion/release. Mutation and frame
    /// extraction remain forbidden until both representations stop coexisting.
    pub fn copy_one(&mut self) {
        let new = self.replacement.as_mut().unwrap();
        if self.copied < self.arena.length {
            // SAFETY: same initialized/distinct one-element ranges as Storage.
            unsafe { copy_element(&self.arena, new, self.copied) };
            self.copied += 1;
        } else {
            let new = self.replacement.take().unwrap();
            self.release_current();
            self.arena = new;
        }
    }
    pub fn push(&mut self, frame: G) {
        assert!(!self.transferring());
        self.arena.push(frame).unwrap();
    }
    pub fn last(&self) -> &G {
        assert!(!self.transferring());
        self.arena.as_slice().last().unwrap()
    }
    pub fn last_mut(&mut self) -> &mut G {
        assert!(!self.transferring() && !self.is_empty());
        // SAFETY: unique borrow of the initialized last element.
        unsafe { &mut *self.arena.pointer.as_ptr().add(self.len() - 1) }
    }
    pub fn pop(&mut self) -> G {
        assert!(!self.transferring() && !self.is_empty());
        self.arena.length -= 1;
        // SAFETY: move out one initialized element, shortening the prefix.
        unsafe { self.arena.pointer.as_ptr().add(self.len()).read() }
    }
    pub fn resources(&self) -> FrameResources {
        FrameResources {
            source_live: self.accounting.source_live,
            temporary_live: self.accounting.temporary_live,
            live: self.accounting.ledger.live_bytes(),
            temporary_peak: self.accounting.temporary_peak,
            conversion_peak: self.accounting.conversion_peak,
            reservation_peak: self.accounting.ledger.peak_live_bytes(),
            largest_request: self.accounting.largest_request,
        }
    }
    fn release_current(&mut self) {
        if self.arena.request_bytes != 0 {
            *self.releases += 1;
        }
        self.accounting.release(&mut self.arena, Account::Temporary);
    }
    pub fn clear(&mut self) {
        if let Some(mut new) = self.replacement.take() {
            *self.releases += 1;
            self.accounting.release(&mut new, Account::Temporary);
        }
        self.release_current();
    }
}
impl<G> Drop for Frames<'_, G> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Disjoint borrower for canonical construction. This uses the same four
/// temporary sites as literal construction, never a second ledger/allowance.
pub struct Rebuild<'a, G> {
    accounting: &'a mut Accounting,
    allocations: &'a mut u64,
    releases: &'a mut u64,
    nodes: &'a mut Arena<RetainedNode>,
    edges: &'a mut Arena<RetainedEdge>,
    bytes: &'a mut Arena<u8>,
    frames: Arena<G>,
    transfer: Option<Transfer<G>>,
    copied: usize,
}
impl<G> Rebuild<'_, G> {
    pub fn edge(&self, index: usize) -> RetainedEdge {
        self.edges.as_slice()[index]
    }
    pub fn byte(&self, index: usize) -> u8 {
        self.bytes.as_slice()[index]
    }
    pub fn len(&self, site: Site) -> usize {
        match site {
            Site::Nodes => self.nodes.length,
            Site::Edges => self.edges.length,
            Site::Bytes => self.bytes.length,
            Site::Frames => self.frames.length,
        }
    }
    pub fn capacity(&self, site: Site) -> usize {
        match site {
            Site::Nodes => self.nodes.capacity,
            Site::Edges => self.edges.capacity,
            Site::Bytes => self.bytes.capacity,
            Site::Frames => self.frames.capacity,
        }
    }
    pub fn check_grow(&self, site: Site, capacity: usize) -> Result<(), Error> {
        assert!(self.transfer.is_none() && capacity > self.capacity(site));
        match site {
            Site::Nodes => self
                .accounting
                .request::<RetainedNode>(capacity, Account::Temporary),
            Site::Edges => self
                .accounting
                .request::<RetainedEdge>(capacity, Account::Temporary),
            Site::Bytes => self.accounting.request::<u8>(capacity, Account::Temporary),
            Site::Frames => self.accounting.request::<G>(capacity, Account::Temporary),
        }
        .map(|_| ())
    }
    pub fn begin_grow(&mut self, site: Site, capacity: usize) -> Result<(), Error> {
        self.check_grow(site, capacity)?;
        macro_rules! replacement {
            ($variant:ident) => {{
                let mut arena = Arena::default();
                self.accounting
                    .replace(&mut arena, capacity, Account::Temporary)?;
                Transfer::$variant(arena)
            }};
        }
        self.transfer = Some(match site {
            Site::Nodes => replacement!(Nodes),
            Site::Edges => replacement!(Edges),
            Site::Bytes => replacement!(Bytes),
            Site::Frames => replacement!(Frames),
        });
        *self.allocations += 1;
        self.copied = 0;
        Ok(())
    }
    pub fn transferring(&self) -> Option<Site> {
        self.transfer.as_ref().map(|transfer| match transfer {
            Transfer::Nodes(_) => Site::Nodes,
            Transfer::Edges(_) => Site::Edges,
            Transfer::Bytes(_) => Site::Bytes,
            Transfer::Frames(_) => Site::Frames,
        })
    }
    pub fn copy_pending(&self) -> bool {
        self.transferring()
            .is_some_and(|site| self.copied < self.len(site))
    }
    pub fn copy_one(&mut self) {
        let site = self.transferring().unwrap();
        if self.copy_pending() {
            // SAFETY: checked distinct replacement, initialized old prefix;
            // all mutation/extraction is forbidden until transfer completes.
            unsafe {
                match self.transfer.as_mut().unwrap() {
                    Transfer::Nodes(new) => copy_element(self.nodes, new, self.copied),
                    Transfer::Edges(new) => copy_element(self.edges, new, self.copied),
                    Transfer::Bytes(new) => copy_element(self.bytes, new, self.copied),
                    Transfer::Frames(new) => copy_element(&self.frames, new, self.copied),
                }
            }
            self.copied += 1;
        } else {
            let transfer = self.transfer.take().unwrap();
            *self.releases += u64::from(self.capacity(site) != 0);
            match transfer {
                Transfer::Nodes(new) => {
                    self.accounting.release(self.nodes, Account::Temporary);
                    *self.nodes = new;
                }
                Transfer::Edges(new) => {
                    self.accounting.release(self.edges, Account::Temporary);
                    *self.edges = new;
                }
                Transfer::Bytes(new) => {
                    self.accounting.release(self.bytes, Account::Temporary);
                    *self.bytes = new;
                }
                Transfer::Frames(new) => {
                    self.accounting
                        .release(&mut self.frames, Account::Temporary);
                    self.frames = new;
                }
            }
        }
    }
    pub fn push_node(&mut self, node: RetainedNode) {
        assert!(self.transfer.is_none());
        self.nodes.push(node).unwrap();
    }
    pub fn push_edge(&mut self, edge: RetainedEdge) {
        assert!(self.transfer.is_none());
        self.edges.push(edge).unwrap();
    }
    pub fn push_byte(&mut self, byte: u8) {
        assert!(self.transfer.is_none());
        self.bytes.push(byte).unwrap();
    }
    pub fn set_node(&mut self, index: usize, node: RetainedNode) {
        assert!(self.transfer.is_none() && index < self.nodes.length);
        // SAFETY: unique initialized non-dropping slot.
        unsafe { self.nodes.pointer.as_ptr().add(index).write(node) };
    }
    pub fn set_edge(&mut self, index: usize, edge: RetainedEdge) {
        assert!(self.transfer.is_none() && index < self.edges.length);
        // SAFETY: unique initialized non-dropping slot.
        unsafe { self.edges.pointer.as_ptr().add(index).write(edge) };
    }
    pub fn node(&self, index: usize) -> RetainedNode {
        self.nodes.as_slice()[index]
    }
    pub fn push_frame(&mut self, frame: G) {
        assert!(self.transfer.is_none());
        self.frames.push(frame).unwrap();
    }
    pub fn frame_mut(&mut self) -> &mut G {
        assert!(self.transfer.is_none() && self.frames.length != 0);
        // SAFETY: unique last initialized slot.
        unsafe { &mut *self.frames.pointer.as_ptr().add(self.frames.length - 1) }
    }
    pub fn pop_frame(&mut self) -> G {
        assert!(self.transfer.is_none() && self.frames.length != 0);
        self.frames.length -= 1;
        // SAFETY: initialized last element moved out, prefix shortened.
        unsafe { self.frames.pointer.as_ptr().add(self.frames.length).read() }
    }
}
impl<G> Drop for Rebuild<'_, G> {
    fn drop(&mut self) {
        if let Some(transfer) = self.transfer.take() {
            *self.releases += 1;
            match transfer {
                Transfer::Nodes(mut new) => self.accounting.release(&mut new, Account::Temporary),
                Transfer::Edges(mut new) => self.accounting.release(&mut new, Account::Temporary),
                Transfer::Bytes(mut new) => self.accounting.release(&mut new, Account::Temporary),
                Transfer::Frames(mut new) => self.accounting.release(&mut new, Account::Temporary),
            }
        }
        *self.releases += u64::from(self.frames.capacity != 0);
        self.accounting
            .release(&mut self.frames, Account::Temporary);
    }
}

impl<F> Drop for Storage<F> {
    fn drop(&mut self) {
        self.clear();
    }
}

unsafe fn copy_element<T>(old: &Arena<T>, new: &mut Arena<T>, index: usize) {
    // SAFETY: the caller establishes the single initialized source and target
    // ranges. This helper never executes an input-sized bulk copy.
    unsafe {
        ptr::copy_nonoverlapping(
            old.pointer.as_ptr().add(index),
            new.pointer.as_ptr().add(index),
            1,
        );
    }
    new.length += 1;
}

const _: () = assert!(!mem::needs_drop::<RetainedNode>());
