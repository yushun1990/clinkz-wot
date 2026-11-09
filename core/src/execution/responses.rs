//! Prepared response meaning, independent of protocol and ownership strategy.
use crate::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, InteractionOutputView, InteractionStatus,
    Operation, ResponseSelection, RetryClass, data_schema::PreparedSchema,
};
use alloc::{boxed::Box, vec::Vec};

/// An alternative either retains the operation's ordinary output role, or
/// replaces it with a present value. Any is a present unconstrained value;
/// it must never be confused with an absent output role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResponseData {
    Operation,
    Value(PreparedSchema),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseAlternative {
    pub content_type: Box<str>,
    pub success: bool,
    pub data: ResponseData,
}
/// Pure common meaning. Native discriminators and executable services belong
/// to the qualified mapping. Order preserves TD additional-response identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteResponses {
    primary: ResponseAlternative,
    additional: Vec<ResponseAlternative>,
}
impl FiniteResponses {
    pub const MAX_ADDITIONAL: usize = 32;
    pub fn try_new(
        primary: ResponseAlternative,
        additional: Vec<ResponseAlternative>,
    ) -> CoreResult<Self> {
        if !primary.success
            || additional.len() > Self::MAX_ADDITIONAL
            || core::iter::once(&primary)
                .chain(additional.iter())
                .any(|r| r.content_type.is_empty() || r.content_type.len() > 256)
        {
            return Err(invalid());
        }
        Ok(Self {
            primary,
            additional,
        })
    }
    pub fn get(&self, selection: ResponseSelection) -> Option<&ResponseAlternative> {
        match selection {
            ResponseSelection::Primary => Some(&self.primary),
            ResponseSelection::Additional(i) => self.additional.get(usize::from(i)),
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &ResponseAlternative> {
        core::iter::once(&self.primary).chain(self.additional.iter())
    }
    pub fn additional(&self) -> &[ResponseAlternative] {
        &self.additional
    }
    pub fn retained_bytes(&self) -> usize {
        self.additional.capacity() * core::mem::size_of::<ResponseAlternative>()
            + self
                .iter()
                .map(|r| {
                    r.content_type.len()
                        + match &r.data {
                            ResponseData::Operation => 0,
                            ResponseData::Value(s) => s.retained_bytes(),
                        }
                })
                .sum::<usize>()
    }
    pub fn drop_work(&self) -> u64 {
        self.iter()
            .map(|r| {
                2 + match &r.data {
                    ResponseData::Operation => 0,
                    ResponseData::Value(s) => s.drop_work(),
                }
            })
            .sum()
    }
    /// Bounded shape projection captured from prepared meaning before native
    /// handoff. Supplying output metadata cannot expand this admitted set.
    pub fn shape(&self, operation_has_data: bool) -> FiniteResponseShape {
        let mut shape = FiniteResponseShape {
            admitted: 0,
            successes: 0,
            required: 0,
            forbidden: 0,
        };
        for (i, r) in self.iter().enumerate() {
            let bit = 1u64 << i;
            shape.admitted |= bit;
            if r.success {
                shape.successes |= bit;
            }
            if matches!(r.data, ResponseData::Value(_)) || operation_has_data {
                shape.required |= bit;
            } else {
                shape.forbidden |= bit;
            }
        }
        shape
    }
}
/// Allocation-free, protocol-neutral expectation for the identity/shape seal.
/// Full media/schema validation remains mandatory at the owning boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FiniteResponseShape {
    admitted: u64,
    successes: u64,
    required: u64,
    forbidden: u64,
}
impl FiniteResponseShape {
    pub const fn primary(operation: Operation) -> Self {
        Self {
            admitted: 1,
            successes: 1,
            required: if matches!(
                operation,
                Operation::ReadProperty
                    | Operation::ReadAllProperties
                    | Operation::ReadMultipleProperties
                    | Operation::QueryAllActions
            ) {
                1
            } else {
                0
            },
            forbidden: if matches!(
                operation,
                Operation::WriteProperty
                    | Operation::WriteAllProperties
                    | Operation::WriteMultipleProperties
            ) {
                1
            } else {
                0
            },
        }
    }
    /// The owning data boundary retains schema-dependent presence failures.
    /// Ordinary primary Read/Write retain their established carrier shape;
    /// Action and additional-response presence is checked with captured roles.
    #[cfg(target_has_atomic = "ptr")]
    pub(crate) const fn binding_seal(mut self, operation: Operation) -> Self {
        self.required &= 1;
        self.forbidden &= 1;
        if matches!(operation, Operation::InvokeAction) {
            self.required = 0;
            self.forbidden = 0;
        }
        self
    }
    pub fn validate(
        self,
        selection: ResponseSelection,
        output: InteractionOutputView<'_>,
    ) -> CoreResult<()> {
        let i = match selection {
            ResponseSelection::Primary => 0,
            ResponseSelection::Additional(i) => u32::from(i) + 1,
        };
        let bit = 1u64.checked_shl(i).ok_or_else(invalid)?;
        let status = if self.successes & bit != 0 {
            InteractionStatus::Ok
        } else {
            InteractionStatus::ApplicationError
        };
        if self.admitted & bit == 0
            || output.status() != status
            || self.required & bit != 0 && output.data().is_none()
            || self.forbidden & bit != 0 && output.data().is_some()
            || selection != ResponseSelection::Primary
                && output.metadata().property_write_report().is_some()
        {
            return Err(invalid());
        }
        Ok(())
    }
}
fn invalid() -> CoreError {
    CoreError::Validation(ErrorContext::new(ErrorPhase::Validate, RetryClass::Never))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BindingGeneration, BindingId, BindingResponseMetadata, InteractionOutputMetadata,
        PayloadView, PlanId, ResponsePayloadRole,
    };
    use clinkz_wot_foundation::{Generation, SlotIndex};

    fn contract() -> FiniteResponses {
        FiniteResponses::try_new(
            ResponseAlternative {
                content_type: "application/json".into(),
                success: true,
                data: ResponseData::Operation,
            },
            alloc::vec![ResponseAlternative {
                content_type: "application/json".into(),
                success: false,
                data: ResponseData::Value(PreparedSchema::Any),
            }],
        )
        .unwrap()
    }
    fn check(
        selection: ResponseSelection,
        status: InteractionStatus,
        data: bool,
        admitted: FiniteResponseShape,
        native_status: Option<u16>,
    ) -> CoreResult<()> {
        let plan = PlanId::new(SlotIndex::new(4), Generation::INITIAL);
        let mut response = BindingResponseMetadata::new(
            BindingId::new(2),
            BindingGeneration::INITIAL,
            plan,
            selection,
        );
        if let Some(code) = native_status {
            response = response.with_native_status(code);
        }
        let metadata =
            InteractionOutputMetadata::default().with_untrusted_binding_response(response);
        let output = InteractionOutputView::new(
            data.then_some(PayloadView {
                body: b"null",
                content_type: "application/json",
                content_coding: None,
            }),
            status,
            &metadata,
        );
        crate::execution::validate_finite_response(
            BindingId::new(2),
            BindingGeneration::INITIAL,
            plan,
            Operation::WriteProperty,
            admitted,
            output,
        )
    }
    #[test]
    fn original_admitted_shape_not_untrusted_metadata_selects_available_branches() {
        let selected = ResponseSelection::Additional(0);
        assert!(
            check(
                selected,
                InteractionStatus::ApplicationError,
                true,
                contract().shape(false),
                None
            )
            .is_ok()
        );
        assert!(
            check(
                selected,
                InteractionStatus::ApplicationError,
                true,
                FiniteResponseShape::primary(Operation::WriteProperty),
                None
            )
            .is_err()
        );
        for selection in [
            ResponseSelection::Additional(1),
            ResponseSelection::Additional(u16::MAX),
        ] {
            assert!(
                check(
                    selection,
                    InteractionStatus::ApplicationError,
                    true,
                    contract().shape(false),
                    None
                )
                .is_err()
            );
        }
    }
    #[test]
    fn application_failure_shape_and_success_are_checked_without_native_status_semantics() {
        for code in [None, Some(0), Some(200), Some(65535)] {
            assert!(
                check(
                    ResponseSelection::Additional(0),
                    InteractionStatus::ApplicationError,
                    true,
                    contract().shape(false),
                    code
                )
                .is_ok()
            );
            assert!(
                check(
                    ResponseSelection::Additional(0),
                    InteractionStatus::Ok,
                    true,
                    contract().shape(false),
                    code
                )
                .is_err()
            );
            assert!(
                check(
                    ResponseSelection::Additional(0),
                    InteractionStatus::ApplicationError,
                    false,
                    contract().shape(false),
                    code
                )
                .is_err()
            );
        }
        assert!(
            check(
                ResponseSelection::Primary,
                InteractionStatus::Ok,
                false,
                contract().shape(false),
                None
            )
            .is_ok()
        );
        assert!(
            check(
                ResponseSelection::Primary,
                InteractionStatus::ApplicationError,
                false,
                contract().shape(false),
                None
            )
            .is_err()
        );
    }
    #[test]
    fn operation_status_and_foreign_identity_cannot_become_application_responses() {
        let plan = PlanId::new(SlotIndex::new(4), Generation::INITIAL);
        let response = BindingResponseMetadata::new(
            BindingId::new(3),
            BindingGeneration::INITIAL,
            plan,
            ResponseSelection::Primary,
        );
        let metadata = InteractionOutputMetadata::default()
            .with_untrusted_binding_response(response)
            .with_payload_role(ResponsePayloadRole::OperationStatus);
        let output = InteractionOutputView::new(None, InteractionStatus::Ok, &metadata);
        assert!(
            crate::execution::validate_finite_response(
                BindingId::new(2),
                BindingGeneration::INITIAL,
                plan,
                Operation::WriteProperty,
                contract().shape(false),
                output
            )
            .is_err()
        );
    }
}
