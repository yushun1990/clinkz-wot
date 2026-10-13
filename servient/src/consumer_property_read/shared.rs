//! Fixed-graph, fallible Host ownership. No public Clone surface and no weak
//! references. AtomicPtr plus PhantomData<Arc<T>> derives exactly Arc's Send /
//! Sync restrictions structurally; there is no unsafe auto-trait assertion.
use alloc::{
    alloc::{alloc, dealloc},
    sync::Arc,
};
use core::{
    alloc::Layout,
    marker::PhantomData,
    mem::ManuallyDrop,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering, fence},
};

pub(super) trait FinalRelease {
    fn before_release(&mut self) {}
}
struct Node<T> {
    references: AtomicUsize,
    value: ManuallyDrop<T>,
}
pub(super) struct Shared<T: FinalRelease> {
    pointer: AtomicPtr<Node<T>>,
    traits: PhantomData<Arc<T>>,
}
impl<T: FinalRelease> Shared<T> {
    pub(super) fn allocation_layout() -> Layout {
        Layout::new::<Node<T>>()
    }
    pub(super) fn try_new(value: T) -> Result<Self, T> {
        let layout = Self::allocation_layout();
        // SAFETY: nonzero checked native Layout; failure leaves value untouched.
        let pointer = unsafe { alloc(layout) }.cast::<Node<T>>();
        if pointer.is_null() {
            return Err(value);
        }
        // SAFETY: the unique block is aligned and sized for this initialized node.
        unsafe {
            pointer.write(Node {
                references: AtomicUsize::new(1),
                value: ManuallyDrop::new(value),
            });
        }
        Ok(Self {
            pointer: AtomicPtr::new(pointer),
            traits: PhantomData,
        })
    }
    fn node(&self) -> &Node<T> {
        // SAFETY: this non-Clone owner retains one strong reference; the pointer
        // never changes and the node is released only by the last reference.
        unsafe { &*self.pointer.load(Ordering::Relaxed) }
    }
    pub(super) fn get(&self) -> &T {
        &self.node().value
    }
    #[allow(deprecated)] // Preserve the accepted Rust 1.95 cell as well.
    pub(super) fn retain(&self) -> Self {
        // The runtime admits at most root, handle, ticket, driver and build
        // owners. Report has exactly root and observer. No user can mint refs.
        self.node()
            .references
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                (n < 8).then_some(n + 1)
            })
            .expect("fixed Host ownership graph");
        Self {
            pointer: AtomicPtr::new(self.pointer.load(Ordering::Relaxed)),
            traits: PhantomData,
        }
    }
}
impl<T: FinalRelease> Drop for Shared<T> {
    fn drop(&mut self) {
        if self.node().references.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }
        fence(Ordering::Acquire);
        let pointer = self.pointer.load(Ordering::Relaxed);
        // SAFETY: last owner, no borrows survive it. Move the payload into the
        // prepaid terminal transport, release the physical node, then destroy
        // children and release payload accounts/parents. ManuallyDrop prevents
        // a second destruction. This is the only freeing path.
        let mut value = unsafe { ManuallyDrop::take(&mut (*pointer).value) };
        value.before_release();
        unsafe {
            dealloc(pointer.cast(), Self::allocation_layout());
        }
        drop(value);
    }
}
