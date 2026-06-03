use alloc::{borrow::Cow, string::String, vec::Vec};

/// An encoded interaction payload plus protocol-neutral media metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    /// Encoded payload bytes.
    pub body: Vec<u8>,
    /// Media type for the encoded payload.
    pub content_type: String,
    /// Optional content coding such as gzip.
    pub content_coding: Option<String>,
}

impl Payload {
    /// Creates a payload with explicit media metadata.
    pub fn new(body: impl Into<Vec<u8>>, content_type: impl Into<String>) -> Self {
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
    /// Optional application data type name.
    pub data_type: Option<&'a str>,
}

/// Protocol-neutral payload codec contract.
pub trait PayloadCodec {
    /// Returns the media type handled by this codec.
    fn content_type(&self) -> Cow<'_, str>;

    /// Encodes application bytes into a payload.
    fn encode(&self, input: CodecInput<'_>) -> crate::CoreResult<Payload>;

    /// Decodes a payload into application bytes.
    fn decode(&self, payload: &Payload) -> crate::CoreResult<Vec<u8>>;
}
