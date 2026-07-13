use core::fmt;
use core::time::Duration;

use clinkz_wot_foundation::ResourceKind;
use clinkz_wot_td::data_type::Operation;

use crate::identity::{
    AffordanceSlotId, BindingGeneration, BindingId, CorrelationId, PlanId, ThingSlotId,
};
const REDACTED_CAUSE_CAPACITY: usize = 96;

/// Caller-visible retry classification for a structured core error.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum RetryClass {
    /// Retrying cannot succeed without changing input or state.
    Never,
    /// The failed operation is known to be safe to retry.
    Safe,
    /// The engine cannot prove whether retrying is safe.
    CallerDecision,
}

/// Bounded processing phase associated with an error.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ErrorPhase {
    /// No more precise phase is available.
    Unknown,
    /// Document parsing or decoding.
    Parse,
    /// Structural or schema validation.
    Validate,
    /// Admission reservation or private-state construction.
    Admission,
    /// Candidate or security selection.
    Selection,
    /// Binding route preparation.
    Prepare,
    /// Prepared-route readiness.
    Readiness,
    /// Binding route activation.
    Activate,
    /// Publication or binding commit.
    Commit,
    /// Application handler execution.
    Handler,
    /// Payload encoding or decoding.
    Codec,
    /// Protocol binding execution.
    Binding,
    /// Response or subscription-item delivery.
    Delivery,
    /// Cancellation, teardown, or retained cleanup.
    Cleanup,
}

/// Reason that candidate selection failed before binding execution began.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum SelectionFailureReason {
    /// The addressed affordance does not exist.
    AffordanceMissing,
    /// The affordance does not support the requested operation.
    OperationUnsupported,
    /// Forms exist, but none declares the requested operation.
    NoFormSupportsOperation,
    /// Resolving the target or URI template failed.
    TargetResolutionFailed,
    /// No registered binding supports the resolved form.
    NoSupportingBinding,
    /// More than one binding claims exclusive ownership of the form.
    AmbiguousBindingOwner,
    /// Required credentials or a security provider are unavailable.
    SecurityUnavailable,
    /// A caller-pinned form, binding, or security branch cannot be selected.
    StrictSelectionMismatch,
}

/// Redacted reason for a committed security failure.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum SecurityFailureReason {
    /// Required credentials were not supplied.
    MissingCredentials,
    /// Supplied credentials failed authentication.
    InvalidCredentials,
    /// The authenticated principal lacks required authorization.
    AuthorizationDenied,
    /// The selected security scheme cannot be executed.
    UnsupportedScheme,
    /// A selected provider failed without exposing its raw cause.
    ProviderFailure,
}

/// Fixed-capacity diagnostic context shared by structured core errors.
///
/// The cause buffer accepts only an already-redacted message. It truncates at a
/// UTF-8 boundary and never retains provider errors, payloads, or credentials.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ErrorContext {
    thing: Option<ThingSlotId>,
    target: Option<AffordanceSlotId>,
    operation: Option<Operation>,
    form_index: Option<u32>,
    plan: Option<PlanId>,
    binding: Option<(BindingId, BindingGeneration)>,
    correlation: Option<CorrelationId>,
    phase: ErrorPhase,
    retry_class: RetryClass,
    retry_after: Option<Duration>,
    cause_code: Option<u16>,
    cause_bytes: [u8; REDACTED_CAUSE_CAPACITY],
    cause_len: u8,
    cause_truncated: bool,
}

impl ErrorContext {
    /// Creates an empty context with explicit phase and retry advice.
    pub const fn new(phase: ErrorPhase, retry_class: RetryClass) -> Self {
        Self {
            thing: None,
            target: None,
            operation: None,
            form_index: None,
            plan: None,
            binding: None,
            correlation: None,
            phase,
            retry_class,
            retry_after: None,
            cause_code: None,
            cause_bytes: [0; REDACTED_CAUSE_CAPACITY],
            cause_len: 0,
            cause_truncated: false,
        }
    }

    /// Adds a generation-bearing Thing identity.
    #[must_use]
    pub const fn with_thing(mut self, thing: ThingSlotId) -> Self {
        self.thing = Some(thing);
        self
    }

    /// Adds a generation-bearing affordance identity.
    #[must_use]
    pub const fn with_target(mut self, target: AffordanceSlotId) -> Self {
        self.target = Some(target);
        self
    }

    /// Adds the applicable operation.
    #[must_use]
    pub const fn with_operation(mut self, operation: Operation) -> Self {
        self.operation = Some(operation);
        self
    }

    /// Adds the original form index.
    #[must_use]
    pub const fn with_form_index(mut self, form_index: u32) -> Self {
        self.form_index = Some(form_index);
        self
    }

    /// Adds the selected immutable plan identity.
    #[must_use]
    pub const fn with_plan(mut self, plan: PlanId) -> Self {
        self.plan = Some(plan);
        self
    }

    /// Adds the selected binding identity and generation.
    #[must_use]
    pub const fn with_binding(mut self, binding: BindingId, generation: BindingGeneration) -> Self {
        self.binding = Some((binding, generation));
        self
    }

    /// Adds the core correlation token.
    #[must_use]
    pub const fn with_correlation(mut self, correlation: CorrelationId) -> Self {
        self.correlation = Some(correlation);
        self
    }

    /// Adds a retry-after hint without changing the retry class.
    #[must_use]
    pub const fn with_retry_after(mut self, retry_after: Duration) -> Self {
        self.retry_after = Some(retry_after);
        self
    }

    /// Replaces the bounded cause with an already-redacted code and message.
    #[must_use]
    pub fn with_redacted_cause(mut self, code: u16, message: &str) -> Self {
        let mut end = message.len().min(REDACTED_CAUSE_CAPACITY);
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        self.cause_bytes.fill(0);
        self.cause_bytes[..end].copy_from_slice(&message.as_bytes()[..end]);
        self.cause_len = end as u8;
        self.cause_code = Some(code);
        self.cause_truncated = end < message.len();
        self
    }

    /// Returns the Thing identity when known.
    pub const fn thing(&self) -> Option<ThingSlotId> {
        self.thing
    }

    /// Returns the affordance identity when known.
    pub const fn target(&self) -> Option<AffordanceSlotId> {
        self.target
    }

    /// Returns the operation when known.
    pub const fn operation(&self) -> Option<Operation> {
        self.operation
    }

    /// Returns the original form index when known.
    pub const fn form_index(&self) -> Option<u32> {
        self.form_index
    }

    /// Returns the selected plan identity when known.
    pub const fn plan(&self) -> Option<PlanId> {
        self.plan
    }

    /// Returns the binding identity and generation when known.
    pub const fn binding(&self) -> Option<(BindingId, BindingGeneration)> {
        self.binding
    }

    /// Returns the correlation token when known.
    pub const fn correlation(&self) -> Option<CorrelationId> {
        self.correlation
    }

    /// Returns the processing phase.
    pub const fn phase(&self) -> ErrorPhase {
        self.phase
    }

    /// Returns the retry classification.
    pub const fn retry_class(&self) -> RetryClass {
        self.retry_class
    }

    /// Returns the optional retry-after hint.
    pub const fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }

    /// Returns the redacted cause code when supplied.
    pub const fn cause_code(&self) -> Option<u16> {
        self.cause_code
    }

    /// Returns the already-redacted cause text.
    pub fn redacted_cause(&self) -> Option<&str> {
        self.cause_code?;
        core::str::from_utf8(&self.cause_bytes[..usize::from(self.cause_len)]).ok()
    }

    /// Returns whether the redacted cause was truncated.
    pub const fn cause_was_truncated(&self) -> bool {
        self.cause_truncated
    }
}

impl fmt::Debug for ErrorContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ErrorContext")
            .field("thing", &self.thing)
            .field("target", &self.target)
            .field("plan", &self.plan)
            .field("binding", &self.binding)
            .field("correlation", &self.correlation)
            .field("phase", &self.phase)
            .field("cause_code", &self.cause_code)
            .field("redacted_cause", &self.redacted_cause())
            .finish()
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "phase={:?}", self.phase)?;
        if let Some(thing) = self.thing {
            write!(formatter, " thing={thing}")?;
        }
        if let Some(target) = self.target {
            write!(formatter, " target={target}")?;
        }
        if let Some(plan) = self.plan {
            write!(formatter, " plan={plan}")?;
        }
        if let Some((binding, generation)) = self.binding {
            write!(formatter, " binding={binding}@{generation}")?;
        }
        if let Some(correlation) = self.correlation {
            write!(formatter, " correlation={correlation}")?;
        }
        if let Some(code) = self.cause_code {
            write!(formatter, " cause_code={code}")?;
        }
        if let Some(cause) = self.redacted_cause() {
            write!(formatter, " cause={cause:?}")?;
        }
        Ok(())
    }
}

/// Result type used by protocol-neutral core traits.
pub type CoreResult<T> = Result<T, CoreError>;

/// Protocol-neutral errors surfaced by core runtime abstractions.
///
/// Non-exhaustive: future engine concerns may add variants. Callers should
/// keep a `_` arm in `match` expressions.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum CoreError {
    /// Parsing or decoding an external document failed.
    InvalidDocument(ErrorContext),
    /// A value failed structural or semantic validation.
    Validation(ErrorContext),
    /// A deterministic configured ceiling was exceeded.
    LimitExceeded {
        /// Exhausted resource kind.
        resource: ResourceKind,
        /// Configured ceiling.
        limit: u64,
        /// Requested amount when known.
        requested: Option<u64>,
        /// Observed amount when known.
        observed: Option<u64>,
        /// Bounded diagnostic context.
        context: ErrorContext,
    },
    /// A requested entity or generation-bearing handle was not found.
    NotFound(ErrorContext),
    /// The requested operation is not supported at the applicable boundary.
    UnsupportedOperation(ErrorContext),
    /// Candidate selection failed before execution.
    Selection {
        /// Structured selection reason.
        reason: SelectionFailureReason,
        /// Bounded diagnostic context.
        context: ErrorContext,
    },
    /// A committed security operation failed.
    Security {
        /// Redacted security reason.
        reason: SecurityFailureReason,
        /// Bounded diagnostic context.
        context: ErrorContext,
    },
    /// Application code failed or panicked.
    Application(ErrorContext),
    /// Protocol binding execution failed.
    Binding(ErrorContext),
    /// Payload encoding, decoding, media, or representation validation failed.
    Payload(ErrorContext),
    /// Shared queue, slot, or concurrency capacity is currently occupied.
    Backpressure(ErrorContext),
    /// The operation was cancelled while a reply opportunity remained.
    Cancelled(ErrorContext),
    /// A deadline expired.
    TimedOut(ErrorContext),
    /// A generation-bearing handle no longer identifies the live object.
    StaleHandle(ErrorContext),
    /// A lifecycle transition could not complete.
    Lifecycle(ErrorContext),
    /// Explicit cleanup failed or retained residual state.
    Cleanup(ErrorContext),
    /// A detected engine invariant was violated.
    InternalInvariant(ErrorContext),
}

impl CoreError {
    /// Returns the bounded diagnostic context for every error category.
    pub const fn context(&self) -> &ErrorContext {
        match self {
            Self::InvalidDocument(context)
            | Self::Validation(context)
            | Self::NotFound(context)
            | Self::UnsupportedOperation(context)
            | Self::Application(context)
            | Self::Binding(context)
            | Self::Payload(context)
            | Self::Backpressure(context)
            | Self::Cancelled(context)
            | Self::TimedOut(context)
            | Self::StaleHandle(context)
            | Self::Lifecycle(context)
            | Self::Cleanup(context)
            | Self::InternalInvariant(context)
            | Self::LimitExceeded { context, .. }
            | Self::Selection { context, .. }
            | Self::Security { context, .. } => context,
        }
    }

    /// Returns the retry classification carried by this error.
    pub const fn retry_class(&self) -> RetryClass {
        self.context().retry_class()
    }

    /// Returns the optional retry-after hint carried by this error.
    pub const fn retry_after(&self) -> Option<Duration> {
        self.context().retry_after()
    }

    /// Returns the structured selection reason when this is a selection error.
    pub const fn selection_reason(&self) -> Option<SelectionFailureReason> {
        match self {
            Self::Selection { reason, .. } => Some(*reason),
            _ => None,
        }
    }

    /// Returns the structured security reason when this is a security error.
    pub const fn security_reason(&self) -> Option<SecurityFailureReason> {
        match self {
            Self::Security { reason, .. } => Some(*reason),
            _ => None,
        }
    }

    /// Returns structured limit details when this is a limit error.
    pub const fn limit_details(&self) -> Option<(ResourceKind, u64, Option<u64>, Option<u64>)> {
        match self {
            Self::LimitExceeded {
                resource,
                limit,
                requested,
                observed,
                ..
            } => Some((*resource, *limit, *requested, *observed)),
            _ => None,
        }
    }

    const fn category(&self) -> &'static str {
        match self {
            Self::InvalidDocument(_) => "InvalidDocument",
            Self::Validation(_) => "Validation",
            Self::LimitExceeded { .. } => "LimitExceeded",
            Self::NotFound(_) => "NotFound",
            Self::UnsupportedOperation(_) => "UnsupportedOperation",
            Self::Selection { .. } => "Selection",
            Self::Security { .. } => "Security",
            Self::Application(_) => "Application",
            Self::Binding(_) => "Binding",
            Self::Payload(_) => "Payload",
            Self::Backpressure(_) => "Backpressure",
            Self::Cancelled(_) => "Cancelled",
            Self::TimedOut(_) => "TimedOut",
            Self::StaleHandle(_) => "StaleHandle",
            Self::Lifecycle(_) => "Lifecycle",
            Self::Cleanup(_) => "Cleanup",
            Self::InternalInvariant(_) => "InternalInvariant",
        }
    }
}

impl fmt::Debug for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CoreError")
            .field("category", &self.category())
            .field("context", self.context())
            .finish()
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.category(), self.context())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use core::mem::size_of;

    use clinkz_wot_foundation::{Generation, ResourceKind, SlotIndex};
    use clinkz_wot_td::data_type::Operation;

    use super::{
        CoreError, ErrorContext, ErrorPhase, REDACTED_CAUSE_CAPACITY, RetryClass,
        SecurityFailureReason, SelectionFailureReason,
    };
    use crate::{AffordanceSlotId, BindingGeneration, BindingId, CorrelationId, ThingSlotId};

    #[test]
    fn error_context_retains_compact_structured_identity() {
        let context = ErrorContext::new(ErrorPhase::Handler, RetryClass::CallerDecision)
            .with_thing(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL))
            .with_target(AffordanceSlotId::new(
                SlotIndex::new(2),
                Generation::INITIAL,
            ))
            .with_operation(Operation::InvokeAction)
            .with_form_index(3)
            .with_binding(BindingId::new(4), BindingGeneration::INITIAL)
            .with_correlation(CorrelationId::new(5));

        assert_eq!(
            context.thing().map(ThingSlotId::slot),
            Some(SlotIndex::new(1))
        );
        assert_eq!(context.operation(), Some(Operation::InvokeAction));
        assert_eq!(context.form_index(), Some(3));
        assert_eq!(context.correlation().map(CorrelationId::get), Some(5));
        assert_eq!(context.phase(), ErrorPhase::Handler);
        assert_eq!(context.retry_class(), RetryClass::CallerDecision);
    }

    #[test]
    fn redacted_cause_truncates_at_a_utf8_boundary() {
        let mut message = "a".repeat(95);
        message.push('界');
        let context = ErrorContext::new(ErrorPhase::Binding, RetryClass::Safe)
            .with_redacted_cause(17, &message);

        assert_eq!(context.cause_code(), Some(17));
        assert_eq!(context.redacted_cause().map(str::len), Some(95));
        assert!(context.cause_was_truncated());
        assert!(!format!("{context:?}").contains('界'));
    }

    #[test]
    fn replacing_a_cause_clears_invisible_buffer_tail() {
        let base = ErrorContext::new(ErrorPhase::Binding, RetryClass::Safe);
        let replaced = base
            .with_redacted_cause(17, &"x".repeat(REDACTED_CAUSE_CAPACITY))
            .with_redacted_cause(18, "short");
        let direct = base.with_redacted_cause(18, "short");

        assert_eq!(replaced, direct);
        assert_eq!(replaced.redacted_cause(), Some("short"));
    }

    #[test]
    fn error_context_is_copyable_and_fixed_capacity() {
        const fn assert_copy<T: Copy>() {}

        assert_copy::<ErrorContext>();
        assert!(size_of::<ErrorContext>() <= 256);
    }

    #[test]
    fn core_error_exposes_structured_details_without_recovering_text() {
        let context = ErrorContext::new(ErrorPhase::Selection, RetryClass::Never);
        let selection = CoreError::Selection {
            reason: SelectionFailureReason::NoSupportingBinding,
            context,
        };
        assert_eq!(
            selection.selection_reason(),
            Some(SelectionFailureReason::NoSupportingBinding)
        );
        assert_eq!(selection.security_reason(), None);
        assert_eq!(selection.context(), &context);

        let security = CoreError::Security {
            reason: SecurityFailureReason::AuthorizationDenied,
            context,
        };
        assert_eq!(
            security.security_reason(),
            Some(SecurityFailureReason::AuthorizationDenied)
        );

        let limit = CoreError::LimitExceeded {
            resource: ResourceKind::PayloadBytesMax,
            limit: 128,
            requested: Some(129),
            observed: None,
            context,
        };
        assert_eq!(
            limit.limit_details(),
            Some((ResourceKind::PayloadBytesMax, 128, Some(129), None))
        );
    }

    #[test]
    fn core_error_formatting_is_bounded_and_redacted() {
        let context = ErrorContext::new(ErrorPhase::Binding, RetryClass::CallerDecision)
            .with_redacted_cause(7, "reviewed diagnostic");
        let error = CoreError::Binding(context);
        let debug = format!("{error:?}");
        let display = format!("{error}");

        assert!(debug.contains("Binding"));
        assert!(debug.contains("reviewed diagnostic"));
        assert!(display.contains("cause_code=7"));
        assert!(!debug.contains("CallerDecision"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn core_error_never_retains_a_source_chain() {
        use std::error::Error;

        let error = CoreError::Application(ErrorContext::new(
            ErrorPhase::Handler,
            RetryClass::CallerDecision,
        ));
        assert!(error.source().is_none());
    }
}
