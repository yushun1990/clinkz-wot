//! Opaque identity and correlation tokens used across the inbound and
//! outbound interaction paths.
//!
//! Human-readable boundary identities own their storage. Runtime and arena
//! identities use bounded numeric or slot/generation representations so stale
//! handles cannot alias reused storage.

use alloc::{string::String, sync::Arc, vec::Vec};
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

/// Opaque, core-owned correlation token echoed between an inbound request and
/// its response.
///
/// A binding fills it from its transport (for example a zenoh query id) and
/// echoes it unchanged in the matching [`crate::InboundResponse`]. It is owned
/// by core; bindings only supply the byte content. See baseline addendum §1.1.
///
/// The token is backed by `Arc<[u8]>` rather than `Vec<u8>`: every inbound
/// request clones the correlation id at least once (the Servient driving loop
/// clones it out of the request to echo back in the response, and the zenoh
/// server inserts another clone into its `reply_targets` map). `Arc` makes
/// every clone a refcount bump instead of a heap allocation, while derived
/// `Hash`/`Ord`/`Eq` keep the same byte-wise semantics as `Vec<u8>` so map keys
/// behave identically.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct CorrelationId(Arc<[u8]>);

impl CorrelationId {
    /// Creates a correlation token from owned bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(Arc::from(bytes))
    }

    /// Creates an empty correlation token for flows that have no transport
    /// correlation (for example in-process dispatch).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Returns the token bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Returns the underlying owned token bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl AsRef<[u8]> for CorrelationId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for CorrelationId {
    fn from(bytes: Vec<u8>) -> Self {
        Self(Arc::from(bytes))
    }
}

impl From<&[u8]> for CorrelationId {
    fn from(bytes: &[u8]) -> Self {
        Self(Arc::from(bytes))
    }
}

impl From<u64> for CorrelationId {
    /// Encodes the integer as 8 big-endian bytes for a deterministic, portable
    /// representation across hosts of different endianness.
    fn from(value: u64) -> Self {
        Self(Arc::from(value.to_be_bytes()))
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("(none)");
        }
        for byte in self.0.iter() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
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
    fn correlation_id_round_trips_bytes() {
        let id = CorrelationId::new(alloc::vec![0x01, 0x02, 0x03]);
        assert_eq!(id.as_bytes(), &[0x01, 0x02, 0x03]);
        assert_eq!(id.into_bytes(), alloc::vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn correlation_id_from_u64_is_big_endian_and_stable() {
        let id = CorrelationId::from(0x0102_0304_0506_0708u64);
        assert_eq!(
            id.as_bytes(),
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
    }

    #[test]
    fn correlation_id_empty_displays_as_none() {
        assert_eq!(format!("{}", CorrelationId::empty()), "(none)");
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
