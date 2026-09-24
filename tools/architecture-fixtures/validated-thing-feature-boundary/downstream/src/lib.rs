#![no_std]

#[cfg(test)]
mod tests {
    extern crate alloc;

    #[cfg(not(any(feature = "ap", feature = "validated-thing")))]
    use alloc::string::ToString;
    use alloc::{format, vec::Vec};
    use clinkz_wot_td::{
        data_schema::DataSchema,
        thing::Thing,
        validate::{Validate, ValidationLevel},
    };
    use number_boundary_td_prototype::{PredicateNumber, predicate_number};
    use serde_json::{Number, Value};

    fn thing(fields: &str) -> Thing {
        let input = format!(
            r#"{{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"probe","security":["none"],"securityDefinitions":{{"none":{{"scheme":"nosec"}}}},"schemaDefinitions":{{"probe":{{"type":"string",{fields}}}}}}}"#
        );
        serde_json::from_str(&input).unwrap()
    }

    fn basic(thing: &Thing) -> bool {
        thing.validate_with_level(ValidationLevel::Basic).is_ok()
    }

    #[test]
    fn local_capability_is_not_inferred_from_dependency_features() {
        assert_eq!(
            number_boundary_td_prototype::LOCAL_VALIDATED_THING,
            cfg!(feature = "validated-thing")
        );
        let number: Number = "9007199254740993.0000000000000000000001".parse().unwrap();
        #[cfg(any(feature = "ap", feature = "validated-thing"))]
        assert_eq!(number.as_str(), "9007199254740993.0000000000000000000001");
        #[cfg(not(any(feature = "ap", feature = "validated-thing")))]
        assert_ne!(
            number.to_string(),
            "9007199254740993.0000000000000000000001"
        );
    }

    #[test]
    fn selected_public_basic_projection_works_without_capability() {
        assert_eq!(predicate_number(None), PredicateNumber::Absent);
        for text in ["null", "true", "\"1\"", "[]", "{}"] {
            let value: Value = serde_json::from_str(text).unwrap();
            assert_eq!(predicate_number(Some(&value)), PredicateNumber::Absent);
        }
        for (text, expected) in [
            ("0", 0.0),
            ("-0.0", -0.0),
            ("1", 1.0),
            ("2.5", 2.5),
            ("9007199254740993", 9007199254740992.0),
        ] {
            let value: Value = serde_json::from_str(text).unwrap();
            assert_eq!(
                predicate_number(Some(&value)),
                PredicateNumber::Finite(expected)
            );
        }
        let tiny: Value = serde_json::from_str("1e-4000").unwrap();
        assert_eq!(predicate_number(Some(&tiny)), PredicateNumber::Finite(0.0));
        #[cfg(any(feature = "ap", feature = "validated-thing"))]
        {
            let overflow: Value = serde_json::from_str("1e309").unwrap();
            assert_eq!(predicate_number(Some(&overflow)), PredicateNumber::Invalid);
        }
    }

    #[test]
    fn actual_synchronous_td_basic_base_rule_and_required_delta() {
        // All four bound pairings use binary64 rounding in the selected rule.
        for lower in ["minimum", "exclusiveMinimum"] {
            for upper in ["maximum", "exclusiveMaximum"] {
                let td = thing(&format!(
                    "\"{lower}\":9007199254740993,\"{upper}\":9007199254740992"
                ));
                assert!(basic(&td));
                let DataSchema::String(schema) = &td.schema_definitions.as_ref().unwrap()["probe"]
                else {
                    panic!()
                };
                let a = &schema._context._extra_fields[lower];
                let b = &schema._context._extra_fields[upper];
                assert!(a.as_u64().unwrap() > b.as_u64().unwrap());
                assert_eq!(predicate_number(Some(a)), predicate_number(Some(b)));
            }
        }
        assert!(basic(&thing("\"multipleOf\":2.5")));
        assert!(!basic(&thing("\"multipleOf\":0")));
        assert!(!basic(&thing("\"multipleOf\":-1")));
        assert!(basic(&thing("\"multipleOf\":\"not-a-number\"")));

        // AP accepts a short Number that current TD Basic silently skips
        // at each of the five extension predicates. Future shared Basic must
        // return InvalidSchema for a failed Number projection.
        #[cfg(any(feature = "ap", feature = "validated-thing"))]
        {
            for name in [
                "minimum",
                "exclusiveMinimum",
                "maximum",
                "exclusiveMaximum",
                "multipleOf",
            ] {
                let td = thing(&format!("\"{name}\":1e309"));
                assert!(basic(&td));
                let DataSchema::String(schema) = &td.schema_definitions.as_ref().unwrap()["probe"]
                else {
                    panic!()
                };
                assert_eq!(
                    predicate_number(schema._context._extra_fields.get(name)),
                    PredicateNumber::Invalid
                );
            }
        }
    }

    #[cfg(feature = "validated-thing")]
    #[test]
    fn number_step_configuration_precedes_progress_and_has_profile_coordinates() {
        use clinkz_wot_foundation::{
            BenchmarkStaticReferenceV1, DirectoryClientDefaultV1, GatewayDefaultV1, ResourceKind,
            StaticResourceProfile, WorkBudget, WorkClass,
        };
        use number_boundary_td_prototype::bounded::{
            ConfigError, ConfigErrorKind, NumberStepConfig, ProjectionProgress, project,
        };

        assert_eq!(
            NumberStepConfig::try_from_limits(GatewayDefaultV1::limits())
                .unwrap()
                .ceiling(),
            256
        );
        // This profile has unrelated Host Consumer fields marked NA.
        assert_eq!(
            BenchmarkStaticReferenceV1::limits().pending_client_calls_per_binding_max(),
            None
        );
        assert_eq!(
            NumberStepConfig::try_from_limits(BenchmarkStaticReferenceV1::limits())
                .unwrap()
                .ceiling(),
            64
        );
        assert_eq!(
            NumberStepConfig::try_from_limits(DirectoryClientDefaultV1::limits()).err(),
            Some(ConfigError {
                kind: ConfigErrorKind::MissingAdmissionLimit,
                resource_kind: ResourceKind::NumberLexemeBytesMax,
                configured: None,
                supported_max: None,
            })
        );
        let work = GatewayDefaultV1::limits()
            .document_validation_work_units_max()
            .unwrap();
        let unsupported = GatewayDefaultV1::limits()
            .clone()
            .with_limit(ResourceKind::NumberLexemeBytesMax, Some(work + 1));
        assert_eq!(
            NumberStepConfig::try_from_limits(&unsupported).err(),
            Some(ConfigError {
                kind: ConfigErrorKind::UnsupportedLimit,
                resource_kind: ResourceKind::NumberLexemeBytesMax,
                configured: Some(work + 1),
                supported_max: Some(work),
            })
        );
        let missing_work = GatewayDefaultV1::limits()
            .clone()
            .with_limit(ResourceKind::DocumentValidationWorkUnitsMax, None);
        assert_eq!(
            NumberStepConfig::try_from_limits(&missing_work)
                .err()
                .unwrap()
                .resource_kind,
            ResourceKind::DocumentValidationWorkUnitsMax
        );

        // The prototype has no universal 256-byte ceiling: a valid selected
        // policy can pass a larger Number through this narrow step.
        let larger = GatewayDefaultV1::limits()
            .clone()
            .with_limit(ResourceKind::NumberLexemeBytesMax, Some(257));
        let config = NumberStepConfig::try_from_limits(&larger).unwrap();
        let number: Number = format!("1e+{}1", "0".repeat(253)).parse().unwrap();
        assert_eq!(number.as_str().len(), 257);
        let mut budget = WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, 257);
        let mut lifetime = 257;
        assert_eq!(
            project(&number, &config, &mut budget, &mut lifetime, || false),
            ProjectionProgress::Complete(PredicateNumber::Finite(10.0))
        );
    }

    #[cfg(feature = "validated-thing")]
    #[test]
    fn borrowed_lexeme_and_precharged_projection_are_capability_scoped() {
        use clinkz_wot_foundation::{GatewayDefaultV1, StaticResourceProfile};
        use clinkz_wot_foundation::{WorkBudget, WorkClass};
        use number_boundary_td_prototype::bounded::{
            NumberStepConfig, Progress, ProjectionProgress, Scan, decimal, project,
        };

        fn budget(n: u64) -> WorkBudget {
            WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, n)
        }

        let number: Number = "2.5".parse().unwrap();
        let config = NumberStepConfig::try_from_limits(GatewayDefaultV1::limits()).unwrap();
        let source = decimal(&number);
        assert_eq!(source, "2.5");
        assert_eq!(source.as_ptr(), number.as_str().as_ptr());
        let mut scan = Scan::new(&number, 3);
        let mut bytes = Vec::new();
        assert_eq!(
            scan.step(&mut budget(0), false, |_| panic!()),
            Progress::Pending
        );
        assert_eq!(scan.position, 0);
        assert_eq!(
            scan.step(&mut budget(2), false, |byte| bytes.push(byte)),
            Progress::Pending
        );
        assert_eq!(
            scan.step(&mut budget(1), false, |byte| bytes.push(byte)),
            Progress::Complete
        );
        assert_eq!(bytes, source.as_bytes());
        assert_eq!(scan.lifetime, 0);

        let mut lifetime = 3;
        assert_eq!(
            project(&number, &config, &mut budget(2), &mut lifetime, || false),
            ProjectionProgress::Pending
        );
        assert_eq!(lifetime, 3);
        assert_eq!(
            project(&number, &config, &mut budget(3), &mut 2, || false),
            ProjectionProgress::Limit
        );
        let mut allowance = budget(3);
        let mut cancellation_checks = 0;
        assert_eq!(
            project(&number, &config, &mut allowance, &mut lifetime, || {
                cancellation_checks += 1;
                false
            }),
            ProjectionProgress::Complete(PredicateNumber::Finite(2.5))
        );
        assert_eq!(cancellation_checks, 2);
        assert_eq!(allowance.remaining(WorkClass::CodecInputBytes), 0);
        assert_eq!(lifetime, 0);

        let mut lifetime = 3;
        assert_eq!(
            project(&number, &config, &mut budget(3), &mut lifetime, || true),
            ProjectionProgress::Cancelled
        );
        assert_eq!(lifetime, 3);
        let mut checks = 0;
        let mut allowance = budget(3);
        assert_eq!(
            project(&number, &config, &mut allowance, &mut lifetime, || {
                checks += 1;
                checks == 2
            }),
            ProjectionProgress::Cancelled
        );
        assert_eq!(checks, 2);
        assert_eq!(allowance.remaining(WorkClass::CodecInputBytes), 0);
        assert_eq!(lifetime, 0);
        let overflow: Number = "1e309".parse().unwrap();
        let mut overflow_len = overflow.as_str().len() as u64;
        assert_eq!(
            project(
                &overflow,
                &config,
                &mut budget(overflow_len),
                &mut overflow_len,
                || false
            ),
            ProjectionProgress::Complete(PredicateNumber::Invalid)
        );
        assert_eq!(
            project(
                &number,
                &NumberStepConfig::try_from_limits(&GatewayDefaultV1::limits().clone().with_limit(
                    clinkz_wot_foundation::ResourceKind::NumberLexemeBytesMax,
                    Some(0),
                ),)
                .unwrap(),
                &mut budget(3),
                &mut 3,
                || false,
            ),
            ProjectionProgress::Limit
        );

        // Typed public-borrow threshold; strict tokenization is out of scope.
        for ceiling in [64, 256] {
            let config =
                NumberStepConfig::try_from_limits(&GatewayDefaultV1::limits().clone().with_limit(
                    clinkz_wot_foundation::ResourceKind::NumberLexemeBytesMax,
                    Some(ceiling as u64),
                ))
                .unwrap();
            for length in [ceiling - 1, ceiling, ceiling + 1] {
                let result = if length <= ceiling {
                    ProjectionProgress::Complete(PredicateNumber::Finite(10.0))
                } else {
                    ProjectionProgress::Limit
                };
                let spelling = format!("1e+{}1", "0".repeat(length - 4));
                assert_eq!(spelling.len(), length);
                let number: Number = spelling.parse().unwrap();
                assert_eq!(decimal(&number).len(), length);
                let mut lifetime = length as u64;
                assert_eq!(
                    project(
                        &number,
                        &config,
                        &mut budget(length as u64),
                        &mut lifetime,
                        || false
                    ),
                    result
                );
                assert_eq!(lifetime, if length <= ceiling { 0 } else { length as u64 });
            }
        }

        let config = NumberStepConfig::try_from_limits(GatewayDefaultV1::limits()).unwrap();
        let mut lifetime = 6;
        let mut allowance = budget(6);
        for remaining in [3, 0] {
            assert_eq!(
                project(&number, &config, &mut allowance, &mut lifetime, || false),
                ProjectionProgress::Complete(PredicateNumber::Finite(2.5))
            );
            assert_eq!(allowance.remaining(WorkClass::CodecInputBytes), remaining);
            assert_eq!(lifetime, remaining);
        }
    }
}
