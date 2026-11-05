//! Source-independent meaning of supported Property and Event collection data.
//! Native representation and aggregation remain codec/Binding/application work.
use crate::{CoreResult, value_type::TypeSchema};
use alloc::{boxed::Box, vec::Vec};

/// One Event's roles. Absence of data is distinct from the JSON null schema;
/// a dataResponse schema alone grants no native reply opportunity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventMemberSchema {
    pub name: Box<str>,
    pub subscription: Option<TypeSchema>,
    pub data: Option<TypeSchema>,
    pub data_response: Option<TypeSchema>,
    pub cancellation: Option<TypeSchema>,
}
/// Original named Event roles, not a Property map or a set of leaf drivers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventCollectionSchema {
    members: Vec<EventMemberSchema>,
}
impl EventCollectionSchema {
    pub const MAX_MEMBERS: usize = 32;
    pub const MAX_NAME_BYTES: usize = 256;
    pub const MAX_TOTAL_NAME_BYTES: usize = 4096;
    pub fn try_new(members: Vec<EventMemberSchema>) -> CoreResult<Self> {
        if members.len() > Self::MAX_MEMBERS
            || members.capacity() > Self::MAX_MEMBERS
            || members.iter().map(|m| m.name.len()).sum::<usize>() > Self::MAX_TOTAL_NAME_BYTES
            || members.iter().enumerate().any(|(i, m)| {
                m.name.len() > Self::MAX_NAME_BYTES
                    || members[..i].iter().any(|old| old.name == m.name)
            })
        {
            return Err(invalid_selection());
        }
        Ok(Self { members })
    }
    pub fn members(&self) -> &[EventMemberSchema] {
        &self.members
    }
    pub fn member(&self, name: &str) -> Option<&EventMemberSchema> {
        self.members.iter().find(|m| m.name.as_ref() == name)
    }
    pub fn retained_bytes(&self) -> usize {
        self.members.capacity() * core::mem::size_of::<EventMemberSchema>()
            + self.members.iter().map(|m| m.name.len()).sum::<usize>()
    }
    pub const fn backing_bound() -> usize {
        Self::MAX_MEMBERS * core::mem::size_of::<EventMemberSchema>() + Self::MAX_TOTAL_NAME_BYTES
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyMemberSchema {
    pub name: Box<str>,
    pub schema: TypeSchema,
}

/// Eligible member schemas, derived independently for each data role. Read
/// schemas exclude writeOnly; write schemas exclude readOnly. Finite all-member
/// operations and complete observation samples require every eligible member
/// exactly once; selected operations carry a separate per-call set. Excluded
/// members are never optional unvalidated values. This is the bounded type-only
/// subset, not a general DataSchema tree or a snapshot/atomicity promise.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyCollectionSchema {
    members: Vec<PropertyMemberSchema>,
}
/// A bounded set of indices into one retained PropertyCollectionSchema. This
/// value has meaning only with that original schema; it owns no names or TD.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct PropertySelection(u32);
impl PropertySelection {
    pub fn contains(self, index: usize) -> bool {
        index < 32 && self.0 & (1u32 << index) != 0
    }
    pub fn bits(self) -> u32 {
        self.0
    }
}
impl PropertyCollectionSchema {
    pub const MAX_MEMBERS: usize = 32;
    pub const MAX_NAME_BYTES: usize = 256;
    pub const MAX_TOTAL_NAME_BYTES: usize = 4096;
    pub fn try_new(members: Vec<PropertyMemberSchema>) -> CoreResult<Self> {
        if members.len() > Self::MAX_MEMBERS
            || members.capacity() > Self::MAX_MEMBERS
            || members.iter().map(|m| m.name.len()).sum::<usize>() > Self::MAX_TOTAL_NAME_BYTES
            || members.iter().enumerate().any(|(i, m)| {
                m.name.len() > Self::MAX_NAME_BYTES
                    || members[..i].iter().any(|old| old.name == m.name)
            })
        {
            return Err(crate::CoreError::Validation(crate::ErrorContext::new(
                crate::ErrorPhase::Admission,
                crate::RetryClass::Never,
            )));
        }
        Ok(Self { members })
    }
    pub fn members(&self) -> &[PropertyMemberSchema] {
        &self.members
    }
    pub fn all(&self) -> PropertySelection {
        PropertySelection(((1u64 << self.members.len()) - 1) as u32)
    }
    pub fn selection_from_bits(&self, bits: u32) -> CoreResult<PropertySelection> {
        if bits & !self.all().bits() != 0 {
            return Err(invalid_selection());
        }
        Ok(PropertySelection(bits))
    }
    /// Exact names, without trimming, normalization or deduplication. Empty is
    /// a valid selection and requires an empty result. The caller funds the
    /// bounded name walk before entry; no allocation or codec work occurs here.
    pub fn select(&self, names: &[&str]) -> CoreResult<PropertySelection> {
        if names.len() > Self::MAX_MEMBERS {
            return Err(invalid_selection());
        }
        let mut bits = 0;
        let mut bytes = 0;
        for name in names {
            bytes += name.len().min(Self::MAX_NAME_BYTES + 1);
            if name.len() > Self::MAX_NAME_BYTES || bytes > Self::MAX_TOTAL_NAME_BYTES {
                return Err(invalid_selection());
            }
            let index = self
                .members
                .iter()
                .position(|m| m.name.as_ref() == *name)
                .ok_or_else(invalid_selection)?;
            let bit = 1u32 << index;
            if bits & bit != 0 {
                return Err(invalid_selection());
            }
            bits |= bit;
        }
        Ok(PropertySelection(bits))
    }
    pub fn retained_bytes(&self) -> usize {
        self.members.capacity() * core::mem::size_of::<PropertyMemberSchema>()
            + self.members.iter().map(|m| m.name.len()).sum::<usize>()
    }
    /// Bound for preparation scratch and complete retained backing before
    /// construction/cloning. The concrete codec bounds traversal separately.
    pub const fn backing_bound() -> usize {
        Self::MAX_MEMBERS * core::mem::size_of::<PropertyMemberSchema>()
            + Self::MAX_TOTAL_NAME_BYTES
    }
}

fn invalid_selection() -> crate::CoreError {
    crate::CoreError::Validation(crate::ErrorContext::new(
        crate::ErrorPhase::Admission,
        crate::RetryClass::Never,
    ))
}

/// Evidence supplied by the selected native mapping/application, never inferred
/// from one request, a successful member, or local cancellation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum WriteAtomicity {
    Unspecified,
    Atomic,
    NonAtomic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemberWriteEffect {
    Completed,
    NotApplied,
    Unknown,
}

/// Bounded terminal member evidence for one original writable schema/selection.
/// Unclassified selected members remain Unknown. Native status is opaque
/// provenance and cannot establish an effect. This carrier adds no allocation,
/// arbitrary diagnostic tree, transaction coordinator or replay authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct PropertyWriteReport {
    selected: PropertySelection,
    completed: u32,
    not_applied: u32,
    atomicity: WriteAtomicity,
    status_present: u32,
    statuses: [u16; PropertyCollectionSchema::MAX_MEMBERS],
}
impl PropertyWriteReport {
    pub fn try_new(
        selected: PropertySelection,
        completed: u32,
        not_applied: u32,
        atomicity: WriteAtomicity,
    ) -> CoreResult<Self> {
        let bits = selected.bits();
        if (completed | not_applied) & !bits != 0
            || completed & not_applied != 0
            || (atomicity == WriteAtomicity::Atomic
                && ((completed != 0 && completed != bits)
                    || (not_applied != 0 && not_applied != bits)))
        {
            return Err(invalid_report());
        }
        Ok(Self {
            selected,
            completed,
            not_applied,
            atomicity,
            status_present: 0,
            statuses: [0; PropertyCollectionSchema::MAX_MEMBERS],
        })
    }
    pub fn with_native_status(mut self, index: usize, status: u16) -> CoreResult<Self> {
        if !self.selected.contains(index) || self.status_present & (1u32 << index) != 0 {
            return Err(invalid_report());
        }
        self.status_present |= 1u32 << index;
        self.statuses[index] = status;
        Ok(self)
    }
    pub fn selection(&self) -> PropertySelection {
        self.selected
    }
    pub fn effect(&self, index: usize) -> Option<MemberWriteEffect> {
        if !self.selected.contains(index) {
            None
        } else if self.completed & (1u32 << index) != 0 {
            Some(MemberWriteEffect::Completed)
        } else if self.not_applied & (1u32 << index) != 0 {
            Some(MemberWriteEffect::NotApplied)
        } else {
            Some(MemberWriteEffect::Unknown)
        }
    }
    pub fn native_status(&self, index: usize) -> Option<u16> {
        (index < PropertyCollectionSchema::MAX_MEMBERS
            && self.status_present & (1u32 << index) != 0)
            .then(|| self.statuses[index])
    }
    pub fn atomicity(&self) -> WriteAtomicity {
        self.atomicity
    }
    pub fn is_complete(&self) -> bool {
        self.completed == self.selected.bits()
    }
    /// Exact original invocation association; a later call's set cannot replace it.
    pub fn validate_selection(&self, selected: PropertySelection) -> CoreResult<()> {
        if self.selected == selected {
            Ok(())
        } else {
            Err(invalid_report())
        }
    }
    /// Incomplete native reports are evidence, not whole-group success.
    pub fn completion(&self) -> CoreResult<()> {
        if self.is_complete() {
            Ok(())
        } else {
            Err(crate::CoreError::Application(
                crate::ErrorContext::new(
                    crate::ErrorPhase::Binding,
                    crate::RetryClass::CallerDecision,
                )
                .with_redacted_cause(626, "collection write has incomplete member outcomes"),
            ))
        }
    }
}

fn invalid_report() -> crate::CoreError {
    crate::CoreError::Validation(crate::ErrorContext::new(
        crate::ErrorPhase::Validate,
        crate::RetryClass::Never,
    ))
}
