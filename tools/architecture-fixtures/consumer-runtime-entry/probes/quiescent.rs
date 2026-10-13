// EXPERIMENT ONLY. This appendix is compiled beside an otherwise unchanged
// copy of Planning's production program. It is not a public product API.
// No unsafe Send/Sync implementation, copied output or replacement allocator.

pub struct UniqueOutput<A: 'static> {
    node: ManuallyDrop<Box<Node<A>>>,
    owner: (SlotIndex, Generation),
    bytes: u64,
}

pub struct UniqueCleanup<A: 'static>(UniqueOutput<A>);

impl<A: 'static> UniqueOutput<A> {
    pub fn address(&self) -> usize {
        (&**self.node as *const Node<A>) as usize
    }
    pub fn try_take(
        draft: ConsumerPropertyReadDraft<A>,
        mut slot: ConsumerPropertyReadCleanupSlot<A>,
        budget: &mut WorkBudget,
    ) -> Result<
        Self,
        (
            ConsumerPropertyReadDraft<A>,
            ConsumerPropertyReadCleanupSlot<A>,
        ),
    > {
        // Identity of the actual allocation, not equal public reservation tuples.
        if slot.fixed.is_some()
            || slot.root.as_ref().is_none_or(|root| {
                root.pointer != draft.root.pointer
                    || root.node().state.get() != RootState::Draft
                    || root.node().references.get() != 2
            })
            || budget.consume(W::PlanningItems, 1).is_err()
        {
            return Err((draft, slot));
        }
        let pointer = draft.root.pointer;
        let owner = slot.owner;
        let bytes = slot.bytes;
        // Safety: Draft and its matching slot are the only two references.
        // Their consuming arguments end every selection loan. Both old Root
        // destructors are suppressed, and exactly one Box now owns the original
        // alloc(Layout::new::<Node<A>>()) block. ManuallyDrop<Data<A>> prevents
        // variable destruction. No reference can remain on the sending thread.
        core::mem::forget(draft);
        core::mem::forget(slot.root.take().unwrap());
        let node = unsafe { Box::from_raw(pointer.as_ptr()) };
        node.references.set(0);
        Ok(Self {
            node: ManuallyDrop::new(node),
            owner,
            bytes,
        })
    }

    pub fn into_local(
        mut self,
    ) -> (
        ConsumerPropertyReadDraft<A>,
        ConsumerPropertyReadCleanupSlot<A>,
    ) {
        // Safety: self uniquely owns the original allocation. Reconstitute the
        // existing two-root local protocol before exposing any borrow. This
        // experiment always returns to that production protocol for selection,
        // abandonment and bounded reclaim; it has no second cleanup algorithm.
        let node = unsafe { ManuallyDrop::take(&mut self.node) };
        node.references.set(2);
        let pointer = NonNull::new(Box::into_raw(node)).unwrap();
        let root = || Root {
            pointer,
            _local: PhantomData,
        };
        (
            ConsumerPropertyReadDraft { root: root() },
            ConsumerPropertyReadCleanupSlot {
                root: Some(root()),
                fixed: None,
                owner: self.owner,
                bytes: self.bytes,
            },
        )
    }
}

impl<A: 'static> UniqueCleanup<A> {
    pub fn address(&self) -> usize {
        self.0.address()
    }
    pub fn paid_bytes(&self) -> u64 {
        // Unique ownership; this is only a scalar observation for the probe.
        unsafe { &*self.0.node.data.get() }.reclaim.bytes_paid
    }
    pub fn try_take(
        cleanup: ConsumerPropertyReadRollback<A>,
        mut slot: ConsumerPropertyReadCleanupSlot<A>,
        budget: &mut WorkBudget,
    ) -> Result<
        Self,
        (
            ConsumerPropertyReadRollback<A>,
            ConsumerPropertyReadCleanupSlot<A>,
        ),
    > {
        let RollbackOwner::Shared(root) = &cleanup.owner else {
            return Err((cleanup, slot));
        };
        if slot.fixed.is_some()
            || slot.root.as_ref().is_none_or(|anchor| {
                anchor.pointer != root.pointer
                    || root.node().state.get() != RootState::Rollback
                    || root.node().references.get() != 2
            })
            || budget.consume(W::PlanningItems, 1).is_err()
        {
            return Err((cleanup, slot));
        }
        let pointer = root.pointer;
        // The same two-reference transfer proof as the Frozen case above.
        core::mem::forget(cleanup);
        core::mem::forget(slot.root.take().unwrap());
        let node = unsafe { Box::from_raw(pointer.as_ptr()) };
        node.references.set(0);
        Ok(Self(UniqueOutput {
            node: ManuallyDrop::new(node),
            owner: slot.owner,
            bytes: slot.bytes,
        }))
    }
    pub fn into_local(
        mut self,
    ) -> (
        ConsumerPropertyReadRollback<A>,
        ConsumerPropertyReadCleanupSlot<A>,
    ) {
        // Reconstitute the original rollback lease and matching anchor. There
        // is no draft destructor here to change the retained rollback state.
        let node = unsafe { ManuallyDrop::take(&mut self.0.node) };
        node.references.set(2);
        let pointer = NonNull::new(Box::into_raw(node)).unwrap();
        let root = || Root {
            pointer,
            _local: PhantomData,
        };
        (
            ConsumerPropertyReadRollback {
                owner: RollbackOwner::Shared(root()),
            },
            ConsumerPropertyReadCleanupSlot {
                root: Some(root()),
                fixed: None,
                owner: self.0.owner,
                bytes: self.0.bytes,
            },
        )
    }
}

pub fn rollback_paid_bytes<A: 'static>(cleanup: &ConsumerPropertyReadRollback<A>) -> u64 {
    match &cleanup.owner {
        RollbackOwner::Shared(root) => root.data().reclaim.bytes_paid,
        RollbackOwner::Fixed(_) => 0,
    }
}

// This advanced experimental owner has the same manual-retention obligation
// as the original root: dropping it with live output does not reclaim it.
// A runtime must retain it until bounded reclaim; it is not a drop safeguard,
// a Published record, an executor task, or a publication/capacity permit.
