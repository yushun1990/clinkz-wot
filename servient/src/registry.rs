//! Exposed/consumed registries (baseline v4.0 §7.1 / phase-p3 §3.1).
//!
//! v1 uses `WotLock<BTreeMap<..>>` for both registries (clone-out dispatch
//! discipline — handler invocation is outside the lock). The std
//! `arc_swap::ArcSwap<Arc<im::HashMap>>` lock-free snapshot is a documented
//! perf refinement (AD2/C1/AD54), not a correctness requirement.

use alloc::{collections::BTreeSet, sync::Arc};

use clinkz_wot_core::{ExposedThing, ThingId, WotLock};

/// A registered exposed Thing: the core [`ExposedThing`] plus a `draining` flag
/// for `destroy()` quiescing (AD15).
pub(crate) struct ExposedThingSlot {
    pub thing: ExposedThing,
    /// Set during `destroy()`: not-yet-dispatched requests targeting this Thing
    /// are rejected (request/response → "Thing gone" error; streaming dropped).
    pub draining: core::sync::atomic::AtomicBool,
}

impl ExposedThingSlot {
    pub(crate) fn new(thing: ExposedThing) -> Self {
        Self {
            thing,
            draining: core::sync::atomic::AtomicBool::new(false),
        }
    }
}

/// Registry of servable exposed Things, keyed by [`ThingId`].
#[derive(Clone, Default)]
pub(crate) struct ExposedThingRegistry {
    entries: WotLock<alloc::collections::BTreeMap<ThingId, Arc<WotLock<ExposedThingSlot>>>>,
}

impl ExposedThingRegistry {
    /// Atomically inserts `slot` unless `id` is already exposed (AD33: duplicate
    /// `expose` rejected with `AlreadyExposed`).
    pub(crate) fn insert(
        &self,
        id: ThingId,
        slot: Arc<WotLock<ExposedThingSlot>>,
    ) -> Result<(), ThingId> {
        self.entries.with(|m| {
            if m.contains_key(&id) {
                return Err(id.clone());
            }
            m.insert(id, slot);
            Ok(())
        })
    }

    /// Looks up a slot by id (clones the `Arc` out under a brief lock).
    pub(crate) fn get(&self, id: &ThingId) -> Option<Arc<WotLock<ExposedThingSlot>>> {
        self.entries.with_read(|m| m.get(id).cloned())
    }

    /// Removes a slot by id; returns whether it was present (idempotent destroy —
    /// AD27/E13).
    pub(crate) fn remove(&self, id: &ThingId) -> bool {
        self.entries.with(|m| m.remove(id).is_some())
    }

    /// Whether `id` is currently exposed.
    pub(crate) fn contains(&self, id: &ThingId) -> bool {
        self.entries.with_read(|m| m.contains_key(id))
    }

    /// Number of exposed Things.
    #[allow(dead_code)]
    pub(crate) fn len(&self) -> usize {
        self.entries.with_read(|m| m.len())
    }
}

/// Registry of consumed Things (directory-invalidation tracking; AD53/E7).
/// The handle owns the live [`clinkz_wot_core::ConsumedThing`]; this registry
/// records the consumed ids so directory `update`/`unregister` can invalidate
/// them.
#[derive(Clone, Default)]
pub(crate) struct ConsumedThingRegistry {
    ids: WotLock<BTreeSet<ThingId>>,
}

impl ConsumedThingRegistry {
    pub(crate) fn track(&self, id: ThingId) {
        self.ids.with(|s| {
            s.insert(id);
        });
    }

    pub(crate) fn untrack(&self, id: &ThingId) {
        self.ids.with(|s| {
            s.remove(id);
        });
    }

    /// Invalidate a consumed Thing (directory-driven); returns whether it was
    /// tracked. The handle's wire subscriptions are closed separately (E7).
    #[allow(dead_code)]
    pub(crate) fn invalidate(&self, id: &ThingId) -> bool {
        self.remove(id)
    }

    pub(crate) fn remove(&self, id: &ThingId) -> bool {
        self.ids.with(|s| s.remove(id))
    }
}
