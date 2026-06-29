//! WoT Scripting API discovery surface.
//!
//! Provides a discovery process object with an async `next()` method and
//! fragment-based filtering aligned with the W3C WoT Scripting API.
//!
//! # Discovery methods
//!
//! [`ThingFilter::method`] selects the discovery mechanism:
//!
//! - [`DiscoveryMethod::Local`] — search the local in-memory directory. Only
//!   the `fragment` filter is applied.
//! - [`DiscoveryMethod::Everything`] — search all available sources. In the
//!   current implementation this is equivalent to `Local` because no remote
//!   transports are wired.
//! - [`DiscoveryMethod::Directory`] / [`DiscoveryMethod::Multicast`] — require
//!   a protocol-specific transport that is not yet available. `discover()`
//!   returns a [`ThingDiscovery`] whose first `next()` / `next_now()` call
//!   yields `None` with an error message explaining the deferral.

use alloc::{
    collections::{BTreeMap, VecDeque},
    string::{String, ToString},
};

use clinkz_wot_td::{data_type::ExtensionMap, thing::Thing};
use serde::Serialize;
use serde_json::Value;

use crate::{DirectoryQuery, DiscoveryResult, QueryFilter, ThingDirectory};

/// Discovery mechanism selection (WoT Scripting API `DiscoveryMethod`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiscoveryMethod {
    /// Search a local Thing Description Directory.
    ///
    /// Only the `fragment` and `query` filters are applied against the local
    /// directory's contents.
    #[default]
    Local,
    /// Search a remote Thing Description Directory at `ThingFilter::url`.
    ///
    /// Requires a protocol-specific transport (e.g., HTTP TDD client) that is
    /// not yet wired. `discover()` returns a discovery with an error.
    Directory,
    /// Search via multicast (e.g., CoAP multicast discovery).
    ///
    /// Requires a protocol-specific transport that is not yet wired.
    Multicast,
    /// Search all available sources (local + remote + multicast).
    ///
    /// In the current implementation this is equivalent to `Local`.
    Everything,
}

/// Fragment filter used by the discovery API.
pub type ThingFragment = ExtensionMap;

/// Filter for discovery.
///
/// Mirrors the W3C WoT Scripting API `ThingFilter` with `method`, `url`,
/// `query`, and `fragment` dimensions.
#[derive(Debug, Clone, Default)]
pub struct ThingFilter {
    /// Discovery mechanism to use.
    pub method: DiscoveryMethod,
    /// URL of the remote directory (used with [`DiscoveryMethod::Directory`]).
    pub url: Option<String>,
    /// Query string for structured queries (e.g., SPARQL, JSONPath).
    ///
    /// When set alongside `fragment`, both must match for a Thing to be
    /// included in the results. The local directory applies `query` as a
    /// substring match against the Thing's searchable identifier fields
    /// (`id`, `title`, `base`, and every property/action/event affordance
    /// name) — see [`discover`]. Remote directory backends may interpret
    /// `query` as SPARQL, JSONPath, or another structured query language.
    pub query: Option<String>,
    /// Fragment filter matching TD fields.
    pub fragment: Option<ThingFragment>,
}

impl ThingFilter {
    /// Creates an empty filter with the default method (`Local`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the discovery method.
    pub fn method(mut self, method: DiscoveryMethod) -> Self {
        self.method = method;
        self
    }

    /// Sets the remote directory URL.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the query string.
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Sets the fragment filter.
    pub fn fragment(mut self, fragment: impl Into<ThingFragment>) -> Self {
        self.fragment = Some(fragment.into());
        self
    }

    /// Adds a single fragment field.
    pub fn fragment_field(mut self, key: impl Into<String>, value: Value) -> Self {
        self.fragment
            .get_or_insert_with(BTreeMap::new)
            .insert(key.into(), value);
        self
    }

    /// Replaces the fragment filter.
    pub fn with_fragment(mut self, fragment: impl Into<ThingFragment>) -> Self {
        self.fragment = Some(fragment.into());
        self
    }
}

/// Discovery process object.
pub struct ThingDiscovery {
    filter: ThingFilter,
    results: VecDeque<Thing>,
    done: bool,
    error: Option<String>,
}

impl ThingDiscovery {
    /// Creates an empty discovery process with the given filter.
    pub(crate) fn new(filter: ThingFilter) -> Self {
        Self {
            filter,
            results: VecDeque::new(),
            done: false,
            error: None,
        }
    }

    /// Fills the result buffer from a completed directory query.
    pub(crate) fn set_results(&mut self, things: alloc::collections::VecDeque<Thing>) {
        self.results = things;
    }

    /// Sets an error message on the discovery process.
    #[allow(dead_code)]
    pub(crate) fn set_error(&mut self, message: impl Into<String>) {
        self.error = Some(message.into());
        self.done = true;
    }

    /// Stops the discovery process and discards buffered results.
    pub fn stop(&mut self) {
        self.done = true;
        self.results.clear();
    }

    /// Returns whether the discovery process is complete.
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Returns the last error message, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Returns the number of buffered results remaining.
    pub fn remaining(&self) -> usize {
        self.results.len()
    }

    /// Returns a reference to the filter used by this discovery.
    pub fn filter_ref(&self) -> &ThingFilter {
        &self.filter
    }

    /// Returns the next discovered Thing synchronously.
    pub fn next_now(&mut self) -> Option<Thing> {
        if self.done {
            return None;
        }

        let thing = self.results.pop_front();
        if thing.is_none() {
            self.done = true;
        }
        thing
    }

    /// Returns the next discovered Thing asynchronously.
    pub async fn next(&mut self) -> Option<Thing> {
        self.next_now()
    }
}

impl core::fmt::Debug for ThingDiscovery {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ThingDiscovery")
            .field("remaining", &self.results.len())
            .field("done", &self.done)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

/// Runs discovery against a directory backend using the given filter.
///
/// For [`DiscoveryMethod::Local`] and [`DiscoveryMethod::Everything`], this
/// searches the directory with fragment and query filtering.
///
/// For [`DiscoveryMethod::Directory`] and [`DiscoveryMethod::Multicast`], the
/// returned [`ThingDiscovery`] has an error set, because remote transports are
/// not yet wired. Callers should check [`ThingDiscovery::error`] after
/// draining results.
///
/// Fragment matching prefers direct `Thing` field comparison over
/// `serde_json::to_value` for the common TD top-level fields (`id`, `title`,
/// `base`, affordance maps, etc.), avoiding the cost of serializing each
/// candidate TD to a JSON tree. Unknown extension fields fall back to a lookup
/// in [`Thing::_extra_fields`](clinkz_wot_td::thing::Thing) (which already
/// stores them as `serde_json::Value`).
pub fn discover<D>(directory: &D, filter: ThingFilter) -> DiscoveryResult<ThingDiscovery>
where
    D: ThingDirectory,
{
    // Methods that require a protocol-specific transport return an error
    // discovery immediately.
    match filter.method {
        DiscoveryMethod::Directory => {
            let mut discovery = ThingDiscovery::new(filter);
            discovery.set_error(
                "remote directory discovery requires a protocol-specific transport; \
                 set ThingFilter::url and register a directory client binding",
            );
            return Ok(discovery);
        }
        DiscoveryMethod::Multicast => {
            let mut discovery = ThingDiscovery::new(filter);
            discovery.set_error(
                "multicast discovery requires a protocol-specific transport \
                 (e.g., CoAP multicast)",
            );
            return Ok(discovery);
        }
        DiscoveryMethod::Local | DiscoveryMethod::Everything => {}
    }

    let mut discovery = ThingDiscovery::new(filter);
    let fragment = discovery.filter_ref().fragment.as_ref();
    let query = discovery.filter_ref().query.as_deref();
    let pushed_query = build_discovery_query(discovery.filter_ref());

    let matches_filter = |thing: &Thing| {
        fragment.is_none_or(|frag| matches_fragment(thing, frag))
            && query.is_none_or(|q| matches_query(thing, q))
    };

    // Matches are cloned eagerly into the discovery's VecDeque.
    //
    // A truly lazy cursor (resolve each Thing on `next_now` instead of cloning
    // all matches up front) is blocked by two architectural constraints:
    //  1. `ThingDiscovery` is an owned, `'static` process object (per the W3C
    //     Scripting API shape), so it cannot borrow the directory.
    //  2. `Servient::discover` runs discovery inside a `with_directory` lock
    //     closure, so a borrowed `ThingDiscovery<'a>` could not escape it, and
    //     building an `Arc<dyn Fn(&str)->Option<Thing> + Send + Sync>` lookup
    //     closure would require tightening `Servient::discover`'s `D` bound to
    //     `Send + Sync`.
    // When part of the filter maps to the portable `DirectoryQuery` model
    // (`id`, exact `title`, and affordance-name fragments), we push that slice
    // down first so indexed backends can shrink the candidate set before the
    // residual local match runs. The discovery object still owns eager clones
    // of the final matches, but common local searches avoid a full-table scan.
    let results = if let Some(query) = pushed_query {
        directory
            .query(query)
            .entries
            .into_iter()
            .filter_map(|entry| matches_filter(&entry.thing).then_some(entry.thing))
            .collect::<VecDeque<_>>()
    } else {
        // The filter still runs *before* cloning, so only matching entries are
        // cloned — non-matches are never materialized.
        let mut results = VecDeque::new();
        directory.for_each_thing(|thing| {
            if matches_filter(thing) {
                results.push_back(thing.clone());
            }
        });
        results
    };
    discovery.set_results(results);
    Ok(discovery)
}

/// Extracts the portion of a local discovery filter that can be pushed into the
/// protocol-neutral directory query model.
///
/// Only exact-match fields that preserve current `discover()` semantics are
/// lowered here:
/// - fragment `id: "..."` -> [`QueryFilter::Id`]
/// - fragment `title: "..."` -> [`QueryFilter::Title`]
/// - fragment `properties` / `actions` / `events` object keys ->
///   affordance-name filters
///
/// Residual conditions (`query` substring matching, `security`, extension
/// fields, non-string top-level values, etc.) are still evaluated locally
/// against the candidate set returned by [`ThingDirectory::query`].
fn build_discovery_query(filter: &ThingFilter) -> Option<DirectoryQuery> {
    let mut query = DirectoryQuery::all();
    let mut pushed_any = false;

    let Some(fragment) = filter.fragment.as_ref() else {
        return None;
    };

    for (key, value) in fragment {
        match key.as_str() {
            "id" => {
                if let Some(id) = value.as_str() {
                    query = query.and(QueryFilter::id(id));
                    pushed_any = true;
                }
            }
            "title" => {
                if let Some(title) = value.as_str() {
                    query = query.and(QueryFilter::title(title));
                    pushed_any = true;
                }
            }
            "properties" => {
                if let Some(properties) = value.as_object() {
                    for name in properties.keys() {
                        query = query.and(QueryFilter::property(name.clone()));
                        pushed_any = true;
                    }
                }
            }
            "actions" => {
                if let Some(actions) = value.as_object() {
                    for name in actions.keys() {
                        query = query.and(QueryFilter::action(name.clone()));
                        pushed_any = true;
                    }
                }
            }
            "events" => {
                if let Some(events) = value.as_object() {
                    for name in events.keys() {
                        query = query.and(QueryFilter::event(name.clone()));
                        pushed_any = true;
                    }
                }
            }
            _ => {}
        }
    }

    pushed_any.then_some(query)
}

/// Returns `true` when `thing` matches the query string.
///
/// The query is applied as a case-sensitive substring match against the
/// Thing's searchable identifier fields: `id`, `title`, `base`, and the
/// name of every property, action, and event affordance. Evaluation
/// short-circuits on the first matching field.
///
/// This replaces an earlier implementation that serialized the whole TD to
/// JSON and ran a substring search over the serialized bytes. That was
/// O(TD size) per candidate and also matched JSON structural noise (key
/// names, punctuation, nested schema text), producing false positives. The
/// field-level match is both cheaper and more accurate. Remote directory
/// backends may interpret `query` as SPARQL, JSONPath, or another structured
/// query language; this substring match is the local fallback only.
fn matches_query(thing: &Thing, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    thing
        .id
        .as_ref()
        .is_some_and(|id| id.as_str().contains(query))
        || thing
            ._metadata
            .title
            .as_deref()
            .is_some_and(|title| title.contains(query))
        || thing
            .base
            .as_ref()
            .is_some_and(|base| base.as_str().contains(query))
        || names_contain(&thing.properties, query)
        || names_contain(&thing.actions, query)
        || names_contain(&thing.events, query)
}

/// Returns `true` when any affordance name in `map` contains `query` as a
/// substring. Used by [`matches_query`] to check property/action/event names
/// without serializing the affordance bodies.
fn names_contain<T>(map: &Option<BTreeMap<String, T>>, query: &str) -> bool {
    map.as_ref()
        .is_some_and(|m| m.keys().any(|name| name.contains(query)))
}

/// Returns `true` when `thing` matches every `(key, value)` pair in `fragment`.
///
/// Each pair is matched independently via [`match_fragment_field`]; the overall
/// match is a logical AND.
fn matches_fragment(thing: &Thing, fragment: &ThingFragment) -> bool {
    fragment
        .iter()
        .all(|(key, fragment_value)| match_fragment_field(thing, key, fragment_value))
}

/// Matches a single fragment field against a Thing without serializing the
/// whole TD to a JSON tree.
///
/// Behavior by key:
///
/// - `id`, `title`, `base` — compared as strings when the fragment value is a
///   string, or as JSON for non-string fragments.
/// - `properties`, `actions`, `events` — when the fragment value is an object,
///   every key in the fragment must name an affordance that exists on the
///   Thing. Deeper matching falls back to JSON.
/// - `security` — when the fragment value is a string, the Thing's `security`
///   list must contain it. Arrays/objects fall back to JSON.
/// - Any other key — looked up in `thing._extra_fields` (already a JSON value)
///   and matched via [`matches_json`].
fn match_fragment_field(thing: &Thing, key: &str, fragment_value: &Value) -> bool {
    match key {
        "id" => match_str_or_json(fragment_value, thing.id.as_ref().map(|id| id.as_str())),
        "title" => match_str_or_json(fragment_value, thing._metadata.title.as_deref()),
        "base" => match_str_or_json(fragment_value, thing.base.as_ref().map(|b| b.as_str())),
        "properties" => match_affordance_keys(&thing.properties, fragment_value),
        "actions" => match_affordance_keys(&thing.actions, fragment_value),
        "events" => match_affordance_keys(&thing.events, fragment_value),
        "security" => match_security_field(&thing.security, fragment_value),
        _ => thing
            ._extra_fields
            .get(key)
            .is_some_and(|v| matches_json(v, fragment_value)),
    }
}

/// Matches a fragment value against a `Option<&str>` field.
///
/// String fragments compare by equality. Non-string fragments fall back to
/// serializing the field (when present) and matching via [`matches_json`].
fn match_str_or_json(fragment_value: &Value, field: Option<&str>) -> bool {
    match (field, fragment_value) {
        (Some(s), Value::String(want)) => s == want.as_str(),
        (Some(s), other) => matches_json(&Value::String(s.to_string()), other),
        (None, Value::Null) => true,
        (None, _) => false,
    }
}

/// Matches an affordance-map fragment: every key in the fragment object must
/// name an affordance that exists on the Thing. Non-object fragments fall back
/// to JSON comparison of the affordance map's serialized form.
fn match_affordance_keys<T: Serialize>(
    affordances: &Option<BTreeMap<String, T>>,
    fragment_value: &Value,
) -> bool {
    let Some(obj) = fragment_value.as_object() else {
        // Non-object fragment: fall back to JSON comparison of the field.
        return match affordances {
            Some(map) => serde_json::to_value(map)
                .ok()
                .is_some_and(|v| matches_json(&v, fragment_value)),
            None => matches_json(&Value::Null, fragment_value),
        };
    };
    let Some(map) = affordances else {
        return obj.is_empty();
    };
    obj.keys().all(|name| map.contains_key(name))
}

/// Matches the `security` field: string fragments check membership; other
/// shapes fall back to JSON comparison.
fn match_security_field(security: &[String], fragment_value: &Value) -> bool {
    match fragment_value {
        Value::String(want) => security.iter().any(|s| s == want.as_str()),
        Value::Array(want) => want.iter().all(|v| match v.as_str() {
            Some(name) => security.iter().any(|s| s == name),
            None => false,
        }),
        other => {
            let serialized = serde_json::to_value(security).unwrap_or(Value::Null);
            matches_json(&serialized, other)
        }
    }
}

fn matches_json(json: &Value, fragment: &Value) -> bool {
    match (json, fragment) {
        (Value::Object(json_object), Value::Object(fragment_object)) => {
            fragment_object.iter().all(|(key, fragment_value)| {
                json_object
                    .get(key)
                    .is_some_and(|value| matches_json(value, fragment_value))
            })
        }
        _ => json == fragment,
    }
}
