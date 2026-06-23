//! CBOR payload codec for the clinkz-wot engine.
//!
//! Implements [`PayloadCodec`] for the `application/cbor` media type. This is
//! the first concrete codec in the workspace and the reference example for
//! H6's "normalize round-trip" pattern: when an application registers a
//! [`CborCodec`], every consumed `application/cbor` payload is parsed to
//! [`ciborium::Value`] and re-encoded deterministically before being handed
//! to interaction handlers.
//!
//! # Canonicalization
//!
//! Both [`CborCodec::encode`] and [`CborCodec::decode`] canonicalize their
//! input by round-tripping through [`ciborium::Value`]. The output uses the
//! deterministic encoding rules that `ciborium` applies by default:
//!
//! - Integers serialize to the smallest lossless width (RFC 8949 §3.1 / §4.2.1).
//! - Map keys serialize in the order produced by `ciborium::Value`, which
//!   preserves the input order rather than re-sorting. Applications that need
//!   length-first map ordering (RFC 8949 §4.2.3) should pre-sort map keys
//!   before encoding.
//!
//! This is enough to give downstream code (signing, hashing, equality
//! comparison) a stable byte representation for the common cases that matter
//! in WoT interactions: integers, booleans, strings, bytes, arrays, and maps.
//!
//! # Example
//!
//! ```
//! use clinkz_wot_codec_cbor::CborCodec;
//! use clinkz_wot_core::{CodecInput, PayloadCodec};
//!
//! let codec = CborCodec::new();
//! // Build CBOR bytes for `{ "status": 1 }` using ciborium directly.
//! let mut wire = Vec::new();
//! ciborium::ser::into_writer(
//!     &ciborium::Value::Map([
//!         (
//!             ciborium::Value::Text("status".into()),
//!             ciborium::Value::Integer(1.into()),
//!         ),
//!     ].into_iter().collect::<Vec<_>>()),
//!     &mut wire,
//! ).unwrap();
//!
//! let payload = codec.encode(CodecInput { body: &wire, data_type: None }).unwrap();
//! assert_eq!(payload.content_type, "application/cbor");
//! ```

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::{borrow::Cow, format, vec::Vec};

use clinkz_wot_core::{CodecInput, CoreError, CoreResult, Payload, PayloadCodec};

/// Media type handled by [`CborCodec`].
pub const CBOR_CONTENT_TYPE: &str = "application/cbor";

/// CBOR payload codec for the [`application/cbor`](CBOR_CONTENT_TYPE) media
/// type.
///
/// Canonicalizes CBOR by parsing to [`ciborium::Value`] and re-encoding
/// deterministically. See the [crate-level docs](self) for the exact
/// canonicalization rules and the H6 round-trip implications.
///
/// `CborCodec` is zero-sized — register one instance per
/// [`Servient`](clinkz_wot_core::PayloadCodec) and the engine will use it for
/// every `application/cbor` payload that flows through consumed interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CborCodec;

impl CborCodec {
    /// Creates a new CBOR codec.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CborCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl PayloadCodec for CborCodec {
    fn content_type(&self) -> Cow<'_, str> {
        Cow::Borrowed(CBOR_CONTENT_TYPE)
    }

    fn encode(&self, input: CodecInput<'_>) -> CoreResult<Payload> {
        let body = canonicalize(input.body)?;
        Ok(Payload::new(body, CBOR_CONTENT_TYPE))
    }

    fn decode(&self, payload: &Payload) -> CoreResult<Vec<u8>> {
        // "Application bytes" for this codec are canonical CBOR, matching the
        // encode() output so the normalize round-trip is stable and so callers
        // that hash or sign decoded bytes see the same representation as the
        // wire payload.
        canonicalize(payload.body.as_slice())
    }
}

/// Parses `bytes` as CBOR and re-serializes deterministically.
///
/// Returns [`CoreError::Payload`] when the input is not well-formed CBOR or
/// when re-serialization fails (the latter is exceedingly rare for in-memory
/// `Vec<u8>` writes).
fn canonicalize(bytes: &[u8]) -> CoreResult<Vec<u8>> {
    let value: ciborium::Value = ciborium::de::from_reader(bytes)
        .map_err(|err| CoreError::Payload(format!("CBOR decode failed: {}", err)))?;
    let mut out = Vec::with_capacity(bytes.len());
    ciborium::ser::into_writer(&value, &mut out)
        .map_err(|err| CoreError::Payload(format!("CBOR encode failed: {}", err)))?;
    Ok(out)
}
