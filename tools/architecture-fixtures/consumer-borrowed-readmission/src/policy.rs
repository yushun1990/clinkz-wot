//! Operation-qualified projection; caller and wire storage are not TD charges.
use clinkz_wot_foundation::WorkClass as W;
use clinkz_wot_foundation::{ResourceKind as R, ResourceLimits};
use core::{alloc::Layout, mem};

pub const RESOURCE_INTERPRETATION_REVISION: u32 = 2;
pub const OPERATION: &str = "typed-content-v1";
pub const CATALOG: &[R] = &[
    R::DocumentBytesMax,
    R::StringBytesMax,
    R::ExtensionBytesMax,
    R::GeneratedEffectiveDocumentBytesMax,
    R::NumberLexemeBytesMax,
    R::JsonNestingDepthMax,
    R::JsonMembersPerObjectMax,
    R::JsonArrayItemsMax,
    R::JsonValueNodesPerDocumentMax,
    R::AffordancesPerThingMax,
    R::FormsPerContextMax,
    R::FormsPerThingMax,
    R::AdditionalResponsesPerFormMax,
    R::UriVariablesPerFormMax,
    R::SchemaNodesPerDocumentMax,
    R::SchemaCompositionDepthMax,
    R::SchemaReferenceEdgesPerDocumentMax,
    R::SecurityExpressionDepthMax,
    R::SecurityBranchesPerPlanMax,
    R::UriTemplateSourceBytesMax,
    R::DocumentValidationWorkUnitsMax,
    R::AdmissionTemporaryBytesPerOperationMax,
    R::AdmissionTemporaryBytesGlobalMax,
    R::PeakLiveBytesPerAdmissionMax,
    R::AdmissionPeakLiveBytesGlobalMax,
    R::EngineLiveBytesGlobalMax,
    R::LargestContiguousAllocationBytesMax,
];
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingConfigErrorKind {
    MissingAdmissionLimit,
    UnsupportedLimit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedThingConfigError {
    kind: ValidatedThingConfigErrorKind,
    resource: R,
    configured: Option<u64>,
    supported: Option<u64>,
}
impl ValidatedThingConfigError {
    pub fn kind(&self) -> ValidatedThingConfigErrorKind {
        self.kind
    }
    pub fn resource_kind(&self) -> R {
        self.resource
    }
    pub fn configured(&self) -> Option<u64> {
        self.configured
    }
    pub fn supported_max(&self) -> Option<u64> {
        self.supported
    }
}
/// Opaque checked values. This is copyable policy, never a lifetime allowance.
pub struct ValidatedThingAdmissionConfig {
    pub(super) policy: Policy,
}
#[derive(Clone, Copy)]
pub(super) struct Policy {
    pub limits: [u64; CATALOG.len()],
    pub frames: usize,
}
impl Policy {
    pub fn get(self, r: R) -> u64 {
        self.limits[CATALOG.iter().position(|&kind| kind == r).unwrap()]
    }
}
impl ValidatedThingAdmissionConfig {
    pub fn try_from_limits(limits: &ResourceLimits) -> Result<Self, ValidatedThingConfigError> {
        let mut values = [0; CATALOG.len()];
        for (n, &resource) in CATALOG.iter().enumerate() {
            values[n] = limits.get(resource).ok_or(ValidatedThingConfigError {
                kind: ValidatedThingConfigErrorKind::MissingAdmissionLimit,
                resource,
                configured: None,
                supported: None,
            })?;
        }
        let policy = Policy {
            limits: values,
            frames: 0,
        };
        let unsupported = |resource: R, max| ValidatedThingConfigError {
            kind: ValidatedThingConfigErrorKind::UnsupportedLimit,
            resource,
            configured: limits.get(resource),
            supported: Some(max),
        };
        // All position/capacity arithmetic is representable on the selected target.
        for &resource in CATALOG {
            if policy.get(resource) > isize::MAX as u64 {
                return Err(unsupported(resource, isize::MAX as u64));
            }
        }
        let work = policy.get(R::DocumentValidationWorkUnitsMax);
        let number = policy.get(R::NumberLexemeBytesMax);
        if number > work.saturating_sub(512) {
            return Err(unsupported(
                R::NumberLexemeBytesMax,
                work.saturating_sub(512),
            ));
        }
        for resource in [
            R::AffordancesPerThingMax,
            R::FormsPerContextMax,
            R::FormsPerThingMax,
        ] {
            if policy.get(resource) > u32::MAX as u64 + 1 {
                return Err(unsupported(resource, u32::MAX as u64 + 1));
            }
        }
        for resource in [R::JsonMembersPerObjectMax, R::JsonArrayItemsMax] {
            if policy.get(resource).checked_add(1).is_none_or(|n| n > work) {
                return Err(unsupported(resource, work.saturating_sub(1)));
            }
        }
        let initialization = policy
            .get(R::JsonMembersPerObjectMax)
            .checked_add(policy.get(R::AffordancesPerThingMax))
            .and_then(|n| n.checked_add(4))
            .ok_or_else(|| unsupported(R::AffordancesPerThingMax, work.saturating_sub(4)))?;
        if initialization > work {
            return Err(unsupported(
                R::AffordancesPerThingMax,
                work.saturating_sub(policy.get(R::JsonMembersPerObjectMax))
                    .saturating_sub(4),
            ));
        }
        if policy
            .get(R::JsonMembersPerObjectMax)
            .checked_add(1)
            .and_then(|n| n.checked_mul(2))
            .is_none_or(|n| n > work)
        {
            return Err(unsupported(
                R::JsonMembersPerObjectMax,
                work.saturating_div(2).saturating_sub(1),
            ));
        }
        if policy
            .get(R::JsonArrayItemsMax)
            .checked_add(1)
            .and_then(|n| n.checked_mul(2))
            .is_none_or(|n| n > work)
        {
            return Err(unsupported(
                R::JsonArrayItemsMax,
                work.saturating_div(2).saturating_sub(1),
            ));
        }
        let field_atomic = policy
            .get(R::JsonMembersPerObjectMax)
            .max(policy.get(R::JsonArrayItemsMax))
            .checked_add(1)
            .and_then(|n| n.checked_add(512))
            .and_then(|n| n.checked_add(number))
            // A security-extra recognition action also visits one branch.
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| unsupported(R::NumberLexemeBytesMax, work.saturating_sub(512)))?;
        if field_atomic > work {
            return Err(ValidatedThingConfigError {
                kind: ValidatedThingConfigErrorKind::UnsupportedLimit,
                resource: R::DocumentValidationWorkUnitsMax,
                configured: Some(work),
                supported: None,
            });
        }
        let depth = policy.get(R::JsonNestingDepthMax);
        let schema_depth = policy.get(R::SchemaCompositionDepthMax);
        let depth_kind = if schema_depth > depth {
            R::SchemaCompositionDepthMax
        } else {
            R::JsonNestingDepthMax
        };
        let max_depth = (isize::MAX as u64 / mem::size_of::<super::Job<'static>>() as u64)
            .saturating_sub(16)
            / 2;
        let frames = depth
            .max(schema_depth)
            .checked_mul(2)
            .and_then(|v| v.checked_add(16))
            .and_then(|v| usize::try_from(v).ok())
            .ok_or_else(|| unsupported(depth_kind, max_depth))?;
        Layout::array::<super::Job<'static>>(frames)
            .map_err(|_| unsupported(depth_kind, max_depth))?;
        Layout::array::<u8>(policy.get(R::UriTemplateSourceBytesMax) as usize)
            .map_err(|_| unsupported(R::UriTemplateSourceBytesMax, isize::MAX as u64))?;
        Ok(Self {
            policy: Policy { frames, ..policy },
        })
    }
    pub fn resource_interpretation_revision(&self) -> u32 {
        RESOURCE_INTERPRETATION_REVISION
    }
    pub fn operation(&self) -> &'static str {
        OPERATION
    }
    pub fn largest_atomic_debits(&self) -> [u64; 12] {
        let mut costs = [0; 12];
        costs[W::CodecInputBytes as usize] = 512 + self.policy.get(R::NumberLexemeBytesMax);
        costs[W::DocumentNodes as usize] = (self
            .policy
            .get(R::JsonMembersPerObjectMax)
            .max(self.policy.get(R::JsonArrayItemsMax))
            + 1)
        .max(
            self.policy.get(R::JsonMembersPerObjectMax)
                + self.policy.get(R::AffordancesPerThingMax)
                + 4,
        )
        .max(2 * (self.policy.get(R::JsonMembersPerObjectMax) + 1));
        costs[W::JsonSchemaNodes as usize] = 2
            * (self
                .policy
                .get(R::JsonMembersPerObjectMax)
                .max(self.policy.get(R::JsonArrayItemsMax))
                + 1);
        costs[W::SecurityBranches as usize] = self.policy.get(R::JsonMembersPerObjectMax) + 1;
        costs[W::UriBytes as usize] = 4;
        costs[W::CodecOutputBytes as usize] = 2;
        costs[W::CleanupItems as usize] = 1;
        costs
    }
}
