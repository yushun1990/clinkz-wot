//! Source-independent meaning of the supported Property collection response.
//! Native representation and aggregation remain codec/Binding/application work.
use crate::{CoreResult, value_type::TypeSchema};
use alloc::{boxed::Box, vec::Vec};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyMemberSchema {
    pub name: Box<str>,
    pub schema: TypeSchema,
}

/// Every member is required exactly once. Undescribed and writeOnly members
/// are excluded, not unvalidated optional values. This is the bounded type-only
/// subset, not a general DataSchema tree or a snapshot/atomicity promise.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyCollectionSchema {
    members: Vec<PropertyMemberSchema>,
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
