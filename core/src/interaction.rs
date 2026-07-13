//! Interaction input/output value types aligned with the WoT Scripting API
//! (baseline v4.0 §4.3).
//!
//! These are the protocol-neutral value types that flow through both the
//! inbound (exposed) and outbound (consumed) interaction paths. The handler
//! traits in [`crate::thing`] operate on [`InteractionInput`] /
//! [`InteractionOutput`].

use alloc::{collections::BTreeMap, string::String, vec::Vec};

use core::time::Duration;

use clinkz_wot_foundation::ResourceLimits;

use crate::identity::{ActionInvocationRef, BindingGeneration, BindingId, PlanId};
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
/// [`CoreError::Payload`](crate::CoreError::Payload) with codec-phase context
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
    /// form no binding can drive yields a
    /// [`CoreError::Selection`](crate::CoreError::Selection) with
    /// `StrictSelectionMismatch` (baseline §5.1 / AD47).
    pub form_index: Option<usize>,
    /// Optional encoded request payload.
    pub data: Option<Payload>,
    /// Outbound timeout. Enforced via build-time cfg (AD39/H2): `std`/embassy
    /// apply it; bare `no_std` without a timer returns
    /// [`CoreError::UnsupportedOperation`](crate::CoreError::UnsupportedOperation)
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

/// Normalized completion status of a successful interaction.
///
/// Request failures use [`CoreError`](crate::CoreError), while concrete
/// protocol status codes remain in binding-response metadata.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum InteractionStatus {
    /// Normal successful completion.
    #[default]
    Ok,
    /// An addressable asynchronous action status resource was created.
    Created,
    /// An asynchronous action was accepted without an addressable status resource.
    Accepted,
}

/// Role of the single payload carried by a successful interaction output.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ResponsePayloadRole {
    /// Application-defined response data.
    #[default]
    Application,
    /// Action lifecycle state for an invocation, query, or cancellation response.
    OperationStatus,
}

/// Response branch selected from a compiled form response plan.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ResponseSelection {
    /// The form's primary response.
    Primary,
    /// A zero-based entry in the form's compiled additional-response plan.
    Additional(u16),
}

/// Fixed-size, protocol-neutral metadata reported by a binding response.
///
/// Construction establishes bounded shape only. Consumers must still validate
/// these values against the live request and compiled response plan.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingResponseMetadata {
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    plan_id: PlanId,
    selection: ResponseSelection,
    status_code: u16,
}

impl BindingResponseMetadata {
    /// Creates metadata for the primary response branch.
    pub const fn primary(
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        plan_id: PlanId,
        status_code: u16,
    ) -> Self {
        Self {
            binding_id,
            binding_generation,
            plan_id,
            selection: ResponseSelection::Primary,
            status_code,
        }
    }

    /// Creates metadata for a bounded additional-response branch.
    ///
    /// Returns `None` when the limit is not applicable, additional responses
    /// are disabled, or `index` reaches the configured ceiling.
    pub fn try_additional(
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        plan_id: PlanId,
        index: u16,
        status_code: u16,
        limits: &ResourceLimits,
    ) -> Option<Self> {
        let limit = limits.additional_responses_per_form_max()?;
        if u64::from(index) >= limit {
            return None;
        }

        Some(Self {
            binding_id,
            binding_generation,
            plan_id,
            selection: ResponseSelection::Additional(index),
            status_code,
        })
    }

    /// Returns the binding registration identity.
    pub const fn binding_id(&self) -> BindingId {
        self.binding_id
    }

    /// Returns the binding registration generation.
    pub const fn binding_generation(&self) -> BindingGeneration {
        self.binding_generation
    }

    /// Returns the compiled interaction plan identity.
    pub const fn plan_id(&self) -> PlanId {
        self.plan_id
    }

    /// Returns the response branch selection.
    pub const fn selection(&self) -> ResponseSelection {
        self.selection
    }

    /// Returns the opaque binding-native status code.
    pub const fn status_code(&self) -> u16 {
        self.status_code
    }
}

/// Fixed-size metadata accompanying a successful interaction output.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InteractionOutputMetadata {
    action_invocation: Option<ActionInvocationRef>,
    binding_response: Option<BindingResponseMetadata>,
    payload_role: ResponsePayloadRole,
}

impl InteractionOutputMetadata {
    /// Returns metadata carrying an action invocation reference.
    #[must_use]
    pub const fn with_action_invocation(mut self, action_invocation: ActionInvocationRef) -> Self {
        self.action_invocation = Some(action_invocation);
        self
    }

    /// Returns metadata with the role of the output's single payload.
    #[must_use]
    pub const fn with_payload_role(mut self, payload_role: ResponsePayloadRole) -> Self {
        self.payload_role = payload_role;
        self
    }

    /// Returns metadata carrying an untrusted binding response description.
    #[must_use]
    pub const fn with_untrusted_binding_response(
        mut self,
        binding_response: BindingResponseMetadata,
    ) -> Self {
        self.binding_response = Some(binding_response);
        self
    }

    /// Returns the action invocation reference, when supplied.
    pub const fn action_invocation(&self) -> Option<ActionInvocationRef> {
        self.action_invocation
    }

    /// Returns the untrusted binding response metadata, when supplied.
    pub const fn binding_response(&self) -> Option<BindingResponseMetadata> {
        self.binding_response
    }

    /// Returns the role of the output's single payload.
    pub const fn payload_role(&self) -> ResponsePayloadRole {
        self.payload_role
    }
}

/// Output returned by an interaction handler or consumed Thing call.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct InteractionOutput {
    data: Option<Payload>,
    status: InteractionStatus,
    metadata: InteractionOutputMetadata,
}

impl InteractionOutput {
    /// Creates an empty output with `Ok` status.
    pub const fn empty() -> Self {
        Self {
            data: None,
            status: InteractionStatus::Ok,
            metadata: InteractionOutputMetadata {
                action_invocation: None,
                binding_response: None,
                payload_role: ResponsePayloadRole::Application,
            },
        }
    }

    /// Creates an output containing a payload with `Ok` status.
    pub fn with_data(data: Payload) -> Self {
        Self {
            data: Some(data),
            ..Self::empty()
        }
    }

    /// Returns the output with a different successful completion status.
    #[must_use]
    pub const fn with_status(mut self, status: InteractionStatus) -> Self {
        self.status = status;
        self
    }

    /// Tries to attach fixed-size output metadata.
    ///
    /// Returns `None` when an operation-status role has no payload to describe.
    /// Binding authenticity and operation-specific combinations are validated
    /// at their owning response boundary.
    #[must_use]
    pub fn try_with_metadata(mut self, metadata: InteractionOutputMetadata) -> Option<Self> {
        if self.data.is_none()
            && matches!(
                metadata.payload_role(),
                ResponsePayloadRole::OperationStatus
            )
        {
            return None;
        }

        self.metadata = metadata;
        Some(self)
    }

    /// Returns the encoded response payload, when present.
    pub fn data(&self) -> Option<&Payload> {
        self.data.as_ref()
    }

    /// Returns the normalized successful completion status.
    pub const fn status(&self) -> InteractionStatus {
        self.status
    }

    /// Returns the fixed-size output metadata.
    pub const fn metadata(&self) -> &InteractionOutputMetadata {
        &self.metadata
    }

    /// Returns the encoded response payload without cloning it.
    pub fn into_data(self) -> Option<Payload> {
        self.data
    }

    /// Splits the output into its payload, status, and metadata without cloning.
    pub fn into_parts(
        self,
    ) -> (
        Option<Payload>,
        InteractionStatus,
        InteractionOutputMetadata,
    ) {
        (self.data, self.status, self.metadata)
    }
}
