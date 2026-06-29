use alloc::{
    borrow::ToOwned,
    collections::{BTreeMap, BTreeSet, btree_map::Entry},
    string::String,
    vec,
    vec::Vec,
};

use clinkz_wot_td::{
    thing::Thing,
    validate::{Validate, ValidationLevel},
};

use crate::{DirectoryQuery, DiscoveryError, DiscoveryResult, QueryFilter, QueryPredicate};

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
///
/// # Relationship to W3C WoT Discovery
///
/// W3C WoT Discovery models a TDD as an HTTP/CoAP REST API in which
/// `PUT /td/{id}` is an *upsert* (201 Created on first registration, 200 OK on
/// update) and `POST /td/{id}` applies a JSON Merge Patch. This trait is
/// deliberately protocol-neutral and **splits create and replace into two
/// explicit methods** ([`register`](Self::register) is create-only and rejects
/// duplicates; [`update`](Self::update) is replace-only and rejects missing
/// ids) so that caller intent is unambiguous and storage backends are not
/// forced to emulate HTTP semantics.
///
/// Consequences for binding authors:
///
/// - A future HTTP/CoAP directory binding that maps `PUT /td/{id}` to this
///   trait must decide create-vs-replace itself (e.g. attempt [`update`](Self::update),
///   fall back to [`register`](Self::register) on "not found"), since there is
///   no upsert primitive on the trait. Partial update (`PATCH` / JSON Merge
///   Patch) is likewise out of scope here and belongs to a binding-level
///   helper.
/// - System-generated identifiers (W3C `POST /td` may mint an id) are not
///   modeled: [`register`](Self::register) requires the TD to carry an `id`.
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

    /// Visits each Thing in the directory with a borrowed reference, invoking
    /// `f` for every entry.
    ///
    /// The default implementation clones every entry via
    /// [`query`](Self::query). Backends with in-memory storage should override
    /// this to iterate without cloning, so callers (e.g. [`discover`]) can
    /// filter before cloning only the matching entries.
    ///
    /// [`discover`]: crate::discover
    fn for_each_thing(&self, mut f: impl FnMut(&Thing)) {
        for entry in self.query(DirectoryQuery::all()).entries {
            f(&entry.thing);
        }
    }
}

/// Deterministic in-memory Thing Description Directory.
///
/// Storage is backed by a primary `BTreeMap<id, Thing>` plus secondary indexes
/// over the indexable [`QueryFilter`] kinds (id, title, property, action,
/// event, and any-affordance fragment names). Indexes are kept in sync on
/// every register/update/delete, so directory queries that are fully
/// expressible through the standard filters are served by BTreeSet
/// intersection followed by a direct id-ordered walk with offset/limit — no
/// full-table scan and no re-scan from offset 0 on deep pages.
#[derive(Debug, Clone)]
pub struct InMemoryThingDirectory {
    things: BTreeMap<String, Thing>,
    title_index: BTreeMap<String, BTreeSet<String>>,
    /// Union of every affordance name (property + action + event) for the
    /// `QueryFilter::Fragment` predicate.
    affordance_index: BTreeMap<String, BTreeSet<String>>,
    property_index: BTreeMap<String, BTreeSet<String>>,
    action_index: BTreeMap<String, BTreeSet<String>>,
    event_index: BTreeMap<String, BTreeSet<String>>,
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
            title_index: BTreeMap::new(),
            affordance_index: BTreeMap::new(),
            property_index: BTreeMap::new(),
            action_index: BTreeMap::new(),
            event_index: BTreeMap::new(),
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

    /// Retrieves a borrowed reference to the TD identified by `id` without
    /// cloning the stored document.
    ///
    /// Use this inherent accessor on the concrete directory type when the
    /// caller does not need an owned [`Thing`] — for example, to inspect a
    /// field, render a fragment, or hand a reference to a downstream
    /// consumer. The trait method [`ThingDirectory::get`] remains
    /// allocation-heavy because it must work for backends that cannot lend
    /// out references.
    pub fn get_ref(&self, id: &str) -> Option<&Thing> {
        self.things.get(id)
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

    /// Add `id` to every index keyed by the relevant fields of `thing`.
    fn index_insert(
        title_index: &mut BTreeMap<String, BTreeSet<String>>,
        affordance_index: &mut BTreeMap<String, BTreeSet<String>>,
        property_index: &mut BTreeMap<String, BTreeSet<String>>,
        action_index: &mut BTreeMap<String, BTreeSet<String>>,
        event_index: &mut BTreeMap<String, BTreeSet<String>>,
        id: &str,
        thing: &Thing,
    ) {
        if let Some(title) = thing._metadata.title.as_deref() {
            title_index
                .entry(title.to_owned())
                .or_default()
                .insert(id.to_owned());
        }
        // One pass per affordance map: update both the union
        // (`affordance_index`, matched by [`QueryFilter::Fragment`]) and the
        // type-specific index. Iterating the map keys directly avoids the
        // `Vec<String>` allocation that the removed `affordance_names` helper
        // used to build on every insert.
        if let Some(properties) = thing.properties.as_ref() {
            for name in properties.keys() {
                insert_index_entry(affordance_index, name, id);
                insert_index_entry(property_index, name, id);
            }
        }
        if let Some(actions) = thing.actions.as_ref() {
            for name in actions.keys() {
                insert_index_entry(affordance_index, name, id);
                insert_index_entry(action_index, name, id);
            }
        }
        if let Some(events) = thing.events.as_ref() {
            for name in events.keys() {
                insert_index_entry(affordance_index, name, id);
                insert_index_entry(event_index, name, id);
            }
        }
    }

    /// Remove `id` from every index. Uses the supplied `thing` snapshot to
    /// know which keys to evict without re-scanning the index maps.
    fn index_remove(
        title_index: &mut BTreeMap<String, BTreeSet<String>>,
        affordance_index: &mut BTreeMap<String, BTreeSet<String>>,
        property_index: &mut BTreeMap<String, BTreeSet<String>>,
        action_index: &mut BTreeMap<String, BTreeSet<String>>,
        event_index: &mut BTreeMap<String, BTreeSet<String>>,
        id: &str,
        thing: &Thing,
    ) {
        if let Some(title) = thing._metadata.title.as_deref() {
            remove_index_entry(title_index, title, id);
        }
        // One pass per affordance map: evict from both the union
        // (`affordance_index`) and the type-specific index, mirroring
        // [`index_insert`](Self::index_insert).
        if let Some(properties) = thing.properties.as_ref() {
            for name in properties.keys() {
                remove_index_entry(affordance_index, name, id);
                remove_index_entry(property_index, name, id);
            }
        }
        if let Some(actions) = thing.actions.as_ref() {
            for name in actions.keys() {
                remove_index_entry(affordance_index, name, id);
                remove_index_entry(action_index, name, id);
            }
        }
        if let Some(events) = thing.events.as_ref() {
            for name in events.keys() {
                remove_index_entry(affordance_index, name, id);
                remove_index_entry(event_index, name, id);
            }
        }
    }

    /// Compute the candidate id set for the given directory query from the
    /// secondary indexes, or return `None` when at least one filter is not
    /// indexable and a full scan is required.
    ///
    /// The returned set, when `Some`, is always sorted by Thing id because
    /// the underlying indexes store `BTreeSet<String>`.
    ///
    /// Intersects by **reference** against the stored index sets: only the
    /// final result set is materialized (one clone), instead of cloning every
    /// matching `BTreeSet` per filter and discarding the copies after each
    /// intersection step.
    fn indexed_candidates(&self, query: &DirectoryQuery) -> Option<BTreeSet<String>> {
        let mut candidates: Option<BTreeSet<String>> = None;
        for filter in &query.filters {
            candidates = Some(match filter {
                QueryFilter::Id(id) => {
                    let present = self.things.contains_key(id);
                    match candidates {
                        None => {
                            let mut set = BTreeSet::new();
                            if present {
                                set.insert(id.clone());
                            }
                            set
                        }
                        Some(mut existing) => {
                            if present {
                                existing.retain(|x| x == id);
                            } else {
                                existing.clear();
                            }
                            existing
                        }
                    }
                }
                QueryFilter::Title(title) => intersect_set(candidates, self.title_index.get(title)),
                QueryFilter::Fragment(name) => {
                    intersect_set(candidates, self.affordance_index.get(name))
                }
                QueryFilter::Property(name) => {
                    intersect_set(candidates, self.property_index.get(name))
                }
                QueryFilter::Action(name) => intersect_set(candidates, self.action_index.get(name)),
                QueryFilter::Event(name) => intersect_set(candidates, self.event_index.get(name)),
            });
        }
        candidates
    }

    /// Direct BTreeMap lookup for a single-Id query (fast path).
    ///
    /// The map key is the Thing id, so an exact-id query is an O(log n) lookup
    /// instead of an O(n) scan. Pagination is honored: the single match (if
    /// found) is returned only when it falls within the `[offset, offset+limit)`
    /// window.
    fn query_by_id(&self, id: &str, query: &DirectoryQuery) -> DirectoryPage {
        let limit = query.limit.unwrap_or(usize::MAX);
        let page_end = query.offset.saturating_add(limit);

        match self.things.get(id) {
            Some(thing) => {
                let entries = if query.offset == 0 && page_end >= 1 {
                    vec![Self::owned_entry(id, thing)]
                } else {
                    Vec::new()
                };
                DirectoryPage {
                    entries,
                    total: 1,
                    offset: query.offset,
                    limit: query.limit,
                }
            }
            None => DirectoryPage {
                entries: Vec::new(),
                total: 0,
                offset: query.offset,
                limit: query.limit,
            },
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
        match self.things.entry(id.clone()) {
            Entry::Occupied(_) => Err(DiscoveryError::DuplicateThingId(id)),
            Entry::Vacant(vacant) => {
                let stored = vacant.insert(thing);
                // `stored` is `&mut Thing` pointing into the map; reborrow it
                // immutably for index population to avoid moving the value.
                Self::index_insert(
                    &mut self.title_index,
                    &mut self.affordance_index,
                    &mut self.property_index,
                    &mut self.action_index,
                    &mut self.event_index,
                    &id,
                    stored,
                );
                Ok(Self::owned_entry(&id, stored))
            }
        }
    }

    fn update(&mut self, thing: Thing) -> DiscoveryResult<DirectoryEntry> {
        let id = self.validate_for_write(&thing)?;
        // Read the existing TD through an immutable borrow first so we can
        // retire it from the indexes, then take a fresh mutable borrow to
        // replace it. The two-phase lookup avoids holding a mutable borrow of
        // `self.things` across calls into the index helpers.
        if !self.things.contains_key(&id) {
            return Err(DiscoveryError::ThingNotFound(id));
        }
        let old = self
            .things
            .get(&id)
            .expect("Thing present after contains_key check");
        Self::index_remove(
            &mut self.title_index,
            &mut self.affordance_index,
            &mut self.property_index,
            &mut self.action_index,
            &mut self.event_index,
            &id,
            old,
        );
        let slot = self
            .things
            .get_mut(&id)
            .expect("Thing present after removal");
        *slot = thing;
        Self::index_insert(
            &mut self.title_index,
            &mut self.affordance_index,
            &mut self.property_index,
            &mut self.action_index,
            &mut self.event_index,
            &id,
            slot,
        );
        Ok(Self::owned_entry(&id, slot))
    }

    fn delete(&mut self, id: &str) -> DiscoveryResult<Thing> {
        let removed = self
            .things
            .remove(id)
            .ok_or_else(|| DiscoveryError::ThingNotFound(id.to_owned()))?;
        Self::index_remove(
            &mut self.title_index,
            &mut self.affordance_index,
            &mut self.property_index,
            &mut self.action_index,
            &mut self.event_index,
            id,
            &removed,
        );
        Ok(removed)
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
        // Fast path: a single Id filter maps directly to the BTreeMap key,
        // turning an O(n) full-table scan into an O(log n) lookup. This is the
        // common query pattern for "fetch a specific TD by id".
        if query.filters.len() == 1
            && let QueryFilter::Id(ref id) = query.filters[0]
        {
            return self.query_by_id(id, &query);
        }

        let limit = query.limit.unwrap_or(usize::MAX);
        let page_start = query.offset;
        let page_end = query.offset.saturating_add(limit);

        // Indexed path: when every filter is expressible through the secondary
        // indexes, intersect the candidate id sets and walk the result in
        // Thing-id order. Pagination advances by skipping leading entries of
        // the sorted candidate set rather than re-scanning the whole
        // directory from id 0, so deep-page queries stay cheap.
        if let Some(candidates) = self.indexed_candidates(&query) {
            let mut total = 0usize;
            let mut entries: Vec<DirectoryEntry> = Vec::new();
            for id in candidates {
                let index = total;
                total += 1;
                if index >= page_start && index < page_end {
                    // Existence in the candidate set implies the Thing is
                    // present in the map, because indexes are maintained
                    // in lockstep with map mutations.
                    let thing = self.things.get(&id).expect("index references live Thing");
                    entries.push(Self::owned_entry(&id, thing));
                }
            }
            return DirectoryPage {
                entries,
                total,
                offset: query.offset,
                limit: query.limit,
            };
        }

        // General path: single pass over filtered matches counting `total`,
        // but cloning TDs only for the requested page. For directories with many
        // entries and small page sizes this avoids both a full match-list
        // allocation and a second filter pass.
        let mut total = 0usize;
        let mut entries: Vec<DirectoryEntry> = Vec::new();

        for (id, thing) in self.things.iter().filter(|(_, thing)| query.matches(thing)) {
            let index = total;
            total += 1;
            if index >= page_start && index < page_end {
                entries.push(Self::owned_entry(id, thing));
            }
        }

        DirectoryPage {
            entries,
            total,
            offset: query.offset,
            limit: query.limit,
        }
    }

    fn for_each_thing(&self, mut f: impl FnMut(&Thing)) {
        for thing in self.things.values() {
            f(thing);
        }
    }
}

/// Insert `id` into the set stored at `key` in `index`, creating the set if
/// needed. Mirrors [`remove_index_entry`] for symmetry.
fn insert_index_entry(index: &mut BTreeMap<String, BTreeSet<String>>, key: &str, id: &str) {
    index
        .entry(key.to_owned())
        .or_default()
        .insert(id.to_owned());
}

/// Intersect the accumulated candidate set with a borrowed index set.
///
/// `existing` is the running candidate set (taken by value so it can be
/// mutated in place via `retain`). `indexed` is a borrowed slice of the stored
/// `BTreeSet` for the current filter. Only this final/running set is ever
/// materialized — the stored index sets are intersected by reference instead
/// of being cloned per filter.
fn intersect_set(
    existing: Option<BTreeSet<String>>,
    indexed: Option<&BTreeSet<String>>,
) -> BTreeSet<String> {
    match (existing, indexed) {
        (None, None) | (Some(_), None) => BTreeSet::new(),
        (None, Some(set)) => set.clone(),
        (Some(mut acc), Some(set)) => {
            acc.retain(|id| set.contains(id));
            acc
        }
    }
}

/// Remove `id` from the set stored at `key` in `index`, dropping the entry
/// entirely when the resulting set becomes empty so the index does not
/// accumulate empty buckets over churn.
///
/// Probes with `get_mut` (borrowed key) and removes after emptiness, avoiding
/// the `Entry` API's owned-key probe allocation on every removal.
fn remove_index_entry(index: &mut BTreeMap<String, BTreeSet<String>>, key: &str, id: &str) {
    let mut remove_key = false;
    if let Some(set) = index.get_mut(key) {
        set.remove(id);
        remove_key = set.is_empty();
    }
    if remove_key {
        index.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_query_scales_without_full_scan() {
        let mut directory = InMemoryThingDirectory::new();
        for i in 0..50 {
            let id = alloc::format!("urn:thing:{i:03}");
            directory
                .register(thing_with_property(
                    &id,
                    &alloc::format!("Thing {i}"),
                    "status",
                ))
                .expect("registration succeeds");
        }

        let page = directory.query(DirectoryQuery::property("status").offset(10).limit(5));
        assert_eq!(page.total, 50);
        assert_eq!(page.entries.len(), 5);
        assert_eq!(page.entries[0].id, "urn:thing:010");
        assert_eq!(page.entries[4].id, "urn:thing:014");
    }

    #[test]
    fn indexed_query_intersection_returns_sorted_ids() {
        let mut directory = InMemoryThingDirectory::new();
        directory
            .register(thing_with_property("urn:thing:c", "Lamp", "status"))
            .expect("registration succeeds");
        directory
            .register(thing_with_property("urn:thing:a", "Lamp", "status"))
            .expect("registration succeeds");
        directory
            .register(thing_with_property("urn:thing:b", "Lamp", "level"))
            .expect("registration succeeds");

        let page =
            directory.query(DirectoryQuery::title("Lamp").and(QueryFilter::property("status")));
        assert_eq!(page.total, 2);
        let ids: Vec<_> = page.entries.into_iter().map(|entry| entry.id).collect();
        assert_eq!(ids, vec!["urn:thing:a", "urn:thing:c"]);
    }

    #[test]
    fn fragment_filter_uses_affordance_index() {
        let mut directory = InMemoryThingDirectory::new();
        directory
            .register(thing_with_property("urn:thing:lamp", "Lamp", "status"))
            .expect("registration succeeds");
        directory
            .register(thing_with_action("urn:thing:button", "Button", "press"))
            .expect("registration succeeds");

        let page = directory.query(DirectoryQuery::fragment("status"));
        assert_eq!(page.total, 1);
        assert_eq!(page.entries[0].id, "urn:thing:lamp");

        let page = directory.query(DirectoryQuery::fragment("press"));
        assert_eq!(page.total, 1);
        assert_eq!(page.entries[0].id, "urn:thing:button");
    }

    #[test]
    fn indexes_are_maintained_across_update_and_delete() {
        let mut directory = InMemoryThingDirectory::new();
        directory
            .register(thing_with_property("urn:thing:lamp", "Lamp", "status"))
            .expect("registration succeeds");
        // Update: change property name.
        directory
            .update(thing_with_property("urn:thing:lamp", "Lamp", "level"))
            .expect("update succeeds");
        let stale = directory.query(DirectoryQuery::property("status"));
        assert_eq!(stale.total, 0);
        let fresh = directory.query(DirectoryQuery::property("level"));
        assert_eq!(fresh.total, 1);

        // Delete: index should no longer reference the removed id.
        directory
            .delete("urn:thing:lamp")
            .expect("deletion succeeds");
        let gone = directory.query(DirectoryQuery::title("Lamp"));
        assert_eq!(gone.total, 0);
    }

    #[test]
    fn get_ref_returns_borrowed_thing_without_clone() {
        let mut directory = InMemoryThingDirectory::new();
        directory
            .register(thing("urn:thing:lamp", "Lamp"))
            .expect("registration succeeds");
        let borrowed = directory.get_ref("urn:thing:lamp").expect("TD is present");
        assert_eq!(borrowed._metadata.title.as_deref(), Some("Lamp"));
        assert!(directory.get_ref("urn:thing:missing").is_none());
    }

    fn thing(id: &str, title: &str) -> Thing {
        Thing::builder(title)
            .id(id)
            .nosec()
            .build()
            .expect("valid Thing Description")
    }

    fn thing_with_property(id: &str, title: &str, property: &str) -> Thing {
        use clinkz_wot_td::{affordance::PropertyAffordance, data_schema::DataSchema};
        Thing::builder(title)
            .id(id)
            .nosec()
            .property(
                property,
                PropertyAffordance::builder(DataSchema::string())
                    .build()
                    .unwrap(),
            )
            .build()
            .expect("valid Thing Description")
    }

    fn thing_with_action(id: &str, title: &str, action: &str) -> Thing {
        use clinkz_wot_td::{
            affordance::{ActionAffordance, PropertyAffordance},
            data_schema::DataSchema,
        };
        Thing::builder(title)
            .id(id)
            .nosec()
            .property(
                action,
                PropertyAffordance::builder(DataSchema::string())
                    .build()
                    .unwrap(),
            )
            .action(action, ActionAffordance::builder().build().unwrap())
            .build()
            .expect("valid Thing Description")
    }
}
