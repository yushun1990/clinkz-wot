#![no_std]
// Each graph is checked independently. In negative graphs serde AP alone must
// not enable the TD capability. Positive graphs are ordinary downstream edges.
#[cfg(any(feature = "positive", feature = "negative"))]
pub use td::{
    ValidatedPropertyReadCursor, ValidatedPropertyReadEvent, ValidatedPropertyReadForm,
    ValidatedPropertyReadStep, ValidatedTextSequence, ValidatedThing,
    ValidatedThingAdmissionConfig, ValidatedThingCause, ValidatedThingCursor, ValidatedThingPhase,
    ValidatedThingProgress,
};
#[cfg(feature = "positive")]
pub fn checked(
    limits: &foundation::ResourceLimits,
) -> Result<ValidatedThingAdmissionConfig, td::ValidatedThingConfigError> {
    ValidatedThingAdmissionConfig::try_from_limits(limits)
}
