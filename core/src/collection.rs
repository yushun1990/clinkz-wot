//! Source-independent meaning of supported Property and Event collection data.
//! Native representation and aggregation remain codec/Binding/application work.
use crate::{CoreResult, data_schema::PreparedSchema};
use alloc::{boxed::Box, vec::Vec};

/// One Event's roles. Absence of data is distinct from the JSON null schema;
/// a dataResponse schema alone grants no native reply opportunity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventMemberSchema {
    pub name: Box<str>,
    pub subscription: Option<PreparedSchema>,
    pub data: Option<PreparedSchema>,
    pub data_response: Option<PreparedSchema>,
    pub cancellation: Option<PreparedSchema>,
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
        let schemas = || members.iter().flat_map(EventMemberSchema::schemas);
        if members.len() > Self::MAX_MEMBERS
            || members.capacity() > Self::MAX_MEMBERS
            || members.iter().map(|m| m.name.len()).sum::<usize>() > Self::MAX_TOTAL_NAME_BYTES
            || members.iter().enumerate().any(|(i, m)| {
                m.name.len() > Self::MAX_NAME_BYTES
                    || members[..i].iter().any(|old| old.name == m.name)
            })
            || schemas().map(PreparedSchema::node_count).sum::<usize>() > Self::MAX_SCHEMA_NODES
            || schemas().map(PreparedSchema::name_bytes).sum::<usize>()
                > PreparedSchema::MAX_TOTAL_NAME_BYTES
            || schemas().map(PreparedSchema::retained_bytes).sum::<usize>()
                > Self::SCHEMA_BACKING_BOUND
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
            + self
                .members
                .iter()
                .flat_map(EventMemberSchema::schemas)
                .map(PreparedSchema::retained_bytes)
                .sum::<usize>()
    }
    // Preserve the original 32 x four scalar roles while bounding combined
    // variable constraints, rather than multiplying a full tree per member.
    pub const MAX_SCHEMA_NODES: usize = Self::MAX_MEMBERS * 4;
    pub const SCHEMA_BACKING_BOUND: usize = Self::MAX_SCHEMA_NODES
        * core::mem::size_of::<crate::data_schema::SchemaProperty>()
        + PreparedSchema::MAX_TOTAL_NAME_BYTES;
    pub const fn backing_bound() -> usize {
        Self::MAX_MEMBERS * core::mem::size_of::<EventMemberSchema>()
            + Self::MAX_TOTAL_NAME_BYTES
            + Self::SCHEMA_BACKING_BOUND
    }
}
impl EventMemberSchema {
    pub fn schemas(&self) -> impl Iterator<Item = &PreparedSchema> {
        [
            &self.subscription,
            &self.data,
            &self.data_response,
            &self.cancellation,
        ]
        .into_iter()
        .flatten()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyMemberSchema {
    pub name: Box<str>,
    pub schema: PreparedSchema,
}

/// Eligible member schemas, derived independently for each data role. Read
/// schemas exclude writeOnly; write schemas exclude readOnly. Finite all-member
/// operations and complete observation samples require every eligible member
/// exactly once; selected operations carry a separate per-call set. Excluded
/// members are never optional unvalidated values. Each member retains the common
/// bounded DataSchema meaning; this supplies no snapshot/atomicity promise.
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
        if members.iter().map(|m| m.schema.node_count()).sum::<usize>() > PreparedSchema::MAX_NODES
            || members.iter().map(|m| m.schema.name_bytes()).sum::<usize>()
                > PreparedSchema::MAX_TOTAL_NAME_BYTES
            || members
                .iter()
                .map(|m| m.schema.retained_bytes())
                .sum::<usize>()
                > PreparedSchema::BACKING_BOUND
            || members.len() > Self::MAX_MEMBERS
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
            + self
                .members
                .iter()
                .map(|m| m.name.len() + m.schema.retained_bytes())
                .sum::<usize>()
    }
    /// Bound for preparation scratch and complete retained backing before
    /// construction/cloning. The concrete codec bounds traversal separately.
    pub const fn backing_bound() -> usize {
        Self::MAX_MEMBERS * core::mem::size_of::<PropertyMemberSchema>()
            + Self::MAX_TOTAL_NAME_BYTES
            + PreparedSchema::BACKING_BOUND
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

#[cfg(test)]
mod schema_tests {
    use super::*;
    use crate::data_schema::{SchemaProperty, ValueType};
    use alloc::{format, vec};

    fn events(fields: usize) -> Vec<EventMemberSchema> {
        (0..EventCollectionSchema::MAX_MEMBERS)
            .map(|i| EventMemberSchema {
                name: format!("event{i}").into(),
                subscription: None,
                data_response: None,
                cancellation: None,
                data: Some(
                    PreparedSchema::object(
                        (0..fields)
                            .map(|i| SchemaProperty {
                                name: format!("field{i}").into(),
                                required: true,
                                schema: PreparedSchema::Type(ValueType::Integer),
                            })
                            .collect(),
                    )
                    .unwrap(),
                ),
            })
            .collect()
    }
    fn named_fields(count: usize, name_bytes: usize) -> PreparedSchema {
        PreparedSchema::object(
            (0..count)
                .map(|i| SchemaProperty {
                    name: format!("{i:02}{}", "x".repeat(name_bytes - 2)).into(),
                    required: true,
                    schema: PreparedSchema::Type(ValueType::Integer),
                })
                .collect(),
        )
        .unwrap()
    }
    #[test]
    fn aggregate_constraint_names_are_bounded_independently_of_unused_field_capacity() {
        let event = |bytes| {
            let role = Some(named_fields(8, bytes));
            EventMemberSchema {
                name: "event".into(),
                subscription: role.clone(),
                data: role.clone(),
                data_response: role.clone(),
                cancellation: role,
            }
        };
        assert!(EventCollectionSchema::try_new(vec![event(128)]).is_ok());
        assert!(EventCollectionSchema::try_new(vec![event(129)]).is_err());
        let properties = |bytes| {
            (0..2)
                .map(|i| PropertyMemberSchema {
                    name: format!("property{i}").into(),
                    schema: named_fields(16, bytes),
                })
                .collect()
        };
        assert!(PropertyCollectionSchema::try_new(properties(128)).is_ok());
        assert!(PropertyCollectionSchema::try_new(properties(129)).is_err());
    }
    #[test]
    fn event_table_accounts_for_variable_roles_and_bounds_combined_constraints() {
        let members = events(2);
        let bytes = members.capacity() * core::mem::size_of::<EventMemberSchema>()
            + members
                .iter()
                .map(|m| m.name.len() + m.data.as_ref().unwrap().retained_bytes())
                .sum::<usize>();
        let schema = EventCollectionSchema::try_new(members).unwrap();
        assert_eq!(schema.retained_bytes(), bytes);
        assert!(bytes <= EventCollectionSchema::backing_bound());
        assert!(EventCollectionSchema::try_new(events(4)).is_err());
        let role = Some(PreparedSchema::Type(ValueType::Integer));
        assert!(
            EventCollectionSchema::try_new(
                (0..32)
                    .map(|i| EventMemberSchema {
                        name: format!("event{i}").into(),
                        subscription: role.clone(),
                        data: role.clone(),
                        data_response: role.clone(),
                        cancellation: role.clone(),
                    })
                    .collect()
            )
            .is_ok(),
            "the original four scalar roles per member still fit"
        );
        // All roles participate, including a schema outside the delivery role.
        let mut member = events(2).remove(0);
        member.subscription = member.data.clone();
        member.data_response = member.data.clone();
        member.cancellation = member.data.clone();
        let expected = core::mem::size_of::<EventMemberSchema>()
            + member.name.len()
            + 4 * member.data.as_ref().unwrap().retained_bytes();
        assert_eq!(
            EventCollectionSchema::try_new(vec![member])
                .unwrap()
                .retained_bytes(),
            expected
        );
    }
}
