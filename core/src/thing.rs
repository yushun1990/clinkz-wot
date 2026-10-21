//! Source-free affordance addressing.
use alloc::sync::Arc;
/// Location of an affordance within a Thing Description.
///
/// Affordance names are stored as [`Arc<str>`] rather than [`String`] so that
/// cloning an `AffordanceTarget` is a single atomic refcount bump. This matters
/// on the inbound and consumed-interaction hot paths.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AffordanceTarget {
    /// A form declared at Thing level.
    Thing,
    /// A property affordance by name.
    Property(Arc<str>),
    /// An action affordance by name.
    Action(Arc<str>),
    /// An event affordance by name.
    Event(Arc<str>),
}

impl AffordanceTarget {
    /// Returns the affordance name when this target points at a property,
    /// action, or event, or `None` for the Thing-level target.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Thing => None,
            Self::Property(name) | Self::Action(name) | Self::Event(name) => Some(&**name),
        }
    }

    /// Returns the [`AffordanceKind`] discriminant of this target, or `None`
    /// for the Thing-level target.
    pub fn kind(&self) -> Option<AffordanceKind> {
        match self {
            Self::Thing => None,
            Self::Property(_) => Some(AffordanceKind::Property),
            Self::Action(_) => Some(AffordanceKind::Action),
            Self::Event(_) => Some(AffordanceKind::Event),
        }
    }
}

/// Closed discriminant for the three interaction affordance kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AffordanceKind {
    Property,
    Action,
    Event,
}

impl AffordanceKind {
    /// Returns the canonical lowercase kind name (`"property"`, `"action"`,
    /// `"event"`), matching the W3C TD interaction affordance collection names.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Property => "property",
            Self::Action => "action",
            Self::Event => "event",
        }
    }
}

impl core::fmt::Display for AffordanceKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}
