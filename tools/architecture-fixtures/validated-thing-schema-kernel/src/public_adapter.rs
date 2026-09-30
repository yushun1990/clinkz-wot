// Appended to the generated candidate's real data_schema.rs; the original
// source remains the oracle. Only the five failed-projection predicates differ.
impl Validate for DataSchema {
    fn validate_with_level(&self, level: ValidationLevel) -> Result<(), ValidateError> {
        if matches!(level, ValidationLevel::Minimal) {
            return Ok(());
        }
        crate::schema_kernel::validate(&crate::schema_access::TypedAccess, self, &PublicSchemaSink)
    }
}

struct PublicSchemaSink;

impl<'a> crate::schema_kernel::DiagnosticSink<'a, crate::schema_access::TypedAccess>
    for PublicSchemaSink
{
    type Error = ValidateError;
    fn reject(
        &self,
        _: &crate::schema_access::TypedAccess,
        schema: &'a DataSchema,
        _: u64,
        rule: crate::schema_kernel::Rule,
    ) -> ValidateError {
        use crate::schema_kernel::Rule;
        let message = match rule {
            Rule::TypeMismatch => format!(
                "type '{}' does not match {} schema",
                schema.context().data_type.as_deref().unwrap(),
                schema.expected_data_type(),
            ),
            Rule::ReadWrite => String::from("readOnly and writeOnly must not both be true"),
            Rule::Ordered(min, max) => format!(
                "{} must be less than or equal to {}",
                min.name(),
                max.name(),
            ),
            Rule::Positive(field) => format!("{} must be greater than 0", field.name()),
            Rule::FailedProjection(field) => {
                format!("{} Number has no finite binary64 projection", field.name())
            }
        };
        ValidateError::InvalidSchema(message)
    }
    fn child(
        &self,
        site: crate::schema_kernel::ChildSite<'a>,
        error: ValidateError,
    ) -> ValidateError {
        use crate::schema_kernel::ChildSite;
        let ValidateError::InvalidSchema(message) = error else {
            unreachable!()
        };
        ValidateError::InvalidSchema(match site {
            ChildSite::Indexed(index) => format!("[{}]: {}", index, message),
            ChildSite::Property(name) => format!("properties.{}: {}", name, message),
        })
    }
}
