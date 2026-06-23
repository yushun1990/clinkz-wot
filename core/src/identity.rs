//! Opaque identity and correlation tokens used across the inbound and
//! outbound interaction paths.
//!
//! These newtypes are owned (`String` / `Vec<u8>`) so they are `'static` and
//! can cross a spawnable future boundary. See the design baseline addendum
//! §1.1.

use alloc::{string::String, vec::Vec};
use core::fmt;

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
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct CorrelationId(Vec<u8>);

impl CorrelationId {
    /// Creates a correlation token from owned bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
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
        self.0
    }
}

impl AsRef<[u8]> for CorrelationId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for CorrelationId {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<u64> for CorrelationId {
    /// Encodes the integer as 8 big-endian bytes for a deterministic, portable
    /// representation across hosts of different endianness.
    fn from(value: u64) -> Self {
        Self(value.to_be_bytes().into())
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("(none)");
        }
        for byte in &self.0 {
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
}
