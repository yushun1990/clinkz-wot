#![no_std]
pub const ACTIVE: bool = cfg!(feature = "validated-thing");
#[cfg(feature = "validated-thing")]
#[allow(dead_code)]
mod exact {
    use candidate::*;
    use clinkz_wot_foundation::{AdmissionLedger, ResourceKind, ResourceLimits, WorkBudget};
    fn config(
        l: &ResourceLimits,
    ) -> Result<ValidatedThingAdmissionConfig, ValidatedThingConfigError> {
        ValidatedThingAdmissionConfig::try_from_limits(l)
    }
    fn enter<'a>(
        t: &'a thing::Thing,
        c: &ValidatedThingAdmissionConfig,
        l: AdmissionLedger,
    ) -> ValidatedThingCursor<'a> {
        ValidatedThingCursor::from_thing(t, c, l)
    }
    fn step<'a>(c: ValidatedThingCursor<'a>, w: &mut WorkBudget) -> ValidatedThingProgress<'a> {
        c.step(w, false)
    }
    fn loan<'a>(
        c: &'a mut ValidatedPropertyReadCursor<'_>,
        w: &mut WorkBudget,
    ) -> Result<ValidatedPropertyReadStep<'a>, ValidatedThingCause> {
        c.step(w, false)
    }
    fn text<'a>(v: &ValidatedTextSequence<'a>) -> impl ExactSizeIterator<Item = &'a str> + 'a {
        v.iter()
    }
    fn consume<'a>(v: ValidatedThing<'a>) -> ValidatedPropertyReadCursor<'a> {
        v.into_property_read()
    }
    fn vocabulary(v: ValidatedThingProgress<'_>) {
        match v {
            ValidatedThingProgress::Pending(_)
            | ValidatedThingProgress::Complete(_)
            | ValidatedThingProgress::Failed(_) => {}
        }
    }
    fn facts<'a>(f: ValidatedPropertyReadForm<'a>) {
        let _: u32 = f.property_ordinal();
        let _: &'a str = f.property_name();
        let _: u32 = f.original_index();
        let _: &'a str = f.href();
        let _: &'a str = f.resolved_href();
        let _: &'a str = f.content_type();
        let _: Option<&'a str> = f.content_coding();
        let _: Option<&'a str> = f.subprotocol();
        let _: bool = f.readable();
        let _: u64 = f.security_count();
        let _: Option<&'a str> = f.security_name();
        let _: Option<&'a str> = f.security_scheme();
        let _: u64 = f.copy_bytes();
        let scopes: ValidatedTextSequence<'a> = f.scopes();
        let _: usize = scopes.len();
        let _: u64 = scopes.byte_len();
    }
    fn diagnostics(
        c: ValidatedThingConfigError,
        i: ValidatedThingInvalid,
        l: ValidatedThingLimit,
        f: ValidatedThingFailure,
    ) {
        let _: ValidatedThingConfigErrorKind = c.kind();
        let _: ResourceKind = c.resource_kind();
        let _: Option<u64> = c.configured();
        let _: Option<u64> = c.supported_max();
        let _: ValidatedThingInvalidKind = i.kind();
        let _: ValidatedThingPhase = i.phase();
        let _: u64 = i.node_ordinal();
        let _: ResourceKind = l.kind();
        let _: u64 = l.configured();
        let _: u64 = l.observed();
        let _: ValidatedThingPhase = l.phase();
        let _: ValidatedThingFailureKind = f.kind();
        let _: ValidatedThingPhase = f.phase();
        let _: u64 = f.requested_bytes();
    }
    fn finish(mut c: ValidatedPropertyReadCursor<'_>) {
        let _: Option<&str> = c.id();
        c.acknowledge();
        let _ = c.rewind();
    }
}
