//! Optional source preparation for the fixture's integer datagram profile.
extern crate alloc;

use crate::Program;
use alloc::boxed::Box;
use clinkz_wot_core::{
    AffordanceSite, CoreError, CoreResult, ErrorContext, ErrorPhase, Operation, PlanId, RetryClass,
    data_schema::{PreparedSchema, ValueSchemas},
    execution::ValueExecution,
    security::SecurityRequirement,
};
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, SlotIndex, StaticResourceProfile, WorkBudget,
    WorkClass,
};
use clinkz_wot_td::{
    ValidatedPropertyReadEvent, ValidatedPropertyReadStep, ValidatedThingAdmissionConfig,
    ValidatedThingCursor, ValidatedThingProgress, data_schema::DataSchema, thing::Thing,
};

/// Owned startup output. Its program and meaning can be lent after the TD and
/// validation cursor are gone. The runtime itself needs neither this owner
/// shape nor the preparation feature.
pub struct PreparedRead {
    name: Box<str>,
    target: Box<str>,
    schemas: ValueSchemas,
    plan: PlanId,
}
impl PreparedRead {
    pub fn meaning(&self) -> ValueExecution<'_> {
        ValueExecution::try_new(
            AffordanceSite::Property(&self.name),
            Operation::ReadProperty,
            &self.schemas,
            &SecurityRequirement::NOSEC,
        )
        .expect("prepared Property Read")
    }
    pub fn program(&self) -> Program<'_> {
        Program {
            target: &self.target,
            plan: self.plan,
        }
    }
}

/// Compile one explicitly selected coordinate while the validated source is
/// lent. Unconsumed work stays in the same bounded preparation job.
pub fn prepare(thing: &Thing, name: &str, form: u32, plan: PlanId) -> CoreResult<PreparedRead> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS)
        .map_err(|_| unsupported())?;
    let mut work = WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, class| {
            b.with_remaining(class, 1_000_000)
        });
    let remaining = |work: &WorkBudget| WorkClass::ALL.map(|class| work.remaining(class));
    let mut cursor = ValidatedThingCursor::from_thing(
        thing,
        &config,
        AdmissionLedger::new(
            SlotIndex::new(0),
            Generation::INITIAL,
            0,
            GatewayDefaultV1::LIMITS
                .get(clinkz_wot_foundation::ResourceKind::AdmissionTemporaryBytesPerOperationMax)
                .unwrap_or(0),
            0,
            0,
            0,
            0,
        ),
    );
    let mut forms = loop {
        let before = remaining(&work);
        match cursor.step(&mut work, false) {
            ValidatedThingProgress::Complete(thing) => break thing.into_property_read(),
            ValidatedThingProgress::Pending(next) if remaining(&work) != before => cursor = next,
            _ => return Err(unsupported()),
        }
    };
    let property = thing
        .properties
        .as_ref()
        .and_then(|p| p.get(name))
        .ok_or_else(unsupported)?;
    let DataSchema::Integer(schema) = &property._schema else {
        return Err(unsupported());
    };
    let base = &schema._context;
    if base.write_only
        || base.constant.is_some()
        || base.enumerate.is_some()
        || base.one_of.is_some()
        || base.format.is_some()
        || !base._extra_fields.is_empty()
        || schema.multiple_of.is_some()
    {
        return Err(unsupported());
    }
    loop {
        let before = remaining(&work);
        match forms.step(&mut work, false).map_err(|_| unsupported())? {
            ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Form(f))
                if f.property_name() == name && f.original_index() == form =>
            {
                let target = f.resolved_href();
                if name.len() > 64
                    || target.len() > 64
                    || !target.starts_with("test-datagram:")
                    || target.contains('{')
                    || target.contains('}')
                    || f.security_scheme() != Some("nosec")
                    || !matches!(
                        f.security_name().and_then(|name| thing.security_definitions.get(name)),
                        Some(clinkz_wot_td::security_scheme::SecurityScheme::NoSec(s)) if s._context.proxy.is_none()
                    )
                    || f.content_type() != "application/x-test-number"
                    || f.content_coding().is_some()
                    || f.subprotocol().is_some()
                    || f.scopes().len() != 0
                    || f.has_additional_form_metadata()
                {
                    return Err(unsupported());
                }
                return Ok(PreparedRead {
                    name: name.into(),
                    target: target.into(),
                    plan,
                    schemas: ValueSchemas {
                        input: None,
                        output: Some(PreparedSchema::Integer {
                            minimum: schema.minimum,
                            exclusive_minimum: schema.exclusive_minimum,
                            maximum: schema.maximum,
                            exclusive_maximum: schema.exclusive_maximum,
                        }),
                    },
                });
            }
            ValidatedPropertyReadStep::Ready(_) => forms.acknowledge(),
            ValidatedPropertyReadStep::Pending if remaining(&work) != before => {}
            _ => return Err(unsupported()),
        }
    }
}

fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}
