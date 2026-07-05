//! In-memory reference directory backend (baseline v4.0 §6 / phase-p1 §1.10).
//!
//! Implements [`DirectoryReader`], [`DirectoryPublisher`],
//! [`ThingDescriptionResolver`], and (std-gated) [`DirectoryWatch`]. Keeps
//! secondary indexes (title / affordance-name / property / action / event) for
//! O(log n) filtering, but serves via continuation sessions with a **sorted-id
//! live cursor** (audit O3/AD44 corrected by H3): each `next()` reads items
//! with `id > cursor` from the live `BTreeMap` under a brief shared lock and
//! advances the cursor — already-emitted ids never re-emit (Live rule 4),
//! regardless of subsequent updates.

use alloc::{
    borrow::ToOwned,
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    format,
    string::String,
    vec::Vec,
};

use clinkz_wot_core::{ThingId, WotLock};
use clinkz_wot_td::{
    AbsoluteUri,
    thing::Thing,
    validate::{Validate, ValidationLevel},
};

use crate::{
    CapabilityFilter, ContinuationToken, CountMode, CountValue, DirectoryBatch, DirectoryChange,
    DirectoryFilter, DirectoryItem::*, DirectoryPatch, DirectoryPublisher, DirectoryQuery,
    DirectoryReader, DirectoryRegistration, DirectorySession, DirectoryStats, DiscoveryError,
    DiscoveryResult, LeaseState, LeaseToken, ProjectionMode, RegistrationAck, Revision,
    SummaryFields, ThingDescriptionResolver, ThingFragment,
};
#[cfg(feature = "std")]
use crate::DirectoryWatch;

// ---------------------------------------------------------------------------
// State (primary map + secondary indexes + lease/revision counters).
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct State {
    things: BTreeMap<ThingId, Thing>,
    revisions: BTreeMap<ThingId, Revision>,
    next_revision: u64,
    title_index: BTreeMap<String, BTreeSet<ThingId>>,
    affordance_index: BTreeMap<String, BTreeSet<ThingId>>,
    property_index: BTreeMap<String, BTreeSet<ThingId>>,
    action_index: BTreeMap<String, BTreeSet<ThingId>>,
    event_index: BTreeMap<String, BTreeSet<ThingId>>,
    leases: BTreeMap<ThingId, LeaseState>,
    next_lease_token: u64,
    validation_level: ValidationLevel,
    /// Watch version counter + change log (std). A simple ring of recent
    /// changes; watchers drain from their own cursor.
    #[cfg(feature = "std")]
    change_log: std::sync::Mutex<std::collections::VecDeque<DirectoryChange>>,
}

impl State {
    fn new(validation_level: ValidationLevel) -> Self {
        Self {
            things: BTreeMap::new(),
            revisions: BTreeMap::new(),
            next_revision: 1,
            title_index: BTreeMap::new(),
            affordance_index: BTreeMap::new(),
            property_index: BTreeMap::new(),
            action_index: BTreeMap::new(),
            event_index: BTreeMap::new(),
            leases: BTreeMap::new(),
            next_lease_token: 1,
            validation_level,
            #[cfg(feature = "std")]
            change_log: std::sync::Mutex::new(std::collections::VecDeque::new()),
        }
    }

    fn index_insert(&mut self, id: &ThingId, thing: &Thing) {
        if let Some(title) = thing._metadata.title.as_deref() {
            insert_idx(&mut self.title_index, title, id);
        }
        if let Some(properties) = thing.properties.as_ref() {
            for name in properties.keys() {
                insert_idx(&mut self.affordance_index, name, id);
                insert_idx(&mut self.property_index, name, id);
            }
        }
        if let Some(actions) = thing.actions.as_ref() {
            for name in actions.keys() {
                insert_idx(&mut self.affordance_index, name, id);
                insert_idx(&mut self.action_index, name, id);
            }
        }
        if let Some(events) = thing.events.as_ref() {
            for name in events.keys() {
                insert_idx(&mut self.affordance_index, name, id);
                insert_idx(&mut self.event_index, name, id);
            }
        }
    }

    fn index_remove(&mut self, id: &ThingId, thing: &Thing) {
        if let Some(title) = thing._metadata.title.as_deref() {
            remove_idx(&mut self.title_index, title, id);
        }
        if let Some(properties) = thing.properties.as_ref() {
            for name in properties.keys() {
                remove_idx(&mut self.affordance_index, name, id);
                remove_idx(&mut self.property_index, name, id);
            }
        }
        if let Some(actions) = thing.actions.as_ref() {
            for name in actions.keys() {
                remove_idx(&mut self.affordance_index, name, id);
                remove_idx(&mut self.action_index, name, id);
            }
        }
        if let Some(events) = thing.events.as_ref() {
            for name in events.keys() {
                remove_idx(&mut self.affordance_index, name, id);
                remove_idx(&mut self.event_index, name, id);
            }
        }
    }

    /// Candidate id set from indexes, or `None` if a full scan is required.
    fn indexed_candidates(&self, filter: &DirectoryFilter) -> Option<BTreeSet<ThingId>> {
        match filter {
            DirectoryFilter::Any => None,
            DirectoryFilter::And(parts) => {
                let mut acc: Option<BTreeSet<ThingId>> = None;
                for part in parts {
                    let set = self.indexed_candidates(part)?;
                    acc = Some(match acc {
                        None => set,
                        Some(mut existing) => {
                            existing.retain(|id| set.contains(id));
                            existing
                        }
                    });
                }
                acc
            }
            DirectoryFilter::Or(parts) => {
                let mut acc: BTreeSet<ThingId> = BTreeSet::new();
                let mut any_indexed = false;
                for part in parts {
                    if let Some(set) = self.indexed_candidates(part) {
                        any_indexed = true;
                        acc.extend(set);
                    } else {
                        return None;
                    }
                }
                if any_indexed { Some(acc) } else { None }
            }
            DirectoryFilter::ByExample(f) => {
                if let Some(id) = &f.id {
                    return Some(if self.things.contains_key(id) {
                        [id.clone()].into_iter().collect()
                    } else {
                        BTreeSet::new()
                    });
                }
                None
            }
            DirectoryFilter::Text(_) | DirectoryFilter::Capability(_) => None,
        }
    }
}

fn insert_idx(index: &mut BTreeMap<String, BTreeSet<ThingId>>, key: &str, id: &ThingId) {
    index.entry(key.to_owned()).or_default().insert(id.clone());
}

fn remove_idx(index: &mut BTreeMap<String, BTreeSet<ThingId>>, key: &str, id: &ThingId) {
    let mut drop_key = false;
    if let Some(set) = index.get_mut(key) {
        set.remove(id);
        drop_key = set.is_empty();
    }
    if drop_key {
        index.remove(key);
    }
}

// ---------------------------------------------------------------------------
// InMemoryDirectory.
// ---------------------------------------------------------------------------

/// Deterministic in-memory directory. Implements all four discovery capability
/// traits. `Clone` (cheap, `WotLock` is `Arc`-backed) so a backend can be
/// shared across a reader/publisher/watcher/discoverer.
#[derive(Debug, Clone)]
pub struct InMemoryDirectory {
    state: WotLock<State>,
}

impl Default for InMemoryDirectory {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryDirectory {
    /// Creates an empty directory that validates writes at `Basic` level.
    pub fn new() -> Self {
        Self::with_validation_level(ValidationLevel::Basic)
    }

    /// Creates an empty directory with the requested TD validation level.
    pub fn with_validation_level(validation_level: ValidationLevel) -> Self {
        Self {
            state: WotLock::new(State::new(validation_level)),
        }
    }

    /// Number of TDs stored.
    pub fn len(&self) -> usize {
        self.state.with_read(|s| s.things.len())
    }

    /// Whether the directory is empty.
    pub fn is_empty(&self) -> bool {
        self.state.with_read(|s| s.things.is_empty())
    }

    fn validate_and_id(&self, thing: &Thing) -> DiscoveryResult<ThingId> {
        let level = self.state.with_read(|s| s.validation_level);
        thing.validate_with_level(level)?;
        thing
            .id
            .as_ref()
            .map(|id| ThingId::from(id.as_str()))
            .ok_or(DiscoveryError::MissingThingId)
    }
}

// ---------------------------------------------------------------------------
// Reader + filtering predicate.
// ---------------------------------------------------------------------------

/// Matches a Thing against a filter (full predicate, used when indexes can't
/// serve the filter or to refine a candidate set).
fn matches_filter(filter: &DirectoryFilter, thing: &Thing) -> bool {
    match filter {
        DirectoryFilter::Any => true,
        DirectoryFilter::ByExample(f) => matches_fragment(f, thing),
        DirectoryFilter::Text(needle) => text_contains(thing, needle),
        DirectoryFilter::Capability(c) => matches_capability(c, thing),
        DirectoryFilter::And(parts) => parts.iter().all(|p| matches_filter(p, thing)),
        DirectoryFilter::Or(parts) => parts.iter().any(|p| matches_filter(p, thing)),
    }
}

fn matches_fragment(f: &ThingFragment, thing: &Thing) -> bool {
    if let Some(title_needle) = &f.title {
        let title = thing._metadata.title.as_deref().unwrap_or("");
        if !title.contains(title_needle.as_str()) {
            return false;
        }
    }
    if let Some(id) = &f.id
        && thing.id.as_ref().map(|x| x.as_str()) != Some(id.as_str())
    {
        return false;
    }
    for ty in &f.types {
        let Some(tags) = thing._metadata.tags.as_ref() else {
            return false;
        };
        if !tags.iter().any(|t| t == ty) {
            return false;
        }
    }
    names_present(thing.properties.as_ref(), &f.properties)
        && names_present(thing.actions.as_ref(), &f.actions)
        && names_present(thing.events.as_ref(), &f.events)
}

fn names_present<V>(affordances: Option<&BTreeMap<String, V>>, required: &[String]) -> bool {
    let Some(map) = affordances else {
        return required.is_empty();
    };
    required.iter().all(|name| map.contains_key(name))
}

fn text_contains(thing: &Thing, needle: &str) -> bool {
    let n = needle.to_ascii_lowercase();
    let title = thing
        ._metadata
        .title
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    title.contains(&n)
}

fn matches_capability(c: &CapabilityFilter, thing: &Thing) -> bool {
    if let Some(name) = &c.affordance {
        let in_prop = thing
            .properties
            .as_ref()
            .is_some_and(|m| m.contains_key(name));
        let in_act = thing.actions.as_ref().is_some_and(|m| m.contains_key(name));
        let in_evt = thing.events.as_ref().is_some_and(|m| m.contains_key(name));
        if !(in_prop || in_act || in_evt) {
            return false;
        }
    }
    true
}

/// Summary fields extracted from a Thing for `Summary` projection.
fn summary_of(thing: &Thing) -> SummaryFields {
    SummaryFields {
        title: thing._metadata.title.clone(),
        types: thing._metadata.tags.clone().unwrap_or_default(),
        property_count: thing.properties.as_ref().map(|m| m.len()).unwrap_or(0),
        action_count: thing.actions.as_ref().map(|m| m.len()).unwrap_or(0),
        event_count: thing.events.as_ref().map(|m| m.len()).unwrap_or(0),
    }
}

#[async_trait::async_trait]
impl DirectoryReader for InMemoryDirectory {
    async fn get(&self, id: &ThingId) -> DiscoveryResult<Option<Thing>> {
        Ok(self.state.with_read(|s| s.things.get(id).cloned()))
    }

    async fn open_search(
        &self,
        query: DirectoryQuery,
    ) -> DiscoveryResult<Box<dyn DirectorySession>> {
        Ok(Box::new(InMemorySession {
            dir: self.clone(),
            query,
            cursor: None,
            done: false,
            error: None,
        }))
    }
}

/// Live directory session backed by a sorted-id cursor (audit H3).
struct InMemorySession {
    dir: InMemoryDirectory,
    query: DirectoryQuery,
    cursor: Option<ThingId>,
    done: bool,
    error: Option<DiscoveryError>,
}

#[async_trait::async_trait]
impl DirectorySession for InMemorySession {
    async fn next(&mut self) -> DiscoveryResult<Option<DirectoryBatch>> {
        if self.done || self.error.is_some() {
            return Ok(None);
        }
        let page_size = if self.query.page_size == 0 {
            64
        } else {
            self.query.page_size as usize
        };
        let projection = self.query.projection;
        let cursor = self.cursor.clone();

        // Read one consistent batch under a brief shared lock.
        let (items, next_cursor, has_more) = self.dir.state.with_read(|s| {
            // Candidate set from indexes, or scan.
            let candidates: BTreeSet<ThingId> = match s.indexed_candidates(&self.query.filter) {
                Some(set) => set,
                None => scan_matches(s, &self.query.filter),
            };
            let mut items: Vec<crate::DirectoryItem> = Vec::new();
            let mut emitted = 0usize;
            let mut last: Option<ThingId> = None;
            for id in candidates.iter() {
                if let Some(c) = &cursor
                    && id <= c
                {
                    continue;
                }
                let Some(thing) = s.things.get(id) else {
                    continue;
                };
                // Refine: indexed candidates for ByExample(id) are exact, but
                // And/Or subtrees or non-indexed filters need a predicate check.
                if !matches_filter(&self.query.filter, thing) {
                    continue;
                }
                items.push(project(projection, id, thing));
                last = Some(id.clone());
                emitted += 1;
                if emitted >= page_size {
                    break;
                }
            }
            let has_more = match &last {
                Some(last_id) => candidates.iter().any(|id| id > last_id),
                None => false,
            };
            (items, last, has_more)
        });

        self.cursor = next_cursor.clone();
        let continuation = next_cursor.map(|c| ContinuationToken(c.into_string().into_bytes()));

        // Count (point-in-time over the full matching set).
        let count = if self.query.count_mode == CountMode::None {
            None
        } else {
            let total =
                self.dir
                    .state
                    .with_read(|s| match s.indexed_candidates(&self.query.filter) {
                        Some(set) => set.len() as u64,
                        None => scan_matches(s, &self.query.filter).len() as u64,
                    });
            // Backend can always count exactly in-memory; upgrade Estimate.
            Some(CountValue::Exact(total))
        };

        if !has_more {
            self.done = true;
        }

        Ok(Some(DirectoryBatch {
            items,
            continuation,
            stats: DirectoryStats { has_more, count },
        }))
    }

    async fn stop(&mut self) -> DiscoveryResult<()> {
        self.done = true;
        Ok(())
    }

    fn error(&self) -> Option<&DiscoveryError> {
        self.error.as_ref()
    }
}

fn scan_matches(s: &State, filter: &DirectoryFilter) -> BTreeSet<ThingId> {
    s.things
        .iter()
        .filter(|(_, thing)| matches_filter(filter, thing))
        .map(|(id, _)| id.clone())
        .collect()
}

fn project(projection: ProjectionMode, id: &ThingId, thing: &Thing) -> crate::DirectoryItem {
    match projection {
        ProjectionMode::IdOnly => Id(id.clone()),
        ProjectionMode::Summary => Summary {
            id: id.clone(),
            summary: summary_of(thing),
        },
        ProjectionMode::FullThingDescription => Full(thing.clone()),
    }
}

// ---------------------------------------------------------------------------
// Publisher.
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl DirectoryPublisher for InMemoryDirectory {
    async fn register(&self, r: DirectoryRegistration) -> DiscoveryResult<RegistrationAck> {
        let id = self.validate_and_id(&r.td)?;
        let (revision, lease) = self.state.with(|s| {
            let existing = s.things.remove(&id);
            if let Some(old) = &existing {
                s.index_remove(&id, old);
            }
            s.things.insert(id.clone(), r.td.clone());
            s.index_insert(&id, &r.td);
            let rev = Revision(s.next_revision);
            s.next_revision += 1;
            s.revisions.insert(id.clone(), rev);
            let lease = r.ttl.as_ref().map(|_| {
                let tok = format!("lease-{}", s.next_lease_token);
                s.next_lease_token += 1;
                LeaseState {
                    token: LeaseToken(tok.into_bytes()),
                    expires_at: r.ttl,
                }
            });
            if let Some(l) = &lease {
                s.leases.insert(id.clone(), l.clone());
            }
            #[cfg(feature = "std")]
            if existing.is_none() {
                push_change(s, DirectoryChange::Added(r.td.clone()));
            } else {
                push_change(s, DirectoryChange::Updated(r.td.clone()));
            }
            (rev, lease)
        });
        Ok(RegistrationAck {
            id,
            revision,
            lease,
        })
    }

    async fn renew(&self, lease: LeaseToken) -> DiscoveryResult<LeaseState> {
        let needle = alloc::string::String::from_utf8(lease.0.clone())
            .map_err(|_| DiscoveryError::LeaseExpired)?;
        self.state
            .with_read(|s| {
                s.leases
                    .iter()
                    .find(|(_, state)| state.token.0 == needle.as_bytes())
                    .map(|(_, state)| state.clone())
            })
            .ok_or(DiscoveryError::LeaseExpired)
    }

    async fn update(&self, id: &ThingId, patch: DirectoryPatch) -> DiscoveryResult<Revision> {
        // Apply a JSON Merge Patch (RFC 7386) to the stored TD's JSON form,
        // then re-deserialize. The patch representation is declared by
        // `content_type`; v1 in-memory backend handles `application/json`.
        if patch.content_type.as_str() != "application/json" {
            return Err(DiscoveryError::UnsupportedProjection);
        }
        let updated: Thing = self
            .state
            .with_read(|s| -> DiscoveryResult<Option<Thing>> {
                let Some(existing) = s.things.get(id) else {
                    return Ok(None);
                };
                let mut target = serde_json::to_value(existing).map_err(|e| {
                    DiscoveryError::ResolverFailed(format!("serialize TD failed: {e}"))
                })?;
                let patch_val: serde_json::Value =
                    serde_json::from_slice(&patch.body).map_err(|e| {
                        DiscoveryError::ResolverFailed(format!("parse patch failed: {e}"))
                    })?;
                json_merge(&mut target, patch_val);
                let thing: Thing = serde_json::from_value(target).map_err(|e| {
                    DiscoveryError::ResolverFailed(format!("patched TD invalid: {e}"))
                })?;
                Ok(Some(thing))
            })?
            .ok_or_else(|| DiscoveryError::UnknownThing(id.clone()))?;
        // Re-validate + store + bump revision.
        self.validate_and_id(&updated)?;
        let rev = self.state.with(|s| {
            if let Some(old) = s.things.get(id).cloned() {
                s.index_remove(id, &old);
            }
            s.things.insert(id.clone(), updated.clone());
            s.index_insert(id, &updated);
            let rev = Revision(s.next_revision);
            s.next_revision += 1;
            s.revisions.insert(id.clone(), rev);
            #[cfg(feature = "std")]
            push_change(s, DirectoryChange::Updated(updated));
            rev
        });
        Ok(rev)
    }

    async fn unregister(&self, id: &ThingId) -> DiscoveryResult<()> {
        let removed = self.state.with(|s| {
            let old = s.things.remove(id);
            if let Some(old) = &old {
                s.index_remove(id, old);
                s.revisions.remove(id);
                s.leases.remove(id);
            }
            old
        });
        if let Some(thing) = removed {
            #[cfg(feature = "std")]
            self.state
                .with(|s| push_change(s, DirectoryChange::Removed(id.clone())));
            // `thing` observed; suppress unused warning without dropping it.
            let _ = &thing;
        }
        Ok(())
    }
}

/// RFC 7386 JSON Merge Patch: recursively merge `patch` into `target`.
fn json_merge(target: &mut serde_json::Value, patch: serde_json::Value) {
    match (target, patch) {
        (serde_json::Value::Object(target_map), serde_json::Value::Object(patch_map)) => {
            for (key, value) in patch_map {
                match value {
                    serde_json::Value::Null => {
                        target_map.remove(&key);
                    }
                    _ => {
                        json_merge(
                            target_map.entry(key).or_insert(serde_json::Value::Null),
                            value,
                        );
                    }
                }
            }
        }
        (target_slot, patch) => *target_slot = patch,
    }
}

#[cfg(feature = "std")]
fn push_change(s: &State, change: DirectoryChange) {
    if let Ok(mut log) = s.change_log.lock() {
        log.push_back(change);
        // Bounded ring; keep the most recent.
        while log.len() > 256 {
            log.pop_front();
        }
    }
}

// ---------------------------------------------------------------------------
// ThingDescriptionResolver (v1 local: URL → ThingId lookup).
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl ThingDescriptionResolver for InMemoryDirectory {
    async fn request_thing_description(&self, url: &AbsoluteUri) -> DiscoveryResult<Thing> {
        // v1 local resolver: interpret the URL string as the Thing id (URN ids
        // are valid absolute URIs). Real HTTP/CoAP fetch is a future backend.
        let id = ThingId::from(url.as_str());
        self.state
            .with_read(|s| s.things.get(&id).cloned())
            .ok_or_else(|| DiscoveryError::UnknownThing(id))
    }
}

// ---------------------------------------------------------------------------
// DirectoryWatch (std-gated). Drains the change log from a cursor.
// ---------------------------------------------------------------------------

#[cfg(feature = "std")]
pub struct InMemoryWatch {
    dir: InMemoryDirectory,
    seen: usize,
    done: bool,
}

#[cfg(feature = "std")]
impl InMemoryDirectory {
    /// Opens a watch stream of directory changes (Added/Updated/Removed),
    /// independent of any open search session.
    pub fn watch(&self) -> InMemoryWatch {
        InMemoryWatch {
            dir: self.clone(),
            seen: 0,
            done: false,
        }
    }
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl DirectoryWatch for InMemoryWatch {
    async fn next(&mut self) -> DiscoveryResult<Option<DirectoryChange>> {
        if self.done {
            return Ok(None);
        }
        let change = self.dir.state.with_read(|s| {
            let log = s.change_log.lock().ok()?;
            log.get(self.seen).cloned()
        });
        if let Some(change) = change {
            self.seen += 1;
            Ok(Some(change))
        } else {
            // No new change right now. v1 watch is non-blocking: returns None
            // when caught up. A future remote watch blocks until a change.
            Ok(None)
        }
    }

    async fn stop(&mut self) -> DiscoveryResult<()> {
        self.done = true;
        Ok(())
    }
}
