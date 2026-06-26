//! Interned live-instance registry for consumed (remote) Things (baseline v3.0
//! §5.1).
//!
//! [`ConsumedThingRegistry`] is the interning map of [`ConsumedThingEntry`]
//! instances keyed by Thing identity. `consume()` of the same Thing returns
//! handles that share one canonical live entry, so form selections, binding
//! plans, and (future) open binding sessions are computed once and reused.
//!
//! The registry is always in-memory and never persisted. On restart it is
//! lazily rebuilt as the application calls `consume()` again (baseline §5.1).
//! [`invalidate`](ConsumedThingRegistry::invalidate) removes a stale entry so
//! the next `consume()` rebuilds its form selections and binding plans from
//! the updated TD (baseline §5.2).

use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec::Vec};

use clinkz_wot_core::{MapLock, MapLockError, SubscriptionGuard};
use clinkz_wot_td::thing::Thing;

use crate::{BindingPlan, SelectedFormCacheKey};

/// Key identifying one active streaming subscription within a
/// [`ConsumedThingEntry`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SubscriptionKey {
    target_kind: &'static str,
    target_name: String,
    operation: String,
}

impl SubscriptionKey {
    pub(crate) fn new(target: &clinkz_wot_core::AffordanceTarget, operation: &str) -> Self {
        let (target_kind, target_name) = match target {
            clinkz_wot_core::AffordanceTarget::Property(name) => ("property", name.as_str()),
            clinkz_wot_core::AffordanceTarget::Action(name) => ("action", name.as_str()),
            clinkz_wot_core::AffordanceTarget::Event(name) => ("event", name.as_str()),
            clinkz_wot_core::AffordanceTarget::Thing => ("thing", ""),
        };
        Self {
            target_kind,
            target_name: target_name.into(),
            operation: operation.into(),
        }
    }
}

/// Type-erased subscription guard stored in the entry.
#[cfg(not(feature = "std"))]
pub(crate) type BoxedGuard = Box<dyn SubscriptionGuard + Send>;
#[cfg(feature = "std")]
pub(crate) type BoxedGuard = Box<dyn SubscriptionGuard + Send + Sync>;

/// One interned consumed-Thing entry (baseline v3.0 §5.1).
///
/// Holds the Thing Description plus internalized form selections and binding
/// plans computed once and reused across interactions. This replaces the
/// per-call recomputation and the separate Servient-level caches.
///
/// The Thing is stored as `Arc<Thing>` so each interaction can clone a cheap
/// reference instead of deep-cloning the full TD on every consumed request.
pub(crate) struct ConsumedThingEntry {
    thing: Arc<Thing>,
    plan_cache: MapLock<BTreeMap<SelectedFormCacheKey, BindingPlan>>,
    subscriptions: MapLock<BTreeMap<SubscriptionKey, Vec<BoxedGuard>>>,
}

impl ConsumedThingEntry {
    fn new(thing: Thing) -> Self {
        Self {
            thing: Arc::new(thing),
            plan_cache: MapLock::new(BTreeMap::new()),
            subscriptions: MapLock::new(BTreeMap::new()),
        }
    }

    /// Returns the Thing Description for this entry.
    pub(crate) fn thing(&self) -> &Thing {
        &self.thing
    }

    /// Returns a cheap clone of the shared Thing Description.
    ///
    /// Use this in interaction paths instead of `thing().clone()` to avoid
    /// deep-cloning the full TD on every consumed request.
    pub(crate) fn thing_arc(&self) -> Arc<Thing> {
        Arc::clone(&self.thing)
    }

    /// Retrieves a cached binding plan by key.
    pub(crate) fn get_plan(&self, key: &SelectedFormCacheKey) -> Option<BindingPlan> {
        self.plan_cache
            .with_recover(|cache| cache.get(key).cloned())
    }

    /// Inserts or replaces a cached binding plan.
    pub(crate) fn insert_plan(&self, key: SelectedFormCacheKey, plan: BindingPlan) {
        let _ = self.plan_cache.with(|cache| {
            cache.insert(key, plan);
        });
    }

    /// Removes a cached binding plan.
    pub(crate) fn remove_plan(&self, key: &SelectedFormCacheKey) {
        let _ = self.plan_cache.with(|cache| {
            cache.remove(key);
        });
    }

    /// Updates the cached binding plan's factory generation without
    /// revalidating the form. Used after revalidation succeeds to keep the
    /// cache entry fresh against the current registry generation.
    pub(crate) fn update_plan_generation(&self, key: &SelectedFormCacheKey, generation: u64) {
        let _ = self.plan_cache.with(|cache| {
            if let Some(plan) = cache.get_mut(key) {
                plan.factory_generation = generation;
            }
        });
    }

    /// Stores a subscription guard so the underlying wire subscription stays
    /// alive until [`stop_subscriptions`](Self::stop_subscriptions) is called.
    pub(crate) fn store_subscription(&self, key: SubscriptionKey, guard: BoxedGuard) {
        let _ = self.subscriptions.with(|map| {
            map.entry(key).or_default().push(guard);
        });
    }

    /// Stops and removes all wire subscriptions matching `key`.
    pub(crate) fn stop_subscriptions(&self, key: &SubscriptionKey) {
        let guards = self.subscriptions.with_recover(|map| map.remove(key));
        if let Some(guards) = guards {
            for guard in guards {
                guard.close();
            }
        }
    }

    /// Stops and removes all wire subscriptions for this entry.
    pub(crate) fn stop_all_subscriptions(&self) {
        let drained = self.subscriptions.with_recover(core::mem::take);
        for (_, guards) in drained {
            for guard in guards {
                guard.close();
            }
        }
    }
}

/// Interning map of live consumed-Thing entries (baseline v3.0 §5.1).
///
/// Fully interior-mutable; every method takes `&self`.
pub(crate) struct ConsumedThingRegistry {
    entries: MapLock<BTreeMap<String, Arc<ConsumedThingEntry>>>,
}

impl ConsumedThingRegistry {
    /// Creates an empty consumed-Thing registry.
    pub(crate) fn new() -> Self {
        Self {
            entries: MapLock::new(BTreeMap::new()),
        }
    }

    /// Interns a consumed Thing: returns the existing entry for the same id, or
    /// creates and inserts a new one (baseline v3.0 §5.1 identity interning).
    ///
    /// The caller must supply the Thing id separately so the registry does not
    /// need to re-extract it from the TD.
    ///
    /// Returns [`MapLockError`] if the registry lock was poisoned; the interning
    /// is then skipped rather than applied to inconsistent state.
    pub(crate) fn get_or_insert(
        &self,
        id: String,
        thing: Thing,
    ) -> Result<Arc<ConsumedThingEntry>, MapLockError> {
        self.entries.with(|map| {
            if let Some(existing) = map.get(&id) {
                Arc::clone(existing)
            } else {
                let entry = Arc::new(ConsumedThingEntry::new(thing));
                map.insert(id, Arc::clone(&entry));
                entry
            }
        })
    }

    /// Removes the entry for `id` so the next `consume()` rebuilds form
    /// selections and binding plans from the updated TD (baseline v3.0 §5.2).
    ///
    /// Also stops all active streaming subscriptions for this entry so wire
    /// resources are released.
    pub(crate) fn invalidate(&self, id: &str) {
        let entry = self.entries.with_recover(|map| map.remove(id));
        if let Some(entry) = entry {
            entry.stop_all_subscriptions();
        }
    }
}

impl Default for ConsumedThingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        boxed::Box,
        string::{String, ToString},
        sync::Arc,
    };
    use core::sync::atomic::{AtomicBool, Ordering};

    use clinkz_wot_core::{AffordanceTarget, SubscriptionGuard};
    use clinkz_wot_td::thing::Thing;

    use super::{ConsumedThingRegistry, SubscriptionKey};

    struct ReentrantInvalidateGuard {
        registry: Arc<ConsumedThingRegistry>,
        id: String,
        reentered: Arc<AtomicBool>,
    }

    impl SubscriptionGuard for ReentrantInvalidateGuard {
        fn close(self: Box<Self>) {
            let missing = self
                .registry
                .entries
                .with_recover(|entries| !entries.contains_key(&self.id));
            self.reentered.store(missing, Ordering::Relaxed);
        }
    }

    #[test]
    fn invalidate_removes_entry_before_closing_subscription_guards() {
        let registry = Arc::new(ConsumedThingRegistry::new());
        let thing = Thing::builder("Reentrant Invalidate")
            .id("urn:thing:reentrant-invalidate")
            .nosec()
            .build()
            .unwrap();
        let entry = registry
            .get_or_insert("urn:thing:reentrant-invalidate".to_string(), thing)
            .unwrap();
        let reentered = Arc::new(AtomicBool::new(false));

        entry.store_subscription(
            SubscriptionKey::new(&AffordanceTarget::Event("startup".into()), "subscribeevent"),
            Box::new(ReentrantInvalidateGuard {
                registry: Arc::clone(&registry),
                id: "urn:thing:reentrant-invalidate".to_string(),
                reentered: Arc::clone(&reentered),
            }),
        );

        registry.invalidate("urn:thing:reentrant-invalidate");

        assert!(reentered.load(Ordering::Relaxed));
        assert!(registry.entries.with_recover(|entries| entries.is_empty()));
    }
}
