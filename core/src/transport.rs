use alloc::{collections::BTreeMap, string::String};

use crate::{CoreResult, Payload};

/// Protocol-neutral request exchanged by a binding through a transport adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportRequest {
    /// Binding-specific target after form selection and target resolution.
    pub target: String,
    /// Binding-specific operation or method name.
    pub method: String,
    /// Header-like metadata.
    pub metadata: BTreeMap<String, String>,
    /// Optional encoded payload.
    pub payload: Option<Payload>,
}

impl TransportRequest {
    /// Creates a transport request.
    pub fn new(target: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            method: method.into(),
            metadata: BTreeMap::new(),
            payload: None,
        }
    }
}

/// Protocol-neutral response returned by a transport adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportResponse {
    /// Header-like metadata.
    pub metadata: BTreeMap<String, String>,
    /// Optional encoded payload.
    pub payload: Option<Payload>,
}

impl TransportResponse {
    /// Creates an empty response.
    pub fn empty() -> Self {
        Self {
            metadata: BTreeMap::new(),
            payload: None,
        }
    }
}

/// Transport adapter supplied by a platform or binding implementation.
pub trait TransportAdapter {
    /// Sends one request and returns one response.
    fn exchange(&mut self, request: TransportRequest) -> CoreResult<TransportResponse>;
}
