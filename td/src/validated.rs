//! Bounded admission of a caller-owned, immutable typed Thing.
//!
//! Inspection and Basic share one move-only work/account owner. The proof is
//! deliberately opaque: semantic lending consumes this owner, never
//! reconstructing a proof from a raw Thing.
use crate::validate::{basic_kernel::BasicAccess, schema_kernel::SchemaAccess};
use crate::{
    data_schema::DataSchema,
    form::Form,
    security_scheme::SecurityScheme,
    thing::Thing,
    validate::{
        basic_kernel as b, basic_typed as bt,
        schema_access::{Children as SchemaChildren, TypedAccess},
        schema_kernel as s,
    },
};
use alloc::{
    alloc::{alloc, dealloc},
    collections::{BTreeMap, btree_map},
    string::String,
};
use clinkz_wot_foundation::{
    AdmissionLedger, ResourceKind as R, ResourceLimits, WorkBudget, WorkClass as W,
};
use core::{alloc::Layout, mem, ops::ControlFlow, ptr};
use serde_json::{Number, Value};

/// The current admission phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingPhase {
    Inspect,
    Basic,
    Semantics,
    Rollback,
}
/// Basic rejection category, independent of resource rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingInvalidKind {
    MissingRequiredField,
    InvalidOperation,
    InvalidSchema,
    InvalidSecurity,
    InvalidUri,
    InvalidReference,
    InvalidContext,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rule {
    Missing,
    Scheme,
    ComboMissing,
    ComboCardinality,
    ComboEmpty,
    Flow,
    Undefined,
    Uri,
    Operation(crate::data_type::Operation),
    Schema(s::Rule),
}
/// Fixed diagnostic at an original shared-program coordinate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedThingInvalid {
    kind: ValidatedThingInvalidKind,
    phase: ValidatedThingPhase,
    node_ordinal: u64,
    site: b::Site,
    rule: Rule,
}
impl ValidatedThingInvalid {
    pub fn kind(&self) -> ValidatedThingInvalidKind {
        self.kind
    }
    pub fn phase(&self) -> ValidatedThingPhase {
        self.phase
    }
    pub fn node_ordinal(&self) -> u64 {
        self.node_ordinal
    }
}
/// A configured resource ceiling exceeded by this input or build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedThingLimit {
    kind: R,
    configured: u64,
    observed: u64,
    phase: ValidatedThingPhase,
}
impl ValidatedThingLimit {
    pub fn kind(&self) -> R {
        self.kind
    }
    pub fn configured(&self) -> u64 {
        self.configured
    }
    pub fn observed(&self) -> u64 {
        self.observed
    }
    pub fn phase(&self) -> ValidatedThingPhase {
        self.phase
    }
}
/// Failure of checked arithmetic or an actual allocation request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingFailureKind {
    CheckedArithmetic,
    AllocationFailed,
}
/// Fixed failure data; no error strings are allocated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedThingFailure {
    kind: ValidatedThingFailureKind,
    phase: ValidatedThingPhase,
    requested_bytes: u64,
}
impl ValidatedThingFailure {
    pub fn kind(&self) -> ValidatedThingFailureKind {
        self.kind
    }
    pub fn phase(&self) -> ValidatedThingPhase {
        self.phase
    }
    pub fn requested_bytes(&self) -> u64 {
        self.requested_bytes
    }
}
/// The original terminal admission cause.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingCause {
    Invalid(ValidatedThingInvalid),
    Limit(ValidatedThingLimit),
    Cancelled { phase: ValidatedThingPhase },
    Failed(ValidatedThingFailure),
}
type Cause = ValidatedThingCause;
type Phase = ValidatedThingPhase;
fn arithmetic(phase: Phase) -> Cause {
    Cause::Failed(ValidatedThingFailure {
        kind: ValidatedThingFailureKind::CheckedArithmetic,
        phase,
        requested_bytes: 0,
    })
}
fn add(a: u64, b: u64, phase: Phase) -> Result<u64, Cause> {
    a.checked_add(b).ok_or(arithmetic(phase))
}
fn invalid(site: b::Site, ordinal: u64, rule: b::Rule<'_>) -> Cause {
    use ValidatedThingInvalidKind as K;
    let (kind, rule) = match rule {
        b::Rule::Missing => (K::MissingRequiredField, Rule::Missing),
        b::Rule::UnsupportedScheme(_) => (K::InvalidSecurity, Rule::Scheme),
        b::Rule::ComboMissing => (K::InvalidSecurity, Rule::ComboMissing),
        b::Rule::ComboCardinality => (K::InvalidSecurity, Rule::ComboCardinality),
        b::Rule::ComboEmpty => (K::InvalidSecurity, Rule::ComboEmpty),
        b::Rule::UnsupportedFlow(_) => (K::InvalidSecurity, Rule::Flow),
        b::Rule::Undefined(_) => (K::InvalidReference, Rule::Undefined),
        b::Rule::Operation(op) => (K::InvalidOperation, Rule::Operation(op)),
    };
    Cause::Invalid(ValidatedThingInvalid {
        kind,
        phase: Phase::Basic,
        node_ordinal: ordinal,
        site,
        rule,
    })
}
fn schema_invalid(site: b::Site, ordinal: u64, rule: s::Rule) -> Cause {
    Cause::Invalid(ValidatedThingInvalid {
        kind: ValidatedThingInvalidKind::InvalidSchema,
        phase: Phase::Basic,
        node_ordinal: ordinal,
        site,
        rule: Rule::Schema(rule),
    })
}

const CATALOG: [R; 27] = [
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
/// Configuration is checked before a progress owner, input scan or allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedThingConfigErrorKind {
    MissingAdmissionLimit,
    UnsupportedLimit,
}
/// Constant-size operation-qualified configuration error.
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
/// Checked typed-content-v1 (resource interpretation revision 2) projection.
/// This policy can be reused; it is never a work allowance or input proof.
///
/// ```compile_fail
/// use clinkz_wot_td::ValidatedThingAdmissionConfig;
/// let forged = ValidatedThingAdmissionConfig { policy: unsafe { core::mem::zeroed() } };
/// ```
pub struct ValidatedThingAdmissionConfig {
    policy: Policy,
}
#[derive(Clone, Copy)]
struct Policy {
    values: [u64; 27],
    frames: usize,
    atomic: [u64; 12],
}
impl Policy {
    fn get(self, kind: R) -> u64 {
        self.values[CATALOG.iter().position(|&v| v == kind).expect("TD catalog")]
    }
    fn check(self, kind: R, observed: u64, phase: Phase) -> Result<(), Cause> {
        let configured = self.get(kind);
        if observed > configured {
            Err(Cause::Limit(ValidatedThingLimit {
                kind,
                configured,
                observed,
                phase,
            }))
        } else {
            Ok(())
        }
    }
}
impl ValidatedThingAdmissionConfig {
    pub fn try_from_limits(limits: &ResourceLimits) -> Result<Self, ValidatedThingConfigError> {
        let mut values = [0; 27];
        for (i, resource) in CATALOG.iter().copied().enumerate() {
            values[i] = limits.get(resource).ok_or(ValidatedThingConfigError {
                kind: ValidatedThingConfigErrorKind::MissingAdmissionLimit,
                resource,
                configured: None,
                supported: None,
            })?;
        }
        let mut policy = Policy {
            values,
            frames: 0,
            atomic: [0; 12],
        };
        let unsupported = |resource, supported| ValidatedThingConfigError {
            kind: ValidatedThingConfigErrorKind::UnsupportedLimit,
            resource,
            configured: limits.get(resource),
            supported: Some(supported),
        };
        // Storage/position fields must fit the selected target; work and logical
        // byte totals remain u64, including on a real 32-bit no_std target.
        for resource in [
            R::JsonNestingDepthMax,
            R::SchemaCompositionDepthMax,
            R::JsonMembersPerObjectMax,
            R::JsonArrayItemsMax,
            R::UriTemplateSourceBytesMax,
            R::NumberLexemeBytesMax,
        ] {
            if policy.get(resource) > isize::MAX as u64 {
                return Err(unsupported(resource, isize::MAX as u64));
            }
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
        let depth = policy
            .get(R::JsonNestingDepthMax)
            .max(policy.get(R::SchemaCompositionDepthMax));
        let max_depth =
            (isize::MAX as u64 / mem::size_of::<Frame<'static>>() as u64).saturating_sub(8) / 2;
        policy.frames = depth
            .checked_mul(2)
            .and_then(|v| v.checked_add(8))
            .and_then(|v| usize::try_from(v).ok())
            .ok_or_else(|| unsupported(R::JsonNestingDepthMax, max_depth))?;
        Layout::array::<Frame<'static>>(policy.frames)
            .map_err(|_| unsupported(R::JsonNestingDepthMax, max_depth))?;
        // The semantic continuation has one current byte block, not a whole TD
        // table. Preserve the named URI envelopes; no quadratic resolver cap.
        Layout::array::<u8>(policy.get(R::UriTemplateSourceBytesMax) as usize)
            .map_err(|_| unsupported(R::UriTemplateSourceBytesMax, isize::MAX as u64))?;
        let work = policy.get(R::DocumentValidationWorkUnitsMax);
        let number = policy.get(R::NumberLexemeBytesMax);
        if number > work.saturating_sub(1) {
            return Err(unsupported(R::NumberLexemeBytesMax, work.saturating_sub(1)));
        }
        let native = policy
            .get(R::JsonMembersPerObjectMax)
            .checked_add(1)
            .unwrap();
        if native.saturating_add(1) > work {
            return Err(unsupported(
                R::JsonMembersPerObjectMax,
                work.saturating_sub(2),
            ));
        }
        policy.atomic[W::DocumentNodes as usize] = native.max(1);
        policy.atomic[W::CodecInputBytes as usize] = number.max(2);
        policy.atomic[W::JsonSchemaNodes as usize] = 1;
        policy.atomic[W::SecurityBranches as usize] = 1;
        policy.atomic[W::CleanupItems as usize] = 1;
        policy.atomic[W::UriBytes as usize] = crate::data_type::uri::lending::Resolution::WORK;
        // Recognition compares only fixed vocabulary with equal-length keys.
        // Each comparison is a separate, fully prepaid action.
        policy.atomic[W::CodecInputBytes as usize] =
            policy.atomic[W::CodecInputBytes as usize].max(108);
        let required = native.saturating_add(1).max(109);
        if work < required {
            return Err(ValidatedThingConfigError {
                kind: ValidatedThingConfigErrorKind::UnsupportedLimit,
                resource: R::DocumentValidationWorkUnitsMax,
                configured: Some(work),
                supported: None,
            });
        }
        Ok(Self { policy })
    }
}

/// A proof of complete typed inspection and shared Basic validity.
/// Neither the input loan nor the remaining lifetime allowance can be cloned.
/// Semantic lending consumes this same private owner.
///
/// ```compile_fail
/// use clinkz_wot_td::ValidatedThing;
/// fn refill(proof: ValidatedThing<'_>) -> ValidatedThing<'_> { proof.clone() }
/// ```
pub struct ValidatedThing<'td> {
    owner: ValidatedThingCursor<'td>,
}
/// Consuming progress prevents retaining scratch loans across owner moves.
pub enum ValidatedThingProgress<'td> {
    Pending(ValidatedThingCursor<'td>),
    Complete(ValidatedThing<'td>),
    Failed(ValidatedThingCause),
}
/// The bounded inspection/Basic owner. It never owns or drops the caller Thing.
///
/// ```compile_fail
/// use clinkz_wot_td::{thing::Thing, ValidatedThingCursor, ValidatedThingAdmissionConfig};
/// use clinkz_wot_foundation::{AdmissionLedger, WorkBudget};
/// fn destroy_input(thing: Thing, config: &ValidatedThingAdmissionConfig, ledger: AdmissionLedger) {
///     let cursor = ValidatedThingCursor::from_thing(&thing, config, ledger);
///     drop(thing);
///     let result = cursor.step(&mut WorkBudget::new(), false);
/// }
/// ```
pub struct ValidatedThingCursor<'td> {
    thing: &'td Thing,
    policy: Policy,
    phase: Phase,
    remaining: u64,
    counts: Counts,
    stack: Stack<'td>,
    started: bool,
    walk: b::Walk,
    definitions: Option<btree_map::Iter<'td, String, SecurityScheme>>,
    affordances: [Option<bt::AffordanceIter<'td>>; 3],
    current: Option<(b::Owner, bt::Affordance<'td>)>,
    next_schema: u64,
    #[cfg(test)]
    trace: Trace,
}
impl<'td> ValidatedThingCursor<'td> {
    /// Advanced child entry. The caller supplies a capped unpublished ledger,
    /// pairs it with parent allowances, and provisions the inline owner/return
    /// slots. Constructor work is fixed and does not inspect input or allocate.
    pub fn from_thing(
        thing: &'td Thing,
        config: &ValidatedThingAdmissionConfig,
        ledger: AdmissionLedger,
    ) -> Self {
        Self {
            thing,
            policy: config.policy,
            phase: Phase::Inspect,
            remaining: config.policy.get(R::DocumentValidationWorkUnitsMax),
            counts: Counts::default(),
            stack: Stack::new(ledger),
            started: false,
            walk: b::Walk::default(),
            definitions: None,
            affordances: [None, None, None],
            current: None,
            next_schema: 0,
            #[cfg(test)]
            trace: Trace::default(),
        }
    }
    pub fn step(
        mut self,
        budget: &mut WorkBudget,
        cancel_requested: bool,
    ) -> ValidatedThingProgress<'td> {
        if cancel_requested {
            return ValidatedThingProgress::Failed(Cause::Cancelled { phase: self.phase });
        }
        // No terminal, sizing, allocation or iterator work at zero credit.
        if budget.is_exhausted() {
            return ValidatedThingProgress::Pending(self);
        }
        match self.tick(budget) {
            Ok(true) => ValidatedThingProgress::Complete(ValidatedThing { owner: self }),
            Ok(false) => ValidatedThingProgress::Pending(self),
            Err(cause) => ValidatedThingProgress::Failed(cause),
        }
        // On Failed, Stack's fixed prepaid Drop physically releases its blocks
        // before the source-free result returns. There is no recursive TD Drop.
    }
    fn pay(&mut self, budget: &mut WorkBudget, costs: &[(W, u64)]) -> Result<bool, Cause> {
        // All classes and lifetime checked together. Repeated classes are not
        // permitted by these closed call sites.
        if costs.iter().any(|&(class, n)| budget.remaining(class) < n) {
            return Ok(false);
        }
        let total = costs
            .iter()
            .try_fold(0, |sum, &(_, n)| add(sum, n, self.phase))?;
        if total > self.remaining {
            let configured = self.policy.get(R::DocumentValidationWorkUnitsMax);
            return Err(Cause::Limit(ValidatedThingLimit {
                kind: R::DocumentValidationWorkUnitsMax,
                configured,
                observed: add(configured - self.remaining, total, self.phase)?,
                phase: self.phase,
            }));
        }
        for &(class, n) in costs {
            budget.consume(class, n).expect("whole debit checked");
            #[cfg(test)]
            {
                self.trace.work[class as usize] += n;
            }
        }
        self.remaining -= total;
        Ok(true)
    }
}

// Representation facts for the supplied production model. No strict decoder,
// canonical arena, serialized length or fixture discriminant participates.
mod fields {
    use super::*;
    use crate::{
        affordance::{
            ActionAffordance, EventAffordance, InteractionAffordance, PropertyAffordance,
        },
        context::{Context, ContextEntry},
        data_schema::DataSchemaContext,
        data_type::{
            AbsoluteUri, AdditionalExpectedResponse, ExpectedResponse, Metadata, Operation,
            VersionInfo,
        },
        link::Link,
        security_scheme::{Qop, SecurityLocation, SecuritySchemeContext},
    };
    #[derive(Clone, Copy)]
    pub(super) enum Node<'a> {
        Thing(&'a Thing),
        Metadata(&'a Metadata),
        Schema(&'a DataSchema),
        SchemaContext(&'a DataSchemaContext),
        Interaction(&'a InteractionAffordance),
        Property(&'a PropertyAffordance),
        Action(&'a ActionAffordance),
        Event(&'a EventAffordance),
        Form(&'a Form),
        Link(&'a Link),
        Date(&'a time::OffsetDateTime),
        Version(&'a VersionInfo),
        Response(&'a ExpectedResponse),
        Additional(&'a AdditionalExpectedResponse),
        Security(&'a SecurityScheme),
        SecurityContext(&'a SecuritySchemeContext),
        Context(&'a Context),
        Strings(&'a [String]),
        Schemas(&'a [DataSchema]),
        Values(&'a [Value]),
        Forms(&'a [Form]),
        Links(&'a [Link]),
        Additionals(&'a [AdditionalExpectedResponse]),
        Uris(&'a [AbsoluteUri]),
        Operations(&'a [Operation]),
        Map(Map<'a>),
        Entry(&'a str, Atom<'a>),
        Value(&'a Value),
        Text(&'a str),
        Number(&'a Number),
        Scalar(u64),
    }
    #[derive(Clone, Copy)]
    pub(super) enum Map<'a> {
        Json(&'a serde_json::Map<String, Value>),
        Values(&'a BTreeMap<String, Value>),
        Strings(&'a BTreeMap<String, String>),
        Schemas(&'a BTreeMap<String, DataSchema>),
        Properties(&'a BTreeMap<String, PropertyAffordance>),
        Actions(&'a BTreeMap<String, ActionAffordance>),
        Events(&'a BTreeMap<String, EventAffordance>),
        Security(&'a BTreeMap<String, SecurityScheme>),
    }
    #[derive(Clone, Copy)]
    pub(super) enum Atom<'a> {
        Value(&'a Value),
        Text(&'a str),
        Schema(&'a DataSchema),
        Property(&'a PropertyAffordance),
        Action(&'a ActionAffordance),
        Event(&'a EventAffordance),
        Security(&'a SecurityScheme),
    }
    pub(super) enum Iter<'a> {
        Json(serde_json::map::Iter<'a>),
        Values(btree_map::Iter<'a, String, Value>),
        Strings(btree_map::Iter<'a, String, String>),
        Schemas(btree_map::Iter<'a, String, DataSchema>),
        Properties(btree_map::Iter<'a, String, PropertyAffordance>),
        Actions(btree_map::Iter<'a, String, ActionAffordance>),
        Events(btree_map::Iter<'a, String, EventAffordance>),
        Security(btree_map::Iter<'a, String, SecurityScheme>),
    }
    impl<'a> Map<'a> {
        pub(super) fn len(self) -> usize {
            match self {
                Self::Json(v) => v.len(),
                Self::Values(v) => v.len(),
                Self::Strings(v) => v.len(),
                Self::Schemas(v) => v.len(),
                Self::Properties(v) => v.len(),
                Self::Actions(v) => v.len(),
                Self::Events(v) => v.len(),
                Self::Security(v) => v.len(),
            }
        }
        pub(super) fn iter(self) -> Iter<'a> {
            match self {
                Self::Json(v) => Iter::Json(v.iter()),
                Self::Values(v) => Iter::Values(v.iter()),
                Self::Strings(v) => Iter::Strings(v.iter()),
                Self::Schemas(v) => Iter::Schemas(v.iter()),
                Self::Properties(v) => Iter::Properties(v.iter()),
                Self::Actions(v) => Iter::Actions(v.iter()),
                Self::Events(v) => Iter::Events(v.iter()),
                Self::Security(v) => Iter::Security(v.iter()),
            }
        }
    }
    impl<'a> Iter<'a> {
        pub(super) fn next(&mut self) -> Option<Node<'a>> {
            let (key, value) = match self {
                Self::Json(v) => v.next().map(|(k, v)| (k, Atom::Value(v))),
                Self::Values(v) => v.next().map(|(k, v)| (k, Atom::Value(v))),
                Self::Strings(v) => v.next().map(|(k, v)| (k, Atom::Text(v))),
                Self::Schemas(v) => v.next().map(|(k, v)| (k, Atom::Schema(v))),
                Self::Properties(v) => v.next().map(|(k, v)| (k, Atom::Property(v))),
                Self::Actions(v) => v.next().map(|(k, v)| (k, Atom::Action(v))),
                Self::Events(v) => v.next().map(|(k, v)| (k, Atom::Event(v))),
                Self::Security(v) => v.next().map(|(k, v)| (k, Atom::Security(v))),
            }?;
            Some(Node::Entry(key, value))
        }
    }
    impl<'a> Node<'a> {
        pub(super) fn resolved(self) -> Self {
            match self {
                Self::Value(v) => match v {
                    Value::Null => Self::Scalar(0),
                    Value::Bool(v) => Self::Scalar(*v as u64),
                    Value::Number(v) => Self::Number(v),
                    Value::String(v) => Self::Text(v),
                    Value::Array(v) => Self::Values(v),
                    Value::Object(v) => Self::Map(Map::Json(v)),
                },
                v => v,
            }
        }
        pub(super) fn array(self) -> bool {
            matches!(
                self,
                Self::Context(_)
                    | Self::Strings(_)
                    | Self::Schemas(_)
                    | Self::Values(_)
                    | Self::Forms(_)
                    | Self::Links(_)
                    | Self::Additionals(_)
                    | Self::Uris(_)
                    | Self::Operations(_)
            )
        }
        pub(super) fn container(self) -> bool {
            !matches!(
                self,
                Self::Text(_) | Self::Number(_) | Self::Scalar(_) | Self::Entry(..)
            )
        }
        pub(super) fn arity(self) -> usize {
            match self {
                Self::Thing(_) => 19,
                Self::Metadata(_) => 5,
                Self::Schema(v) => match v {
                    DataSchema::Array(_) => 4,
                    DataSchema::Object(_) => 3,
                    DataSchema::String(_) | DataSchema::Number(_) | DataSchema::Integer(_) => 6,
                    _ => 1,
                },
                Self::SchemaContext(_) => 11,
                Self::Interaction(_) => 2,
                Self::Property(_) => 3,
                Self::Action(_) => 8,
                Self::Event(_) => 7,
                Self::Form(_) => 10,
                Self::Link(_) => 7,
                Self::Date(_) => 8,
                Self::Version(_) => 3,
                Self::Response(_) => 2,
                Self::Additional(_) => 4,
                Self::Security(v) => {
                    1 + match v {
                        SecurityScheme::NoSec(_) | SecurityScheme::Auto(_) => 0,
                        SecurityScheme::Combo(_)
                        | SecurityScheme::Basic(_)
                        | SecurityScheme::APIKey(_) => 2,
                        SecurityScheme::Digest(_) => 3,
                        SecurityScheme::Bearer(_) | SecurityScheme::OAuth2(_) => 5,
                        SecurityScheme::PSK(_) => 1,
                    }
                }
                Self::SecurityContext(_) => 6,
                Self::Context(v) => v.admission_entries().len(),
                Self::Strings(v) => v.len(),
                Self::Schemas(v) => v.len(),
                Self::Values(v) => v.len(),
                Self::Forms(v) => v.len(),
                Self::Links(v) => v.len(),
                Self::Additionals(v) => v.len(),
                Self::Uris(v) => v.len(),
                Self::Operations(v) => v.len(),
                Self::Map(v) => v.len(),
                Self::Entry(..) => 2,
                Self::Text(_) | Self::Number(_) | Self::Scalar(_) => 0,
                Self::Value(_) => unreachable!(),
            }
        }
        pub(super) fn extra_field(self, i: usize) -> bool {
            match self {
                Self::Thing(_) => i == 18,
                Self::SchemaContext(_) => matches!(i, 1 | 2 | 5 | 10),
                Self::Action(_) => i == 7,
                Self::Event(_) => i == 6,
                Self::Form(_) => i == 9,
                Self::Link(_) => i == 6,
                Self::Version(_) => i == 2,
                Self::Response(_) => i == 1,
                Self::Additional(_) => i == 3,
                Self::SecurityContext(_) => i == 5,
                _ => false,
            }
        }
        pub(super) fn variable_field(self, i: usize) -> bool {
            matches!((self, i), (Self::Thing(_), 17) | (Self::Interaction(_), 1))
        }
        pub(super) fn child(self, i: usize) -> Option<Self> {
            use Node as N;
            Some(match self {
                N::Thing(v) => match i {
                    0 => N::Context(&v.context),
                    1 => return v.id.as_ref().map(|v| N::Text(v.as_str())),
                    2 => N::Metadata(&v._metadata),
                    3 => return v.version.as_ref().map(N::Version),
                    4 => return v.created.as_ref().map(N::Date),
                    5 => return v.modified.as_ref().map(N::Date),
                    6 => return v.support.as_ref().map(|v| N::Text(v.as_str())),
                    7 => return v.base.as_ref().map(|v| N::Text(v.as_str())),
                    8 => return v.properties.as_ref().map(|v| N::Map(Map::Properties(v))),
                    9 => return v.actions.as_ref().map(|v| N::Map(Map::Actions(v))),
                    10 => return v.events.as_ref().map(|v| N::Map(Map::Events(v))),
                    11 => return v.links.as_deref().map(N::Links),
                    12 => return v.forms.as_deref().map(N::Forms),
                    13 => N::Strings(&v.security),
                    14 => N::Map(Map::Security(&v.security_definitions)),
                    15 => return v.profile.as_deref().map(N::Uris),
                    16 => {
                        return v
                            .schema_definitions
                            .as_ref()
                            .map(|v| N::Map(Map::Schemas(v)));
                    }
                    17 => return v.uri_variables.as_ref().map(|v| N::Map(Map::Schemas(v))),
                    18 => N::Map(Map::Values(&v._extra_fields)),
                    _ => unreachable!(),
                },
                N::Metadata(v) => match i {
                    0 => return v.tags.as_deref().map(N::Strings),
                    1 => return v.title.as_deref().map(N::Text),
                    2 => return v.titles.as_ref().map(|v| N::Map(Map::Strings(v.as_map()))),
                    3 => return v.description.as_deref().map(N::Text),
                    4 => {
                        return v
                            .descriptions
                            .as_ref()
                            .map(|v| N::Map(Map::Strings(v.as_map())));
                    }
                    _ => unreachable!(),
                },
                N::SchemaContext(v) => match i {
                    0 => N::Metadata(&v._metadata),
                    1 => return v.constant.as_ref().map(N::Value),
                    2 => return v.default.as_ref().map(N::Value),
                    3 => return v.unit.as_deref().map(N::Text),
                    4 => return v.one_of.as_deref().map(N::Schemas),
                    5 => return v.enumerate.as_deref().map(N::Values),
                    6 => N::Scalar(v.read_only as u64),
                    7 => N::Scalar(v.write_only as u64),
                    8 => return v.format.as_deref().map(N::Text),
                    9 => return v.data_type.as_deref().map(N::Text),
                    10 => N::Map(Map::Values(&v._extra_fields)),
                    _ => unreachable!(),
                },
                N::Schema(v) => {
                    if i == 0 {
                        N::SchemaContext(v.context())
                    } else {
                        return match v {
                            DataSchema::Array(v) => match i {
                                1 => v.items.as_deref().map(N::Schemas),
                                2 => v.min_items.map(|v| N::Scalar(v as u64)),
                                3 => v.max_items.map(|v| N::Scalar(v as u64)),
                                _ => unreachable!(),
                            },
                            DataSchema::Object(v) => match i {
                                1 => v.properties.as_ref().map(|v| N::Map(Map::Schemas(v))),
                                2 => v.required.as_deref().map(N::Strings),
                                _ => unreachable!(),
                            },
                            DataSchema::String(v) => match i {
                                1 => v.min_length.map(|v| N::Scalar(v as u64)),
                                2 => v.max_length.map(|v| N::Scalar(v as u64)),
                                3 => v.pattern.as_deref().map(N::Text),
                                4 => v.content_encoding.as_deref().map(N::Text),
                                5 => v.content_media_type.as_deref().map(N::Text),
                                _ => unreachable!(),
                            },
                            DataSchema::Number(v) => [
                                v.minimum,
                                v.exclusive_minimum,
                                v.maximum,
                                v.exclusive_maximum,
                                v.multiple_of,
                            ][i - 1]
                                .map(|v| N::Scalar(v.to_bits())),
                            DataSchema::Integer(v) => [
                                v.minimum,
                                v.exclusive_minimum,
                                v.maximum,
                                v.exclusive_maximum,
                                v.multiple_of,
                            ][i - 1]
                                .map(|v| N::Scalar(v as u64)),
                            _ => unreachable!(),
                        };
                    }
                }
                N::Interaction(v) => match i {
                    0 => N::Forms(&v.forms),
                    1 => return v.uri_variables.as_ref().map(|v| N::Map(Map::Schemas(v))),
                    _ => unreachable!(),
                },
                N::Property(v) => match i {
                    0 => N::Schema(&v._schema),
                    1 => N::Interaction(&v._interaction),
                    2 => N::Scalar(v.observable as u64),
                    _ => unreachable!(),
                },
                N::Action(v) => match i {
                    0 => N::Metadata(&v._metadata),
                    1 => N::Interaction(&v._interaction),
                    2 => return v.input.as_ref().map(N::Schema),
                    3 => return v.output.as_ref().map(N::Schema),
                    4 => N::Scalar(v.safe as u64),
                    5 => N::Scalar(v.idempotent as u64),
                    6 => {
                        #[cfg(feature = "td2-preview")]
                        {
                            return v.synchronous.map(|v| N::Scalar(v as u64));
                        }
                        #[cfg(not(feature = "td2-preview"))]
                        {
                            return None;
                        }
                    }
                    7 => N::Map(Map::Values(&v._extra_fields)),
                    _ => unreachable!(),
                },
                N::Event(v) => match i {
                    0 => N::Metadata(&v._metadata),
                    1 => N::Interaction(&v._interaction),
                    2 => return v.subscription.as_ref().map(N::Schema),
                    3 => return v.data.as_ref().map(N::Schema),
                    4 => return v.data_response.as_ref().map(N::Schema),
                    5 => return v.cancellation.as_ref().map(N::Schema),
                    6 => N::Map(Map::Values(&v._extra_fields)),
                    _ => unreachable!(),
                },
                N::Form(v) => match i {
                    0 => N::Text(v.href.as_str()),
                    1 => N::Text(&v.content_type),
                    2 => return v.content_coding.as_deref().map(N::Text),
                    3 => return v.security.as_deref().map(N::Strings),
                    4 => return v.scopes.as_deref().map(N::Strings),
                    5 => return v.response.as_ref().map(N::Response),
                    6 => return v.additional_responses.as_deref().map(N::Additionals),
                    7 => return v.subprotocol.as_deref().map(N::Text),
                    8 => return v.op.as_deref().map(N::Operations),
                    9 => N::Map(Map::Values(&v._extra_fields)),
                    _ => unreachable!(),
                },
                N::Link(v) => match i {
                    0 => N::Text(v.href.as_str()),
                    1 => return v.content_type.as_deref().map(N::Text),
                    2 => return v.rel.as_deref().map(N::Text),
                    3 => return v.anchor.as_ref().map(|v| N::Text(v.as_str())),
                    4 => return v.sizes.as_deref().map(N::Text),
                    5 => return v.hreflang.as_deref().map(N::Strings),
                    6 => N::Map(Map::Values(&v._extra_fields)),
                    _ => unreachable!(),
                },
                N::Date(v) => N::Scalar(match i {
                    0 => v.year() as u64,
                    1 => v.month() as u64,
                    2 => v.day() as u64,
                    3 => v.hour() as u64,
                    4 => v.minute() as u64,
                    5 => v.second() as u64,
                    6 => v.nanosecond() as u64,
                    7 => v.offset().whole_seconds() as u64,
                    _ => unreachable!(),
                }),
                N::Version(v) => match i {
                    0 => N::Text(&v.instance),
                    1 => return v.model.as_deref().map(N::Text),
                    2 => N::Map(Map::Values(&v._extra_fields)),
                    _ => unreachable!(),
                },
                N::Response(v) => match i {
                    0 => N::Text(&v.content_type),
                    1 => N::Map(Map::Values(&v._extra_fields)),
                    _ => unreachable!(),
                },
                N::Additional(v) => match i {
                    0 => return v.content_type.as_deref().map(N::Text),
                    1 => return v.schema.as_deref().map(N::Text),
                    2 => N::Scalar(v.success as u64),
                    3 => N::Map(Map::Values(&v._extra_fields)),
                    _ => unreachable!(),
                },
                N::Security(v) => {
                    if i == 0 {
                        N::SecurityContext(bt::security_context(v))
                    } else {
                        return security_child(v, i - 1);
                    }
                }
                N::SecurityContext(v) => match i {
                    0 => return v.tags.as_deref().map(N::Strings),
                    1 => return v.description.as_deref().map(N::Text),
                    2 => {
                        return v
                            .descriptions
                            .as_ref()
                            .map(|v| N::Map(Map::Strings(v.as_map())));
                    }
                    3 => return v.proxy.as_ref().map(|v| N::Text(v.as_str())),
                    4 => N::Text(&v.scheme),
                    5 => N::Map(Map::Values(&v._extra_fields)),
                    _ => unreachable!(),
                },
                N::Context(v) => match &v.admission_entries()[i] {
                    ContextEntry::Uri(v) => N::Text(v.as_str()),
                    ContextEntry::Object(v) => N::Map(Map::Values(v)),
                },
                N::Strings(v) => N::Text(&v[i]),
                N::Schemas(v) => N::Schema(&v[i]),
                N::Values(v) => N::Value(&v[i]),
                N::Forms(v) => N::Form(&v[i]),
                N::Links(v) => N::Link(&v[i]),
                N::Additionals(v) => N::Additional(&v[i]),
                N::Uris(v) => N::Text(v[i].as_str()),
                N::Operations(v) => N::Text(v[i].as_str()),
                N::Entry(k, v) => {
                    if i == 0 {
                        N::Text(k)
                    } else {
                        match v {
                            Atom::Value(v) => N::Value(v),
                            Atom::Text(v) => N::Text(v),
                            Atom::Schema(v) => N::Schema(v),
                            Atom::Property(v) => N::Property(v),
                            Atom::Action(v) => N::Action(v),
                            Atom::Event(v) => N::Event(v),
                            Atom::Security(v) => N::Security(v),
                        }
                    }
                }
                _ => unreachable!(),
            })
        }
    }
    fn location(v: &SecurityLocation) -> Node<'static> {
        Node::Text(match v {
            SecurityLocation::Header => "header",
            SecurityLocation::Query => "query",
            SecurityLocation::Body => "body",
            SecurityLocation::Cookie => "cookie",
            SecurityLocation::Auto => "auto",
            SecurityLocation::Uri => "uri",
        })
    }
    fn security_child(v: &SecurityScheme, i: usize) -> Option<Node<'_>> {
        use Node as N;
        Some(match v {
            SecurityScheme::Combo(v) => N::Strings(if i == 0 { &v.one_of } else { &v.all_of }),
            SecurityScheme::Basic(v) => match i {
                0 => return v.name.as_deref().map(N::Text),
                1 => location(&v.location),
                _ => unreachable!(),
            },
            SecurityScheme::Digest(v) => match i {
                0 => return v.name.as_deref().map(N::Text),
                1 => location(&v.location),
                2 => N::Text(match v.qop {
                    Qop::Auth => "auth",
                    Qop::AuthInt => "auth-int",
                }),
                _ => unreachable!(),
            },
            SecurityScheme::APIKey(v) => match i {
                0 => return v.name.as_deref().map(N::Text),
                1 => location(&v.location),
                _ => unreachable!(),
            },
            SecurityScheme::Bearer(v) => match i {
                0 => return v.authorization.as_ref().map(|v| N::Text(v.as_str())),
                1 => return v.name.as_deref().map(N::Text),
                2 => N::Text(&v.alg),
                3 => N::Text(&v.format),
                4 => location(&v.location),
                _ => unreachable!(),
            },
            SecurityScheme::PSK(v) => return v.identity.as_deref().map(N::Text),
            SecurityScheme::OAuth2(v) => match i {
                0 => return v.authorization.as_ref().map(|v| N::Text(v.as_str())),
                1 => return v.token.as_ref().map(|v| N::Text(v.as_str())),
                2 => return v.refresh.as_ref().map(|v| N::Text(v.as_str())),
                3 => return v.scopes.as_deref().map(N::Strings),
                4 => N::Text(&v.flow),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        })
    }
}
use fields::{Map, Node};
#[derive(Clone, Copy, Default)]
struct Scope {
    depth: u64,
    schema_depth: u64,
    extension: bool,
    variables: bool,
}
#[derive(Default)]
struct Counts {
    nodes: u64,
    content: u64,
    strings: u64,
    extensions: u64,
    schemas: u64,
    edges: u64,
    affordances: u64,
    forms: u64,
}
impl Counts {
    fn visit(&mut self, node: Node<'_>, mut scope: Scope, policy: Policy) -> Result<Scope, Cause> {
        let phase = Phase::Inspect;
        // The admitted typed traversal counts each association as well as its
        // separately visited key and value occurrences.
        self.nodes = add(self.nodes, 1, phase)?;
        policy.check(R::JsonValueNodesPerDocumentMax, self.nodes, phase)?;
        if node.container() {
            scope.depth = add(scope.depth, 1, phase)?;
            policy.check(R::JsonNestingDepthMax, scope.depth, phase)?;
        }
        if node.array() {
            policy.check(R::JsonArrayItemsMax, node.arity() as u64, phase)?;
        }
        if let Node::Map(m) = node {
            policy.check(R::JsonMembersPerObjectMax, m.len() as u64, phase)?;
            if matches!(m, Map::Properties(_) | Map::Actions(_) | Map::Events(_)) {
                self.affordances = add(self.affordances, m.len() as u64, phase)?;
                policy.check(R::AffordancesPerThingMax, self.affordances, phase)?;
            }
            if scope.variables {
                policy.check(R::UriVariablesPerFormMax, m.len() as u64, phase)?;
            }
        }
        if let Node::Forms(v) = node {
            self.forms = add(self.forms, v.len() as u64, phase)?;
            policy.check(R::FormsPerContextMax, v.len() as u64, phase)?;
            policy.check(R::FormsPerThingMax, self.forms, phase)?;
        }
        if let Node::Additionals(v) = node {
            policy.check(R::AdditionalResponsesPerFormMax, v.len() as u64, phase)?;
        }
        if let Node::Additional(v) = node {
            if v.schema.is_some() {
                self.edges = add(self.edges, 1, phase)?;
                policy.check(R::SchemaReferenceEdgesPerDocumentMax, self.edges, phase)?;
            }
        }
        if matches!(node, Node::Schema(_)) {
            self.schemas = add(self.schemas, 1, phase)?;
            policy.check(R::SchemaNodesPerDocumentMax, self.schemas, phase)?;
            if scope.schema_depth != 0 {
                self.edges = add(self.edges, 1, phase)?;
                policy.check(R::SchemaReferenceEdgesPerDocumentMax, self.edges, phase)?;
            }
            scope.schema_depth = add(scope.schema_depth, 1, phase)?;
            policy.check(R::SchemaCompositionDepthMax, scope.schema_depth, phase)?;
        }
        if let Node::Form(v) = node {
            policy.check(
                R::UriTemplateSourceBytesMax,
                v.href.as_str().len() as u64,
                phase,
            )?;
        }
        let bytes = match node {
            Node::Text(v) => v.len() as u64,
            Node::Number(v) => v.as_str().len() as u64,
            Node::Scalar(_) => 8,
            _ => 0,
        };
        self.content = add(self.content, bytes, phase)?;
        policy.check(R::DocumentBytesMax, self.content, phase)?;
        policy.check(R::GeneratedEffectiveDocumentBytesMax, self.content, phase)?;
        match node {
            Node::Number(_) => policy.check(R::NumberLexemeBytesMax, bytes, phase)?,
            Node::Text(_) => {
                self.strings = add(self.strings, bytes, phase)?;
                policy.check(R::StringBytesMax, self.strings, phase)?;
            }
            _ => {}
        }
        if scope.extension {
            self.extensions = add(self.extensions, bytes, phase)?;
            policy.check(R::ExtensionBytesMax, self.extensions, phase)?;
        }
        Ok(scope)
    }
}

struct Structure<'a> {
    node: Node<'a>,
    scope: Scope,
    next: usize,
    members: u64,
    iter: Option<fields::Iter<'a>>,
}
struct Schema<'a> {
    node: &'a DataSchema,
    site: b::Site,
    ordinal: u64,
    walk: s::Walk,
    children: Option<SchemaChildren<'a>>,
    extras: Option<btree_map::Iter<'a, String, Value>>,
    extra: Option<(&'a str, &'a Value)>,
    match_index: usize,
    discovered: bool,
    unsigned: [Option<u64>; 4],
    unsigned_pending: Option<(usize, &'a Number)>,
    numbers: [Option<&'a Number>; 5],
    numeric: s::NumericCursor,
}
impl<'a> Schema<'a> {
    fn new(node: &'a DataSchema, site: b::Site, ordinal: u64) -> Self {
        Self {
            node,
            site,
            ordinal,
            walk: s::Walk::default(),
            children: None,
            extras: None,
            extra: None,
            match_index: 0,
            discovered: false,
            unsigned: [None; 4],
            unsigned_pending: None,
            numbers: [None; 5],
            numeric: s::NumericCursor::default(),
        }
    }
}
#[derive(Clone, Copy)]
enum Names<'a> {
    Strings(&'a [String]),
    Json(&'a [Value]),
    One(&'a str),
    Empty,
}
impl<'a> Names<'a> {
    fn len(self) -> usize {
        match self {
            Self::Strings(v) => v.len(),
            Self::Json(v) => v.len(),
            Self::One(_) => 1,
            Self::Empty => 0,
        }
    }
    fn at(self, i: usize) -> Option<&'a str> {
        match self {
            Self::Strings(v) => Some(&v[i]),
            Self::Json(v) => v[i].as_str(),
            Self::One(v) => Some(v),
            Self::Empty => None,
        }
    }
    fn value(v: &'a Value) -> Self {
        match v {
            Value::String(v) => Self::One(v),
            Value::Array(v) => Self::Json(v),
            _ => Self::Empty,
        }
    }
}
struct Definition<'a> {
    node: &'a SecurityScheme,
    site: b::Site,
    stage: u8,
    scheme: Option<b::Scheme>,
    extras: Option<btree_map::Iter<'a, String, Value>>,
    extra: Option<(&'a str, &'a Value)>,
    match_index: usize,
    one: Names<'a>,
    all: Names<'a>,
    name: Option<&'a str>,
    flow: &'a str,
    authorization: bool,
    token: bool,
    counts: [usize; 2],
    empty: [Option<usize>; 2],
    index: usize,
    member: usize,
}
impl<'a> Definition<'a> {
    fn new(node: &'a SecurityScheme, owner: b::Owner) -> Self {
        Self {
            node,
            site: b::Site::new(owner, b::Field::Scheme),
            stage: 0,
            scheme: None,
            extras: None,
            extra: None,
            match_index: 0,
            one: Names::Empty,
            all: Names::Empty,
            name: None,
            flow: "",
            authorization: false,
            token: false,
            counts: [0; 2],
            empty: [None; 2],
            index: 0,
            member: 0,
        }
    }
}
struct Search<'a> {
    target: &'a str,
    iter: Option<btree_map::Iter<'a, String, SecurityScheme>>,
    current: Option<&'a str>,
    byte: usize,
    site: b::Site,
}
enum Frame<'a> {
    Enter(Node<'a>, Scope),
    Structure(Structure<'a>),
    Text(&'a str, usize),
    Schema(Schema<'a>),
    SchemaMap {
        map: &'a BTreeMap<String, DataSchema>,
        iter: Option<btree_map::Iter<'a, String, DataSchema>>,
        index: usize,
        site: b::Site,
    },
    References(Names<'a>, usize, b::Site),
    Lookup(Search<'a>),
    Definition(Definition<'a>),
    Operations(&'a [Form], usize, usize, b::Owner),
    FormSecurity(&'a [Form], usize, b::Owner),
}
const _: () = assert!(!mem::needs_drop::<Frame<'static>>());
struct Block<'a> {
    pointer: *mut Frame<'a>,
    capacity: usize,
    len: usize,
}
impl Default for Block<'_> {
    fn default() -> Self {
        Self {
            pointer: ptr::null_mut(),
            capacity: 0,
            len: 0,
        }
    }
}
struct Stack<'a> {
    ledger: AdmissionLedger,
    frames: Block<'a>,
    transfer: Option<Block<'a>>,
    moved: usize,
}
impl<'a> Stack<'a> {
    fn new(ledger: AdmissionLedger) -> Self {
        Self {
            ledger,
            frames: Block::default(),
            transfer: None,
            moved: 0,
        }
    }
    fn top(&self) -> &Frame<'a> {
        assert!(self.transfer.is_none() && self.frames.len > 0);
        unsafe { &*self.frames.pointer.add(self.frames.len - 1) }
    }
    fn pop(&mut self) -> Frame<'a> {
        assert!(self.transfer.is_none() && self.frames.len > 0);
        self.frames.len -= 1;
        unsafe { self.frames.pointer.add(self.frames.len).read() }
    }
    fn push(&mut self, value: Frame<'a>) {
        assert!(self.transfer.is_none() && self.frames.len < self.frames.capacity);
        unsafe {
            self.frames.pointer.add(self.frames.len).write(value);
        }
        self.frames.len += 1;
    }
    fn release(&mut self, block: Block<'a>) {
        if block.capacity == 0 {
            return;
        }
        let layout = Layout::array::<Frame<'a>>(block.capacity).expect("checked on acquisition");
        // Frame contains only scalars and external loans/native iterators with
        // no Drop. Physical child release precedes its ledger release.
        unsafe {
            dealloc(block.pointer.cast(), layout);
        }
        assert!(self.ledger.release_temporary(layout.size() as u64));
    }
    fn grow(&mut self, capacity: usize, policy: Policy, phase: Phase) -> Result<(), Cause> {
        assert!(self.transfer.is_none());
        let layout = Layout::array::<Frame<'a>>(capacity).map_err(|_| arithmetic(phase))?;
        let bytes = layout.size() as u64;
        // Inline owner and return overlap is capacity, never a fake allocation
        // request. Its parent provision belongs to the advanced child caller.
        let inline = inline_bytes();
        let retained = Layout::array::<Frame<'a>>(self.frames.capacity)
            .expect("checked on acquisition")
            .size() as u64;
        // Only TD-owned old/new blocks and inline capacity consume this
        // child's temporary/additional-admission ceiling. Existing upstream
        // accounts remain in the aggregate for global live/peak checks.
        let temporary = add(add(retained, bytes, phase)?, inline, phase)?;
        for kind in [
            R::AdmissionTemporaryBytesPerOperationMax,
            R::AdmissionTemporaryBytesGlobalMax,
            R::PeakLiveBytesPerAdmissionMax,
        ] {
            policy.check(kind, temporary, phase)?;
        }
        let live = add(add(self.ledger.live_bytes(), bytes, phase)?, inline, phase)?;
        for kind in [
            R::AdmissionPeakLiveBytesGlobalMax,
            R::EngineLiveBytesGlobalMax,
        ] {
            policy.check(kind, live, phase)?;
        }
        policy.check(R::LargestContiguousAllocationBytesMax, bytes, phase)?;
        let reservation = self
            .ledger
            .try_reserve_temporary(R::AdmissionTemporaryBytesPerOperationMax, bytes)
            .ok_or(Cause::Limit(ValidatedThingLimit {
                kind: R::AdmissionTemporaryBytesPerOperationMax,
                configured: policy.get(R::AdmissionTemporaryBytesPerOperationMax),
                observed: temporary,
                phase,
            }))?;
        // SAFETY: checked nonzero Layout, charge held before the allocator. No
        // recursive source, owned String or element Drop is stored in this block.
        let pointer = unsafe { alloc(layout) }.cast::<Frame<'a>>();
        if pointer.is_null() {
            return Err(Cause::Failed(ValidatedThingFailure {
                kind: ValidatedThingFailureKind::AllocationFailed,
                phase,
                requested_bytes: bytes,
            }));
        }
        reservation.commit();
        self.transfer = Some(Block {
            pointer,
            capacity,
            len: 0,
        });
        self.moved = 0;
        Ok(())
    }
    fn move_one(&mut self) {
        if self.moved < self.frames.len {
            // Distinct blocks; neither source nor destination is inspected while
            // a transfer is pending. Each initialized frame moves exactly once.
            unsafe {
                self.transfer
                    .as_mut()
                    .unwrap()
                    .pointer
                    .add(self.moved)
                    .write(self.frames.pointer.add(self.moved).read());
            }
            self.moved += 1;
        } else {
            let mut next = self.transfer.take().unwrap();
            next.len = self.frames.len;
            let old = mem::replace(&mut self.frames, next);
            self.release(old);
        }
    }
}
impl Drop for Stack<'_> {
    fn drop(&mut self) {
        if let Some(block) = self.transfer.take() {
            self.release(block);
        }
        let block = mem::take(&mut self.frames);
        self.release(block);
        // Any pre-existing upstream charge is not ours to free or reclassify.
    }
}
fn iter_cost(len: usize) -> u64 {
    len as u64 + 1
}
fn equal_cost(a: &str, b: &str) -> u64 {
    if a.len() == b.len() {
        2 * a.len() as u64
    } else {
        0
    }
}
fn equal_fixed(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(true, |equal, (a, b)| equal & (a == b))
}
const SCHEMA_KEYS: [&str; 9] = [
    "minItems",
    "maxItems",
    "minLength",
    "maxLength",
    "minimum",
    "exclusiveMinimum",
    "maximum",
    "exclusiveMaximum",
    "multipleOf",
];
const SECURITY_KEYS: [&str; 6] = ["oneOf", "allOf", "name", "flow", "authorization", "token"];

impl<'td> ValidatedThingCursor<'td> {
    fn room(&mut self, budget: &mut WorkBudget) -> Result<bool, Cause> {
        if self.stack.transfer.is_some() {
            if !self.pay(budget, &[(W::DocumentNodes, 1)])? {
                return Ok(false);
            }
            self.stack.move_one();
            #[cfg(test)]
            {
                self.trace.moves += 1;
            }
            return Ok(false);
        }
        if self.stack.frames.len + 2 > self.stack.frames.capacity {
            let capacity = self
                .stack
                .frames
                .capacity
                .max(2)
                .checked_mul(2)
                .ok_or(arithmetic(self.phase))?
                .min(self.policy.frames);
            if capacity < self.stack.frames.len + 2 {
                return Err(arithmetic(self.phase));
            }
            // One fixed allocation action and its entire future block release.
            if !self.pay(budget, &[(W::DocumentNodes, 1), (W::CleanupItems, 1)])? {
                return Ok(false);
            }
            self.stack.grow(capacity, self.policy, self.phase)?;
            return Ok(false);
        }
        Ok(true)
    }
    fn tick(&mut self, budget: &mut WorkBudget) -> Result<bool, Cause> {
        if !self.room(budget)? {
            return Ok(false);
        }
        if !self.started {
            if !self.pay(budget, &[(W::DocumentNodes, 1)])? {
                return Ok(false);
            }
            self.started = true;
            self.stack
                .push(Frame::Enter(Node::Thing(self.thing), Scope::default()));
            return Ok(false);
        }
        if self.stack.frames.len == 0 {
            if self.phase == Phase::Inspect {
                if !self.pay(budget, &[(W::DocumentNodes, 1)])? {
                    return Ok(false);
                }
                self.phase = Phase::Basic;
            }
            return self.global(budget);
        }
        // Copy only external handles or scalar action metadata before borrowing
        // the account owner. Never retain a loan into the movable frame block.
        let costs = match self.stack.top() {
            Frame::Enter(node, _) => {
                let node = node.resolved();
                let class = match node {
                    Node::Schema(_) => W::JsonSchemaNodes,
                    Node::Security(_) => W::SecurityBranches,
                    _ => W::DocumentNodes,
                };
                if let Node::Map(map) = node {
                    let len = map.len();
                    // Reach rejection within the supported step envelope. An
                    // oversized length needs only fixed structural work; no
                    // iterator is initialized until counts.visit admits it.
                    let debit = if len as u64 > self.policy.get(R::JsonMembersPerObjectMax) {
                        1
                    } else {
                        iter_cost(len)
                    };
                    [
                        (W::DocumentNodes, debit),
                        (W::CodecInputBytes, 0),
                        (class, 0),
                    ]
                } else {
                    [(W::DocumentNodes, 0), (W::CodecInputBytes, 0), (class, 1)]
                }
            }
            Frame::Structure(f) => [
                (
                    W::DocumentNodes,
                    if let Node::Map(m) = f.node {
                        iter_cost(m.len())
                    } else {
                        1
                    },
                ),
                (W::CodecInputBytes, 0),
                (W::SecurityBranches, 0),
            ],
            Frame::Text(text, at) => [
                (W::DocumentNodes, if *at == text.len() { 1 } else { 0 }),
                (W::CodecInputBytes, if *at < text.len() { 1 } else { 0 }),
                (W::SecurityBranches, 0),
            ],
            Frame::Operations(..) | Frame::FormSecurity(..) => [
                (W::DocumentNodes, 1),
                (W::CodecInputBytes, 0),
                (W::SecurityBranches, 0),
            ],
            Frame::References(..) => [
                (W::DocumentNodes, 0),
                (W::CodecInputBytes, 0),
                (W::SecurityBranches, 1),
            ],
            Frame::Lookup(f) => [
                (
                    W::DocumentNodes,
                    if f.iter.is_none() || f.current.is_none() {
                        iter_cost(self.thing.security_definitions.len())
                    } else {
                        1
                    },
                ),
                (
                    W::CodecInputBytes,
                    if f.current.is_some_and(|v| f.byte < v.len()) {
                        2
                    } else {
                        0
                    },
                ),
                (W::SecurityBranches, 0),
            ],
            Frame::SchemaMap { map, .. } => [
                (W::DocumentNodes, iter_cost(map.len())),
                (W::CodecInputBytes, 0),
                (W::SecurityBranches, 0),
            ],
            Frame::Schema(_) => return self.schema_tick(budget),
            Frame::Definition(_) => return self.definition_tick(budget),
        };
        // Each closed action uses one slot per class; remove zero duplicates.
        let mut debits = [
            (W::DocumentNodes, 0),
            (W::CodecInputBytes, 0),
            (W::SecurityBranches, 0),
            (W::JsonSchemaNodes, 0),
        ];
        for (class, n) in costs {
            if n != 0 {
                let i = debits.iter().position(|(v, _)| *v == class).unwrap();
                debits[i].1 += n;
            }
        }
        if !self.pay(budget, &debits)? {
            return Ok(false);
        }
        let frame = self.stack.pop();
        match frame {
            Frame::Enter(node, scope) => {
                let node = node.resolved();
                let scope = self.counts.visit(node, scope, self.policy)?;
                #[cfg(test)]
                {
                    self.trace.entered += 1;
                }
                match node {
                    Node::Text(v) => self.stack.push(Frame::Text(v, 0)),
                    Node::Number(v) => self.stack.push(Frame::Text(v.as_str(), 0)),
                    Node::Scalar(v) => {
                        #[cfg(test)]
                        {
                            self.trace.scalar_checksum = self.trace.scalar_checksum.wrapping_add(v);
                        }
                        #[cfg(not(test))]
                        {
                            let _ = v;
                        }
                    }
                    _ => {
                        let iter = if let Node::Map(v) = node {
                            Some(v.iter())
                        } else {
                            None
                        };
                        self.stack.push(Frame::Structure(Structure {
                            node,
                            scope,
                            next: 0,
                            members: 0,
                            iter,
                        }));
                    }
                }
            }
            Frame::Structure(mut f) => {
                let index = f.next;
                let child = if let Some(iter) = &mut f.iter {
                    #[cfg(test)]
                    {
                        self.trace.nexts += 1;
                    }
                    iter.next()
                } else if index < f.node.arity() {
                    f.node.child(index)
                } else {
                    None
                };
                let done = if matches!(f.node, Node::Map(_)) {
                    child.is_none()
                } else {
                    index == f.node.arity()
                };
                if !done {
                    f.next += 1;
                    let mut scope = f.scope;
                    scope.extension |= f.node.extra_field(index);
                    scope.variables = f.node.variable_field(index);
                    if child.is_some() && f.node.container() && !f.node.array() {
                        f.members = add(f.members, 1, self.phase)?;
                        self.policy
                            .check(R::JsonMembersPerObjectMax, f.members, self.phase)?;
                    }
                    self.stack.push(Frame::Structure(f));
                    if let Some(child) = child {
                        self.stack.push(Frame::Enter(child, scope));
                    }
                }
            }
            Frame::Text(text, at) => {
                if at < text.len() {
                    let byte = text.as_bytes()[at];
                    #[cfg(test)]
                    {
                        self.trace.reads += 1;
                        self.trace.checksum = self.trace.checksum.wrapping_add(byte as u64 + 1);
                    }
                    #[cfg(not(test))]
                    {
                        let _ = byte;
                    }
                    self.stack.push(Frame::Text(text, at + 1));
                }
            }
            Frame::References(names, index, mut site) => {
                if index < names.len() {
                    let name = names.at(index);
                    let selected = site;
                    if name.is_some() {
                        site.member += 1;
                    }
                    self.stack.push(Frame::References(names, index + 1, site));
                    if let Some(name) = name {
                        let site = selected;
                        self.stack.push(Frame::Lookup(Search {
                            target: name,
                            iter: None,
                            current: None,
                            byte: 0,
                            site,
                        }));
                    }
                }
            }
            Frame::Lookup(mut f) => {
                if f.iter.is_none() {
                    f.iter = Some(self.thing.security_definitions.iter());
                    self.stack.push(Frame::Lookup(f));
                } else if let Some(key) = f.current {
                    if f.byte == key.len() { /* equality complete; reference valid */
                    } else if key.as_bytes()[f.byte] == f.target.as_bytes()[f.byte] {
                        f.byte += 1;
                        #[cfg(test)]
                        {
                            self.trace.compared += 2;
                        }
                        self.stack.push(Frame::Lookup(f));
                    } else {
                        #[cfg(test)]
                        {
                            self.trace.compared += 2;
                        }
                        f.current = None;
                        f.byte = 0;
                        self.stack.push(Frame::Lookup(f));
                    }
                } else {
                    #[cfg(test)]
                    {
                        self.trace.nexts += 1;
                    }
                    match f.iter.as_mut().unwrap().next() {
                        None => return Err(invalid(f.site, 0, b::Rule::Undefined(f.target))),
                        Some((key, _)) => {
                            if key.len() == f.target.len() {
                                f.current = Some(key);
                            }
                            self.stack.push(Frame::Lookup(f));
                        }
                    }
                }
            }
            Frame::SchemaMap {
                map,
                mut iter,
                index,
                site,
            } => {
                if iter.is_none() {
                    iter = Some(map.iter());
                    self.stack.push(Frame::SchemaMap {
                        map,
                        iter,
                        index,
                        site,
                    });
                } else {
                    #[cfg(test)]
                    {
                        self.trace.nexts += 1;
                    }
                    if let Some((_, node)) = iter.as_mut().unwrap().next() {
                        let mut child_site = site;
                        child_site.index = index;
                        self.stack.push(Frame::SchemaMap {
                            map,
                            iter,
                            index: index + 1,
                            site,
                        });
                        self.push_schema(node, child_site)?;
                    }
                }
            }
            Frame::Operations(forms, index, member, owner) => {
                if index < forms.len() {
                    match &forms[index].op {
                        Some(ops) if member < ops.len() => {
                            let op = ops[member];
                            if !b::allowed(owner.kind, op) {
                                return Err(invalid(
                                    b::Site {
                                        owner,
                                        field: b::Field::FormOperation,
                                        index,
                                        member,
                                    },
                                    0,
                                    b::Rule::Operation(op),
                                ));
                            }
                            self.stack
                                .push(Frame::Operations(forms, index, member + 1, owner));
                        }
                        _ => self
                            .stack
                            .push(Frame::Operations(forms, index + 1, 0, owner)),
                    }
                }
            }
            Frame::FormSecurity(forms, index, owner) => {
                if index < forms.len() {
                    self.stack
                        .push(Frame::FormSecurity(forms, index + 1, owner));
                    if let Some(names) = forms[index].security.as_deref() {
                        self.stack.push(Frame::References(
                            Names::Strings(names),
                            0,
                            b::Site {
                                owner,
                                field: b::Field::FormSecurity,
                                index,
                                member: 0,
                            },
                        ));
                    }
                }
            }
            Frame::Schema(_) | Frame::Definition(_) => unreachable!(),
        }
        Ok(false)
    }
    fn push_schema(&mut self, node: &'td DataSchema, site: b::Site) -> Result<(), Cause> {
        let ordinal = self.next_schema;
        self.next_schema = add(ordinal, 1, self.phase)?;
        self.stack
            .push(Frame::Schema(Schema::new(node, site, ordinal)));
        Ok(())
    }
}

impl<'td> ValidatedThingCursor<'td> {
    fn global(&mut self, budget: &mut WorkBudget) -> Result<bool, Cause> {
        let access = bt::TypedBasicAccess(Some(self.thing));
        let kinds = [
            b::OwnerKind::Property,
            b::OwnerKind::Action,
            b::OwnerKind::Event,
        ];
        let counts = kinds.map(|kind| access.affordance_count(kind));
        let mut walk = self.walk;
        let action = walk.action(self.thing.security_definitions.len(), counts);
        let selected = match action {
            b::Action::SchemaMap(owner, _)
            | b::Action::Schema(owner, _)
            | b::Action::Operations(owner)
            | b::Action::FormSecurity(owner)
                if owner.kind != b::OwnerKind::Thing =>
            {
                Some(owner)
            }
            _ => None,
        };
        if let Some(owner) = selected {
            if self.current.is_none_or(|(old, _)| old != owner) {
                let index = kinds.iter().position(|&kind| kind == owner.kind).unwrap();
                if !self.pay(budget, &[(W::DocumentNodes, iter_cost(counts[index]))])? {
                    return Ok(false);
                }
                if self.affordances[index].is_none() {
                    self.affordances[index] = Some(access.affordances(owner.kind));
                    return Ok(false);
                }
                #[cfg(test)]
                {
                    self.trace.nexts += 1;
                }
                self.current = Some((
                    owner,
                    self.affordances[index]
                        .as_mut()
                        .unwrap()
                        .next()
                        .expect("inspected immutable count"),
                ));
                return Ok(false);
            }
        }
        if matches!(action, b::Action::Definition(_)) && self.definitions.is_none() {
            if !self.pay(
                budget,
                &[(
                    W::DocumentNodes,
                    iter_cost(self.thing.security_definitions.len()),
                )],
            )? {
                return Ok(false);
            }
            self.definitions = Some(self.thing.security_definitions.iter());
            return Ok(false);
        }
        let cost = if matches!(action, b::Action::Definition(_)) {
            iter_cost(self.thing.security_definitions.len())
        } else {
            1
        };
        if !self.pay(budget, &[(W::DocumentNodes, cost)])? {
            return Ok(false);
        }
        self.walk = walk;
        match action {
            b::Action::Title => b::required(
                !self
                    .thing
                    ._metadata
                    .title
                    .as_deref()
                    .unwrap_or("")
                    .is_empty(),
            )
            .map_err(|r| invalid(b::Site::new(b::ROOT, b::Field::Title), 0, r))?,
            b::Action::RequiredSecurity => b::required_security(self.thing.security.len())
                .map_err(|r| invalid(b::Site::new(b::ROOT, b::Field::Security), 0, r))?,
            b::Action::RootReferences => self.stack.push(Frame::References(
                Names::Strings(&self.thing.security),
                0,
                b::Site::new(b::ROOT, b::Field::Security),
            )),
            b::Action::Definition(owner) => {
                #[cfg(test)]
                {
                    self.trace.nexts += 1;
                }
                let (_, node) = self
                    .definitions
                    .as_mut()
                    .unwrap()
                    .next()
                    .expect("inspected immutable count");
                self.stack
                    .push(Frame::Definition(Definition::new(node, owner)));
            }
            b::Action::SchemaMap(owner, field) => {
                let map = if owner.kind == b::OwnerKind::Thing {
                    access.root_schema_map(field)
                } else {
                    access.uri_variables(self.current.unwrap().1)
                };
                if let Some(map) = map {
                    self.stack.push(Frame::SchemaMap {
                        map,
                        iter: None,
                        index: 0,
                        site: b::Site::new(owner, field),
                    });
                }
            }
            b::Action::Schema(owner, field) => {
                if let Some(node) = access.affordance_schema(self.current.unwrap().1, field) {
                    self.push_schema(node, b::Site::new(owner, field))?;
                }
            }
            b::Action::Operations(owner) | b::Action::FormSecurity(owner) => {
                if let Some(forms) = access.forms(if owner.kind == b::OwnerKind::Thing {
                    None
                } else {
                    Some(self.current.unwrap().1)
                }) {
                    if matches!(action, b::Action::Operations(_)) {
                        self.stack.push(Frame::Operations(forms, 0, 0, owner));
                    } else {
                        self.stack.push(Frame::FormSecurity(forms, 0, owner));
                    }
                }
            }
            b::Action::Done => return Ok(true),
        }
        self.walk.advance();
        Ok(false)
    }
    fn schema_tick(&mut self, budget: &mut WorkBudget) -> Result<bool, Cause> {
        let Frame::Schema(f) = self.stack.top() else {
            unreachable!()
        };
        let mut walk = f.walk;
        let action = walk.action(
            TypedAccess.one_of_count(f.node),
            TypedAccess.child_count(f.node),
        );
        let extras = &f.node.context()._extra_fields;
        let discovery = action == s::Action::Unsigned && !f.discovered;
        let costs = if discovery {
            if let Some((_, number)) = f.unsigned_pending {
                [
                    (W::DocumentNodes, 0),
                    (W::CodecInputBytes, number.as_str().len() as u64),
                    (W::JsonSchemaNodes, 1),
                ]
            } else if let Some((key, _)) = f.extra {
                [
                    (W::DocumentNodes, 1),
                    (
                        W::CodecInputBytes,
                        equal_cost(key, SCHEMA_KEYS[f.match_index]),
                    ),
                    (W::JsonSchemaNodes, 0),
                ]
            } else {
                [
                    (W::DocumentNodes, iter_cost(extras.len())),
                    (W::CodecInputBytes, 0),
                    (W::JsonSchemaNodes, 0),
                ]
            }
        } else if action == s::Action::Numeric {
            // NumericCursor chooses only its next pending projection. The probe
            // mutates a scalar copy; it performs no Number projection or scan.
            let mut numeric = f.numeric;
            let mut bytes = 0;
            let _ = numeric.step(f.numbers, |number| {
                bytes = number.as_str().len() as u64;
                ControlFlow::<(), Option<f64>>::Break(())
            });
            [
                (W::DocumentNodes, 0),
                (W::CodecInputBytes, bytes),
                (W::JsonSchemaNodes, 1),
            ]
        } else if matches!(action, s::Action::Child(_)) && matches!(f.node, DataSchema::Object(_)) {
            [
                (W::DocumentNodes, iter_cost(TypedAccess.child_count(f.node))),
                (W::CodecInputBytes, 0),
                (W::JsonSchemaNodes, 1),
            ]
        } else {
            let bytes = if action == s::Action::Type {
                TypedAccess
                    .data_type(f.node)
                    .map_or(0, |v| equal_cost(v, TypedAccess.kind(f.node).name()))
            } else {
                0
            };
            [
                (W::DocumentNodes, 0),
                (W::CodecInputBytes, bytes),
                (W::JsonSchemaNodes, 1),
            ]
        };
        if !self.pay(budget, &costs)? {
            return Ok(false);
        }
        let Frame::Schema(mut f) = self.stack.pop() else {
            unreachable!()
        };
        f.walk = walk;
        if discovery {
            if let Some((index, number)) = f.unsigned_pending.take() {
                f.unsigned[index] = number.as_u64();
                #[cfg(test)]
                {
                    self.trace.projections += 1;
                }
                f.match_index += 1;
            } else if f.extras.is_none() {
                f.extras = Some(extras.iter());
            } else if let Some((key, value)) = f.extra {
                if equal_fixed(key, SCHEMA_KEYS[f.match_index]) {
                    if f.match_index < 4 {
                        // AP integer projection parses text too. Discovery
                        // retains a handle and a separate whole debit pays it.
                        if let Some(number) = value.as_number() {
                            f.unsigned_pending = Some((f.match_index, number));
                        }
                    } else {
                        f.numbers[f.match_index - 4] = value.as_number();
                    }
                }
                if f.unsigned_pending.is_none() {
                    f.match_index += 1;
                }
                if f.match_index == SCHEMA_KEYS.len() {
                    f.extra = None;
                    f.match_index = 0;
                }
            } else {
                #[cfg(test)]
                {
                    self.trace.nexts += 1;
                }
                match f.extras.as_mut().unwrap().next() {
                    Some((k, v)) => f.extra = Some((k, v)),
                    None => f.discovered = true,
                }
            }
            self.stack.push(Frame::Schema(f));
            return Ok(false);
        }
        let reject = |r| schema_invalid(f.site, f.ordinal, r);
        match action {
            s::Action::Type => {
                // The full equal-length fixed comparison is prepaid. Feeding
                // its result into the shared rule avoids a second text read.
                let data_type = TypedAccess.data_type(f.node);
                let kind = TypedAccess.kind(f.node);
                let mismatch = data_type.is_some_and(|v| !equal_fixed(v, kind.name()));
                s::check_type(if mismatch { Some("") } else { None }, kind).map_err(reject)?;
            }
            s::Action::OneOf(index) => {
                let node = TypedAccess.one_of(f.node, index);
                let site = f.site;
                f.walk.advance();
                self.stack.push(Frame::Schema(f));
                self.push_schema(node, site)?;
                return Ok(false);
            }
            s::Action::Flags => s::check_flags(TypedAccess.flags(f.node)).map_err(reject)?,
            s::Action::Unsigned => {
                for pair in 0..2 {
                    s::check_unsigned_pair(pair, [f.unsigned[pair * 2], f.unsigned[pair * 2 + 1]])
                        .map_err(reject)?;
                }
            }
            s::Action::Numeric => {
                let mut projected = false;
                let result = f.numeric.step(f.numbers, |number| {
                    if projected {
                        return ControlFlow::Break(());
                    }
                    projected = true;
                    #[cfg(test)]
                    {
                        self.trace.projections += 1;
                    }
                    ControlFlow::Continue(number.as_f64())
                });
                // One fully charged at-most-L projection per action, with scalar
                // comparisons belonging to this schema charge. No credit carry.
                if let ControlFlow::Continue(result) = result {
                    result.map_err(reject)?;
                } else {
                    self.stack.push(Frame::Schema(f));
                    return Ok(false);
                }
            }
            s::Action::Typed => match TypedAccess.kind(f.node) {
                s::SchemaKind::Array | s::SchemaKind::String => {
                    let (min, max) = TypedAccess.typed_unsigned(f.node);
                    s::check_typed_unsigned(TypedAccess.kind(f.node), min, max).map_err(reject)?;
                }
                s::SchemaKind::Number => {
                    s::check_typed_numeric(&TypedAccess.typed_float(f.node)).map_err(reject)?
                }
                s::SchemaKind::Integer => {
                    s::check_typed_numeric(&TypedAccess.typed_integer(f.node)).map_err(reject)?
                }
                _ => {}
            },
            s::Action::Child(_) => {
                if f.children.is_none() {
                    f.children = Some(TypedAccess.children(f.node));
                    self.stack.push(Frame::Schema(f));
                    return Ok(false);
                }
                #[cfg(test)]
                {
                    self.trace.nexts += 1;
                }
                let (_, node) = f
                    .children
                    .as_mut()
                    .unwrap()
                    .next()
                    .expect("inspected immutable count");
                let site = f.site;
                f.walk.advance();
                self.stack.push(Frame::Schema(f));
                self.push_schema(node, site)?;
                return Ok(false);
            }
            s::Action::Done => return Ok(false),
        }
        f.walk.advance();
        self.stack.push(Frame::Schema(f));
        Ok(false)
    }
    fn definition_tick(&mut self, budget: &mut WorkBudget) -> Result<bool, Cause> {
        let Frame::Definition(f) = self.stack.top() else {
            unreachable!()
        };
        let extra = &bt::security_context(f.node)._extra_fields;
        let costs = if f.stage == 1 {
            if let Some((key, _)) = f.extra {
                [
                    (W::DocumentNodes, 1),
                    (
                        W::CodecInputBytes,
                        equal_cost(key, SECURITY_KEYS[f.match_index]),
                    ),
                    (W::SecurityBranches, 0),
                ]
            } else {
                [
                    (W::DocumentNodes, iter_cost(extra.len())),
                    (W::CodecInputBytes, 0),
                    (W::SecurityBranches, 0),
                ]
            }
        } else {
            // Scheme/flow vocabulary has a finite fixed set. Unlike source
            // names, these short comparisons can use one prepaid envelope.
            let text = if f.stage == 0 {
                f.node.scheme()
            } else if f.stage == 3 {
                f.flow
            } else {
                ""
            };
            let bytes = if text.len() <= 6 {
                2 * text.len() as u64 * 9
            } else {
                0
            };
            [
                (W::DocumentNodes, 0),
                (W::CodecInputBytes, bytes),
                (W::SecurityBranches, 1),
            ]
        };
        if !self.pay(budget, &costs)? {
            return Ok(false);
        }
        let Frame::Definition(mut f) = self.stack.pop() else {
            unreachable!()
        };
        let reject = |field, rule| invalid(b::Site::new(f.site.owner, field), 0, rule);
        match f.stage {
            0 => {
                // Length mismatch cannot match this vocabulary; no scan needed.
                f.scheme =
                    Some(b::scheme_kind(f.node.scheme()).map_err(|r| reject(b::Field::Scheme, r))?);
                f.stage = 1;
            }
            1 => {
                if f.extras.is_none() {
                    f.extras = Some(extra.iter());
                } else if let Some((key, value)) = f.extra {
                    if equal_fixed(key, SECURITY_KEYS[f.match_index]) {
                        match f.match_index {
                            0 => f.one = Names::value(value),
                            1 => f.all = Names::value(value),
                            2 => f.name = value.as_str(),
                            3 => f.flow = value.as_str().unwrap_or(""),
                            4 => f.authorization = value.as_str().is_some_and(|v| !v.is_empty()),
                            5 => f.token = value.as_str().is_some_and(|v| !v.is_empty()),
                            _ => unreachable!(),
                        }
                    }
                    f.match_index += 1;
                    if f.match_index == SECURITY_KEYS.len() {
                        f.extra = None;
                        f.match_index = 0;
                    }
                } else {
                    #[cfg(test)]
                    {
                        self.trace.nexts += 1;
                    }
                    if let Some((k, v)) = f.extras.as_mut().unwrap().next() {
                        f.extra = Some((k, v));
                    } else {
                        // Typed variant facts override extensions just as in the
                        // synchronous production adapter, even after mutation of
                        // the public scheme discriminator.
                        match f.node {
                            SecurityScheme::Combo(v) => {
                                f.one = Names::Strings(&v.one_of);
                                f.all = Names::Strings(&v.all_of);
                            }
                            SecurityScheme::APIKey(v) => f.name = v.name.as_deref(),
                            SecurityScheme::OAuth2(v) => {
                                f.flow = &v.flow;
                                f.authorization = v.authorization.is_some();
                                f.token = v.token.is_some();
                            }
                            _ => {}
                        }
                        f.stage = 2;
                    }
                }
            }
            2 => {
                if f.scheme == Some(b::Scheme::Combo) {
                    let names = if f.index == 0 { f.one } else { f.all };
                    if f.member < names.len() {
                        if let Some(name) = names.at(f.member) {
                            if name.is_empty() && f.empty[f.index].is_none() {
                                f.empty[f.index] = Some(f.counts[f.index]);
                            }
                            f.counts[f.index] += 1;
                        }
                        f.member += 1;
                    } else if f.index == 0 {
                        f.index = 1;
                        f.member = 0;
                    } else {
                        f.index = 0;
                        f.member = 0;
                        f.stage = 3;
                    }
                } else {
                    f.stage = 3;
                }
            }
            3 => {
                match f.scheme.unwrap() {
                    b::Scheme::Combo => {
                        b::combo_present(f.counts[0], f.counts[1])
                            .map_err(|r| reject(b::Field::Scheme, r))?;
                        // Each group's cardinality precedes its empty-member
                        // rejection; reference checks follow both groups.
                        for i in 0..2 {
                            b::combo_group(f.counts[i], f.empty[i]).map_err(|(member, r)| {
                                invalid(
                                    b::Site {
                                        owner: f.site.owner,
                                        field: if i == 0 {
                                            b::Field::OneOf
                                        } else {
                                            b::Field::AllOf
                                        },
                                        index: 0,
                                        member,
                                    },
                                    0,
                                    r,
                                )
                            })?;
                        }
                        f.stage = 4;
                    }
                    b::Scheme::ApiKey => {
                        b::required(!f.name.unwrap_or("").is_empty())
                            .map_err(|r| reject(b::Field::Name, r))?;
                        return Ok(false);
                    }
                    b::Scheme::OAuth => {
                        if b::flow_requires_endpoints(f.flow)
                            .map_err(|r| reject(b::Field::Flow, r))?
                        {
                            b::required(f.authorization)
                                .map_err(|r| reject(b::Field::Authorization, r))?;
                            b::required(f.token).map_err(|r| reject(b::Field::Token, r))?;
                        }
                        return Ok(false);
                    }
                    b::Scheme::Other => return Ok(false),
                }
            }
            4 => {
                f.stage = 5;
                let names = f.one;
                let site = b::Site::new(f.site.owner, b::Field::OneOf);
                self.stack.push(Frame::Definition(f));
                self.stack.push(Frame::References(names, 0, site));
                return Ok(false);
            }
            5 => {
                self.stack.push(Frame::References(
                    f.all,
                    0,
                    b::Site::new(f.site.owner, b::Field::AllOf),
                ));
                return Ok(false);
            }
            _ => unreachable!(),
        }
        self.stack.push(Frame::Definition(f));
        Ok(false)
    }
}
// The semantic owner keeps the validation owner, its ledger and its exact work
// remainder. It adds one current URI block, never a derived-document table.
use crate::data_type::uri::lending::{self as uri, Output};
use crate::{affordance::PropertyAffordance, data_type::Operation};

fn inline_bytes() -> u64 {
    // Provision movement/return overlap for either consuming validation or
    // semantic ownership. Inline capacity is not an allocator request.
    2 * mem::size_of::<ValidatedThingProgress<'static>>()
        .max(mem::size_of::<ValidatedPropertyReadCursor<'static>>()) as u64
}

#[derive(Clone, Copy, PartialEq)]
enum LendingPhase {
    Start,
    Property,
    PropertyReady,
    Form,
    Operations,
    Security,
    Names,
    Definitions,
    Compare,
    Scopes,
    Resolve,
    Allocate,
    Uri,
    Finish,
    FormReady,
    Done,
}
struct UriBlock {
    pointer: *mut u8,
    capacity: usize,
    len: usize,
}
impl Default for UriBlock {
    fn default() -> Self {
        Self {
            pointer: ptr::null_mut(),
            capacity: 0,
            len: 0,
        }
    }
}
impl Output for UriBlock {
    fn len(&self) -> usize {
        self.len
    }
    fn byte(&self, index: usize) -> u8 {
        assert!(index < self.len);
        unsafe { *self.pointer.add(index) }
    }
    fn push(&mut self, byte: u8) {
        assert!(self.len < self.capacity);
        unsafe {
            self.pointer.add(self.len).write(byte);
        }
        self.len += 1;
    }
    fn set(&mut self, index: usize, byte: u8) {
        assert!(index < self.len);
        unsafe {
            self.pointer.add(index).write(byte);
        }
    }
    fn truncate(&mut self, len: usize) {
        assert!(len <= self.len);
        self.len = len;
    }
}
impl UriBlock {
    fn text(&self) -> &str {
        if self.len == 0 {
            return "";
        }
        // Lent only at FormReady, after the program's charged ASCII pass. The
        // block is private; no mutation or owner move can coexist with this loan.
        unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(self.pointer, self.len))
        }
    }
    fn release(&mut self, ledger: &mut AdmissionLedger) {
        if self.capacity != 0 {
            let layout = Layout::array::<u8>(self.capacity).expect("checked allocation");
            unsafe {
                dealloc(self.pointer, layout);
            }
            assert!(ledger.release_temporary(layout.size() as u64));
            *self = Self::default();
        }
    }
}

/// Paid short loans from one validated input. Ready events remain stable until
/// acknowledgement. Moving/rewinding the owner cannot extend a scratch loan.
///
/// ```compile_fail
/// use clinkz_wot_td::{ValidatedPropertyReadCursor, ValidatedPropertyReadStep};
/// use clinkz_wot_foundation::WorkBudget;
/// fn retain(cursor: &mut ValidatedPropertyReadCursor<'_>, budget: &mut WorkBudget) {
///     let loan = cursor.step(budget, false).unwrap();
///     cursor.acknowledge();
///     if let ValidatedPropertyReadStep::Ready(_) = loan {}
/// }
/// ```
///
/// ```compile_fail
/// use clinkz_wot_td::{ValidatedPropertyReadCursor, ValidatedPropertyReadStep};
/// use clinkz_wot_foundation::WorkBudget;
/// fn move_ready(mut cursor: ValidatedPropertyReadCursor<'_>, budget: &mut WorkBudget) {
///     let loan = cursor.step(budget, false).unwrap();
///     let moved = cursor.rewind();
///     drop(loan);
///     drop(moved);
/// }
/// ```
///
/// ```compile_fail
/// use clinkz_wot_td::ValidatedPropertyReadCursor;
/// fn refill(cursor: ValidatedPropertyReadCursor<'_>) { let duplicate = cursor.clone(); }
/// ```
pub struct ValidatedPropertyReadCursor<'td> {
    owner: ValidatedThingCursor<'td>,
    phase: LendingPhase,
    cause: Option<Cause>,
    properties: Option<btree_map::Iter<'td, String, PropertyAffordance>>,
    property: Option<(&'td str, &'td PropertyAffordance)>,
    ordinal: u64,
    form: usize,
    operation: usize,
    readable: bool,
    names: &'td [String],
    name_index: usize,
    definitions: Option<btree_map::Iter<'td, String, SecurityScheme>>,
    definition: Option<(&'td str, &'td SecurityScheme)>,
    compare: usize,
    scheme: Option<&'td str>,
    scope: usize,
    scope_bytes: u64,
    copy_bytes: u64,
    alias: Option<&'td str>,
    resolver: Option<uri::Resolution<'td>>,
    scratch: UriBlock,
    effective: u64,
}
/// One result of a bounded semantic poll.
pub enum ValidatedPropertyReadStep<'step> {
    Pending,
    Ready(ValidatedPropertyReadEvent<'step>),
    Done,
}
/// Original property order and readable Form coordinates.
pub enum ValidatedPropertyReadEvent<'step> {
    Property { ordinal: u32, name: &'step str },
    Form(ValidatedPropertyReadForm<'step>),
}
/// Fully paid immutable facts. Planning decides eligibility and owns copies.
pub struct ValidatedPropertyReadForm<'step> {
    ordinal: u32,
    name: &'step str,
    index: u32,
    form: &'step Form,
    resolved: &'step str,
    names: &'step [String],
    scheme: Option<&'step str>,
    scope_bytes: u64,
    copy_bytes: u64,
}
impl<'step> ValidatedPropertyReadForm<'step> {
    pub fn property_ordinal(&self) -> u32 {
        self.ordinal
    }
    pub fn property_name(&self) -> &'step str {
        self.name
    }
    pub fn original_index(&self) -> u32 {
        self.index
    }
    pub fn href(&self) -> &'step str {
        self.form.href.as_str()
    }
    pub fn resolved_href(&self) -> &'step str {
        self.resolved
    }
    pub fn content_type(&self) -> &'step str {
        &self.form.content_type
    }
    pub fn content_coding(&self) -> Option<&'step str> {
        self.form.content_coding.as_deref()
    }
    pub fn subprotocol(&self) -> Option<&'step str> {
        self.form.subprotocol.as_deref()
    }
    pub fn scopes(&self) -> ValidatedTextSequence<'step> {
        ValidatedTextSequence {
            source: self.form.scopes.as_deref().unwrap_or(&[]),
            bytes: self.scope_bytes,
        }
    }
    pub fn readable(&self) -> bool {
        true
    }
    pub fn security_count(&self) -> u64 {
        self.names.len() as u64
    }
    pub fn security_name(&self) -> Option<&'step str> {
        if self.names.len() == 1 {
            Some(self.names[0].as_str())
        } else {
            None
        }
    }
    pub fn security_scheme(&self) -> Option<&'step str> {
        self.scheme
    }
    pub fn copy_bytes(&self) -> u64 {
        self.copy_bytes
    }
}
/// Paid sequence size; the receiver pays each advance/copy before using it.
pub struct ValidatedTextSequence<'step> {
    source: &'step [String],
    bytes: u64,
}
impl<'step> ValidatedTextSequence<'step> {
    pub fn len(&self) -> usize {
        self.source.len()
    }
    pub fn byte_len(&self) -> u64 {
        self.bytes
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &'step str> + 'step {
        self.source.iter().map(String::as_str)
    }
}
impl<'td> ValidatedThing<'td> {
    pub fn into_property_read(mut self) -> ValidatedPropertyReadCursor<'td> {
        self.owner.phase = Phase::Semantics;
        let effective = self.owner.counts.content;
        ValidatedPropertyReadCursor {
            owner: self.owner,
            phase: LendingPhase::Start,
            cause: None,
            properties: None,
            property: None,
            ordinal: 0,
            form: 0,
            operation: 0,
            readable: false,
            names: &[],
            name_index: 0,
            definitions: None,
            definition: None,
            compare: 0,
            scheme: None,
            scope: 0,
            scope_bytes: 0,
            copy_bytes: 0,
            alias: None,
            resolver: None,
            scratch: UriBlock::default(),
            effective,
        }
    }
}
impl<'td> ValidatedPropertyReadCursor<'td> {
    pub fn id(&self) -> Option<&str> {
        self.owner.thing.id.as_ref().map(|id| id.as_str())
    }
    pub fn acknowledge(&mut self) {
        match self.phase {
            LendingPhase::PropertyReady => self.phase = LendingPhase::Form,
            LendingPhase::FormReady => {
                self.form += 1;
                self.phase = LendingPhase::Form;
            }
            _ => {}
        }
    }
    pub fn rewind(mut self) -> Self {
        if self.phase == LendingPhase::Done {
            self.phase = LendingPhase::Start;
            self.properties = None;
            self.property = None;
            self.ordinal = 0;
            self.effective = self.owner.counts.content;
        }
        self
    }
    pub fn step(
        &mut self,
        budget: &mut WorkBudget,
        cancel_requested: bool,
    ) -> Result<ValidatedPropertyReadStep<'_>, Cause> {
        if let Some(cause) = self.cause {
            return Err(cause);
        }
        if self.phase == LendingPhase::Done {
            return Ok(ValidatedPropertyReadStep::Done);
        }
        if cancel_requested {
            let cause = Cause::Cancelled {
                phase: Phase::Semantics,
            };
            self.cause = Some(cause);
            return Err(cause);
        }
        if !matches!(
            self.phase,
            LendingPhase::PropertyReady | LendingPhase::FormReady
        ) {
            if budget.is_exhausted() {
                return Ok(ValidatedPropertyReadStep::Pending);
            }
            if let Err(cause) = self.tick_semantics(budget) {
                self.cause = Some(cause);
                return Err(cause);
            }
        }
        let event = match self.phase {
            LendingPhase::Done => return Ok(ValidatedPropertyReadStep::Done),
            LendingPhase::PropertyReady => ValidatedPropertyReadEvent::Property {
                ordinal: self.ordinal as u32,
                name: self.property.unwrap().0,
            },
            LendingPhase::FormReady => {
                ValidatedPropertyReadEvent::Form(ValidatedPropertyReadForm {
                    ordinal: self.ordinal as u32,
                    name: self.property.unwrap().0,
                    index: self.form as u32,
                    form: self.current_form(),
                    resolved: self.alias.unwrap_or_else(|| self.scratch.text()),
                    names: self.names,
                    scheme: self.scheme,
                    scope_bytes: self.scope_bytes,
                    copy_bytes: self.copy_bytes,
                })
            }
            _ => return Ok(ValidatedPropertyReadStep::Pending),
        };
        Ok(ValidatedPropertyReadStep::Ready(event))
    }
    fn current_form(&self) -> &'td Form {
        &self.property.unwrap().1._interaction.forms[self.form]
    }
    fn uri_error(&self) -> Cause {
        let mut site = b::Site::new(
            b::Owner {
                kind: b::OwnerKind::Property,
                ordinal: self.ordinal as usize,
            },
            b::Field::FormHref,
        );
        site.index = self.form;
        Cause::Invalid(ValidatedThingInvalid {
            kind: ValidatedThingInvalidKind::InvalidUri,
            phase: Phase::Semantics,
            node_ordinal: self.ordinal,
            site,
            rule: Rule::Uri,
        })
    }
    fn allocate_uri(&mut self) -> Result<(), Cause> {
        // A fixed upper bound from already inspected lengths, including merge
        // coexistence and the authority-less '/.' escape. Only the *finished*
        // target is subject to the resolved-target ceiling: reducible paths
        // may temporarily be longer, and must not be falsely rejected as URIs.
        let bytes = add(
            add(
                self.owner
                    .thing
                    .base
                    .as_ref()
                    .map_or(0, |b| b.as_str().len()) as u64,
                self.current_form().href.as_str().len() as u64,
                Phase::Semantics,
            )?,
            4,
            Phase::Semantics,
        )?;
        let capacity = usize::try_from(bytes).map_err(|_| arithmetic(Phase::Semantics))?;
        let layout = Layout::array::<u8>(capacity).map_err(|_| arithmetic(Phase::Semantics))?;
        if capacity <= self.scratch.capacity {
            self.scratch.len = 0;
            return Ok(());
        }
        // The previous target is no longer lent. Its block release was prepaid;
        // release before the new acquisition, with no unnecessary copy/growth.
        self.scratch.release(&mut self.owner.stack.ledger);
        let retained = Layout::array::<Frame<'td>>(self.owner.stack.frames.capacity)
            .unwrap()
            .size() as u64;
        let temporary = add(
            add(retained, bytes, Phase::Semantics)?,
            inline_bytes(),
            Phase::Semantics,
        )?;
        for kind in [
            R::AdmissionTemporaryBytesPerOperationMax,
            R::AdmissionTemporaryBytesGlobalMax,
            R::PeakLiveBytesPerAdmissionMax,
        ] {
            self.owner.policy.check(kind, temporary, Phase::Semantics)?;
        }
        let live = add(
            add(
                self.owner.stack.ledger.live_bytes(),
                bytes,
                Phase::Semantics,
            )?,
            inline_bytes(),
            Phase::Semantics,
        )?;
        for kind in [
            R::AdmissionPeakLiveBytesGlobalMax,
            R::EngineLiveBytesGlobalMax,
        ] {
            self.owner.policy.check(kind, live, Phase::Semantics)?;
        }
        self.owner.policy.check(
            R::LargestContiguousAllocationBytesMax,
            bytes,
            Phase::Semantics,
        )?;
        let reservation = self
            .owner
            .stack
            .ledger
            .try_reserve_temporary(R::AdmissionTemporaryBytesPerOperationMax, bytes)
            .ok_or(Cause::Limit(ValidatedThingLimit {
                kind: R::AdmissionTemporaryBytesPerOperationMax,
                configured: self
                    .owner
                    .policy
                    .get(R::AdmissionTemporaryBytesPerOperationMax),
                observed: temporary,
                phase: Phase::Semantics,
            }))?;
        let pointer = unsafe { alloc(layout) };
        if pointer.is_null() {
            return Err(Cause::Failed(ValidatedThingFailure {
                kind: ValidatedThingFailureKind::AllocationFailed,
                phase: Phase::Semantics,
                requested_bytes: bytes,
            }));
        }
        reservation.commit();
        self.scratch = UriBlock {
            pointer,
            capacity,
            len: 0,
        };
        Ok(())
    }
    fn tick_semantics(&mut self, budget: &mut WorkBudget) -> Result<(), Cause> {
        use LendingPhase as P;
        let costs = match self.phase {
            P::Start | P::Property => [
                (
                    W::DocumentNodes,
                    iter_cost(
                        self.owner
                            .thing
                            .properties
                            .as_ref()
                            .map_or(0, BTreeMap::len),
                    ),
                ),
                (W::CodecInputBytes, 0),
            ],
            P::Definitions => [
                (
                    W::DocumentNodes,
                    iter_cost(self.owner.thing.security_definitions.len()),
                ),
                (W::SecurityBranches, 1),
            ],
            P::Compare => [(W::DocumentNodes, 0), (W::CodecInputBytes, 2)],
            P::Security | P::Names => [(W::DocumentNodes, 1), (W::SecurityBranches, 1)],
            P::Uri => [
                (W::UriBytes, self.resolver.as_ref().unwrap().work()),
                (W::DocumentNodes, 0),
            ],
            P::Resolve => [(W::UriBytes, 1), (W::DocumentNodes, 0)],
            P::Allocate => [(W::DocumentNodes, 1), (W::CleanupItems, 1)],
            _ => [(W::DocumentNodes, 1), (W::CodecInputBytes, 0)],
        };
        if !self.owner.pay(budget, &costs)? {
            return Ok(());
        }
        match self.phase {
            P::Start => {
                self.properties = self.owner.thing.properties.as_ref().map(BTreeMap::iter);
                self.phase = P::Property;
            }
            P::Property => {
                self.property = self
                    .properties
                    .as_mut()
                    .and_then(Iterator::next)
                    .map(|(name, p)| (name.as_str(), p));
                if self.property.is_some() {
                    self.form = 0;
                    self.phase = P::PropertyReady;
                } else {
                    self.phase = P::Done;
                }
            }
            P::Form => {
                if self.form == self.property.unwrap().1._interaction.forms.len() {
                    self.ordinal += 1;
                    self.phase = P::Property;
                } else {
                    self.operation = 0;
                    self.readable = false;
                    self.phase = P::Operations;
                }
            }
            P::Operations => {
                let form = self.current_form();
                let ops = crate::td_defaults::effective_form_operations(
                    crate::td_defaults::FormContext::Property(self.property.unwrap().1),
                    form,
                );
                if self.operation < ops.len() {
                    self.readable |= ops[self.operation] == Operation::ReadProperty;
                    self.operation += 1;
                } else if self.readable {
                    self.phase = P::Security;
                } else {
                    self.form += 1;
                    self.phase = P::Form;
                }
            }
            P::Security => {
                self.names = crate::td_defaults::effective_form_security(
                    self.owner.thing,
                    self.current_form(),
                );
                self.name_index = 0;
                self.scheme = None;
                self.phase = P::Names;
            }
            P::Names => {
                if self.name_index < self.names.len() {
                    self.name_index += 1;
                } else if self.names.len() == 1 {
                    // Initialization uses the same conservative native envelope
                    // as advance, separately paid in Definitions.
                    self.definitions = None;
                    self.phase = P::Definitions;
                } else {
                    self.scope = 0;
                    self.scope_bytes = 0;
                    self.phase = P::Scopes;
                }
            }
            P::Definitions => {
                if self.definitions.is_none() {
                    self.definitions = Some(self.owner.thing.security_definitions.iter());
                } else {
                    self.definition = self
                        .definitions
                        .as_mut()
                        .unwrap()
                        .next()
                        .map(|(n, d)| (n.as_str(), d));
                    let (name, definition) =
                        self.definition.expect("Basic proved reference existence");
                    if name.len() == self.names[0].len() {
                        self.compare = 0;
                        if name.is_empty() {
                            self.scheme = Some(&bt::security_context(definition).scheme);
                            self.scope = 0;
                            self.scope_bytes = 0;
                            self.phase = P::Scopes;
                        } else {
                            self.phase = P::Compare;
                        }
                    }
                }
            }
            P::Compare => {
                let (name, definition) = self.definition.unwrap();
                if name.as_bytes()[self.compare] != self.names[0].as_bytes()[self.compare] {
                    self.phase = P::Definitions;
                } else {
                    self.compare += 1;
                    if self.compare == name.len() {
                        self.scheme = Some(&bt::security_context(definition).scheme);
                        self.scope = 0;
                        self.scope_bytes = 0;
                        self.phase = P::Scopes;
                    }
                }
            }
            P::Scopes => {
                let scopes = self.current_form().scopes.as_deref().unwrap_or(&[]);
                if self.scope < scopes.len() {
                    self.scope_bytes = add(
                        self.scope_bytes,
                        scopes[self.scope].len() as u64,
                        Phase::Semantics,
                    )?;
                    self.scope += 1;
                } else {
                    self.phase = P::Resolve;
                }
            }
            P::Resolve => {
                match uri::Target::new(self.owner.thing.base.as_ref(), &self.current_form().href)
                    .map_err(|_| self.uri_error())?
                {
                    uri::Target::Alias(raw) => {
                        self.alias = Some(raw);
                        self.resolver = None;
                        self.phase = P::Finish;
                    }
                    uri::Target::Resolve(resolver) => {
                        self.alias = None;
                        self.resolver = Some(resolver);
                        self.phase = P::Allocate;
                    }
                }
            }
            P::Allocate => {
                self.allocate_uri()?;
                self.phase = P::Uri;
            }
            P::Uri => {
                if self.resolver.as_mut().unwrap().tick(&mut self.scratch) {
                    self.phase = P::Finish;
                }
            }
            P::Finish => {
                let resolved_len = self.alias.map_or(self.scratch.len, str::len) as u64;
                self.owner.policy.check(
                    R::UriTemplateSourceBytesMax,
                    resolved_len,
                    Phase::Semantics,
                )?;
                let effective = add(
                    self.effective,
                    if self.alias.is_none() {
                        resolved_len
                    } else {
                        0
                    },
                    Phase::Semantics,
                )?;
                self.owner.policy.check(
                    R::GeneratedEffectiveDocumentBytesMax,
                    effective,
                    Phase::Semantics,
                )?;
                let form = self.current_form();
                let mut copy = self.scope_bytes;
                for length in [
                    self.id().map_or(0, str::len),
                    self.property.unwrap().0.len(),
                    form.href.as_str().len(),
                    resolved_len as usize,
                    form.content_type.len(),
                    form.content_coding.as_deref().map_or(0, str::len),
                    form.subprotocol.as_deref().map_or(0, str::len),
                ] {
                    copy = add(copy, length as u64, Phase::Semantics)?;
                }
                self.copy_bytes = copy;
                self.effective = effective;
                self.phase = P::FormReady;
            }
            P::PropertyReady | P::FormReady | P::Done => unreachable!("handled before debit"),
        }
        Ok(())
    }
}
impl Drop for ValidatedPropertyReadCursor<'_> {
    fn drop(&mut self) {
        // Only one trivial byte block plus Stack's fixed catalog. No caller TD
        // destruction, recursive output drop, allocation or unpaid traversal.
        self.scratch.release(&mut self.owner.stack.ledger);
    }
}

#[cfg(test)]
#[derive(Default, Clone, Copy, Debug, Eq, PartialEq)]
struct Trace {
    work: [u64; 12],
    entered: u64,
    nexts: u64,
    reads: u64,
    checksum: u64,
    scalar_checksum: u64,
    moves: u64,
    compared: u64,
    projections: u64,
}

#[cfg(test)]
#[path = "../tests/support/validated_admission_private.rs"]
mod tests;
