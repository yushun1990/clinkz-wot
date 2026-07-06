//! Interaction input/output value types aligned with the WoT Scripting API
//! (baseline v4.0 §4.3).
//!
//! These are the protocol-neutral value types that flow through both the
//! inbound (exposed) and outbound (consumed) interaction paths. The handler
//! traits in [`crate::thing`] operate on [`InteractionInput`] /
//! [`InteractionOutput`].

use alloc::{collections::BTreeMap, string::String, vec::Vec};

use core::time::Duration;

use crate::{Payload, Principal};

/// Media type identifier (e.g. `application/json`, `application/cbor`).
///
/// Carried by [`AcceptHint`] and [`Payload`] so a byte-level handler can pick a
/// client-acceptable output content type (baseline §4.3 / AD48). This is a
/// lightweight newtype over [`String`] — it carries no protocol headers and is
/// `no_std + alloc`-safe.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct MediaType(String);

impl MediaType {
    /// Creates a media type from an owned string.
    pub fn new(content_type: String) -> Self {
        Self(content_type)
    }

    /// Returns the media-type string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the underlying owned media-type string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for MediaType {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for MediaType {
    fn from(value: &str) -> Self {
        Self(String::from(value))
    }
}

impl core::fmt::Display for MediaType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Protocol-neutral view of a request's `Accept` / content-type preferences
/// (baseline §4.3 / AD48).
///
/// Populated by the binding at the transport edge and read by a byte-level
/// handler to choose an output [`Payload`] content type the client will accept
/// in a single encode pass. Carries no protocol headers. If the handler ignores
/// the hint and emits a mismatched type, the edge returns
/// [`CoreError::ContentTypeMismatch`](crate::CoreError::ContentTypeMismatch)
/// (the engine does not transcode — deviation §9.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptHint {
    /// The binding's best-effort preferred media type for the reply.
    pub preferred: MediaType,
    /// Optional ordered list of additionally acceptable media types
    /// (most-preferred first after `preferred`).
    pub alternatives: Option<Vec<MediaType>>,
}

impl AcceptHint {
    /// Creates a hint carrying only a single preferred media type.
    pub fn single(preferred: impl Into<MediaType>) -> Self {
        Self {
            preferred: preferred.into(),
            alternatives: None,
        }
    }

    /// Adds an ordered list of alternative media types (builder style).
    pub fn with_alternatives(mut self, alternatives: impl IntoIterator<Item = MediaType>) -> Self {
        let collected: Vec<MediaType> = alternatives.into_iter().collect();
        self.alternatives = if collected.is_empty() {
            None
        } else {
            Some(collected)
        };
        self
    }

    /// Returns true when `content_type` matches the preferred or any
    /// alternative media type.
    pub fn accepts(&self, content_type: &str) -> bool {
        if self.preferred.as_str() == content_type {
            return true;
        }
        self.alternatives
            .as_ref()
            .is_some_and(|alts| alts.iter().any(|m| m.as_str() == content_type))
    }
}

/// Input provided to an inbound (exposed) interaction handler.
///
/// Handlers are byte-level on both sides (baseline §4.3 / AD32): `data` carries
/// already-encoded bytes plus media metadata; the runtime does not auto-encode
/// a logical value. `principal` carries the verified caller identity (or `None`
/// for NoSec / local dispatch). `accept` lets the handler pick a client-
/// acceptable output content type.
///
/// The former `security_metadata` field is removed: security material belongs
/// to the binding/transport layer, not the handler input.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InteractionInput {
    /// Optional encoded payload for write, action, subscription, or cancellation flows.
    pub data: Option<Payload>,
    /// URI-template variables (renamed from `parameters`) supplied by the caller
    /// or resolved by the binding.
    pub uri_variables: BTreeMap<String, String>,
    /// Verified caller identity for inbound interactions; `None` for outbound
    /// or local in-process calls.
    pub principal: Option<Principal>,
    /// Protocol-neutral view of the request's `Accept` preferences, populated by
    /// the binding at the edge so a byte-level handler can pick a matching
    /// output content type.
    pub accept: Option<AcceptHint>,
}

impl InteractionInput {
    /// Creates an empty interaction input.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates an interaction input containing a payload.
    pub fn with_data(data: Payload) -> Self {
        Self {
            data: Some(data),
            ..Self::default()
        }
    }
}

/// Caller-facing options for an outbound (consumed) interaction call
/// (Scripting API §7.1; baseline §4.3).
///
/// Mirrors [`InteractionInput`] for the concepts shared between inbound and
/// outbound, plus outbound-only `form_index` and `timeout`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InteractionOptions {
    /// URI-template variables to expand into the selected form's target.
    pub uri_variables: BTreeMap<String, String>,
    /// Explicit form index, bypassing binding `supports` selection. A pinned
    /// form no binding can drive yields
    /// [`CoreError::UnsupportedForm`](crate::CoreError::UnsupportedForm)
    /// (baseline §5.1 / AD47).
    pub form_index: Option<usize>,
    /// Optional encoded request payload.
    pub data: Option<Payload>,
    /// Outbound timeout. Enforced via build-time cfg (AD39/H2): `std`/embassy
    /// apply it; bare `no_std` without a timer returns
    /// [`CoreError::TimeoutUnsupported`](crate::CoreError::TimeoutUnsupported)
    /// rather than silently ignoring it (AD45).
    pub timeout: Option<Duration>,
}

impl InteractionOptions {
    /// Creates empty interaction options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience constructor that pre-seeds `data`. Equivalent to
    /// `Self::new()` followed by setting the `data` field, but reads more
    /// naturally at call sites:
    ///
    /// ```ignore
    /// handle.write_property("on", InteractionOptions::with_data(payload)).await?;
    /// ```
    pub fn with_data(data: Payload) -> Self {
        Self {
            data: Some(data),
            ..Self::default()
        }
    }

    /// Builder-style setter for a single URI-template variable. Chains:
    ///
    /// ```ignore
    /// let opts = InteractionOptions::with_uri_variable("brightness", "75")
    ///     .with_uri_variable("zone", "north");
    /// ```
    ///
    /// Consumes and returns self; for non-chaining use, set
    /// [`uri_variables`](Self::uri_variables) directly.
    pub fn with_uri_variable(mut self, key: &str, value: &str) -> Self {
        self.uri_variables.insert(key.into(), value.into());
        self
    }
}

/// Completion status of an interaction (baseline §4.3 / AD21).
///
/// Mirrors the HTTP/CoAP status families the engine can surface. `Accepted`
/// is reserved for the future async-action completion model (deferred; AD29).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum InteractionStatus {
    /// Normal completion (default; HTTP/CoAP 200-equivalent).
    #[default]
    Ok,
    /// A new resource was created (201-equivalent).
    Created,
    /// An async action was accepted, not yet complete (202-equivalent; future).
    Accepted,
}

/// Output returned by an interaction handler or consumed Thing call.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InteractionOutput {
    /// Optional encoded response payload.
    pub data: Option<Payload>,
    /// Completion status (defaults to [`InteractionStatus::Ok`]).
    pub status: InteractionStatus,
}

impl InteractionOutput {
    /// Creates an empty output with `Ok` status.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates an output containing a payload with `Ok` status.
    pub fn with_data(data: Payload) -> Self {
        Self {
            data: Some(data),
            status: InteractionStatus::Ok,
        }
    }
}
