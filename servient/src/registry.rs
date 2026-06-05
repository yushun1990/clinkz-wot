use std::collections::BTreeMap;

use clinkz_wot_core::LocalThing;

/// Registry boundary for locally exposed Thing dispatchers.
pub trait ExposedThingRegistry {
    /// Returns true when the registry contains the given Thing id.
    fn contains_id(&self, id: &str) -> bool;

    /// Inserts a local Thing dispatcher by Thing id.
    fn insert(&mut self, id: String, thing: LocalThing) -> Option<LocalThing>;

    /// Removes a local Thing dispatcher by Thing id.
    fn remove(&mut self, id: &str) -> Option<LocalThing>;

    /// Returns a mutable local Thing dispatcher by Thing id.
    fn get_mut(&mut self, id: &str) -> Option<&mut LocalThing>;
}

/// Deterministic in-memory registry for locally exposed Things.
pub struct InMemoryExposedThingRegistry {
    things: BTreeMap<String, LocalThing>,
}

impl InMemoryExposedThingRegistry {
    /// Creates an empty exposed Thing registry.
    pub fn new() -> Self {
        Self {
            things: BTreeMap::new(),
        }
    }

    /// Returns the number of exposed Things in the registry.
    pub fn len(&self) -> usize {
        self.things.len()
    }

    /// Returns true when the registry contains no exposed Things.
    pub fn is_empty(&self) -> bool {
        self.things.is_empty()
    }
}

impl Default for InMemoryExposedThingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ExposedThingRegistry for InMemoryExposedThingRegistry {
    fn contains_id(&self, id: &str) -> bool {
        self.things.contains_key(id)
    }

    fn insert(&mut self, id: String, thing: LocalThing) -> Option<LocalThing> {
        self.things.insert(id, thing)
    }

    fn remove(&mut self, id: &str) -> Option<LocalThing> {
        self.things.remove(id)
    }

    fn get_mut(&mut self, id: &str) -> Option<&mut LocalThing> {
        self.things.get_mut(id)
    }
}
