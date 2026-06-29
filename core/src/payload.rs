use alloc::{borrow::Cow, string::String, sync::Arc, vec::Vec};

/// An encoded interaction payload plus protocol-neutral media metadata.
///
/// `body` is stored as `Arc<[u8]>` rather than `Vec<u8>` so cloning a `Payload`
/// is a single refcount bump. This matters for the event fan-out path, where
/// one emitted payload is queued into N subscriber buffers. Access the bytes
/// via `body.as_ref()` / `&body[..]` (the `as_slice` method is unstable on
/// `Arc<[u8]>`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Payload {
    /// Encoded payload bytes (shared by reference).
    pub body: Arc<[u8]>,
    /// Media type for the encoded payload.
    pub content_type: String,
    /// Optional content coding such as gzip.
    pub content_coding: Option<String>,
}

impl Payload {
    /// Creates a payload with explicit media metadata.
    pub fn new(body: impl Into<Arc<[u8]>>, content_type: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            content_type: content_type.into(),
            content_coding: None,
        }
    }

    /// Adds content coding metadata.
    pub fn with_content_coding(mut self, content_coding: impl Into<String>) -> Self {
        self.content_coding = Some(content_coding.into());
        self
    }
}

/// Unencoded application data passed to a payload codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodecInput<'a> {
    /// Application-level bytes to encode.
    pub body: &'a [u8],
}

/// Protocol-neutral payload codec contract.
pub trait PayloadCodec {
    /// Returns the media type handled by this codec.
    fn content_type(&self) -> Cow<'_, str>;

    /// Encodes application bytes into a payload.
    fn encode(&self, input: CodecInput<'_>) -> crate::CoreResult<Payload>;

    /// Decodes a payload into application bytes.
    fn decode(&self, payload: &Payload) -> crate::CoreResult<Vec<u8>>;

    /// Returns the application bytes without normalizing, when the codec
    /// supports it.
    ///
    /// The default implementation falls back to [`decode`](Self::decode)
    /// (normalized, owned). Codecs that can hand back the original wire bytes
    /// without re-encoding override this to return [`Cow::Borrowed`], letting
    /// handlers parse the value directly and avoid a normalize-then-reparse
    /// round-trip on the hot path. Use [`decode`](Self::decode) instead when
    /// the bytes must be byte-stable for signing or hashing.
    fn decode_raw<'a>(&self, payload: &'a Payload) -> crate::CoreResult<Cow<'a, [u8]>> {
        self.decode(payload).map(Cow::Owned)
    }
}
