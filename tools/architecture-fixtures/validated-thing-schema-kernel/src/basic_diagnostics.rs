//! Public formatting is separate from rule selection, and can also read a
//! Snapshot. Inline and public sinks observe the same first rejection.
use super::{
    basic_kernel::{BasicAccess, DiagnosticSink, Owner, OwnerKind, Rule, Site},
    schema_kernel::{self, SchemaAccess},
};
use crate::validate::ValidateError;
use alloc::{
    format,
    string::{String, ToString},
};

/// Component entry points omit the enclosing Thing's name prefix.
pub struct PublicSink {
    pub document: bool,
}

fn owner_name<'a, A: BasicAccess<'a>>(access: &A, owner: Owner) -> String {
    match owner.kind {
        OwnerKind::Thing => "Thing.forms".into(),
        OwnerKind::SecurityDefinition => format!(
            "securityDefinitions.{}",
            access.definition_name(owner.ordinal as usize)
        ),
        kind => format!("{} '{}'", kind_name(kind), access.affordance_name(owner)),
    }
}

fn kind_name(kind: OwnerKind) -> &'static str {
    match kind {
        OwnerKind::Property => "Property",
        OwnerKind::Action => "Action",
        OwnerKind::Event => "Event",
        _ => unreachable!(),
    }
}

impl<'a, A: SchemaAccess<'a>> schema_kernel::DiagnosticSink<'a, A> for PublicSink {
    type Error = ValidateError;
    fn reject(
        &self,
        access: &A,
        node: A::Node,
        ordinal: u64,
        rule: schema_kernel::Rule,
    ) -> Self::Error {
        schema_kernel::DiagnosticSink::reject(
            &super::schema_diagnostics::PublicSchemaSink,
            access,
            node,
            ordinal,
            rule,
        )
    }
    fn child(&self, site: schema_kernel::ChildSite<'a>, error: Self::Error) -> Self::Error {
        <super::schema_diagnostics::PublicSchemaSink as schema_kernel::DiagnosticSink<'a, A>>::child(
            &super::schema_diagnostics::PublicSchemaSink,
            site,
            error,
        )
    }
}

impl<'a, A: BasicAccess<'a>> DiagnosticSink<'a, A> for PublicSink {
    fn reject_basic(&self, access: &A, site: Site, rule: Rule<'a>) -> Self::Error {
        use super::basic_kernel::Field;
        let field = site.field.name();
        let error = match rule {
            Rule::Missing => ValidateError::MissingRequiredField(field.into()),
            Rule::UnsupportedScheme(scheme) => {
                ValidateError::InvalidSecurity(format!("unsupported scheme '{}'", scheme))
            }
            Rule::ComboMissing => ValidateError::InvalidSecurity(
                "combo schemes must define at least one of oneOf or allOf".into(),
            ),
            Rule::ComboCardinality => ValidateError::InvalidSecurity(format!(
                "{} must contain at least two references",
                field
            )),
            Rule::ComboEmpty => ValidateError::InvalidSecurity(format!(
                "{} must not contain empty references",
                field
            )),
            Rule::UnsupportedFlow(flow) => {
                ValidateError::InvalidSecurity(format!("unsupported OAuth2 flow '{}'", flow))
            }
            Rule::Undefined(reference) => {
                let context = match (site.owner.kind, site.field) {
                    (OwnerKind::Thing, Field::Security) => "Thing.security".into(),
                    (OwnerKind::SecurityDefinition, _) => {
                        format!("{}.{}", owner_name(access, site.owner), field)
                    }
                    (OwnerKind::Thing, Field::FormSecurity) => {
                        format!("Thing.forms.forms[{}].security", site.index)
                    }
                    (_, Field::FormSecurity) => format!(
                        "{}.forms[{}].security",
                        owner_name(access, site.owner),
                        site.index
                    ),
                    _ => unreachable!(),
                };
                return ValidateError::InvalidReference {
                    context,
                    reference: reference.into(),
                };
            }
            Rule::Operation(op) => {
                let (context, found) = if site.owner.kind == OwnerKind::Thing {
                    ("Thing.forms".into(), op.as_str().to_string())
                } else {
                    (
                        format!("{}Affordance", kind_name(site.owner.kind)),
                        format!("{:?}", op),
                    )
                };
                ValidateError::InvalidOperation { context, found }
            }
        };
        if self.document && site.owner.kind != OwnerKind::Thing {
            crate::validate::prepend_context(owner_name(access, site.owner), error)
        } else {
            error
        }
    }
    fn schema_site(
        &self,
        access: &A,
        site: Site,
        key: Option<&'a str>,
        error: Self::Error,
    ) -> Self::Error {
        use super::basic_kernel::Field;
        let ValidateError::InvalidSchema(message) = error else {
            unreachable!()
        };
        let message = match (site.field, key) {
            (Field::PropertySchema, None) => message,
            (_, Some(key)) => format!("{}.{}: {}", site.field.name(), key, message),
            (_, None) => format!("{}: {}", site.field.name(), message),
        };
        let error = ValidateError::InvalidSchema(message);
        if self.document && site.owner.kind != OwnerKind::Thing {
            crate::validate::prepend_context(owner_name(access, site.owner), error)
        } else {
            error
        }
    }
}
