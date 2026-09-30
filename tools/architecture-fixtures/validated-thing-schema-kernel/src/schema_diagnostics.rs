//! The existing schema formatting sink, shared by schema and Thing checks.
use super::schema_kernel::{self, SchemaAccess};
use crate::validate::ValidateError;
use alloc::{format, string::String};

pub struct PublicSchemaSink;

impl<'a, A: SchemaAccess<'a>> schema_kernel::DiagnosticSink<'a, A> for PublicSchemaSink {
    type Error = ValidateError;
    fn reject(
        &self,
        access: &A,
        schema: A::Node,
        _: u64,
        rule: schema_kernel::Rule,
    ) -> ValidateError {
        use schema_kernel::Rule;
        let message = match rule {
            Rule::TypeMismatch => format!(
                "type '{}' does not match {} schema",
                access.data_type(schema).unwrap(),
                access.kind(schema).name(),
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
    fn child(&self, site: schema_kernel::ChildSite<'a>, error: ValidateError) -> ValidateError {
        use schema_kernel::ChildSite;
        let ValidateError::InvalidSchema(message) = error else {
            unreachable!()
        };
        ValidateError::InvalidSchema(match site {
            ChildSite::Indexed(index) => format!("[{}]: {}", index, message),
            ChildSite::Property(name) => format!("properties.{}: {}", name, message),
        })
    }
}
