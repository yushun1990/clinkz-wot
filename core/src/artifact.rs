//! Source-free native artifacts, compatibility and owned runtime storage.
use crate::*;
use alloc::boxed::Box;
use clinkz_wot_foundation::SlotIndex;
use core::alloc::Layout;
#[cfg(feature = "std")]
use core::any::Any;
#[cfg(feature = "std")]
use core::fmt;
/// Stable compatibility identity shared by one compiler and its artifacts.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct BindingArtifactCompatibility([u8; 16]);

impl BindingArtifactCompatibility {
    /// Creates an identity from its complete fixed-width representation.
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the complete fixed-width representation.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// Execution role for which an immutable artifact was compiled.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BindingArtifactRole {
    /// Consumer request/call preparation.
    ConsumerCall,
    /// Consumer subscription preparation.
    ConsumerSubscription,
    /// Producer inbound route preparation.
    ProducerRoute,
    /// Producer publication preparation.
    ProducerPublication,
}

/// Measured retained lifetime footprint of one compiled artifact.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BindingArtifactFootprint {
    retained_items: u32,
    retained_bytes: u64,
}

impl BindingArtifactFootprint {
    /// Creates an exact measured or admitted footprint.
    pub const fn new(retained_items: u32, retained_bytes: u64) -> Self {
        Self {
            retained_items,
            retained_bytes,
        }
    }

    /// Returns the retained item count.
    pub const fn retained_items(self) -> u32 {
        self.retained_items
    }

    /// Returns the retained byte count.
    pub const fn retained_bytes(self) -> u64 {
        self.retained_bytes
    }

    /// Returns whether this measured footprint fits the admitted ceiling.
    pub const fn fits_within(self, admitted: Self) -> bool {
        self.retained_items <= admitted.retained_items
            && self.retained_bytes <= admitted.retained_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingArtifactIdentity {
    plan_set_generation: PlanSetGeneration,
    plan_id: PlanId,
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    configuration: BindingConfigurationDigest,
    compatibility: BindingArtifactCompatibility,
    role: BindingArtifactRole,
}

impl BindingArtifactIdentity {
    /// Creates the complete immutable artifact identity.
    pub const fn new(
        plan_set_generation: PlanSetGeneration,
        plan_id: PlanId,
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        configuration: BindingConfigurationDigest,
        compatibility: BindingArtifactCompatibility,
        role: BindingArtifactRole,
    ) -> Self {
        Self {
            plan_set_generation,
            plan_id,
            binding_id,
            binding_generation,
            configuration,
            compatibility,
            role,
        }
    }

    /// Returns the plan-set generation.
    pub const fn plan_set_generation(&self) -> PlanSetGeneration {
        self.plan_set_generation
    }

    /// Returns the logical-plan identity.
    pub const fn plan_id(&self) -> PlanId {
        self.plan_id
    }

    /// Returns the binding identity.
    pub const fn binding_id(&self) -> BindingId {
        self.binding_id
    }

    /// Returns the binding generation.
    pub const fn binding_generation(&self) -> BindingGeneration {
        self.binding_generation
    }

    /// Returns the captured binding configuration digest.
    pub const fn configuration(&self) -> BindingConfigurationDigest {
        self.configuration
    }

    /// Returns the compiler/artifact compatibility identity.
    pub const fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    /// Returns the artifact's execution role.
    pub const fn role(&self) -> BindingArtifactRole {
        self.role
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingArtifact<A> {
    compatibility: BindingArtifactCompatibility,
    footprint: BindingArtifactFootprint,
    route_reservation: Option<RouteReservationIdentity>,
    payload: A,
}

impl<A> BindingArtifact<A> {
    /// Creates a measured typed artifact.
    pub const fn new(
        compatibility: BindingArtifactCompatibility,
        footprint: BindingArtifactFootprint,
        payload: A,
    ) -> Self {
        Self {
            compatibility,
            footprint,
            route_reservation: None,
            payload,
        }
    }

    /// Creates a measured Producer-route artifact with its canonical endpoint identity.
    pub const fn producer_route(
        compatibility: BindingArtifactCompatibility,
        footprint: BindingArtifactFootprint,
        reservation: RouteReservationIdentity,
        payload: A,
    ) -> Self {
        Self {
            compatibility,
            footprint,
            route_reservation: Some(reservation),
            payload,
        }
    }

    /// Returns the payload compatibility identity.
    pub const fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    /// Returns the measured retained footprint.
    pub const fn footprint(&self) -> BindingArtifactFootprint {
        self.footprint
    }

    /// Returns the canonical endpoint identity carried by a Producer-route artifact.
    pub const fn route_reservation(&self) -> Option<RouteReservationIdentity> {
        self.route_reservation
    }

    /// Borrows the typed payload.
    pub const fn payload(&self) -> &A {
        &self.payload
    }

    /// Consumes the wrapper and returns the typed payload.
    pub fn into_payload(self) -> A {
        self.payload
    }

    /// Consumes the wrapper and returns every captured part.
    pub fn into_parts(self) -> (BindingArtifactCompatibility, BindingArtifactFootprint, A) {
        (self.compatibility, self.footprint, self.payload)
    }

    /// Consumes the wrapper and returns every part, including route metadata.
    pub fn into_route_parts(
        self,
    ) -> (
        BindingArtifactCompatibility,
        BindingArtifactFootprint,
        Option<RouteReservationIdentity>,
        A,
    ) {
        (
            self.compatibility,
            self.footprint,
            self.route_reservation,
            self.payload,
        )
    }
}

/// Immutable native format, policy identity and bounds for a Host Consumer.
#[cfg(feature = "std")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsumerBindingConfiguration {
    target_capacity: usize,
    compatibility: BindingArtifactCompatibility,
    private_policy: BindingConfigurationDigest,
    result: (u64, u64),
    key: BindingConfigurationDigest,
}

#[cfg(feature = "std")]
impl ConsumerBindingConfiguration {
    /// Identifies the actual native format, private policy and checked bounds.
    /// This identity grants no live execution permission.
    pub fn for_semantic_host(
        target_capacity: usize,
        private_policy: BindingConfigurationDigest,
        compatibility: BindingArtifactCompatibility,
        body_bytes: u64,
        media_capacity: u64,
    ) -> CoreResult<Self> {
        if target_capacity == 0
            || crate::response::consumer_result_backing(body_bytes, media_capacity).is_none()
        {
            return Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Admission,
                RetryClass::Never,
            )));
        }
        let key = BindingConfigurationDigest::new(
            sha2_const_stable::Sha256::new()
                .update(b"clinkz-native-consumer-configuration-v1\0")
                .update(compatibility.as_bytes())
                .update(private_policy.as_bytes())
                .update(&(target_capacity as u64).to_le_bytes())
                .update(&body_bytes.to_le_bytes())
                .update(&media_capacity.to_le_bytes())
                .finalize(),
        );
        Ok(Self {
            target_capacity,
            compatibility,
            private_policy,
            result: (body_bytes, media_capacity),
            key,
        })
    }

    pub const fn target_capacity(&self) -> usize {
        self.target_capacity
    }
    pub const fn native_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }
    pub const fn private_policy(&self) -> BindingConfigurationDigest {
        self.private_policy
    }
    /// Maximum body bytes and capacity of each media String.
    pub const fn result_limits(&self) -> (u64, u64) {
        self.result
    }
    pub const fn configuration(&self) -> BindingConfigurationDigest {
        self.key
    }
}

/// Reason a measured artifact could not enter an immutable plan set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingArtifactRejectionReason {
    /// Compiler and artifact compatibility identities differ.
    CompatibilityMismatch,
    /// Measured retained items or bytes exceed the admitted bound.
    FootprintExceeded,
    /// A Producer-route artifact omitted its canonical endpoint identity.
    MissingRouteReservation,
    /// A non-Producer-route artifact supplied Producer-only endpoint metadata.
    UnexpectedRouteReservation,
}

/// Artifact-admission failure that returns the original typed artifact.
#[derive(Debug, Eq, PartialEq)]
pub struct BindingArtifactRejection<A> {
    reason: BindingArtifactRejectionReason,
    artifact: BindingArtifact<A>,
}

impl<A> BindingArtifactRejection<A> {
    /// Returns the exact rejection class.
    pub const fn reason(&self) -> BindingArtifactRejectionReason {
        self.reason
    }

    /// Borrows the rejected typed artifact.
    pub const fn artifact(&self) -> &BindingArtifact<A> {
        &self.artifact
    }

    /// Consumes the rejection and returns the original artifact.
    pub fn into_artifact(self) -> BindingArtifact<A> {
        self.artifact
    }
}

/// Admitted immutable artifact with complete identity and measured bounds.
#[derive(Debug, Eq, PartialEq)]
pub struct BindingArtifactEnvelope<A> {
    identity: BindingArtifactIdentity,
    admitted: BindingArtifactFootprint,
    artifact: BindingArtifact<A>,
}

impl<A> BindingArtifactEnvelope<A> {
    /// Validates compatibility and retained footprint without losing ownership.
    pub fn try_new(
        identity: BindingArtifactIdentity,
        admitted: BindingArtifactFootprint,
        artifact: BindingArtifact<A>,
    ) -> Result<Self, BindingArtifactRejection<A>> {
        if identity.compatibility() != artifact.compatibility() {
            return Err(BindingArtifactRejection {
                reason: BindingArtifactRejectionReason::CompatibilityMismatch,
                artifact,
            });
        }
        if !artifact.footprint().fits_within(admitted) {
            return Err(BindingArtifactRejection {
                reason: BindingArtifactRejectionReason::FootprintExceeded,
                artifact,
            });
        }
        match (identity.role(), artifact.route_reservation()) {
            (BindingArtifactRole::ProducerRoute, None) => {
                return Err(BindingArtifactRejection {
                    reason: BindingArtifactRejectionReason::MissingRouteReservation,
                    artifact,
                });
            }
            (BindingArtifactRole::ProducerRoute, Some(_)) | (_, None) => {}
            (_, Some(_)) => {
                return Err(BindingArtifactRejection {
                    reason: BindingArtifactRejectionReason::UnexpectedRouteReservation,
                    artifact,
                });
            }
        }
        Ok(Self {
            identity,
            admitted,
            artifact,
        })
    }

    /// Returns the complete generation-qualified identity.
    pub const fn identity(&self) -> BindingArtifactIdentity {
        self.identity
    }

    /// Returns the admitted retained-footprint ceiling.
    pub const fn admitted(&self) -> BindingArtifactFootprint {
        self.admitted
    }

    /// Borrows the admitted typed artifact.
    pub const fn artifact(&self) -> &BindingArtifact<A> {
        &self.artifact
    }

    /// Returns the admitted canonical endpoint identity for a Producer route.
    pub const fn route_reservation(&self) -> Option<RouteReservationIdentity> {
        self.artifact.route_reservation()
    }

    /// Consumes the envelope and returns the typed artifact.
    pub fn into_artifact(self) -> BindingArtifact<A> {
        self.artifact
    }
}

/// Compact reference to one immutable artifact slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BindingArtifactRef {
    identity: BindingArtifactIdentity,
    artifact_slot: SlotIndex,
}

impl BindingArtifactRef {
    /// Creates a generation-qualified artifact reference.
    pub const fn new(identity: BindingArtifactIdentity, artifact_slot: SlotIndex) -> Self {
        Self {
            identity,
            artifact_slot,
        }
    }

    /// Returns the complete referenced artifact identity.
    pub const fn identity(&self) -> BindingArtifactIdentity {
        self.identity
    }

    /// Returns the plan-set-local artifact slot.
    pub const fn artifact_slot(&self) -> SlotIndex {
        self.artifact_slot
    }
}

#[cfg(feature = "std")]
pub struct HostBindingArtifact(Box<dyn HostArtifactStorage>);
#[cfg(feature = "std")]
/// Completed artifact storage; it contains no preparation interface.
pub trait HostArtifactStorage: Send + Sync {
    fn storage_overhead(&self) -> u64;
    fn payload(&self) -> &dyn Any;
    fn take_payload(&mut self, destination: &mut dyn Any);
}
#[cfg(feature = "std")]
impl HostBindingArtifact {
    /// Takes completed storage without another allocation.
    pub fn from_storage(storage: Box<dyn HostArtifactStorage>) -> Self {
        Self(storage)
    }

    /// Engine-owned slot bytes beyond the native inline payload.
    /// Add these to the native footprint when charging retained output; outer
    /// owners and native private backing remain separate physical costs.
    pub fn storage_overhead(&self) -> u64 {
        self.0.storage_overhead()
    }

    fn payload(&self) -> &dyn Any {
        self.0.payload()
    }

    pub fn into_payload<T: Send + Sync + 'static>(mut self) -> Result<T, Self> {
        if !self.payload().is::<T>() {
            return Err(self);
        }
        let mut payload: Option<T> = None;
        self.0.take_payload(&mut payload);
        Ok(payload.expect("completed slot supplied its payload"))
    }
}

/// Fallible owned storage for an alloc-backed representation. No native work
/// is acquired and allocator refusal returns the complete original value.
pub fn try_box<T>(value: T) -> Result<Box<T>, T> {
    let layout = Layout::new::<T>();
    if layout.size() == 0 {
        return Ok(Box::new(value));
    }
    // SAFETY: use the exact nonzero Layout for T and check null before writing.
    // A successful pointer is initialized once and transferred to Box, whose
    // destructor uses the same global allocator and Layout. Failure returns T.
    let pointer = unsafe { alloc::alloc::alloc(layout) }.cast::<T>();
    if pointer.is_null() {
        return Err(value);
    }
    unsafe {
        pointer.write(value);
        Ok(Box::from_raw(pointer))
    }
}

#[cfg(feature = "std")]
impl fmt::Debug for HostBindingArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostBindingArtifact")
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "std")]
impl BindingArtifact<HostBindingArtifact> {
    /// Borrows a matching concrete payload after compatibility/type checks.
    pub fn try_payload<T>(&self, expected: BindingArtifactCompatibility) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        if self.compatibility() != expected {
            return None;
        }
        self.payload.payload().downcast_ref::<T>()
    }

    /// Consumes and extracts a matching concrete payload.
    ///
    /// Either mismatch returns the original erased artifact unchanged.
    pub fn try_into_payload<T>(self, expected: BindingArtifactCompatibility) -> Result<T, Self>
    where
        T: Send + Sync + 'static,
    {
        if self.compatibility != expected {
            return Err(self);
        }
        let Self {
            compatibility,
            footprint,
            route_reservation,
            payload,
        } = self;
        match payload.into_payload::<T>() {
            Ok(payload) => Ok(payload),
            Err(payload) => Err(Self {
                compatibility,
                footprint,
                route_reservation,
                payload,
            }),
        }
    }
}
