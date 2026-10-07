//! Observe the production program before the synchronous formatting adapter.
use super::{basic_kernel as b, basic_typed::TypedBasicAccess, schema_kernel as s};
use crate as td_crate;
use crate::validate::Validate;
use alloc::format;
use core::ops::ControlFlow;

#[path = "basic_corpus_shared.rs"]
#[allow(dead_code)]
mod corpus;

#[derive(Debug)]
struct Rejection<'a> {
    site: b::Site,
    basic: Option<b::Rule<'a>>,
    schema: Option<(u64, s::Rule)>,
}
struct Inline;
impl<'a, A: s::SchemaAccess<'a>> s::DiagnosticSink<'a, A> for Inline {
    type Error = Rejection<'a>;
    fn reject(&self, _: &A, _: A::Node, ordinal: u64, rule: s::Rule) -> Self::Error {
        Rejection {
            site: b::Site::new(b::ROOT, b::Field::PropertySchema),
            basic: None,
            schema: Some((ordinal, rule)),
        }
    }
    fn child(&self, _: s::ChildSite<'a>, error: Self::Error) -> Self::Error {
        error
    }
}
impl<'a, A: b::BasicAccess<'a>> b::DiagnosticSink<'a, A> for Inline {
    fn reject_basic(&self, _: &A, site: b::Site, rule: b::Rule<'a>) -> Self::Error {
        Rejection {
            site,
            basic: Some(rule),
            schema: None,
        }
    }
    fn schema_site(
        &self,
        _: &A,
        site: b::Site,
        _: Option<&'a str>,
        mut error: Self::Error,
    ) -> Self::Error {
        error.site = site;
        error
    }
}

#[test]
fn complete_basic_corpus_preserves_original_coordinates_before_formatting() {
    let cases = corpus::cases();
    assert!(cases.len() > 170);
    for case in cases {
        let result = b::validate(&TypedBasicAccess(Some(&case.thing)), &Inline);
        assert_eq!(result.is_ok(), case.valid, "{}: {result:?}", case.label);
        assert_eq!(
            case.thing.validate().is_ok(),
            result.is_ok(),
            "{}",
            case.label
        );
        if let Some(first) = case.first {
            let rejected = result.unwrap_err();
            let kind = match first.owner {
                "Thing" => b::OwnerKind::Thing,
                "SecurityDefinition" => b::OwnerKind::SecurityDefinition,
                "Property" => b::OwnerKind::Property,
                "Action" => b::OwnerKind::Action,
                "Event" => b::OwnerKind::Event,
                other => panic!("unexpected corpus owner {other}"),
            };
            assert_eq!(
                rejected.site.owner,
                b::Owner {
                    kind,
                    ordinal: first.ordinal
                },
                "{}",
                case.label
            );
            assert_eq!(
                format!("{:?}", rejected.site.field),
                first.field,
                "{}",
                case.label
            );
            assert_eq!(rejected.site.index, first.index, "{}", case.label);
            assert_eq!(rejected.site.member, first.member, "{}", case.label);
            assert_ne!(rejected.basic.is_some(), rejected.schema.is_some());
        }
    }
}

#[test]
fn numeric_callback_suspension_retains_the_current_field_and_completed_projections() {
    let mut cursor = s::NumericCursor::default();
    let numbers = [Some(0), Some(1), Some(2), Some(3), Some(4)];
    let mut calls = alloc::vec::Vec::new();
    assert_eq!(
        cursor.step(numbers, |n| {
            calls.push(n);
            if n == 2 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(Some((n + 1) as f64))
            }
        }),
        ControlFlow::Break(())
    );
    assert_eq!(
        cursor.step(numbers, |n| {
            calls.push(n);
            ControlFlow::<(), _>::Continue(Some((n + 1) as f64))
        }),
        ControlFlow::Continue(Ok(()))
    );
    assert_eq!(calls, [0, 1, 2, 2, 3, 4]);
    assert_eq!(
        cursor.step(numbers, |_| panic!("terminal cursor reprojected")),
        ControlFlow::<(), _>::Continue(Ok(()))
    );
}

#[test]
fn numeric_first_rejection_precedes_multiple_of_and_is_terminal() {
    let numbers = [Some(3.0), None, Some(1.0), None, Some(f64::NAN)];
    let mut cursor = s::NumericCursor::default();
    let mut calls = 0;
    let expected = Err(s::Rule::Ordered(s::Field::Minimum, s::Field::Maximum));
    assert_eq!(
        cursor.step(numbers, |n| {
            calls += 1;
            ControlFlow::<(), _>::Continue(Some(n))
        }),
        ControlFlow::Continue(expected)
    );
    assert_eq!(calls, 2);
    assert_eq!(
        cursor.step(numbers, |_| panic!("failed cursor reprojected")),
        ControlFlow::<(), _>::Continue(expected)
    );
    for projection in [
        None,
        Some(f64::NAN),
        Some(f64::INFINITY),
        Some(f64::NEG_INFINITY),
    ] {
        let mut cursor = s::NumericCursor::default();
        assert_eq!(
            cursor.step([Some(()), None, None, None, None], |_| {
                ControlFlow::<(), _>::Continue(projection)
            }),
            ControlFlow::Continue(Err(s::Rule::FailedProjection(s::Field::Minimum)))
        );
    }
}
