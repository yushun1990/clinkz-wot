//! Source-free native artifacts, compatibility and owned runtime storage.
use crate::*;
#[cfg(feature = "std")]
use alloc::boxed::Box;
use clinkz_wot_foundation::SlotIndex;
#[cfg(feature = "std")]
use core::any::Any;
use core::{alloc::Layout, fmt};
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

/// Checked immutable eligibility of the closed Consumer compiler and client.
///
/// This metadata identifies configuration, not program content or a live lease.
/// The private-policy key must describe the captured client's actual policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsumerBindingConfiguration {
    contract: BindingResourceContract,
    capacity: u8,
    cell: u8,
    default_json_forms: bool,
    #[cfg(feature = "std")]
    native: Option<BindingArtifactCompatibility>,
    #[cfg(feature = "std")]
    native_capacity: usize,
    private_policy: BindingConfigurationDigest,
    result: (u64, u64),
    key: BindingConfigurationDigest,
}

impl ConsumerBindingConfiguration {
    /// Checks the current compilation cell, capacity, and complete result backing.
    /// Derivation performs fixed, allocation-free work without client callbacks.
    pub fn try_new(
        contract: BindingResourceContract,
        target_capacity: usize,
        private_policy: BindingConfigurationDigest,
        body_bytes: u64,
        media_capacity: u64,
    ) -> CoreResult<Self> {
        let error = || {
            CoreError::Validation(
                ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
                    .with_redacted_cause(326, "unsupported Consumer binding configuration"),
            )
        };
        if contract == BindingResourceContract::StrictV1
            || (contract == BindingResourceContract::ConsumerOwnedHostV2 && !cfg!(feature = "std"))
            || !(1..=RESOLVED_TARGET_CAPACITY).contains(&target_capacity)
            || crate::response::consumer_result_backing(body_bytes, media_capacity).is_none()
        {
            return Err(error());
        }
        let cell = if cfg!(feature = "std") {
            2
        } else if cfg!(feature = "async") {
            1
        } else {
            0
        };
        let tag = match contract {
            BindingResourceContract::ConsumerOwnedV2 => 1,
            BindingResourceContract::ConsumerOwnedHostV2 => 2,
            BindingResourceContract::StrictV1 => unreachable!(),
        };
        let key = BindingConfigurationDigest::new(
            sha2_const_stable::Sha256::new()
                .update(b"clinkz-consumer-binding-configuration-v1\0")
                .update(&[tag, cell, target_capacity as u8])
                .update(RESOLVED_TARGET_COMPATIBILITY.as_bytes())
                .update(private_policy.as_bytes())
                .update(&body_bytes.to_le_bytes())
                .update(&media_capacity.to_le_bytes())
                .finalize(),
        );
        // Exclude the entire strict namespace even in the event of collision.
        if (1..=64).any(|capacity| key == strict_configuration(capacity)) {
            return Err(error());
        }
        Ok(Self {
            contract,
            capacity: target_capacity as u8,
            cell,
            default_json_forms: false,
            #[cfg(feature = "std")]
            native: None,
            #[cfg(feature = "std")]
            native_capacity: 0,
            private_policy,
            result: (body_bytes, media_capacity),
            key,
        })
    }
    /// Returns the explicitly selected resource and disposal contract.
    pub const fn resource_contract(&self) -> BindingResourceContract {
        self.contract
    }
    /// Returns the supported target capacity.
    pub const fn target_capacity(&self) -> usize {
        #[cfg(feature = "std")]
        if self.native.is_some() {
            return self.native_capacity;
        }
        self.capacity as usize
    }
    /// Returns the singleton compilation cell captured by this configuration.
    pub const fn compilation_cell(&self) -> &'static str {
        match self.cell {
            2 => "std",
            1 => "async-no-std",
            _ => "no-default",
        }
    }
    /// Returns the actual client's private-policy identity.
    pub const fn private_policy(&self) -> BindingConfigurationDigest {
        self.private_policy
    }
    /// Returns maximum body bytes and capacity of each media String.
    pub const fn result_limits(&self) -> (u64, u64) {
        self.result
    }
    /// Returns the derived eligibility key, shared by compiler and artifacts.
    pub const fn configuration(&self) -> BindingConfigurationDigest {
        self.key
    }

    /// Native Host target capacity is Binding policy, independent of the old
    /// inline copy helper's ceiling. Result backing remains engine-checked.
    #[cfg(feature = "std")]
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
            contract: BindingResourceContract::ConsumerOwnedHostV2,
            capacity: 0,
            cell: 2,
            default_json_forms: false,
            native: Some(compatibility),
            native_capacity: target_capacity,
            private_policy,
            result: (body_bytes, media_capacity),
            key,
        })
    }

    /// Restricts the closed compiler to known metadata-free default JSON Forms.
    /// The derived key preserves every original configuration operand. Repeated
    /// application is idempotent and performs no allocation or client callback.
    pub fn with_default_json_forms(mut self) -> CoreResult<Self> {
        #[cfg(feature = "std")]
        if self.native.is_some() {
            return Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Admission,
                RetryClass::Never,
            )));
        }
        if self.default_json_forms {
            return Ok(self);
        }
        let key = BindingConfigurationDigest::new(
            sha2_const_stable::Sha256::new()
                .update(b"clinkz-consumer-default-json-forms-v1\0")
                .update(self.key.as_bytes())
                .finalize(),
        );
        if key == self.key || (1..=64).any(|capacity| key == strict_configuration(capacity)) {
            return Err(CoreError::Validation(
                ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
                    .with_redacted_cause(327, "Consumer Form restriction identity collision"),
            ));
        }
        self.key = key;
        self.default_json_forms = true;
        Ok(self)
    }

    /// Whether the client and compiler require paid default JSON Form facts.
    pub const fn requires_default_json_forms(&self) -> bool {
        self.default_json_forms
    }
}

pub const fn strict_configuration(capacity: u8) -> BindingConfigurationDigest {
    let mut bytes = [0; 32];
    bytes[0] = 0x63;
    bytes[1] = 0x74;
    bytes[2] = 1;
    bytes[3] = capacity;
    BindingConfigurationDigest::new(bytes)
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

#[cfg(feature = "std")]
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

const RESOLVED_TARGET_CAPACITY: usize = 64;

const RESOLVED_TARGET_COMPATIBILITY: BindingArtifactCompatibility =
    BindingArtifactCompatibility::new(*b"clinkz-target-v1");

impl ConsumerBindingConfiguration {
    /// Native format selected by this immutable policy, if any.
    pub const fn native_compatibility(&self) -> Option<BindingArtifactCompatibility> {
        #[cfg(feature = "std")]
        {
            return self.native;
        }
        #[cfg(not(feature = "std"))]
        {
            None
        }
    }
}
