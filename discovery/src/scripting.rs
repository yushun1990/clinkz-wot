//! WoT Scripting API discovery surface.
//!
//! Provides a discovery process object with an async `next()` method and
//! fragment-based filtering aligned with the W3C WoT Scripting API.

use alloc::{
    collections::{BTreeMap, VecDeque},
    string::{String, ToString},
    vec::Vec,
};

use clinkz_wot_td::{data_type::ExtensionMap, thing::Thing};
use serde::Serialize;
use serde_json::Value;

use crate::{DirectoryQuery, DiscoveryResult, ThingDirectory};

/// Fragment filter used by the discovery API.
pub type ThingFragment = ExtensionMap;

/// Filter for discovery.
#[derive(Debug, Clone, Default)]
pub struct ThingFilter {
    /// Optional fragment filter matching TD fields.
    pub fragment: Option<ThingFragment>,
}

impl ThingFilter {
    /// Creates an empty filter.
    pub fn new() -> Self {
        Self::default()
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
    pub(crate) fn set_results(&mut self, things: Vec<Thing>) {
        self.results = things.into_iter().collect();
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
    let mut discovery = ThingDiscovery::new(filter);
    let page = directory.query(DirectoryQuery::all());
    let fragment = discovery.filter_ref().fragment.as_ref();

    let mut things = Vec::with_capacity(page.entries.len());
    for entry in page.entries {
        if let Some(fragment) = fragment {
            if matches_fragment(&entry.thing, fragment) {
                things.push(entry.thing);
            }
        } else {
            things.push(entry.thing);
        }
    }

    discovery.set_results(things);
    Ok(discovery)
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
