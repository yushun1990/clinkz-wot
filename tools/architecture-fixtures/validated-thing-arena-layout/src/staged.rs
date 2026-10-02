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

/// Fixture scaffolding, not a public TD storage API. F must be non-dropping;
/// it can contain borrowed public iterators, never an owning collection.
pub struct Storage<F> {
    accounting: Accounting,
    nodes: Arena<RetainedNode>,
    edges: Arena<RetainedEdge>,
    bytes: Arena<u8>,
    frames: Arena<F>,
    retained_nodes: Arena<RetainedNode>,
    retained_edges: Arena<RetainedEdge>,
    retained_bytes: Arena<u8>,
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
            retained_nodes: Arena::default(),
            retained_edges: Arena::default(),
            retained_bytes: Arena::default(),
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
                    self.retained_nodes = new;
                } else {
                    self.nodes = new;
                }
            }
            Transfer::Edges(new) => {
                self.release_site(Site::Edges);
                if self.sealing {
                    self.retained_edges = new;
                } else {
                    self.edges = new;
                }
            }
            Transfer::Bytes(new) => {
                self.release_site(Site::Bytes);
                if self.sealing {
                    self.retained_bytes = new;
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

    pub fn release_site(&mut self, site: Site) {
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
        self.nodes.push(node).unwrap();
    }
    pub fn push_edge(&mut self, edge: RetainedEdge) {
        self.edges.push(edge).unwrap();
    }
    pub fn push_byte(&mut self, byte: u8) {
        self.bytes.push(byte).unwrap();
    }
    pub fn push_frame(&mut self, frame: F) {
        self.frames.push(frame).unwrap();
    }

    pub fn pop_frame(&mut self) -> F {
        assert!(self.frames.length != 0);
        self.frames.length -= 1;
        // SAFETY: move out the initialized last element. The shortened prefix
        // no longer contains this non-Copy frame.
        unsafe { self.frames.pointer.as_ptr().add(self.frames.length).read() }
    }

    pub fn frame_mut(&mut self) -> &mut F {
        assert!(self.frames.length != 0 && self.transfer.is_none());
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
        assert!(index < self.nodes.length);
        // SAFETY: initialized index, unique owner, non-dropping scalar value.
        unsafe { self.nodes.pointer.as_ptr().add(index).write(node) };
    }
    pub fn set_edge(&mut self, index: usize, edge: RetainedEdge) {
        assert!(index < self.edges.length);
        // SAFETY: initialized index, unique owner, non-dropping scalar value.
        unsafe { self.edges.pointer.as_ptr().add(index).write(edge) };
    }
    pub fn set_byte(&mut self, index: usize, byte: u8) {
        assert!(index < self.bytes.length);
        // SAFETY: initialized index and unique owner.
        unsafe { self.bytes.pointer.as_ptr().add(index).write(byte) };
    }
    pub fn truncate(&mut self, site: Site, length: usize) {
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
        owned.retained_nodes = mem::take(&mut self.retained_nodes);
        owned.retained_edges = mem::take(&mut self.retained_edges);
        owned.retained_bytes = mem::take(&mut self.retained_bytes);
        owned.allocations = self.allocations;
        owned.releases = self.releases;
        owned.sealed = true;
        owned
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
                self.retained_nodes.request_bytes,
                self.retained_edges.request_bytes,
                self.retained_bytes.request_bytes,
            ]
            .iter()
            .filter(|&&n| n != 0)
            .count() as u64,
            largest_request_bytes: self.accounting.ledger.largest_contiguous_allocation(),
            temporary_peak_bytes: self.accounting.temporary_peak,
            conversion_peak_bytes: self.accounting.ledger.peak_live_bytes(),
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
        if self.retained_nodes.request_bytes != 0 {
            self.releases += 1;
        }
        if self.retained_edges.request_bytes != 0 {
            self.releases += 1;
        }
        if self.retained_bytes.request_bytes != 0 {
            self.releases += 1;
        }
        self.accounting
            .release(&mut self.retained_nodes, Account::Source);
        self.accounting
            .release(&mut self.retained_edges, Account::Source);
        self.accounting
            .release(&mut self.retained_bytes, Account::Source);
        assert_eq!(self.live_bytes(), 0);
    }

    fn release_pending<T>(&mut self, arena: &mut Arena<T>, account: Account) {
        if arena.request_bytes != 0 {
            self.releases += 1;
        }
        self.accounting.release(arena, account);
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
