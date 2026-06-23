//! Integration tests for the CBOR payload codec.
//!
//! Covers canonicalization behavior, round-trip stability, error mapping, and
//! the H6 normalize round-trip pattern that the Servient interaction layer
//! applies when this codec is registered.

use clinkz_wot_codec_cbor::{CBOR_CONTENT_TYPE, CborCodec};
use clinkz_wot_core::{CodecInput, Payload, PayloadCodec};

use ciborium::Value;

/// Encodes a `ciborium::Value` to CBOR bytes using the canonical serde path.
fn cbor_bytes(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes)
        .expect("serializing value to CBOR should succeed");
    bytes
}

/// Builds the CBOR map `{ "status": 1, "level": 200 }`.
fn status_map() -> Value {
    Value::Map(
        [
            (Value::Text("status".into()), Value::Integer(1.into())),
            (Value::Text("level".into()), Value::Integer(200.into())),
        ]
        .into_iter()
        .collect::<Vec<_>>(),
    )
}

#[test]
fn content_type_is_application_cbor() {
    let codec = CborCodec::new();
    assert_eq!(codec.content_type().as_ref(), CBOR_CONTENT_TYPE);
    assert_eq!(codec.content_type().as_ref(), "application/cbor");
}

#[test]
fn encode_advertises_application_cbor_content_type() {
    let codec = CborCodec::new();
    let wire = cbor_bytes(&status_map());
    let payload = codec
        .encode(CodecInput {
            body: &wire,
            data_type: None,
        })
        .expect("encode should succeed");
    assert_eq!(payload.content_type, "application/cbor");
}

#[test]
fn round_trip_through_encode_and_decode_is_stable() {
    let codec = CborCodec::new();
    let canonical = cbor_bytes(&status_map());

    // encode produces canonical bytes; decode of those bytes must be byte-identical
    // (canonical form is idempotent under this codec).
    let payload = codec
        .encode(CodecInput {
            body: &canonical,
            data_type: None,
        })
        .unwrap();
    let decoded = codec.decode(&payload).unwrap();
    assert_eq!(decoded, canonical, "decode must reproduce canonical bytes");

    // Re-encoding the decoded bytes must be idempotent.
    let re_encoded = codec
        .encode(CodecInput {
            body: &decoded,
            data_type: None,
        })
        .unwrap();
    assert_eq!(re_encoded.body, canonical);
}

#[test]
fn canonicalize_compresses_integer_widths() {
    // u16 encoding of `1` is 3 bytes: 0x18 maps to one-byte follow, but a u8
    // positive integer 0..23 fits in a single byte (0x01). ciborium always
    // emits the smallest width, so encoding a map carrying 1u64 must not
    // contain the trailing-u8 form (0x18 0x01).
    let wide = cbor_bytes(&Value::Map(
        [(Value::Text("n".into()), Value::Integer(1u64.into()))]
            .into_iter()
            .collect::<Vec<_>>(),
    ));
    assert!(wide.contains(&0x01));
    assert!(!wide.windows(2).any(|w| w == [0x18, 0x01]));

    let codec = CborCodec::new();
    let canonical = codec
        .encode(CodecInput {
            body: &wide,
            data_type: None,
        })
        .unwrap();
    assert_eq!(canonical.body, wide, "already-canonical input is unchanged");
}

#[test]
fn canonicalize_accepts_non_canonical_input() {
    // Build a non-minimal CBOR integer: 0x18 0x01 is the explicit one-byte
    // form of the integer 1, but ciborium must still accept it on decode and
    // re-emit it in minimal form on encode.
    let non_minimal = [0x18, 0x01]; // unsigned int 1, one-byte follow
    let codec = CborCodec::new();
    let canonical = codec
        .encode(CodecInput {
            body: &non_minimal,
            data_type: None,
        })
        .unwrap();
    assert_eq!(
        canonical.body,
        [0x01],
        "integer 1 must canonicalize to one byte"
    );
}

#[test]
fn encode_rejects_malformed_cbor() {
    // 0x61 is the start of a one-byte text string, but no byte follows.
    let truncated = [0x61];
    let codec = CborCodec::new();
    let err = codec
        .encode(CodecInput {
            body: &truncated,
            data_type: None,
        })
        .unwrap_err();
    let message = format!("{}", err);
    assert!(
        message.contains("CBOR decode failed"),
        "expected CBOR decode error in: {}",
        message
    );
}

#[test]
fn decode_rejects_malformed_cbor() {
    // 0xc0 is a tag header (RFC 3339 date string) that requires a contained
    // item; using it alone is a truncated CBOR stream.
    let payload = Payload::new(vec![0xc0], CBOR_CONTENT_TYPE);
    let codec = CborCodec::new();
    let err = codec.decode(&payload).unwrap_err();
    let message = format!("{}", err);
    assert!(
        message.contains("CBOR decode failed"),
        "expected CBOR decode error in: {}",
        message
    );
}

#[test]
fn round_trip_preserves_nested_structure() {
    let nested = Value::Array(vec![
        Value::Map(
            [(Value::Text("k".into()), Value::Bool(true))]
                .into_iter()
                .collect::<Vec<_>>(),
        ),
        Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
        Value::Null,
    ]);
    let wire = cbor_bytes(&nested);

    let codec = CborCodec::new();
    let payload = codec
        .encode(CodecInput {
            body: &wire,
            data_type: None,
        })
        .unwrap();
    let decoded = codec.decode(&payload).unwrap();

    assert_eq!(decoded, wire, "nested structures must round-trip unchanged");
}

#[test]
fn h6_normalize_round_trip_pattern_is_idempotent_after_first_canonicalization() {
    // This is the H6 reference scenario: when the Servient registers a
    // CborCodec, consumed `application/cbor` payloads flow through
    // `normalize_payload`, which does `decode` then `encode`. For input that
    // is already canonical the second pass must be a no-op at the byte level.
    let codec = CborCodec::new();
    let canonical = cbor_bytes(&status_map());

    let normalized = {
        let payload = Payload::new(canonical.clone(), CBOR_CONTENT_TYPE);
        let decoded = codec.decode(&payload).unwrap();
        codec
            .encode(CodecInput {
                body: &decoded,
                data_type: None,
            })
            .unwrap()
    };
    assert_eq!(normalized.body, canonical);
}
