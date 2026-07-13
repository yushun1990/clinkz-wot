//! Opaque identity and correlation tokens used across the inbound and
//! outbound interaction paths.
//!
//! Human-readable boundary identities own their storage. Runtime and arena
//! identities use bounded numeric or slot/generation representations so stale
//! handles cannot alias reused storage.

use alloc::string::String;
use core::fmt;

use clinkz_wot_foundation::{Generation, SlotIndex};

macro_rules! numeric_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            /// Creates an identity from its bounded numeric representation.
            pub const fn new(value: u32) -> Self {
                Self(value)
            }

            /// Returns the bounded numeric representation.
            pub const fn get(self) -> u32 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl From<u32> for $name {
            fn from(value: u32) -> Self {
                Self::new(value)
            }
        }

        impl From<$name> for u32 {
            fn from(value: $name) -> Self {
                value.get()
            }
        }
    };
}

macro_rules! generation_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name {
            slot: SlotIndex,
            generation: Generation,
        }

        impl $name {
            /// Creates a generation-bearing slot identity.
            pub const fn new(slot: SlotIndex, generation: Generation) -> Self {
                Self { slot, generation }
            }

            /// Returns the bounded slot component.
            pub const fn slot(self) -> SlotIndex {
                self.slot
            }

            /// Returns the nonzero generation component.
            pub const fn generation(self) -> Generation {
                self.generation
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}@{}", self.slot, self.generation)
            }
        }
    };
}

numeric_id!(
    /// Stable identity of one live binding registration.
    BindingId
);

/// Generation of one binding registration identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingGeneration(Generation);

impl BindingGeneration {
    /// The first valid binding generation.
    pub const INITIAL: Self = Self(Generation::INITIAL);

    /// Creates a binding generation from a nonzero foundation generation.
    pub const fn new(generation: Generation) -> Self {
        Self(generation)
    }

    /// Returns the nonzero foundation generation.
    pub const fn get(self) -> Generation {
        self.0
    }

    /// Advances without wrapping.
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_next() {
            Some(generation) => Some(Self(generation)),
            None => None,
        }
    }
}

impl Default for BindingGeneration {
    fn default() -> Self {
        Self::INITIAL
    }
}

impl fmt::Display for BindingGeneration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

generation_id!(
    /// Identity of one immutable logical interaction plan generation.
    PlanId
);
generation_id!(
    /// Identity of one active or terminal-retained subscription.
    SubscriptionId
);
generation_id!(
    /// Identity of one action execution exposed for query or cancellation.
    ActionInvocationRef
);
generation_id!(
    /// Generation-bearing Thing arena slot.
    ThingSlotId
);
generation_id!(
    /// Generation-bearing affordance arena slot.
    AffordanceSlotId
);
generation_id!(
    /// Generation-bearing binding arena slot.
    BindingSlotId
);
generation_id!(
    /// Generation-bearing handler arena slot.
    HandlerSlotId
);
generation_id!(
    /// Generation-bearing plan arena slot.
    PlanSlotId
);
generation_id!(
    /// Generation-bearing subscription arena slot.
    SubscriptionSlotId
);
generation_id!(
    /// Generation-bearing cleanup queue slot.
    CleanupSlotId
);
generation_id!(
    /// Caller-owned identity of a prepared server route.
    PreparedRouteId
);
generation_id!(
    /// Caller-owned identity of an activated server route.
    ActiveRouteId
);
generation_id!(
    /// Stable readiness key for a prepared route generation.
    PreparedRouteKey
);

/// Canonical Thing identity.
///
/// Replaces bare `String` ids across core, discovery, and servient. In v1 of
/// the redesign it is carried by [`crate::InboundRequest`] and used by the
/// exposed Thing registry; later phases migrate directory and registry ids to
/// this type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ThingId(String);

impl ThingId {
    /// Creates a Thing identity from an owned string.
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Returns the identity as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the underlying owned identity string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for ThingId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for ThingId {
    fn from(id: &str) -> Self {
        Self(String::from(id))
    }
}

impl AsRef<str> for ThingId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl core::borrow::Borrow<str> for ThingId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ThingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Opaque, core-owned token for matching one request and response.
///
/// A binding allocates a nonzero token within one live binding generation and
/// retains any protocol-native correlation value in its own bounded in-flight
/// table. Zero is reserved for flows that have no transport correlation.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CorrelationId(u64);

impl CorrelationId {
    /// Creates a correlation token from its binding-local numeric value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the binding-local numeric value.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Creates an empty correlation token for flows that have no transport
    /// correlation (for example in-process dispatch).
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Returns whether this token represents a flow without transport
    /// correlation.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Debug for CorrelationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn thing_id_round_trips_string_and_str() {
        let from_string = ThingId::from(String::from("urn:thing:1"));
        let from_str = ThingId::from("urn:thing:1");
        assert_eq!(from_string, from_str);
        assert_eq!(from_string.as_str(), "urn:thing:1");
        assert_eq!(from_string.into_string(), String::from("urn:thing:1"));
    }

    #[test]
    fn thing_id_orders_and_hashes_like_a_string() {
        let a = ThingId::from("a");
        let b = ThingId::from("b");
        assert!(a < b);
        let mut set = alloc::collections::BTreeSet::new();
        set.insert(a.clone());
        set.insert(b.clone());
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn correlation_id_round_trips_numeric_token() {
        let id = CorrelationId::new(0x0102_0304_0506_0708);
        assert_eq!(id.get(), 0x0102_0304_0506_0708);
    }

    #[test]
    fn correlation_id_is_copyable_and_formats_numerically() {
        let id = CorrelationId::new(42);
        let copied = id;
        assert_eq!(id, copied);
        assert_eq!(format!("{id}"), "42");
        assert_eq!(format!("{id:?}"), "42");
    }

    #[test]
    fn correlation_id_empty_is_reserved_zero() {
        let empty = CorrelationId::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.get(), 0);
        assert_eq!(format!("{empty}"), "0");
        assert!(!CorrelationId::new(1).is_empty());
    }

    #[test]
    fn generation_bearing_ids_reject_stale_equality() {
        let first = PlanId::new(SlotIndex::new(2), Generation::INITIAL);
        let second = PlanId::new(
            SlotIndex::new(2),
            Generation::INITIAL
                .checked_next()
                .expect("generation two exists"),
        );
        assert_ne!(first, second);
        assert_eq!(first.slot(), second.slot());
        assert_ne!(first.generation(), second.generation());
    }

    #[test]
    fn binding_generations_do_not_wrap() {
        assert_eq!(BindingId::new(3).get(), 3);
        assert_eq!(
            BindingGeneration::INITIAL
                .checked_next()
                .map(BindingGeneration::get)
                .map(Generation::get),
            Some(2)
        );
        let maximum = Generation::new(u32::MAX).expect("the maximum is nonzero");
        assert_eq!(BindingGeneration::new(maximum).checked_next(), None);
    }
}
