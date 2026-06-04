use std::collections::BTreeMap;

use clinkz_wot_td::{
    thing::Thing,
    validate::{Validate, ValidationLevel},
};

use crate::{DirectoryQuery, DiscoveryError, DiscoveryResult, QueryPredicate};

/// A borrowed Thing Directory entry.
#[derive(Debug, Clone, Copy)]
pub struct BorrowedDirectoryEntry<'a> {
    /// Stable Thing identifier used as the directory key.
    pub id: &'a str,
    /// Stored Thing Description.
    pub thing: &'a Thing,
}

/// An owned Thing Directory entry.
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    /// Stable Thing identifier used as the directory key.
    pub id: String,
    /// Stored Thing Description.
    pub thing: Thing,
}

/// A deterministic page of Thing Directory entries.
#[derive(Debug, Clone)]
pub struct DirectoryPage {
    /// Entries returned for this page.
    pub entries: Vec<DirectoryEntry>,
    /// Total matching entries before pagination.
    pub total: usize,
    /// Number of matching entries skipped.
    pub offset: usize,
    /// Maximum number of entries requested.
    pub limit: Option<usize>,
}

/// Protocol-neutral Thing Description Directory behavior.
pub trait ThingDirectory {
    /// Registers a new TD and rejects duplicate Thing ids.
    fn register(&mut self, thing: Thing) -> DiscoveryResult<DirectoryEntry>;

    /// Replaces an existing TD with the same Thing id.
    fn update(&mut self, thing: Thing) -> DiscoveryResult<DirectoryEntry>;

    /// Removes a TD by Thing id.
    fn delete(&mut self, id: &str) -> DiscoveryResult<Thing>;

    /// Retrieves a TD by Thing id.
    fn get(&self, id: &str) -> DiscoveryResult<Thing>;

    /// Lists all TDs in deterministic Thing id order.
    fn list(&self) -> DirectoryPage;

    /// Queries TDs using a backend-portable query model.
    fn query(&self, query: DirectoryQuery) -> DirectoryPage;
}

/// Deterministic in-memory Thing Description Directory.
#[derive(Debug)]
pub struct InMemoryThingDirectory {
    things: BTreeMap<String, Thing>,
    validation_level: ValidationLevel,
}

impl InMemoryThingDirectory {
    /// Creates an empty directory that validates writes at `Basic` level.
    pub fn new() -> Self {
        Self::with_validation_level(ValidationLevel::Basic)
    }

    /// Creates an empty directory with the requested TD validation level.
    pub fn with_validation_level(validation_level: ValidationLevel) -> Self {
        Self {
            things: BTreeMap::new(),
            validation_level,
        }
    }

    /// Returns the validation level applied before registration and update.
    pub fn validation_level(&self) -> ValidationLevel {
        self.validation_level
    }

    /// Returns the number of TDs stored in the directory.
    pub fn len(&self) -> usize {
        self.things.len()
    }

    /// Returns true when the directory has no TDs.
    pub fn is_empty(&self) -> bool {
        self.things.is_empty()
    }

    fn validate_for_write(&self, thing: &Thing) -> DiscoveryResult<String> {
        thing.validate_with_level(self.validation_level)?;
        thing
            .id
            .as_ref()
            .map(|id| id.as_str().to_owned())
            .ok_or(DiscoveryError::MissingThingId)
    }

    fn borrowed_entries(&self) -> impl Iterator<Item = BorrowedDirectoryEntry<'_>> {
        self.things
            .iter()
            .map(|(id, thing)| BorrowedDirectoryEntry {
                id: id.as_str(),
                thing,
            })
    }

    fn owned_entry(id: &str, thing: &Thing) -> DirectoryEntry {
        DirectoryEntry {
            id: id.to_owned(),
            thing: thing.clone(),
        }
    }

    /// Filters TDs with a local predicate in deterministic Thing id order.
    pub fn query_predicate<Q>(&self, query: Q) -> Vec<BorrowedDirectoryEntry<'_>>
    where
        Q: QueryPredicate,
    {
        self.borrowed_entries()
            .filter(|entry| query.matches(entry.thing))
            .collect()
    }
}

impl Default for InMemoryThingDirectory {
    fn default() -> Self {
        Self::new()
    }
}

impl ThingDirectory for InMemoryThingDirectory {
    fn register(&mut self, thing: Thing) -> DiscoveryResult<DirectoryEntry> {
        let id = self.validate_for_write(&thing)?;
        if self.things.contains_key(&id) {
            return Err(DiscoveryError::DuplicateThingId(id));
        }

        let stored = self.things.entry(id.clone()).or_insert(thing);
        Ok(Self::owned_entry(&id, stored))
    }

    fn update(&mut self, thing: Thing) -> DiscoveryResult<DirectoryEntry> {
        let id = self.validate_for_write(&thing)?;
        let slot = self
            .things
            .get_mut(&id)
            .ok_or_else(|| DiscoveryError::ThingNotFound(id.clone()))?;
        *slot = thing;
        Ok(Self::owned_entry(&id, slot))
    }

    fn delete(&mut self, id: &str) -> DiscoveryResult<Thing> {
        self.things
            .remove(id)
            .ok_or_else(|| DiscoveryError::ThingNotFound(id.to_owned()))
    }

    fn get(&self, id: &str) -> DiscoveryResult<Thing> {
        self.things
            .get(id)
            .cloned()
            .ok_or_else(|| DiscoveryError::ThingNotFound(id.to_owned()))
    }

    fn list(&self) -> DirectoryPage {
        self.query(DirectoryQuery::all())
    }

    fn query(&self, query: DirectoryQuery) -> DirectoryPage {
        let matches: Vec<_> = self
            .things
            .iter()
            .filter(|(_, thing)| query.matches(thing))
            .collect();
        let total = matches.len();
        let limit = query.limit.unwrap_or(usize::MAX);
        let entries = matches
            .into_iter()
            .skip(query.offset)
            .take(limit)
            .map(|(id, thing)| Self::owned_entry(id, thing))
            .collect();

        DirectoryPage {
            entries,
            total,
            offset: query.offset,
            limit: query.limit,
        }
    }
}
