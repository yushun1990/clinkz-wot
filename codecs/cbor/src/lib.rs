//! CBOR payload codec for the clinkz-wot engine.
//!
//! Implements [`PayloadCodec`] for the `application/cbor` media type. This is
//! the first concrete codec in the workspace and the reference example for
//! the "normalize round-trip" pattern: when an application registers a
//! [`CborCodec`], every consumed `application/cbor` payload is parsed to
//! [`ciborium::Value`] and re-encoded through normalization before being
//! handed to interaction handlers.
//!
//! # Normalization (not RFC 8949 deterministic encoding)
//!
//! Both [`CborCodec::encode`] and [`CborCodec::decode`] normalize their input
//! by round-tripping through [`ciborium::Value`]. This is a *normalization*,
//! **not** [RFC 8949] deterministic encoding:
//!
//! - Integers serialize to the smallest lossless width (RFC 8949 §3.1 / §4.2.1).
//! - Map keys are **not** re-sorted: `ciborium::Value` preserves input order,
//!   so two semantically equal maps with different key orders encode to
//!   different bytes. RFC 8949 §4.2.3 deterministic encoding requires
//!   length-first byte-sorted keys, which this codec does **not** perform.
//!
//! Consequence: the output is a stable byte representation only when the input
//! already has a stable key order. Applications that sign, hash, or
//! byte-compare CBOR must pre-sort map keys (or use a dedicated deterministic
//! encoder) — do **not** rely on this codec producing canonical bytes.
//!
//! [RFC 8949]: https://www.rfc-editor.org/rfc/rfc8949.html
//!
//! # Example
//!
//! ```
//! use clinkz_wot_codec_cbor::CborCodec;
//! use clinkz_wot_core::{CodecInput, PayloadCodec};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
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
//! )?;
//!
//! let payload = codec.encode(CodecInput { body: &wire })?;
//! assert_eq!(payload.content_type, "application/cbor");
//! # Ok(())
//! # }
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
/// Normalizes CBOR by parsing to [`ciborium::Value`] and re-encoding. See the
/// [crate-level docs](self) for why this is a normalization and **not** RFC
/// 8949 deterministic encoding (map keys are not re-sorted).
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
        let body = reencode(input.body)?;
        Ok(Payload::new(body, CBOR_CONTENT_TYPE))
    }

    fn decode(&self, payload: &Payload) -> CoreResult<Vec<u8>> {
        // "Application bytes" for this codec are normalized CBOR, matching the
        // encode() output so the round-trip is stable and so callers that hash
        // or sign decoded bytes see the same representation as the wire payload
        // (provided the input key order is itself stable — see crate-level
        // docs).
        reencode(payload.body.as_ref())
    }

    /// Returns the payload bytes as-is, without the normalize round-trip.
    ///
    /// Handlers that merely want to parse the value (not sign or hash it)
    /// should prefer this over [`PayloadCodec::decode`] to avoid a parse →
    /// re-serialize → re-parse triple pass.
    fn decode_raw<'a>(&self, payload: &'a Payload) -> CoreResult<Cow<'a, [u8]>> {
        // Validate well-formedness once (cheap structural parse) so callers
        // still get a clear error on malformed CBOR, but do not reserialize.
        ciborium::de::from_reader::<ciborium::Value, _>(payload.body.as_ref())
            .map_err(|err| CoreError::Payload(format!("CBOR decode failed: {}", err)))?;
        Ok(Cow::Borrowed(payload.body.as_ref()))
    }
}

/// Parses `bytes` as CBOR and re-serializes via [`ciborium::Value`].
///
/// This *normalizes* the byte representation (e.g. smallest lossless integer
/// width) but does **not** make it canonical: map keys keep their input order
/// instead of being re-sorted per RFC 8949 §4.2.3.
///
/// Returns [`CoreError::Payload`] when the input is not well-formed CBOR or
/// when re-serialization fails (the latter is exceedingly rare for in-memory
/// `Vec<u8>` writes).
fn reencode(bytes: &[u8]) -> CoreResult<Vec<u8>> {
    let value: ciborium::Value = ciborium::de::from_reader(bytes)
        .map_err(|err| CoreError::Payload(format!("CBOR decode failed: {}", err)))?;
    let mut out = Vec::with_capacity(bytes.len());
    ciborium::ser::into_writer(&value, &mut out)
        .map_err(|err| CoreError::Payload(format!("CBOR encode failed: {}", err)))?;
    Ok(out)
}
