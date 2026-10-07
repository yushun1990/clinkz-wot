#![no_std]
use candidate::{
    ValidatedPropertyReadCursor, ValidatedThing, ValidatedThingAdmissionConfig,
    ValidatedThingCursor,
};
fn owned_input_types(
    _: ValidatedThing<'_>,
    _: ValidatedThingCursor<'_>,
    _: ValidatedPropertyReadCursor<'_>,
    _: ValidatedThingAdmissionConfig,
) {
}
