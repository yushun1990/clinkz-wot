//! Source-independent meaning of supported Property collection data.
//! Native representation and aggregation remain codec/Binding/application work.
use crate::{CoreResult, value_type::TypeSchema};
use alloc::{boxed::Box, vec::Vec};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyMemberSchema {
    pub name: Box<str>,
    pub schema: TypeSchema,
}

/// Eligible member schemas, derived independently for each data role. Read
/// schemas exclude writeOnly; write schemas exclude readOnly. All operations
/// require every eligible member exactly once; selected operations carry a
/// separate per-call set. Excluded members are never optional unvalidated values. This is the bounded type-only
/// subset, not a general DataSchema tree or a snapshot/atomicity promise.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyCollectionSchema {
    members: Vec<PropertyMemberSchema>,
}
/// A bounded set of indices into one retained PropertyCollectionSchema. This
/// value has meaning only with that original schema; it owns no names or TD.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
