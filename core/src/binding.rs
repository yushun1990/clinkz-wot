//! Protocol-neutral binding registration, route, delivery, and cleanup contracts.
//!
//! This module is the target-generation binding boundary.  It deliberately
//! contains no registry, handler-dispatch, form-selection, or concrete
//! transport authority.  A binding receives only frozen artifact references,
//! owned route inputs, and a short-lived route activation permit.

use core::mem::{align_of, needs_drop, size_of};
use core::task::{Context, Poll};

#[cfg(feature = "std")]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use core::pin::Pin;
#[cfg(feature = "std")]
use std::any::Any;

use clinkz_wot_foundation::{Generation, WorkBudget};

use crate::{
    AffordanceTarget, BindingArtifactCompatibility, BindingArtifactEnvelope, BindingArtifactRef,
    BindingConfigurationDigest, BindingGeneration, BindingId, CleanupOperation, CleanupRecord,
    CleanupSlotId, CoreError, CoreResult, CorrelationId, Deadline, ErrorContext, ErrorPhase,
    InteractionInput, InteractionOutput, PlanId, PlanSetGeneration, RetryClass, StartStatus,
    ThingId,
};
use crate::{BindingCompilerExtension, StaticBindingCompilerRegistration};
#[cfg(feature = "std")]
use crate::{HostBindingArtifact, HostBindingCompilerRegistration};

// ---------------------------------------------------------------------------
// Registration and bounded-resource declarations
// ---------------------------------------------------------------------------

/// Immutable retained storage declared before a binding operation is admitted.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BindingLifetimeFootprint {
    retained_items: u32,
    retained_bytes: u64,
}

impl BindingLifetimeFootprint {
    /// Creates an exact retained item and byte declaration.
    pub const fn new(retained_items: u32, retained_bytes: u64) -> Self {
        Self {
            retained_items,
            retained_bytes,
        }
    }

    /// Returns the maximum retained item count.
    pub const fn retained_items(self) -> u32 {
        self.retained_items
    }

    /// Returns the maximum retained byte count.
    pub const fn retained_bytes(self) -> u64 {
        self.retained_bytes
    }

    /// Returns whether this declaration fits an admitted ceiling.
    pub const fn fits_within(self, admitted: Self) -> bool {
        self.retained_items <= admitted.retained_items
            && self.retained_bytes <= admitted.retained_bytes
    }
}

/// Peak binding-private temporary storage used by one bounded callback.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BindingTransientFootprint {
    peak_bytes: u64,
}

impl BindingTransientFootprint {
    /// Creates a peak temporary byte declaration.
    pub const fn new(peak_bytes: u64) -> Self {
        Self { peak_bytes }
    }

    /// Returns the maximum temporary bytes live at once.
    pub const fn peak_bytes(self) -> u64 {
        self.peak_bytes
    }
}

/// Bounded item and byte allowance for protocol ingress.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BindingIngressLimits {
    items: u32,
    bytes: u64,
}

impl BindingIngressLimits {
    /// Creates an exact ingress allowance.
    pub const fn new(items: u32, bytes: u64) -> Self {
        Self { items, bytes }
    }

    /// Returns the admitted item count.
    pub const fn items(self) -> u32 {
        self.items
    }

    /// Returns the admitted byte count.
    pub const fn bytes(self) -> u64 {
        self.bytes
    }

    /// Returns whether this allowance fits another scope's ceiling.
    pub const fn fits_within(self, ceiling: Self) -> bool {
        self.items <= ceiling.items && self.bytes <= ceiling.bytes
    }

    const fn is_empty(self) -> bool {
        self.items == 0 && self.bytes == 0
    }
}

/// Maximum representation of one caller-owned constrained state value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BindingStateLayout {
    size: u64,
    alignment: u64,
    lifetime: BindingLifetimeFootprint,
    transient: BindingTransientFootprint,
    trivial_drop: bool,
}

impl BindingStateLayout {
    /// Describes one concrete associated state type.
    pub const fn of<T>(lifetime: BindingLifetimeFootprint) -> Self {
        Self {
            size: size_of::<T>() as u64,
            alignment: align_of::<T>() as u64,
            lifetime,
            transient: BindingTransientFootprint::new(0),
            trivial_drop: !needs_drop::<T>(),
        }
    }

    /// Adds the maximum temporary storage used while polling this state.
    #[must_use]
    pub const fn with_transient(mut self, transient: BindingTransientFootprint) -> Self {
        self.transient = transient;
        self
    }

    /// Returns the state size in bytes.
    pub const fn size(self) -> u64 {
        self.size
    }

    /// Returns the state alignment in bytes.
    pub const fn alignment(self) -> u64 {
        self.alignment
    }

    /// Returns the immutable retained-footprint declaration.
    pub const fn lifetime_footprint(self) -> BindingLifetimeFootprint {
        self.lifetime
    }

    /// Returns the peak per-poll temporary declaration.
    pub const fn transient_footprint(self) -> BindingTransientFootprint {
        self.transient
    }

    /// Returns whether terminal acknowledgement may reclaim without drop work.
    pub const fn has_trivial_drop(self) -> bool {
        self.trivial_drop
    }

    const fn fits_within(self, admitted: BindingLifetimeFootprint) -> bool {
        self.lifetime.fits_within(admitted) && self.size <= admitted.retained_bytes
    }
}

/// Complete immutable identity of one startup binding registration.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BindingRegistrationIdentity {
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    configuration: BindingConfigurationDigest,
    artifact_compatibility: BindingArtifactCompatibility,
    diagnostic_ordinal: u32,
}

impl BindingRegistrationIdentity {
    /// Creates a complete registration identity.
    pub const fn new(
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        configuration: BindingConfigurationDigest,
        artifact_compatibility: BindingArtifactCompatibility,
        diagnostic_ordinal: u32,
    ) -> Self {
        Self {
            binding_id,
            binding_generation,
            configuration,
            artifact_compatibility,
            diagnostic_ordinal,
        }
    }

    /// Returns the stable binding id.
    pub const fn binding_id(self) -> BindingId {
        self.binding_id
    }

    /// Returns the binding registration generation.
    pub const fn binding_generation(self) -> BindingGeneration {
        self.binding_generation
    }

    /// Returns the captured configuration digest.
    pub const fn configuration(self) -> BindingConfigurationDigest {
        self.configuration
    }

    /// Returns the compiler/server artifact compatibility identity.
    pub const fn artifact_compatibility(self) -> BindingArtifactCompatibility {
        self.artifact_compatibility
    }

    /// Returns the deterministic diagnostic-only registration ordinal.
    pub const fn diagnostic_ordinal(self) -> u32 {
        self.diagnostic_ordinal
    }
}

/// Capability roles advertised by the narrow complete registration.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BindingRegistrationCapabilities {
    producer_property_read: bool,
}

impl BindingRegistrationCapabilities {
    /// Creates an explicit Producer Property Read declaration.
    pub const fn new(producer_property_read: bool) -> Self {
        Self {
            producer_property_read,
        }
    }

    /// Declares the active narrow Producer Property Read capability.
    pub const fn producer_property_read() -> Self {
        Self::new(true)
    }

    /// Returns whether Producer Property Read is advertised.
    pub const fn supports_producer_property_read(self) -> bool {
        self.producer_property_read
    }
}

/// Runtime representations supplied for the advertised server capability.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BindingExecutionSupport {
    host_erased: bool,
    application_static: bool,
}

impl BindingExecutionSupport {
    /// Creates an explicit representation declaration.
    pub const fn new(host_erased: bool, application_static: bool) -> Self {
        Self {
            host_erased,
            application_static,
        }
    }

    /// Declares support in both active Property Read representations.
    pub const fn producer_property_read() -> Self {
        Self::new(true, true)
    }

    /// Declares only the host-erased representation.
    pub const fn host_erased() -> Self {
        Self::new(true, false)
    }

    /// Declares only the caller-owned static representation.
    pub const fn application_static() -> Self {
        Self::new(false, true)
    }

    /// Returns whether the `std` host representation is supplied.
    pub const fn supports_host_erased(self) -> bool {
        self.host_erased
    }

    /// Returns whether the portable static representation is supplied.
    pub const fn supports_application_static(self) -> bool {
        self.application_static
    }
}

/// Registration-level declared and admitted retained-resource bounds.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BindingResourceDeclarations {
    declared: BindingLifetimeFootprint,
    admitted: BindingLifetimeFootprint,
    route_state: BindingLifetimeFootprint,
    readiness_state: BindingLifetimeFootprint,
    response_state: BindingLifetimeFootprint,
    transient: BindingTransientFootprint,
}

impl BindingResourceDeclarations {
    /// Creates the registration-wide retained declaration and ceiling.
    pub const fn new(
        declared: BindingLifetimeFootprint,
        admitted: BindingLifetimeFootprint,
    ) -> Self {
        Self {
            declared,
            admitted,
            route_state: admitted,
            readiness_state: admitted,
            response_state: admitted,
            transient: BindingTransientFootprint::new(0),
        }
    }

    /// Sets the three constrained-state retained ceilings.
    #[must_use]
    pub const fn with_state_footprints(
        mut self,
        route: BindingLifetimeFootprint,
        readiness: BindingLifetimeFootprint,
        response: BindingLifetimeFootprint,
    ) -> Self {
        self.route_state = route;
        self.readiness_state = readiness;
        self.response_state = response;
        self
    }

    /// Sets the maximum transient bytes for one callback.
    #[must_use]
    pub const fn with_transient(mut self, transient: BindingTransientFootprint) -> Self {
        self.transient = transient;
        self
    }

    /// Returns the complete registration declaration.
    pub const fn declared(self) -> BindingLifetimeFootprint {
        self.declared
    }

    /// Returns the admitted registration ceiling.
    pub const fn admitted(self) -> BindingLifetimeFootprint {
        self.admitted
    }

    /// Returns the admitted route-state ceiling.
    pub const fn route_state(self) -> BindingLifetimeFootprint {
        self.route_state
    }

    /// Returns the admitted readiness-state ceiling.
    pub const fn readiness_state(self) -> BindingLifetimeFootprint {
        self.readiness_state
    }

    /// Returns the admitted response-state ceiling.
    pub const fn response_state(self) -> BindingLifetimeFootprint {
        self.response_state
    }

    /// Returns the peak temporary callback declaration.
    pub const fn transient(self) -> BindingTransientFootprint {
        self.transient
    }

    const fn is_valid(self) -> bool {
        self.declared.fits_within(self.admitted)
            && self.route_state.fits_within(self.admitted)
            && self.readiness_state.fits_within(self.admitted)
            && self.response_state.fits_within(self.admitted)
    }
}

/// Endpoint behavior while a prepared route is not yet serving.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum RoutePreparationVisibility {
    /// Route preparation has no externally visible endpoint.
    #[default]
    Hidden,
    /// Reject protocol input while the serving gate is closed.
    Reject,
    /// Apply protocol-native bounded backpressure while closed.
    Backpressure,
    /// Retain input only within the declared ingress limits.
    BufferWithinAdmittedLimits,
}

/// Nested route, binding, and process ingress bounds plus closed-gate policy.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BindingIngressPolicy {
    preparation_visibility: RoutePreparationVisibility,
    per_route: BindingIngressLimits,
    per_binding: BindingIngressLimits,
    global: BindingIngressLimits,
}

impl BindingIngressPolicy {
    /// Creates a complete ingress policy.
    pub const fn new(
        preparation_visibility: RoutePreparationVisibility,
        per_route: BindingIngressLimits,
        per_binding: BindingIngressLimits,
        global: BindingIngressLimits,
    ) -> Self {
        Self {
            preparation_visibility,
            per_route,
            per_binding,
            global,
        }
    }

    /// Creates a hidden-preparation policy with no closed-gate buffering.
    pub const fn hidden() -> Self {
        Self::new(
            RoutePreparationVisibility::Hidden,
            BindingIngressLimits::new(0, 0),
            BindingIngressLimits::new(0, 0),
            BindingIngressLimits::new(0, 0),
        )
    }

    /// Returns the prepared-route external visibility.
    pub const fn preparation_visibility(self) -> RoutePreparationVisibility {
        self.preparation_visibility
    }

    /// Returns the per-route ingress ceiling.
    pub const fn per_route(self) -> BindingIngressLimits {
        self.per_route
    }

    /// Returns the per-registration ingress ceiling.
    pub const fn per_binding(self) -> BindingIngressLimits {
        self.per_binding
    }

    /// Returns the process-wide ingress ceiling consumed by this declaration.
    pub const fn global(self) -> BindingIngressLimits {
        self.global
    }

    const fn is_valid(self) -> bool {
        if !self.per_route.fits_within(self.per_binding)
            || !self.per_binding.fits_within(self.global)
        {
            return false;
        }
        !matches!(
            self.preparation_visibility,
            RoutePreparationVisibility::BufferWithinAdmittedLimits
        ) || !self.per_route.is_empty()
    }
}

/// Bounded durable status retained for one registration.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BindingStatusPolicy {
    retained_records: u32,
    retained_bytes: u64,
}

impl BindingStatusPolicy {
    /// Creates a bounded retained-status policy.
    pub const fn new(retained_records: u32, retained_bytes: u64) -> Self {
        Self {
            retained_records,
            retained_bytes,
        }
    }

    /// Returns the retained record ceiling.
    pub const fn retained_records(self) -> u32 {
        self.retained_records
    }

    /// Returns the retained byte ceiling.
    pub const fn retained_bytes(self) -> u64 {
        self.retained_bytes
    }
}

// ---------------------------------------------------------------------------
// Recoverable input failures and cleanup transfer
// ---------------------------------------------------------------------------

/// Structured binding failure, optionally qualified by one exact route.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingOperationalError {
    route: Option<BindingRouteKey>,
    error: CoreError,
}

impl BindingOperationalError {
    /// Creates an error that is not yet associated with a route.
    pub const fn new(error: CoreError) -> Self {
        Self { route: None, error }
    }

    /// Creates an error for one exact route generation.
    pub const fn for_route(route: BindingRouteKey, error: CoreError) -> Self {
        Self {
            route: Some(route),
            error,
        }
    }

    /// Returns the route identity when one is applicable.
    pub const fn route(&self) -> Option<&BindingRouteKey> {
        self.route.as_ref()
    }

    /// Returns the structured core error.
    pub const fn error(&self) -> &CoreError {
        &self.error
    }

    /// Consumes the value into its route and error parts.
    pub fn into_parts(self) -> (Option<BindingRouteKey>, CoreError) {
        (self.route, self.error)
    }
}

/// A pre-acceptance failure that returns the complete caller-owned input.
#[derive(Debug, Eq, PartialEq)]
pub struct BindingInputRejection<T> {
    input: T,
    error: BindingOperationalError,
}

impl<T> BindingInputRejection<T> {
    /// Creates an ownership-preserving rejection.
    pub const fn new(input: T, error: BindingOperationalError) -> Self {
        Self { input, error }
    }

    /// Borrows the unchanged rejected input.
    pub const fn input(&self) -> &T {
        &self.input
    }

    /// Borrows the structured rejection reason.
    pub const fn error(&self) -> &BindingOperationalError {
        &self.error
    }

    /// Consumes the rejection and returns the complete input.
    pub fn into_input(self) -> T {
        self.input
    }

    /// Consumes the rejection into both owned parts.
    pub fn into_parts(self) -> (T, BindingOperationalError) {
        (self.input, self.error)
    }
}

/// Capacity reserved before a binding side effect can create cleanup work.
#[derive(Debug, Eq, PartialEq)]
pub struct CleanupReservation {
    subject: CleanupSlotId,
    lifetime: BindingLifetimeFootprint,
    durable_status_records: u32,
    work: WorkBudget,
}

impl CleanupReservation {
    /// Creates a complete pre-side-effect cleanup reservation.
    pub const fn new(
        subject: CleanupSlotId,
        lifetime: BindingLifetimeFootprint,
        durable_status_records: u32,
        work: WorkBudget,
    ) -> Self {
        Self {
            subject,
            lifetime,
            durable_status_records,
            work,
        }
    }

    /// Returns the generation-bearing reserved subject.
    pub const fn subject(&self) -> CleanupSlotId {
        self.subject
    }

    /// Returns the maximum retained cleanup footprint.
    pub const fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        self.lifetime
    }

    /// Returns the durable record capacity retained by the reservation.
    pub const fn durable_status_records(&self) -> u32 {
        self.durable_status_records
    }

    /// Returns the admitted cleanup work allowance.
    pub const fn work(&self) -> &WorkBudget {
        &self.work
    }
}

/// Immutable first-cause and bound context for one cleanup phase.
#[derive(Debug, Eq, PartialEq)]
pub struct CleanupPhaseContext {
    reservation: CleanupReservation,
    operation: CleanupOperation,
    first_cause: CoreError,
    deadline: Deadline,
}

impl CleanupPhaseContext {
    /// Binds a reservation to one phase without mutating it into later work.
    pub const fn bind(
        reservation: CleanupReservation,
        operation: CleanupOperation,
        first_cause: CoreError,
        deadline: Deadline,
    ) -> Self {
        Self {
            reservation,
            operation,
            first_cause,
            deadline,
        }
    }

    /// Returns the reserved cleanup capacity.
    pub const fn reservation(&self) -> &CleanupReservation {
        &self.reservation
    }

    /// Returns this context's immutable cleanup operation.
    pub const fn operation(&self) -> CleanupOperation {
        self.operation
    }

    /// Returns the immutable first cause.
    pub const fn first_cause(&self) -> &CoreError {
        &self.first_cause
    }

    /// Returns this phase's independent drain deadline.
    pub const fn deadline(&self) -> Deadline {
        self.deadline
    }
}

/// Provisional request to transfer complete cleanup work to a named owner.
#[derive(Debug, Eq, PartialEq)]
pub struct CleanupTransferRequest {
    phase: CleanupPhaseContext,
    requested_owner: CleanupSlotId,
}

impl CleanupTransferRequest {
    /// Creates a bounded transfer request; this alone does not transfer work.
    pub const fn new(phase: CleanupPhaseContext, requested_owner: CleanupSlotId) -> Self {
        Self {
            phase,
            requested_owner,
        }
    }

    /// Returns the immutable cleanup phase.
    pub const fn phase(&self) -> &CleanupPhaseContext {
        &self.phase
    }

    /// Returns the requested generation-bearing owner.
    pub const fn requested_owner(&self) -> CleanupSlotId {
        self.requested_owner
    }

    /// Consumes the request into its complete parts.
    pub fn into_parts(self) -> (CleanupPhaseContext, CleanupSlotId) {
        (self.phase, self.requested_owner)
    }
}

/// One provisional transfer request plus the complete work object.
#[derive(Debug, Eq, PartialEq)]
pub struct CleanupTransferEnvelope<T> {
    request: CleanupTransferRequest,
    work: T,
}

impl<T> CleanupTransferEnvelope<T> {
    /// Creates a transfer envelope without changing ownership.
    pub const fn new(request: CleanupTransferRequest, work: T) -> Self {
        Self { request, work }
    }

    /// Borrows the provisional transfer request.
    pub const fn request(&self) -> &CleanupTransferRequest {
        &self.request
    }

    /// Returns the request and identical complete work object.
    pub fn into_parts(self) -> (CleanupTransferRequest, T) {
        (self.request, self.work)
    }
}

/// Result of offering a complete cleanup envelope to a named owner.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Eq, PartialEq)]
pub enum CleanupTransferAcceptance<T> {
    /// The named owner now retains the complete work and progress lease.
    Accepted(CleanupRecord),
    /// The source still owns the identical complete envelope.
    Rejected(CleanupTransferEnvelope<T>),
}

/// Atomic acceptance boundary for complete cleanup work.
pub trait CleanupTransferTarget<T> {
    /// Accepts ownership or returns the identical envelope to the source.
    fn try_accept(&mut self, transfer: CleanupTransferEnvelope<T>) -> CleanupTransferAcceptance<T>;
}

/// Proof that no operation-specific local cleanup continuation exists.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct NoCleanupSuccessor;

/// Terminal cleanup disposition after cancellation has linearized.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Eq, PartialEq)]
pub enum BindingCancellationDisposition<C> {
    /// Cleanup completed and the typed successor is returned.
    Complete { successor: C },
    /// Complete work must be offered before pending cleanup can be reported.
    TransferRequired(CleanupTransferRequest),
    /// Local work ended with a durable external residual record.
    ResidualExternalState {
        /// The typed continuation or terminal successor.
        successor: C,
        /// Bounded durable residual identity.
        record: CleanupRecord,
    },
}

/// Normal/late result or explicit cancellation settlement.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Eq, PartialEq)]
pub enum BindingCallSettlement<T, C = NoCleanupSuccessor> {
    /// A normal or late terminal value.
    Returned(T),
    /// Cancellation settled with retry advice and explicit cleanup ownership.
    Cancelled {
        /// Retry classification for the cancelled operation.
        retry_class: RetryClass,
        /// Operation-specific cleanup disposition.
        disposition: BindingCancellationDisposition<C>,
    },
}

// ---------------------------------------------------------------------------
// Route identity, requests, responses, outcomes, and caller-owned slots
// ---------------------------------------------------------------------------

/// Stable namespace in which endpoint reservation keys collide.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CollisionDomainId([u8; 16]);

impl CollisionDomainId {
    /// Creates an identity from its complete fixed-width representation.
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the complete representation.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// Canonical endpoint identity within one collision domain.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct EndpointReservationKey([u8; 32]);

impl EndpointReservationKey {
    /// Creates a key from its complete fixed-width representation.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the complete representation.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Generation-independent physical endpoint reservation identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RouteReservationIdentity {
    collision_domain: CollisionDomainId,
    endpoint: EndpointReservationKey,
}

impl RouteReservationIdentity {
    /// Creates a complete endpoint collision identity.
    pub const fn new(
        collision_domain: CollisionDomainId,
        endpoint: EndpointReservationKey,
    ) -> Self {
        Self {
            collision_domain,
            endpoint,
        }
    }

    /// Returns the collision namespace.
    pub const fn collision_domain(self) -> CollisionDomainId {
        self.collision_domain
    }

    /// Returns the canonical endpoint key.
    pub const fn endpoint(self) -> EndpointReservationKey {
        self.endpoint
    }
}

/// Complete generation-bearing identity of one prepared server route.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingRouteKey {
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    route_generation: Generation,
    plan_set_generation: PlanSetGeneration,
    plan_id: PlanId,
    reservation: RouteReservationIdentity,
}

impl BindingRouteKey {
    /// Creates a complete route identity.
    pub const fn new(
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        route_generation: Generation,
        plan_set_generation: PlanSetGeneration,
        plan_id: PlanId,
        reservation: RouteReservationIdentity,
    ) -> Self {
        Self {
            binding_id,
            binding_generation,
            route_generation,
            plan_set_generation,
            plan_id,
            reservation,
        }
    }

    /// Returns the binding id.
    pub const fn binding_id(self) -> BindingId {
        self.binding_id
    }

    /// Returns the binding registration generation.
    pub const fn binding_generation(self) -> BindingGeneration {
        self.binding_generation
    }

    /// Returns the caller-owned route generation.
    pub const fn route_generation(self) -> Generation {
        self.route_generation
    }

    /// Returns the frozen plan-set generation.
    pub const fn plan_set_generation(self) -> PlanSetGeneration {
        self.plan_set_generation
    }

    /// Returns the immutable logical plan id.
    pub const fn plan_id(self) -> PlanId {
        self.plan_id
    }

    /// Returns the generation-independent endpoint reservation.
    pub const fn reservation(self) -> RouteReservationIdentity {
        self.reservation
    }
}

/// Owned input used to begin preparation of one frozen server route.
#[derive(Debug, Eq, PartialEq)]
pub struct PrepareInput {
    route: BindingRouteKey,
    artifact: BindingArtifactRef,
    admitted_footprint: BindingLifetimeFootprint,
}

impl PrepareInput {
    /// Creates a complete route-preparation input.
    pub const fn new(
        route: BindingRouteKey,
        artifact: BindingArtifactRef,
        admitted_footprint: BindingLifetimeFootprint,
    ) -> Self {
        Self {
            route,
            artifact,
            admitted_footprint,
        }
    }

    /// Returns the exact route identity.
    pub const fn route(&self) -> &BindingRouteKey {
        &self.route
    }

    /// Returns the frozen binding-artifact reference.
    pub const fn artifact(&self) -> BindingArtifactRef {
        self.artifact
    }

    /// Returns the admitted lifetime footprint for the route guard/state.
    pub const fn admitted_footprint(&self) -> BindingLifetimeFootprint {
        self.admitted_footprint
    }

    /// Consumes the input into all owned and fixed-size parts.
    pub fn into_parts(
        self,
    ) -> (
        BindingRouteKey,
        BindingArtifactRef,
        BindingLifetimeFootprint,
    ) {
        (self.route, self.artifact, self.admitted_footprint)
    }
}

/// Single-use route and correlation capability for exactly one response.
#[derive(Debug, Eq, PartialEq)]
pub struct RouteResponseOpportunity {
    route: BindingRouteKey,
    correlation: CorrelationId,
}

impl RouteResponseOpportunity {
    /// Creates one generation-bearing single-use opportunity.
    pub const fn new(route: BindingRouteKey, correlation: CorrelationId) -> Self {
        Self { route, correlation }
    }

    /// Returns the exact route identity.
    pub const fn route(&self) -> &BindingRouteKey {
        &self.route
    }

    /// Returns the route-local correlation identity.
    pub const fn correlation(&self) -> CorrelationId {
        self.correlation
    }

    /// Consumes the opportunity into its fixed-size identities.
    pub fn into_parts(self) -> (BindingRouteKey, CorrelationId) {
        (self.route, self.correlation)
    }
}

/// Complete owned Property Read request emitted by one route accept poll.
#[derive(Debug, Eq, PartialEq)]
pub struct RouteInboundRequest {
    route: BindingRouteKey,
    correlation: CorrelationId,
    target: AffordanceTarget,
    input: InteractionInput,
    response: RouteResponseOpportunity,
}

impl RouteInboundRequest {
    /// Creates one request and its unique response opportunity.
    pub fn new(
        route: BindingRouteKey,
        correlation: CorrelationId,
        target: AffordanceTarget,
        input: InteractionInput,
    ) -> Self {
        Self {
            route,
            correlation,
            target,
            input,
            response: RouteResponseOpportunity::new(route, correlation),
        }
    }

    /// Returns the exact route identity.
    pub const fn route(&self) -> &BindingRouteKey {
        &self.route
    }

    /// Returns the route-local correlation identity.
    pub const fn correlation(&self) -> CorrelationId {
        self.correlation
    }

    /// Returns the exact frozen affordance target.
    pub const fn target(&self) -> &AffordanceTarget {
        &self.target
    }

    /// Returns the owned protocol-neutral interaction input.
    pub const fn input(&self) -> &InteractionInput {
        &self.input
    }

    /// Consumes the request and returns every owned part exactly once.
    pub fn into_parts(
        self,
    ) -> (
        BindingRouteKey,
        CorrelationId,
        AffordanceTarget,
        InteractionInput,
        RouteResponseOpportunity,
    ) {
        (
            self.route,
            self.correlation,
            self.target,
            self.input,
            self.response,
        )
    }

    /// Consumes the request and returns its single-use response opportunity.
    pub fn into_response_opportunity(self) -> RouteResponseOpportunity {
        self.response
    }
}

/// Complete success or failure delivered through one response opportunity.
#[derive(Debug, Eq, PartialEq)]
pub struct RouteInboundResponse {
    opportunity: RouteResponseOpportunity,
    result: CoreResult<InteractionOutput>,
}

impl RouteInboundResponse {
    /// Creates a response from its exclusive opportunity and terminal result.
    pub const fn new(
        opportunity: RouteResponseOpportunity,
        result: CoreResult<InteractionOutput>,
    ) -> Self {
        Self {
            opportunity,
            result,
        }
    }

    /// Creates a successful response.
    pub const fn success(opportunity: RouteResponseOpportunity, output: InteractionOutput) -> Self {
        Self::new(opportunity, Ok(output))
    }

    /// Creates a failed response without synthesizing an empty output.
    pub const fn failure(opportunity: RouteResponseOpportunity, error: CoreError) -> Self {
        Self::new(opportunity, Err(error))
    }

    /// Returns the single-use opportunity identity.
    pub const fn opportunity(&self) -> &RouteResponseOpportunity {
        &self.opportunity
    }

    /// Returns the terminal result by reference.
    pub fn result(&self) -> Result<&InteractionOutput, &CoreError> {
        self.result.as_ref()
    }

    /// Consumes the response into its opportunity and terminal result.
    pub fn into_parts(self) -> (RouteResponseOpportunity, CoreResult<InteractionOutput>) {
        (self.opportunity, self.result)
    }
}

/// Result of route preparation while the route slot retains typed state.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Eq, PartialEq)]
pub enum RoutePrepareOutcome<G> {
    /// Preparation completed and returns the stage-specific guard token.
    Prepared(G),
    /// No route resource escaped; the structured failure is terminal.
    RejectedNoResource(BindingOperationalError),
}

/// Result of the distinct prepared-route readiness phase.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Eq, PartialEq)]
pub enum RouteReadinessOutcome<G> {
    /// The same prepared guard is ready for activation.
    Ready(G),
    /// Readiness failed and retains the abortable prepared guard.
    Failed {
        /// Complete prepared guard.
        guard: G,
        /// Structured readiness failure.
        error: BindingOperationalError,
    },
}

/// Result of activating a prepared route while admission remains closed.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Eq, PartialEq)]
pub enum RouteActivationOutcome<P, A> {
    /// Activation succeeded and returns an active guard.
    Active(A),
    /// Activation failed and retains the abortable prepared guard.
    NotActivated {
        /// Complete predecessor guard.
        guard: P,
        /// Structured activation failure.
        error: BindingOperationalError,
    },
}

/// Result of committing an active route to the closed serving gate.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Eq, PartialEq)]
pub enum RouteCommitOutcome<A, C> {
    /// Commit succeeded and returns a distinct committed-closed guard.
    Committed(C),
    /// Commit failed and retains the shutdown-capable active guard.
    NotCommitted {
        /// Complete predecessor guard.
        guard: A,
        /// Structured commit failure.
        error: BindingOperationalError,
    },
}

/// Stage-specific successor retained after route-call cancellation.
#[derive(Debug, Eq, PartialEq)]
pub enum RouteCleanupSuccessor<P, A, C> {
    /// The operation certifies that no protocol route resource exists.
    NoRouteResource { route: BindingRouteKey },
    /// A prepared route still requires abort.
    AbortPrepared(P),
    /// An active route still requires shutdown.
    ShutdownActive(A),
    /// A committed-closed route still requires shutdown.
    ShutdownCommitted(C),
    /// Only a durable route tombstone remains.
    ResidualRouteState { route: BindingRouteKey },
}

/// Terminal result of explicit route abort or shutdown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteCleanupOutcome {
    /// External and local route cleanup completed.
    Complete,
    /// External route state remains and is represented durably.
    ResidualExternalState(CleanupRecord),
}

/// Terminal state emitted by one route accept cursor at most once.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Eq, PartialEq)]
pub enum RouteTerminal {
    /// The route closed normally.
    Closed { route: BindingRouteKey },
    /// The route terminated because of an operational failure.
    Failed {
        /// Exact terminal route generation.
        route: BindingRouteKey,
        /// Structured terminal cause.
        error: BindingOperationalError,
    },
}

impl RouteTerminal {
    /// Returns the exact route identity carried by every terminal branch.
    pub const fn route(&self) -> &BindingRouteKey {
        match self {
            Self::Closed { route } | Self::Failed { route, .. } => route,
        }
    }
}

/// One route-scoped event returned under a borrowed activation permit.
#[derive(Debug, Eq, PartialEq)]
pub enum RouteAcceptEvent {
    /// One owned request and unique response opportunity.
    Request(RouteInboundRequest),
    /// A non-terminal route-local diagnostic failure.
    OperationalError(BindingOperationalError),
    /// The route accept cursor reached its single terminal event.
    Terminal(RouteTerminal),
}

/// Terminal classification of one accepted response delivery.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Eq, PartialEq)]
pub enum BindingDeliveryOutcome {
    /// The response reached the protocol peer's defined delivery boundary.
    Delivered,
    /// Delivery terminated after acceptance with a structured failure.
    Failed(BindingOperationalError),
}

/// Caller-owned typed storage for one server route generation.
#[derive(Debug)]
pub struct ServerRouteSlot<S> {
    input: Option<PrepareInput>,
    state: Option<S>,
}

impl<S> ServerRouteSlot<S> {
    /// Creates an admitted vacant route slot.
    pub const fn new() -> Self {
        Self {
            input: None,
            state: None,
        }
    }

    /// Initializes a vacant slot with the complete preparation input and state.
    ///
    /// # Panics
    ///
    /// Panics when called for a live slot; the caller must acknowledge and
    /// clear the previous generation first.
    pub fn initialize(&mut self, input: PrepareInput, state: S) {
        assert!(
            self.input.is_none() && self.state.is_none(),
            "route slot is live"
        );
        self.input = Some(input);
        self.state = Some(state);
    }

    /// Returns the retained preparation input while the slot is live.
    pub fn input(&self) -> &PrepareInput {
        self.input.as_ref().expect("route slot is vacant")
    }

    /// Returns mutable binding-authored state while the slot is live.
    pub fn state_mut(&mut self) -> &mut S {
        self.state.as_mut().expect("route slot is vacant")
    }

    /// Returns whether this slot currently owns no route input or state.
    pub const fn is_vacant(&self) -> bool {
        self.input.is_none() && self.state.is_none()
    }

    /// Clears an acknowledged terminal route and drops state in caller context.
    pub fn clear(&mut self) {
        assert!(
            self.input.is_some() && self.state.is_some(),
            "route slot is vacant"
        );
        self.state = None;
        self.input = None;
    }
}

impl<S> Default for ServerRouteSlot<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Caller-owned typed storage for one prepared-route readiness operation.
#[derive(Debug)]
pub struct RouteReadinessSlot<S> {
    state: Option<S>,
}

impl<S> RouteReadinessSlot<S> {
    /// Creates an admitted vacant readiness slot.
    pub const fn new() -> Self {
        Self { state: None }
    }

    /// Initializes binding-authored state in a vacant slot.
    pub fn initialize_state(&mut self, state: S) {
        assert!(self.state.is_none(), "readiness slot is live");
        self.state = Some(state);
    }

    /// Returns mutable binding-authored state while the slot is live.
    pub fn state_mut(&mut self) -> &mut S {
        self.state.as_mut().expect("readiness slot is vacant")
    }

    /// Returns whether the slot currently owns no state.
    pub const fn is_vacant(&self) -> bool {
        self.state.is_none()
    }

    /// Clears acknowledged terminal readiness state.
    pub fn clear(&mut self) {
        assert!(self.state.is_some(), "readiness slot is vacant");
        self.state = None;
    }
}

impl<S> Default for RouteReadinessSlot<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Caller-owned typed storage for one accepted response delivery.
#[derive(Debug)]
pub struct ServerResponseSlot<S> {
    response: Option<RouteInboundResponse>,
    state: Option<S>,
}

impl<S> ServerResponseSlot<S> {
    /// Creates an admitted vacant response slot.
    pub const fn new() -> Self {
        Self {
            response: None,
            state: None,
        }
    }

    /// Transfers a complete response and binding state into a vacant slot.
    pub fn initialize(&mut self, response: RouteInboundResponse, state: S) {
        assert!(
            self.response.is_none() && self.state.is_none(),
            "response slot is live"
        );
        self.response = Some(response);
        self.state = Some(state);
    }

    /// Returns the retained response while delivery is live.
    pub fn response(&self) -> &RouteInboundResponse {
        self.response.as_ref().expect("response slot is vacant")
    }

    /// Returns mutable binding-authored delivery state.
    pub fn state_mut(&mut self) -> &mut S {
        self.state.as_mut().expect("response slot is vacant")
    }

    /// Returns whether the slot owns no response or state.
    pub const fn is_vacant(&self) -> bool {
        self.response.is_none() && self.state.is_none()
    }

    /// Clears an acknowledged terminal response and its state.
    pub fn clear(&mut self) {
        assert!(
            self.response.is_some() && self.state.is_some(),
            "response slot is vacant"
        );
        self.state = None;
        self.response = None;
    }
}

impl<S> Default for ServerResponseSlot<S> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Serving activation authority
// ---------------------------------------------------------------------------

/// Immutable authority published atomically with one produced plan set.
pub struct ServingActivationAuthority {
    thing_id: ThingId,
    produced_generation: Generation,
    plan_set_generation: PlanSetGeneration,
}

impl ServingActivationAuthority {
    /// Creates the authority embedded in one produced runtime generation.
    pub fn new(
        thing_id: ThingId,
        produced_generation: Generation,
        plan_set_generation: PlanSetGeneration,
    ) -> Self {
        Self {
            thing_id,
            produced_generation,
            plan_set_generation,
        }
    }

    /// Returns the canonical Thing identity.
    pub const fn thing_id(&self) -> &ThingId {
        &self.thing_id
    }

    /// Returns the produced registry generation.
    pub const fn produced_generation(&self) -> &Generation {
        &self.produced_generation
    }

    /// Returns the atomically published plan-set generation.
    pub const fn plan_set_generation(&self) -> &PlanSetGeneration {
        &self.plan_set_generation
    }

    /// Claims a matching unique route lease for one accept callback.
    pub fn claim_route<'a>(
        &'a self,
        lease: &'a mut RouteAcceptLease,
    ) -> Result<RouteAcceptClaim<'a>, RouteAcceptClaimError> {
        if lease.thing_id != self.thing_id
            || lease.produced_generation != self.produced_generation
            || lease.plan_set_generation != self.plan_set_generation
            || lease.route.plan_set_generation() != self.plan_set_generation
        {
            return Err(RouteAcceptClaimError::AuthorityMismatch);
        }
        Ok(RouteAcceptClaim {
            authority: self,
            lease,
        })
    }
}

/// Caller-owned unique accept lease for one exact route driver.
pub struct RouteAcceptLease {
    thing_id: ThingId,
    produced_generation: Generation,
    plan_set_generation: PlanSetGeneration,
    route: BindingRouteKey,
}

impl RouteAcceptLease {
    /// Creates a lease qualified by its owning activation authority.
    pub fn new(authority: &ServingActivationAuthority, route: BindingRouteKey) -> Self {
        Self {
            thing_id: authority.thing_id.clone(),
            produced_generation: authority.produced_generation,
            plan_set_generation: authority.plan_set_generation,
            route,
        }
    }

    /// Returns the exact route driven by this lease.
    pub const fn route(&self) -> &BindingRouteKey {
        &self.route
    }
}

/// Exclusive matching authority and route-lease borrow.
pub struct RouteAcceptClaim<'a> {
    authority: &'a ServingActivationAuthority,
    lease: &'a mut RouteAcceptLease,
}

impl<'a> RouteAcceptClaim<'a> {
    /// Consumes the claim into the only constructible activation permit.
    pub fn into_permit(self) -> RouteActivationPermit<'a> {
        RouteActivationPermit {
            authority: self.authority,
            lease: self.lease,
        }
    }
}

/// Failure to pair a route lease with the current serving authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteAcceptClaimError {
    /// Thing, produced, plan-set, or route identity does not match.
    AuthorityMismatch,
}

/// Short-lived permit exclusively borrowing one claimed route lease.
pub struct RouteActivationPermit<'a> {
    authority: &'a ServingActivationAuthority,
    lease: &'a mut RouteAcceptLease,
}

impl RouteActivationPermit<'_> {
    /// Returns the exact permitted route identity.
    pub const fn route(&self) -> &BindingRouteKey {
        &self.lease.route
    }

    /// Returns the matching immutable serving authority.
    pub const fn authority(&self) -> &ServingActivationAuthority {
        self.authority
    }
}

// ---------------------------------------------------------------------------
// Portable caller-owned server representation and complete registration
// ---------------------------------------------------------------------------

/// Application-static, manually progressed server binding contract.
pub trait PollServerBinding {
    /// Compiler paired atomically with this server.
    type Compiler: BindingCompilerExtension;
    /// Binding-private route lifecycle state.
    type RouteState;
    /// Binding-private readiness state.
    type ReadinessState;
    /// Binding-private response-delivery state.
    type ResponseState;

    /// Returns the compatibility identity consumed by registration validation.
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility;
    /// Declares the route associated-state layout.
    fn route_state_layout(&self) -> BindingStateLayout;
    /// Declares the readiness associated-state layout.
    fn readiness_state_layout(&self) -> BindingStateLayout;
    /// Declares the response associated-state layout.
    fn response_state_layout(&self) -> BindingStateLayout;

    /// Starts preparation from one checked, non-retainable artifact borrow and
    /// transfers input only on acceptance.
    fn start_prepare(
        &mut self,
        input: PrepareInput,
        artifact: &BindingArtifactEnvelope<<Self::Compiler as BindingCompilerExtension>::Artifact>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<RoutePrepareOutcome<()>>, BindingInputRejection<PrepareInput>>;

    /// Polls accepted route preparation.
    fn poll_prepare(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RoutePrepareOutcome<()>>;

    /// Polls cancellation of accepted preparation.
    fn poll_cancel_prepare(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: &CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RoutePrepareOutcome<()>, ()>>>;

    /// Starts the distinct prepared-route readiness phase.
    fn start_readiness(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        budget: &mut WorkBudget,
    ) -> StartStatus<RouteReadinessOutcome<()>>;

    /// Polls accepted readiness work.
    fn poll_readiness(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteReadinessOutcome<()>>;

    /// Polls cancellation of accepted readiness work.
    fn poll_cancel_readiness(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: &CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RouteReadinessOutcome<()>, ()>>>;

    /// Starts activation while request admission remains closed.
    fn start_activate(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> StartStatus<RouteActivationOutcome<(), ()>>;

    /// Polls accepted activation work.
    fn poll_activate(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteActivationOutcome<(), ()>>;

    /// Polls cancellation of accepted activation work.
    fn poll_cancel_activate(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: &CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RouteActivationOutcome<(), ()>, ()>>>;

    /// Starts commit to the closed serving gate.
    fn start_commit(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> StartStatus<RouteCommitOutcome<(), ()>>;

    /// Polls accepted commit work.
    fn poll_commit(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCommitOutcome<(), ()>>;

    /// Polls cancellation of accepted commit work.
    fn poll_cancel_commit(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: &CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RouteCommitOutcome<(), ()>, ()>>>;

    /// Polls one route accept cursor under a fresh exclusive permit.
    fn poll_accept(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        permit: RouteActivationPermit<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<RouteAcceptEvent>>;

    /// Starts explicit abort of a prepared route.
    fn start_abort(
        &mut self,
        cleanup: CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> StartStatus<RouteCleanupOutcome>;

    /// Polls accepted prepared-route abort.
    fn poll_abort(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome>;

    /// Starts explicit shutdown of an active or committed route.
    fn start_shutdown(
        &mut self,
        cleanup: CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> StartStatus<RouteCleanupOutcome>;

    /// Polls accepted route shutdown.
    fn poll_shutdown(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome>;

    /// Acknowledges terminal route state before caller-owned storage is reused.
    fn acknowledge_route(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
    ) -> CoreResult<()>;

    /// Starts response delivery and transfers the response only on acceptance.
    fn start_response(
        &mut self,
        response: RouteInboundResponse,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<BindingDeliveryOutcome>, BindingInputRejection<RouteInboundResponse>>;

    /// Polls one accepted response delivery.
    fn poll_response(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
        budget: &mut WorkBudget,
    ) -> Poll<BindingDeliveryOutcome>;

    /// Polls cancellation of one accepted response delivery.
    fn poll_cancel_response(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: &CleanupPhaseContext,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<BindingDeliveryOutcome>>>;

    /// Acknowledges terminal response state before storage is reused.
    fn acknowledge_response(
        &mut self,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
    ) -> CoreResult<()>;
}

/// Complete recoverable author input for one static registration.
pub struct StaticBindingRegistrationInput<B>
where
    B: PollServerBinding,
{
    identity: BindingRegistrationIdentity,
    capabilities: BindingRegistrationCapabilities,
    execution: BindingExecutionSupport,
    compiler: StaticBindingCompilerRegistration<B::Compiler>,
    server: B,
    resources: BindingResourceDeclarations,
    ingress: BindingIngressPolicy,
    status: BindingStatusPolicy,
}

impl<B> StaticBindingRegistrationInput<B>
where
    B: PollServerBinding,
{
    /// Assembles a complete input without performing protocol work.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        identity: BindingRegistrationIdentity,
        capabilities: BindingRegistrationCapabilities,
        execution: BindingExecutionSupport,
        compiler: StaticBindingCompilerRegistration<B::Compiler>,
        server: B,
        resources: BindingResourceDeclarations,
        ingress: BindingIngressPolicy,
        status: BindingStatusPolicy,
    ) -> Self {
        Self {
            identity,
            capabilities,
            execution,
            compiler,
            server,
            resources,
            ingress,
            status,
        }
    }

    /// Returns the immutable registration identity.
    pub const fn identity(&self) -> BindingRegistrationIdentity {
        self.identity
    }

    /// Borrows the typed compiler component.
    pub const fn compiler(&self) -> &StaticBindingCompilerRegistration<B::Compiler> {
        &self.compiler
    }

    /// Borrows the static server component.
    pub const fn server(&self) -> &B {
        &self.server
    }
}

/// Validated complete application-static binding bundle.
pub struct StaticBindingRegistration<B>
where
    B: PollServerBinding,
{
    input: StaticBindingRegistrationInput<B>,
}

impl<B> StaticBindingRegistration<B>
where
    B: PollServerBinding,
{
    /// Validates the complete compiler/server/declaration bundle atomically.
    pub fn new(
        input: StaticBindingRegistrationInput<B>,
    ) -> Result<Self, BindingInputRejection<StaticBindingRegistrationInput<B>>> {
        let identity = input.identity;
        let expected = identity.artifact_compatibility();
        let compiler_compatibility = input.compiler.compiler().compatibility();
        let server_compatibility = input.server.artifact_compatibility();
        let layouts_fit = input
            .server
            .route_state_layout()
            .fits_within(input.resources.route_state())
            && input
                .server
                .readiness_state_layout()
                .fits_within(input.resources.readiness_state())
            && input
                .server
                .response_state_layout()
                .fits_within(input.resources.response_state());

        let valid = input.capabilities.supports_producer_property_read()
            && input.execution.supports_application_static()
            && expected == compiler_compatibility
            && expected == server_compatibility
            && input.resources.is_valid()
            && layouts_fit
            && input.ingress.is_valid();
        if !valid {
            let error = registration_error(identity, 300);
            return Err(BindingInputRejection::new(input, error));
        }
        Ok(Self { input })
    }

    /// Returns the immutable registration identity.
    pub const fn identity(&self) -> BindingRegistrationIdentity {
        self.input.identity
    }

    /// Returns advertised capability metadata.
    pub const fn capabilities(&self) -> BindingRegistrationCapabilities {
        self.input.capabilities
    }

    /// Returns execution-representation metadata.
    pub const fn execution(&self) -> BindingExecutionSupport {
        self.input.execution
    }

    /// Borrows the compiler component.
    pub const fn compiler(&self) -> &StaticBindingCompilerRegistration<B::Compiler> {
        &self.input.compiler
    }

    /// Borrows the server component.
    pub const fn server(&self) -> &B {
        &self.input.server
    }

    /// Mutably borrows the server component for manual progress.
    pub fn server_mut(&mut self) -> &mut B {
        &mut self.input.server
    }

    /// Returns the validated resource declarations.
    pub const fn resources(&self) -> BindingResourceDeclarations {
        self.input.resources
    }

    /// Returns the validated ingress policy.
    pub const fn ingress(&self) -> BindingIngressPolicy {
        self.input.ingress
    }

    /// Returns the retained-status policy.
    pub const fn status(&self) -> BindingStatusPolicy {
        self.input.status
    }

    /// Consumes the registration back into the complete validated input.
    pub fn into_input(self) -> StaticBindingRegistrationInput<B> {
        self.input
    }
}

fn registration_error(identity: BindingRegistrationIdentity, code: u16) -> BindingOperationalError {
    BindingOperationalError::new(CoreError::Validation(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
            .with_binding(identity.binding_id(), identity.binding_generation())
            .with_redacted_cause(code, "binding registration declarations are inconsistent"),
    ))
}

// ---------------------------------------------------------------------------
// `std` host erasure and complete registration
// ---------------------------------------------------------------------------

#[cfg(feature = "std")]
struct HostRouteCarrier {
    input: PrepareInput,
    footprint: BindingLifetimeFootprint,
    state: Box<dyn Any + Send>,
}

#[cfg(feature = "std")]
impl HostRouteCarrier {
    fn new<S>(input: PrepareInput, footprint: BindingLifetimeFootprint, state: S) -> Self
    where
        S: Send + 'static,
    {
        Self {
            input,
            footprint,
            state: Box::new(state),
        }
    }

    const fn route(&self) -> &BindingRouteKey {
        self.input.route()
    }

    const fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        self.footprint
    }

    fn try_state_mut<S>(&mut self) -> Option<&mut S>
    where
        S: Send + Unpin + 'static,
    {
        self.state.downcast_mut::<S>()
    }

    fn try_state_pin_mut<S>(&mut self) -> Option<Pin<&mut S>>
    where
        S: Send + 'static,
    {
        let state = self.state.downcast_mut::<S>()?;
        // SAFETY: the concrete state stays in the same heap allocation from
        // carrier construction until carrier drop. Stage conversion moves
        // only this private `Box` handle, and no public API can extract or
        // replace a potentially pinned value.
        Some(unsafe { Pin::new_unchecked(state) })
    }
}

#[cfg(feature = "std")]
/// Prepared route guard with Core-owned safe state erasure.
pub struct HostPreparedRouteGuard {
    carrier: HostRouteCarrier,
}

#[cfg(feature = "std")]
impl HostPreparedRouteGuard {
    /// Creates a prepared guard while preserving the complete preparation input.
    pub fn new<S>(input: PrepareInput, footprint: BindingLifetimeFootprint, state: S) -> Self
    where
        S: Send + 'static,
    {
        Self {
            carrier: HostRouteCarrier::new(input, footprint, state),
        }
    }

    /// Returns the exact route identity.
    pub const fn route(&self) -> &BindingRouteKey {
        self.carrier.route()
    }

    /// Returns the complete retained-footprint declaration.
    pub const fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        self.carrier.lifetime_footprint()
    }

    /// Mutably borrows matching movable state without changing its owner.
    pub fn try_state_mut<S>(&mut self) -> Option<&mut S>
    where
        S: Send + Unpin + 'static,
    {
        self.carrier.try_state_mut::<S>()
    }

    /// Pins and mutably borrows matching erased state without moving it.
    pub fn try_state_pin_mut<S>(self: Pin<&mut Self>) -> Option<Pin<&mut S>>
    where
        S: Send + 'static,
    {
        self.get_mut().carrier.try_state_pin_mut::<S>()
    }
}

#[cfg(feature = "std")]
impl core::fmt::Debug for HostPreparedRouteGuard {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("HostPreparedRouteGuard")
            .field("route", self.route())
            .field("footprint", &self.lifetime_footprint())
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "std")]
/// Activated route guard retained while request admission remains closed.
pub struct HostActiveRouteGuard {
    carrier: HostRouteCarrier,
}

#[cfg(feature = "std")]
impl HostActiveRouteGuard {
    /// Consumes a prepared guard and advances its unchanged carrier.
    ///
    /// A successor cannot accept replacement state:
    ///
    /// ```compile_fail
    /// # use clinkz_wot_core::{HostActiveRouteGuard, HostPreparedRouteGuard};
    /// # fn replace(prepared: HostPreparedRouteGuard) {
    /// let _ = HostActiveRouteGuard::new(prepared, 7_u8);
    /// # }
    /// ```
    pub fn new(prepared: HostPreparedRouteGuard) -> Self {
        Self {
            carrier: prepared.carrier,
        }
    }

    /// Returns the exact route identity.
    pub const fn route(&self) -> &BindingRouteKey {
        self.carrier.route()
    }

    /// Returns the immutable retained-footprint declaration.
    pub const fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        self.carrier.lifetime_footprint()
    }

    /// Mutably borrows matching movable state without changing its owner.
    pub fn try_state_mut<S>(&mut self) -> Option<&mut S>
    where
        S: Send + Unpin + 'static,
    {
        self.carrier.try_state_mut::<S>()
    }

    /// Pins and mutably borrows matching erased state without moving it.
    pub fn try_state_pin_mut<S>(self: Pin<&mut Self>) -> Option<Pin<&mut S>>
    where
        S: Send + 'static,
    {
        self.get_mut().carrier.try_state_pin_mut::<S>()
    }
}

#[cfg(feature = "std")]
impl core::fmt::Debug for HostActiveRouteGuard {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("HostActiveRouteGuard")
            .field("route", self.route())
            .field("footprint", &self.lifetime_footprint())
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "std")]
/// Committed route guard whose request-admission gate remains closed.
pub struct HostCommittedRouteGuard {
    carrier: HostRouteCarrier,
}

#[cfg(feature = "std")]
impl HostCommittedRouteGuard {
    /// Consumes an active guard and advances its unchanged carrier.
    ///
    /// A successor cannot accept replacement state:
    ///
    /// ```compile_fail
    /// # use clinkz_wot_core::{HostActiveRouteGuard, HostCommittedRouteGuard};
    /// # fn replace(active: HostActiveRouteGuard) {
    /// let _ = HostCommittedRouteGuard::new(active, 7_u8);
    /// # }
    /// ```
    pub fn new(active: HostActiveRouteGuard) -> Self {
        Self {
            carrier: active.carrier,
        }
    }

    /// Returns the exact route identity.
    pub const fn route(&self) -> &BindingRouteKey {
        self.carrier.route()
    }

    /// Returns the immutable retained-footprint declaration.
    pub const fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        self.carrier.lifetime_footprint()
    }

    /// Mutably borrows matching movable state without changing its owner.
    pub fn try_state_mut<S>(&mut self) -> Option<&mut S>
    where
        S: Send + Unpin + 'static,
    {
        self.carrier.try_state_mut::<S>()
    }

    /// Pins and mutably borrows matching erased state without moving it.
    pub fn try_state_pin_mut<S>(self: Pin<&mut Self>) -> Option<Pin<&mut S>>
    where
        S: Send + 'static,
    {
        self.get_mut().carrier.try_state_pin_mut::<S>()
    }
}

#[cfg(feature = "std")]
impl core::fmt::Debug for HostCommittedRouteGuard {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("HostCommittedRouteGuard")
            .field("route", self.route())
            .field("footprint", &self.lifetime_footprint())
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "std")]
/// Either legal predecessor guard for explicit host route shutdown.
#[derive(Debug)]
pub enum HostShutdownRouteGuard {
    /// An activated route that did not commit.
    Active(HostActiveRouteGuard),
    /// A committed-closed, serving, or draining route.
    Committed(HostCommittedRouteGuard),
}

#[cfg(feature = "std")]
impl HostShutdownRouteGuard {
    /// Returns the exact route identity in either legal predecessor stage.
    pub const fn route(&self) -> &BindingRouteKey {
        match self {
            Self::Active(guard) => guard.route(),
            Self::Committed(guard) => guard.route(),
        }
    }
}

#[cfg(feature = "std")]
/// Host route-call cleanup successor with every stage represented explicitly.
pub type HostRouteCleanupSuccessor =
    RouteCleanupSuccessor<HostPreparedRouteGuard, HostActiveRouteGuard, HostCommittedRouteGuard>;

#[cfg(feature = "std")]
/// Complete prepared guard plus its independent abort phase.
#[derive(Debug)]
pub struct RouteAbortInput {
    guard: HostPreparedRouteGuard,
    cleanup: CleanupPhaseContext,
}

#[cfg(feature = "std")]
impl RouteAbortInput {
    /// Creates an explicit prepared-route abort input.
    pub const fn new(guard: HostPreparedRouteGuard, cleanup: CleanupPhaseContext) -> Self {
        Self { guard, cleanup }
    }

    /// Consumes and returns the guard and phase exactly once.
    pub fn into_parts(self) -> (HostPreparedRouteGuard, CleanupPhaseContext) {
        (self.guard, self.cleanup)
    }
}

#[cfg(feature = "std")]
/// Complete active/committed guard plus its independent shutdown phase.
#[derive(Debug)]
pub struct RouteShutdownInput {
    guard: HostShutdownRouteGuard,
    cleanup: CleanupPhaseContext,
}

#[cfg(feature = "std")]
impl RouteShutdownInput {
    /// Creates an explicit route shutdown input.
    pub const fn new(guard: HostShutdownRouteGuard, cleanup: CleanupPhaseContext) -> Self {
        Self { guard, cleanup }
    }

    /// Consumes and returns the guard and phase exactly once.
    pub fn into_parts(self) -> (HostShutdownRouteGuard, CleanupPhaseContext) {
        (self.guard, self.cleanup)
    }
}

#[cfg(feature = "std")]
/// One owned, cancellation-aware host binding operation.
pub trait HostBindingCall<T, C = NoCleanupSuccessor>: Send + 'static {
    /// Returns the immutable maximum retained footprint before first poll.
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint;

    /// Polls exactly one terminal result under explicit work budget.
    fn poll_result(self: Pin<&mut Self>, cx: &mut Context<'_>, budget: &mut WorkBudget) -> Poll<T>;

    /// Starts cancellation with one immutable cleanup phase.
    fn start_cancel(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        cleanup: CleanupPhaseContext,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<T, C>>>;

    /// Polls an accepted cancellation to explicit settlement.
    fn poll_cancel(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<T, C>>>;

    /// Returns the next binding-local deadline requiring an independent wake.
    fn next_deadline(&self) -> Option<Deadline>;
}

#[cfg(feature = "std")]
/// Core-erased owned host call retaining its concrete state allocation.
pub struct HostBindingCallBox<T, C = NoCleanupSuccessor>(Pin<Box<dyn HostBindingCall<T, C>>>);

#[cfg(feature = "std")]
impl<T, C> HostBindingCallBox<T, C>
where
    T: 'static,
    C: 'static,
{
    /// Safely erases one complete concrete host call.
    pub fn new<B>(call: B) -> Self
    where
        B: HostBindingCall<T, C>,
    {
        Self(Box::pin(call))
    }

    /// Returns the unique pinned call borrow used for progress.
    pub fn as_pin_mut(&mut self) -> Pin<&mut dyn HostBindingCall<T, C>> {
        self.0.as_mut()
    }
}

#[cfg(feature = "std")]
impl<T, C> core::fmt::Debug for HostBindingCallBox<T, C> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("HostBindingCallBox")
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "std")]
/// Route-scoped host server component without registry or dispatch authority.
pub trait RouteServerBinding: Send + Sync {
    /// Returns the compatibility identity consumed at bundle validation.
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility;

    /// Creates an owned preparation call from one checked, non-retainable
    /// artifact borrow or returns the complete input.
    fn prepare(
        &self,
        input: PrepareInput,
        artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<RoutePrepareOutcome<HostPreparedRouteGuard>, HostRouteCleanupSuccessor>,
        BindingInputRejection<PrepareInput>,
    >;

    /// Creates the unique retained prepared-route readiness call.
    fn start_readiness(
        &self,
        guard: HostPreparedRouteGuard,
    ) -> Result<
        HostBindingCallBox<
            RouteReadinessOutcome<HostPreparedRouteGuard>,
            HostRouteCleanupSuccessor,
        >,
        BindingInputRejection<HostPreparedRouteGuard>,
    >;

    /// Creates an activation call while request admission remains closed.
    fn activate(
        &self,
        guard: HostPreparedRouteGuard,
    ) -> Result<
        HostBindingCallBox<
            RouteActivationOutcome<HostPreparedRouteGuard, HostActiveRouteGuard>,
            HostRouteCleanupSuccessor,
        >,
        BindingInputRejection<HostPreparedRouteGuard>,
    >;

    /// Creates a commit-to-closed call.
    fn commit(
        &self,
        guard: HostActiveRouteGuard,
    ) -> Result<
        HostBindingCallBox<
            RouteCommitOutcome<HostActiveRouteGuard, HostCommittedRouteGuard>,
            HostRouteCleanupSuccessor,
        >,
        BindingInputRejection<HostActiveRouteGuard>,
    >;

    /// Polls one committed route under a fresh exclusive permit.
    fn poll_accept(
        &self,
        route: Pin<&mut HostCommittedRouteGuard>,
        permit: RouteActivationPermit<'_>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<RouteAcceptEvent>>;

    /// Creates an explicit prepared-route abort call.
    fn abort(
        &self,
        input: RouteAbortInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        BindingInputRejection<RouteAbortInput>,
    >;

    /// Creates an explicit active/committed route shutdown call.
    fn shutdown(
        &self,
        input: RouteShutdownInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        BindingInputRejection<RouteShutdownInput>,
    >;

    /// Creates an owned response-delivery call or returns the full response.
    fn deliver_response(
        &self,
        response: RouteInboundResponse,
    ) -> Result<
        HostBindingCallBox<BindingDeliveryOutcome>,
        BindingInputRejection<RouteInboundResponse>,
    >;
}

#[cfg(feature = "std")]
/// Complete recoverable author input for one host registration.
pub struct HostBindingRegistrationInput {
    identity: BindingRegistrationIdentity,
    capabilities: BindingRegistrationCapabilities,
    execution: BindingExecutionSupport,
    compiler: HostBindingCompilerRegistration,
    server: Box<dyn RouteServerBinding>,
    resources: BindingResourceDeclarations,
    ingress: BindingIngressPolicy,
    status: BindingStatusPolicy,
}

#[cfg(feature = "std")]
impl HostBindingRegistrationInput {
    /// Assembles the complete host input without performing protocol work.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: BindingRegistrationIdentity,
        capabilities: BindingRegistrationCapabilities,
        execution: BindingExecutionSupport,
        compiler: HostBindingCompilerRegistration,
        server: Box<dyn RouteServerBinding>,
        resources: BindingResourceDeclarations,
        ingress: BindingIngressPolicy,
        status: BindingStatusPolicy,
    ) -> Self {
        Self {
            identity,
            capabilities,
            execution,
            compiler,
            server,
            resources,
            ingress,
            status,
        }
    }

    /// Returns the immutable registration identity.
    pub const fn identity(&self) -> BindingRegistrationIdentity {
        self.identity
    }

    /// Borrows the host compiler component.
    pub const fn compiler(&self) -> &HostBindingCompilerRegistration {
        &self.compiler
    }

    /// Borrows the route-scoped host server component.
    pub fn server(&self) -> &dyn RouteServerBinding {
        &*self.server
    }
}

#[cfg(feature = "std")]
/// Validated complete host-erased binding bundle.
pub struct HostBindingRegistration {
    input: HostBindingRegistrationInput,
}

#[cfg(feature = "std")]
impl HostBindingRegistration {
    /// Validates compiler/server identity and every narrow declaration.
    pub fn new(
        input: HostBindingRegistrationInput,
    ) -> Result<Self, BindingInputRejection<HostBindingRegistrationInput>> {
        let identity = input.identity;
        let expected = identity.artifact_compatibility();
        let valid = input.capabilities.supports_producer_property_read()
            && input.execution.supports_host_erased()
            && expected == input.compiler.compatibility()
            && expected == input.server.artifact_compatibility()
            && input.resources.is_valid()
            && input.ingress.is_valid();
        if !valid {
            let error = registration_error(identity, 301);
            return Err(BindingInputRejection::new(input, error));
        }
        Ok(Self { input })
    }

    /// Returns the immutable registration identity.
    pub const fn identity(&self) -> BindingRegistrationIdentity {
        self.input.identity
    }

    /// Returns advertised capability metadata.
    pub const fn capabilities(&self) -> BindingRegistrationCapabilities {
        self.input.capabilities
    }

    /// Returns execution-representation metadata.
    pub const fn execution(&self) -> BindingExecutionSupport {
        self.input.execution
    }

    /// Borrows the host compiler component.
    pub const fn compiler(&self) -> &HostBindingCompilerRegistration {
        &self.input.compiler
    }

    /// Borrows the route-scoped host server component.
    pub fn server(&self) -> &dyn RouteServerBinding {
        &*self.input.server
    }

    /// Returns validated retained-resource declarations.
    pub const fn resources(&self) -> BindingResourceDeclarations {
        self.input.resources
    }

    /// Returns the validated ingress policy.
    pub const fn ingress(&self) -> BindingIngressPolicy {
        self.input.ingress
    }

    /// Returns the retained-status policy.
    pub const fn status(&self) -> BindingStatusPolicy {
        self.input.status
    }

    /// Consumes the bundle back into its complete validated input.
    pub fn into_input(self) -> HostBindingRegistrationInput {
        self.input
    }
}
