use alloc::{string::String, vec::Vec};

use clinkz_wot_td::thing::Thing;

/// A local predicate for in-memory Thing Directory filtering.
///
/// Production backends should prefer `DirectoryQuery`, which can be translated
/// to SQL, SPARQL, HTTP query parameters, or another backend-specific query
/// language. This trait is intentionally kept as an in-memory convenience.
pub trait QueryPredicate {
    /// Returns true when the TD should be included in the result set.
    fn matches(&self, thing: &Thing) -> bool;
}

impl<F> QueryPredicate for F
where
    F: Fn(&Thing) -> bool,
{
    fn matches(&self, thing: &Thing) -> bool {
        self(thing)
    }
}

/// Common protocol-neutral Thing Directory query filters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryFilter {
    /// Matches a TD by exact Thing id.
    Id(String),
    /// Matches TDs whose default title equals the provided value.
    Title(String),
    /// Matches TDs that define any affordance with the provided name.
    Fragment(String),
    /// Matches TDs that define a property affordance with the provided name.
    Property(String),
    /// Matches TDs that define an action affordance with the provided name.
    Action(String),
    /// Matches TDs that define an event affordance with the provided name.
    Event(String),
}

impl QueryFilter {
    /// Creates an exact Thing id predicate.
    pub fn id(id: impl Into<String>) -> Self {
        Self::Id(id.into())
    }

    /// Creates an exact default-title predicate.
    pub fn title(title: impl Into<String>) -> Self {
        Self::Title(title.into())
    }

    /// Creates a fragment-name predicate.
    pub fn fragment(name: impl Into<String>) -> Self {
        Self::Fragment(name.into())
    }

    /// Creates a property-name predicate.
    pub fn property(name: impl Into<String>) -> Self {
        Self::Property(name.into())
    }

    /// Creates an action-name predicate.
    pub fn action(name: impl Into<String>) -> Self {
        Self::Action(name.into())
    }

    /// Creates an event-name predicate.
    pub fn event(name: impl Into<String>) -> Self {
        Self::Event(name.into())
    }
}

impl QueryPredicate for QueryFilter {
    fn matches(&self, thing: &Thing) -> bool {
        match self {
            Self::Id(id) => thing
                .id
                .as_ref()
                .is_some_and(|thing_id| thing_id.as_str() == id),
            Self::Title(title) => thing._metadata.title.as_ref() == Some(title),
            Self::Fragment(name) => {
                thing
                    .properties
                    .as_ref()
                    .is_some_and(|properties| properties.contains_key(name))
                    || thing
                        .actions
                        .as_ref()
                        .is_some_and(|actions| actions.contains_key(name))
                    || thing
                        .events
                        .as_ref()
                        .is_some_and(|events| events.contains_key(name))
            }
            Self::Property(name) => thing
                .properties
                .as_ref()
                .is_some_and(|properties| properties.contains_key(name)),
            Self::Action(name) => thing
                .actions
                .as_ref()
                .is_some_and(|actions| actions.contains_key(name)),
            Self::Event(name) => thing
                .events
                .as_ref()
                .is_some_and(|events| events.contains_key(name)),
        }
    }
}

/// A backend-portable Thing Directory query.
///
/// Filters are combined with logical AND. `offset` and `limit` apply after
/// filtering against the backend's deterministic ordering.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirectoryQuery {
    /// Filters that every matching TD must satisfy.
    pub filters: Vec<QueryFilter>,
    /// Number of matching TDs to skip.
    pub offset: usize,
    /// Maximum number of matching TDs to return.
    pub limit: Option<usize>,
}

impl DirectoryQuery {
    /// Creates a query that matches all TDs.
    pub fn all() -> Self {
        Self::default()
    }

    /// Creates a query with one filter.
    pub fn filter(filter: QueryFilter) -> Self {
        Self::default().and(filter)
    }

    /// Creates an exact Thing id query.
    pub fn id(id: impl Into<String>) -> Self {
        Self::filter(QueryFilter::id(id))
    }

    /// Creates an exact default-title query.
    pub fn title(title: impl Into<String>) -> Self {
        Self::filter(QueryFilter::title(title))
    }

    /// Creates a fragment-name query.
    pub fn fragment(name: impl Into<String>) -> Self {
        Self::filter(QueryFilter::fragment(name))
    }

    /// Creates a property-name query.
    pub fn property(name: impl Into<String>) -> Self {
        Self::filter(QueryFilter::property(name))
    }

    /// Creates an action-name query.
    pub fn action(name: impl Into<String>) -> Self {
        Self::filter(QueryFilter::action(name))
    }

    /// Creates an event-name query.
    pub fn event(name: impl Into<String>) -> Self {
        Self::filter(QueryFilter::event(name))
    }

    /// Adds a filter combined with logical AND.
    pub fn and(mut self, filter: QueryFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Sets the number of matching TDs to skip.
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Sets the maximum number of matching TDs to return.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl QueryPredicate for DirectoryQuery {
    fn matches(&self, thing: &Thing) -> bool {
        self.filters.iter().all(|filter| filter.matches(thing))
    }
}
