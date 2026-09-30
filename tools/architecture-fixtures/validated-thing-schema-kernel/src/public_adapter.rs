// Appended to the generated candidate's real data_schema.rs; the original
// source remains the oracle. Only the five failed-projection predicates differ.
impl Validate for DataSchema {
    fn validate_with_level(&self, level: ValidationLevel) -> Result<(), ValidateError> {
        if matches!(level, ValidationLevel::Minimal) {
            return Ok(());
        }
        crate::schema_kernel::validate(
            &crate::schema_access::TypedAccess,
            self,
            &crate::schema_diagnostics::PublicSchemaSink,
        )
    }
}
