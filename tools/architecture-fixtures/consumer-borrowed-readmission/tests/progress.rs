#![cfg(feature = "validated-thing")]
use clinkz_wot_foundation::{
    BenchmarkStaticReferenceV1, GatewayDefaultV1, ResourceKind as R, StaticResourceProfile,
    WorkClass as W,
};
use consumer_borrowed_readmission_probe::{
    data_type::{BaseUri, FormHref},
    td::*,
    validate::{Validate, ValidationLevel},
};
use td_candidate as td_crate;
#[path = "support/case.rs"]
mod case;
#[path = "../../../../td/tests/support/typed_corpus_shared.rs"]
#[allow(dead_code, unexpected_cfgs)]
mod corpus;
#[allow(dead_code)]
mod support;
use case::*;
#[global_allocator]
static ALLOC: support::Allocator = support::Allocator;
#[test]
fn operation_projection_requires_all_applicable_values_and_ignores_unrelated_na() {
    let l = GatewayDefaultV1::limits();
    for &r in CATALOG {
        let mut limits = l.clone();
        assert!(limits.set(r, None));
        let g = support::start(None);
        let e = ValidatedThingAdmissionConfig::try_from_limits(&limits)
            .err()
            .unwrap();
        let n = support::counts();
        drop(g);
        assert_eq!(
            e.kind(),
            ValidatedThingConfigErrorKind::MissingAdmissionLimit
        );
        assert_eq!(e.resource_kind(), r);
        assert_eq!(n.attempts, 0);
    }
    let mut limits = l.clone();
    for r in R::ALL {
        if !CATALOG.contains(&r) {
            assert!(limits.set(r, None));
        }
    }
    let c = ValidatedThingAdmissionConfig::try_from_limits(&limits).unwrap();
    assert_eq!(c.resource_interpretation_revision(), 2);
    assert_eq!(c.operation(), "typed-content-v1");
    for r in [
        R::NumberLexemeBytesMax,
        R::JsonNestingDepthMax,
        R::JsonMembersPerObjectMax,
        R::DocumentBytesMax,
        R::DocumentValidationWorkUnitsMax,
    ] {
        let mut limits = l.clone();
        assert!(limits.set(r, u64::MAX.into()));
        let g = support::start(None);
        let e = ValidatedThingAdmissionConfig::try_from_limits(&limits)
            .err()
            .unwrap();
        let n = support::counts();
        drop(g);
        assert_eq!(e.kind(), ValidatedThingConfigErrorKind::UnsupportedLimit);
        assert_eq!(e.resource_kind(), r);
        assert_eq!(n.attempts, 0);
    }
    let boundary = l
        .clone()
        .with_limit(R::JsonMembersPerObjectMax, Some(8))
        .with_limit(R::JsonArrayItemsMax, Some(8))
        .with_limit(R::AffordancesPerThingMax, Some(1))
        .with_limit(R::NumberLexemeBytesMax, Some(0))
        .with_limit(R::DocumentValidationWorkUnitsMax, Some(521));
    let g = support::start(None);
    let e = ValidatedThingAdmissionConfig::try_from_limits(&boundary)
        .err()
        .unwrap();
    assert_eq!(e.kind(), ValidatedThingConfigErrorKind::UnsupportedLimit);
    assert_eq!(e.resource_kind(), R::DocumentValidationWorkUnitsMax);
    assert_eq!(support::counts().attempts, 0);
    drop(g);
    ValidatedThingAdmissionConfig::try_from_limits(
        &boundary.with_limit(R::DocumentValidationWorkUnitsMax, Some(522)),
    )
    .unwrap();
}
#[test]
fn every_supplied_resource_category_has_an_exact_boundary() {
    let t = corpus::nested_schema_corpus();
    let l = GatewayDefaultV1::limits();
    let trace = validate(&t, l, 4096).unwrap().trace();
    let observations = [
        (R::DocumentBytesMax, trace.content as u64),
        (R::StringBytesMax, trace.text_bytes),
        (R::ExtensionBytesMax, trace.extension_bytes),
        (R::JsonNestingDepthMax, trace.container_depth),
        (R::JsonMembersPerObjectMax, trace.object_members),
        (R::JsonArrayItemsMax, trace.array_items),
        (R::JsonValueNodesPerDocumentMax, trace.nodes as u64),
        (R::AffordancesPerThingMax, trace.affordances),
        (R::FormsPerContextMax, trace.forms_context),
        (R::FormsPerThingMax, trace.forms),
        (R::AdditionalResponsesPerFormMax, trace.responses),
        (R::UriVariablesPerFormMax, trace.variables),
        (R::SchemaNodesPerDocumentMax, trace.supplied_schemas),
        (R::SchemaCompositionDepthMax, trace.schema_depth),
        (R::SchemaReferenceEdgesPerDocumentMax, trace.schema_edges),
        (R::UriTemplateSourceBytesMax, trace.uri_source),
        (R::NumberLexemeBytesMax, trace.number_bytes),
    ];
    for (r, n) in observations {
        assert!(n > 0, "{r:?}");
        let at = l.clone().with_limit(r, Some(n));
        validate(&t, &at, 4096).unwrap_or_else(|e| panic!("at {r:?}={n}: {e:?}"));
        let below = l.clone().with_limit(r, Some(n - 1));
        let e = validate(&t, &below, 4096).err().unwrap();
        assert!(
            matches!(e,ValidatedThingCause::Limit(x) if x.kind()==r),
            "{r:?}={n}: {e:?}"
        );
    }
    assert!(trace.source_text_reads > 4096);
    assert!(trace.content_checksum > 0);
    println!("whole-field categories: {trace:?}");
}
#[test]
fn complete_shared_public_basic_accepts_serializer_failure_and_absent_id() {
    for t in [
        corpus::serializer_failure_thing(),
        corpus::nested_schema_corpus(),
    ] {
        assert!(t.validate_with_level(ValidationLevel::Basic).is_ok());
        validate(&t, GatewayDefaultV1::limits(), 4096).unwrap();
    }
    let t = corpus::serializer_failure_thing();
    assert!(serde_json::to_string(&t).is_err());
    let mut t = source();
    t.id = None;
    validate(&t, GatewayDefaultV1::limits(), 4096).unwrap();
}
#[test]
fn ap_number_boundaries_zero_and_public_projection_slow_paths() {
    let failed: serde_json::Number = serde_json::from_str("1e309").unwrap();
    assert_eq!(failed.as_f64(), None);
    for (l, length) in [
        (GatewayDefaultV1::limits(), 256usize),
        (BenchmarkStaticReferenceV1::limits(), 64),
    ] {
        let config = ValidatedThingAdmissionConfig::try_from_limits(l).unwrap();
        assert!(config.largest_atomic_debits()[W::CodecInputBytes as usize] >= length as u64);
        for n in [length - 1, length, length + 1] {
            let number =
                serde_json::from_str::<serde_json::Value>(&format!("1{}", "0".repeat(n - 1)))
                    .unwrap();
            let mut t = source();
            t._extra_fields.insert("opaque-number".into(), number);
            let result = validate(&t, l, 4096);
            if n <= length {
                assert!(result.is_ok());
            } else {
                assert!(
                    matches!(result.err(),Some(ValidatedThingCause::Limit(x)) if x.kind()==R::NumberLexemeBytesMax)
                );
            }
        }
        let zero = l.clone().with_limit(R::NumberLexemeBytesMax, Some(0));
        let mut t = source();
        t._extra_fields.insert("n".into(), serde_json::json!(0));
        assert!(
            matches!(validate(&t,&zero,4096).err(),Some(ValidatedThingCause::Limit(x)) if x.kind()==R::NumberLexemeBytesMax)
        );
    }
    // Ordinary public AP conversion and the shared amended Basic kernel remain
    // the oracle, including late exponent, rounding, underflow and >binary64.
    for raw in [
        "1e309",
        "1e-9999",
        "9007199254740993",
        "1.00000000000000011102230246251565404236316680908203125",
        "1.234567890123456789e-123",
        "0.000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001",
    ] {
        for field in [
            "minimum",
            "exclusiveMinimum",
            "maximum",
            "exclusiveMaximum",
            "multipleOf",
        ] {
            let mut t = source();
            t.properties.as_mut().unwrap().get_mut("p").unwrap()._schema =
                serde_json::from_str(&format!("{{\"type\":\"string\",\"{field}\":{raw}}}"))
                    .unwrap();
            let public = t.validate_with_level(ValidationLevel::Basic);
            let actual = validate(&t, GatewayDefaultV1::limits(), 4096);
            assert_eq!(public.is_ok(), actual.is_ok(), "{field}={raw}");
            if public.is_ok() {
                assert!(actual.unwrap().trace().projections > 0);
            } else if raw == "1e309" {
                let ValidatedThingCause::Invalid(error) = actual.err().unwrap() else {
                    panic!()
                };
                assert!(matches!(
                    error.diagnostic_for_fixture(),
                    Cause::Invalid(td_candidate::basic_kernel::InlineInvalid {
                        rule: td_candidate::basic_kernel::InlineRule::Schema(
                            td_candidate::schema_kernel::Rule::FailedProjection(_)
                        ),
                        ..
                    })
                ));
            }
        }
    }
    for (minimum, maximum, multiple_of) in [
        (f64::NAN, 1.0, f64::NAN),
        (f64::INFINITY, f64::INFINITY, f64::INFINITY),
        (f64::NEG_INFINITY, f64::INFINITY, 2.5),
        (2.0, 1.0, 0.0),
    ] {
        let mut t = source();
        t.properties.as_mut().unwrap().get_mut("p").unwrap()._schema =
            td_candidate::data_schema::DataSchema::Number(
                td_candidate::data_schema::NumberSchema {
                    minimum: Some(minimum),
                    maximum: Some(maximum),
                    multiple_of: Some(multiple_of),
                    ..Default::default()
                },
            );
        let expected = td_candidate::basic_kernel::validate(
            &td_candidate::basic_typed::TypedBasicAccess(Some(&t)),
            &td_candidate::basic_kernel::InlineSink,
        );
        let actual = validate(&t, GatewayDefaultV1::limits(), 4096);
        match (expected, actual) {
            (Ok(()), Ok(_)) => {}
            (Err(e), Err(ValidatedThingCause::Invalid(v))) => {
                assert_eq!(v.diagnostic_for_fixture(), Cause::Invalid(e))
            }
            (e, a) => panic!("nonfinite parity: {e:?} {:?}", a.err()),
        }
    }
}
#[test]
fn atomic_number_retry_has_no_partial_debit_and_no_projection_replay() {
    let mut t = source();
    let raw = format!("1.{}1", "0".repeat(250));
    t.properties.as_mut().unwrap().get_mut("p").unwrap()._schema =
        serde_json::from_str(&format!("{{\"type\":\"string\",\"minimum\":{raw}}}")).unwrap();
    let l = GatewayDefaultV1::limits();
    let cfg = ValidatedThingAdmissionConfig::try_from_limits(l).unwrap();
    let mut c = ValidatedThingCursor::from_thing(&t, &cfg, ledger(l));
    loop {
        if let Some(n) = c.next_atomic_number_bytes() {
            if n > 0 {
                let before = c.trace();
                let remaining = c.lifetime_remaining();
                for _ in 0..4 {
                    let mut b = budget(4096);
                    b.set_remaining(W::CodecInputBytes, n - 1);
                    let initial = W::ALL.map(|c| b.remaining(c));
                    let ValidatedThingProgress::Pending(next) = c.step(&mut b, false) else {
                        panic!()
                    };
                    c = next;
                    assert_eq!(W::ALL.map(|c| b.remaining(c)), initial);
                    assert_eq!(c.trace(), before);
                    assert_eq!(c.lifetime_remaining(), remaining);
                }
                let mut b = budget(4096);
                b.set_remaining(W::CodecInputBytes, n);
                let ValidatedThingProgress::Pending(next) = c.step(&mut b, false) else {
                    panic!()
                };
                c = next;
                assert_eq!(c.trace().projections, before.projections + 1);
                break;
            }
        }
        match c.step(&mut budget(4096), false) {
            ValidatedThingProgress::Pending(n) => c = n,
            _ => panic!("no atomic projection"),
        }
    }
    let before = c.trace().projections;
    loop {
        match c.step(&mut budget(4096), false) {
            ValidatedThingProgress::Pending(n) => c = n,
            ValidatedThingProgress::Complete(v) => {
                assert_eq!(v.trace().projections, before);
                break;
            }
            ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
        }
    }
}
#[test]
fn exact_lifetime_work_is_monotonic_through_moves_and_rewind() {
    let t = corpus::nested_schema_corpus();
    let l = GatewayDefaultV1::limits();
    let expected = validate(&t, l, 4096).unwrap().trace();
    let total = expected.work.iter().sum::<u64>();
    for n in [total, total - 1] {
        let l = l
            .clone()
            .with_limit(R::DocumentValidationWorkUnitsMax, Some(n));
        let actual = validate(&t, &l, 4096);
        if n == total {
            assert_eq!(actual.unwrap().trace(), expected);
        } else {
            assert!(
                matches!(actual.err(),Some(ValidatedThingCause::Limit(x)) if x.kind()==R::DocumentValidationWorkUnitsMax)
            );
        }
    }
}
#[test]
fn named_uri_targets_advance_with_small_credit_and_ready_retries_do_no_work() {
    for (original, n) in [
        (GatewayDefaultV1::limits(), 16384usize),
        (BenchmarkStaticReferenceV1::limits(), 4096),
    ] {
        let supported = original
            .clone()
            .with_limit(R::DocumentValidationWorkUnitsMax, Some(1_048_576));
        let l = &supported;
        let mut t = source();
        t.base = Some(BaseUri::parse("https://example.test/a/b/").unwrap());
        let prefix = "https://example.test/a/";
        let raw = "x".repeat(n - prefix.len());
        t.properties
            .as_mut()
            .unwrap()
            .get_mut("p")
            .unwrap()
            ._interaction
            .forms[0]
            .href = FormHref::parse(&format!("../{raw}")).unwrap();
        t.properties
            .as_mut()
            .unwrap()
            .get_mut("p")
            .unwrap()
            ._interaction
            .forms[0]
            .scopes = Some(vec!["λ".repeat(12), "scope".into()]);
        if n == 4096 {
            let mut original_read = validate(&t, original, 4096).unwrap().into_property_read();
            let e = drive(&mut original_read, 4).unwrap_err();
            assert!(
                matches!(e,ValidatedThingCause::Limit(x) if x.kind()==R::DocumentValidationWorkUnitsMax)
            );
            println!("named static 4-KiB relative target: {e:?}");
        }
        let mut read = validate(&t, l, 4096).unwrap().into_property_read();
        loop {
            match read.step(&mut budget(4), false).unwrap() {
                ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Property {
                    ..
                }) => read.acknowledge(),
                ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Form(f)) => {
                    assert_eq!(f.resolved_href().len(), n);
                    break;
                }
                _ => {}
            }
        }
        let observed = read.uri_observations();
        assert_eq!(observed.utf8_bytes, n as u64);
        assert!(observed.component_bytes > n as u64);
        assert_eq!(read.scope_visits(), 2);
        let trace = read.trace();
        let life = read.lifetime_remaining();
        for _ in 0..8 {
            match read.step(&mut budget(0), false).unwrap() {
                ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Form(f)) => {
                    assert_eq!(f.scopes().byte_len(), 29)
                }
                _ => panic!(),
            };
            assert_eq!(read.trace(), trace);
            assert_eq!(read.uri_observations(), observed);
            assert_eq!(read.scope_visits(), 2);
            assert_eq!(read.lifetime_remaining(), life);
        }
        read.acknowledge();
        drive(&mut read, 4).unwrap();
        let before = read.lifetime_remaining();
        let mut read = read.rewind();
        drive(&mut read, 4).unwrap();
        assert!(read.lifetime_remaining() < before);
    }
    // The unchanged static policy supports its full source/alias ceiling and
    // ordinary derived targets. Its genuine cross-limit above is neither an
    // artificial URI cap nor a missing support configuration.
    for (base, raw, expected_len) in [
        (None, format!("urn:{}", "x".repeat(4092)), 4096),
        (Some("https://example.test/"), "x".repeat(1004), 1025),
    ] {
        let mut t = source();
        t.base = base.map(|b| BaseUri::parse(b).unwrap());
        t.properties
            .as_mut()
            .unwrap()
            .get_mut("p")
            .unwrap()
            ._interaction
            .forms[0]
            .href = FormHref::parse(&raw).unwrap();
        let mut read = validate(&t, BenchmarkStaticReferenceV1::limits(), 4096)
            .unwrap()
            .into_property_read();
        loop {
            match read.step(&mut budget(4), false).unwrap() {
                ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Property {
                    ..
                }) => read.acknowledge(),
                ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Form(f)) => {
                    assert_eq!(f.resolved_href().len(), expected_len);
                    break;
                }
                ValidatedPropertyReadStep::Pending => {}
                ValidatedPropertyReadStep::Done => panic!("missing static form"),
            }
        }
    }
}
#[test]
fn normalization_caps_original_and_final_targets_instead_of_removed_segments() {
    use consumer_borrowed_readmission_probe::data_type::resolve_form_href;
    // Includes the unchanged gateway counterexample and a base larger than the
    // selected URI ceiling: the ceiling applies to Form targets, not the base.
    for (ceiling, base_segment) in [(16384, 16360), (64, 40), (28, 40)] {
        let l = GatewayDefaultV1::limits()
            .clone()
            .with_limit(R::UriTemplateSourceBytesMax, Some(ceiling));
        let t = normalization_source(base_segment);
        let href = FormHref::parse("bbbbbbbbbbbbbbbbbbbb/../../x").unwrap();
        let expected = resolve_form_href(t.base.as_ref(), &href).unwrap();
        assert_eq!(expected.as_str(), "https://h/x");
        let proof = validate(&t, &l, 4096).unwrap();
        let g = support::start(None);
        let mut read = proof.into_property_read();
        loop {
            match read.step(&mut budget(4), false).unwrap() {
                ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Property {
                    ..
                }) => read.acknowledge(),
                ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Form(f)) => {
                    assert_eq!(f.resolved_href(), expected.as_str());
                    break;
                }
                ValidatedPropertyReadStep::Pending => {}
                ValidatedPropertyReadStep::Done => panic!("missing normalized target"),
            }
        }
        assert!(read.lifetime_remaining() > 0);
        let observed = read.uri_observations();
        assert_eq!(observed.utf8_bytes, 11);
        assert_eq!(observed.emitted_bytes, 11);
        assert!(observed.pop_bytes >= base_segment as u64);
        let counts = support::counts();
        let (requests, n) = support::requests();
        assert_eq!(n, 1);
        assert_eq!(requests[0], (ceiling as usize, 1));
        assert_eq!(
            (counts.live, counts.peak, counts.largest),
            (ceiling as usize, ceiling as usize, ceiling as usize)
        );
        let trace = read.trace();
        let life = read.lifetime_remaining();
        for _ in 0..8 {
            assert!(matches!(
                read.step(&mut budget(0), false).unwrap(),
                ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Form(_))
            ));
            assert_eq!(read.trace(), trace);
            assert_eq!(read.uri_observations(), observed);
            assert_eq!(read.lifetime_remaining(), life);
        }
        drop(read);
        assert_eq!(support::counts().live, 0);
        assert_eq!(support::counts().allocations, support::counts().releases);
        drop(g);
        println!(
            "normalization ceiling={ceiling}: {observed:?}; lifetime spent={}",
            l.get(R::DocumentValidationWorkUnitsMax).unwrap() - life
        );
    }
}
#[test]
fn normalized_path_swap_debits_are_atomic_and_target_limits_keep_their_scope() {
    let original = GatewayDefaultV1::limits();
    let l = original
        .clone()
        .with_limit(R::UriTemplateSourceBytesMax, Some(64));
    let mut t = normalization_source(40);
    t.properties
        .as_mut()
        .unwrap()
        .get_mut("p")
        .unwrap()
        ._interaction
        .forms[0]
        .href = FormHref::parse("bbbbbbbbbbbbbbbbbbbb/../../long-target").unwrap();
    let mut read = validate(&t, &l, 4096).unwrap().into_property_read();
    while read.uri_observations().reversed_bytes == 0 {
        match read.step(&mut budget(4), false).unwrap() {
            ValidatedPropertyReadStep::Ready(_) => read.acknowledge(),
            _ => {}
        }
    }
    let trace = read.trace();
    let observed = read.uri_observations();
    let life = read.lifetime_remaining();
    for (uri, output) in [(3, 2), (4, 1), (0, 0)] {
        let mut b = budget(4);
        b.set_remaining(W::UriBytes, uri);
        b.set_remaining(W::CodecOutputBytes, output);
        let before = W::ALL.map(|c| b.remaining(c));
        assert!(matches!(
            read.step(&mut b, false).unwrap(),
            ValidatedPropertyReadStep::Pending
        ));
        assert_eq!(W::ALL.map(|c| b.remaining(c)), before);
        assert_eq!(read.trace(), trace);
        assert_eq!(read.uri_observations(), observed);
        assert_eq!(read.lifetime_remaining(), life);
    }
    // Move the partially reversed cursor, then finish without replay.
    let mut read = [read].into_iter().next().unwrap();
    drive(&mut read, 4).unwrap();
    let work = l.get(R::DocumentValidationWorkUnitsMax).unwrap() - read.lifetime_remaining();
    let complete_observed = read.uri_observations();
    drop(read);
    for n in [work, work - 1] {
        // Keep the configuration's container atomic envelopes supportable at
        // this deliberately small work threshold; actual fixture counts fit.
        let selected = l
            .clone()
            .with_limit(R::JsonMembersPerObjectMax, Some(64))
            .with_limit(R::JsonArrayItemsMax, Some(64))
            .with_limit(R::AffordancesPerThingMax, Some(16))
            .with_limit(R::DocumentValidationWorkUnitsMax, Some(n));
        let mut read = validate(&t, &selected, 4096).unwrap().into_property_read();
        if n == work {
            drive(&mut read, 4).unwrap();
            assert_eq!(read.lifetime_remaining(), 0);
            assert_eq!(read.uri_observations(), complete_observed);
        } else {
            let e = drive(&mut read, 4).unwrap_err();
            assert!(
                matches!(e, ValidatedThingCause::Limit(x) if x.kind() == R::DocumentValidationWorkUnitsMax && x.configured() == n && x.observed() == work)
            );
            assert_eq!(read.step(&mut budget(0), true).err(), Some(e));
        }
    }
    // Original Form text is 28 bytes; neither the 51-byte base nor the
    // unnormalized merged path acquires this resource's ceiling.
    let t = normalization_source(40);
    let below = original
        .clone()
        .with_limit(R::UriTemplateSourceBytesMax, Some(27));
    let e = validate(&t, &below, 4096).err().unwrap();
    assert!(
        matches!(e, ValidatedThingCause::Limit(x) if x.kind() == R::UriTemplateSourceBytesMax && x.configured() == 27 && x.observed() == 28 && x.phase() == ValidatedThingPhase::Inspect)
    );
    // A genuinely long final target still rejects at its own boundary.
    let mut t = normalization_source(40);
    t.properties
        .as_mut()
        .unwrap()
        .get_mut("p")
        .unwrap()
        ._interaction
        .forms[0]
        .href = FormHref::parse("end").unwrap();
    for ceiling in [54, 53] {
        let selected = original
            .clone()
            .with_limit(R::UriTemplateSourceBytesMax, Some(ceiling));
        let mut read = validate(&t, &selected, 4096).unwrap().into_property_read();
        if ceiling == 54 {
            drive(&mut read, 4).unwrap();
            assert_eq!(read.uri_observations().utf8_bytes, 54);
        } else {
            let e = drive(&mut read, 4).unwrap_err();
            assert!(
                matches!(e, ValidatedThingCause::Limit(x) if x.kind() == R::UriTemplateSourceBytesMax && x.configured() == 53 && x.observed() == 54 && x.phase() == ValidatedThingPhase::Semantics)
            );
            assert_eq!(read.step(&mut budget(0), true).err(), Some(e));
        }
    }
}
#[path = "support/oracle.rs"]
mod oracle;
#[test]
fn independent_public_field_byte_and_actual_read_oracle() {
    let t = corpus::nested_schema_corpus();
    let context = vec![
        serde_json::json!("https://www.w3.org/2022/wot/td/v1.1"),
        serde_json::json!({"ex":"https://example.org/ns#"}),
        serde_json::json!("https://example.org/extra-context"),
    ];
    let o = oracle::thing(&t, &context);
    let trace = validate(&t, GatewayDefaultV1::limits(), 4096)
        .unwrap()
        .trace();
    assert_eq!(trace.content as u64, o.document);
    assert_eq!(trace.text_bytes, o.text);
    assert_eq!(trace.extension_bytes, o.extension);
    assert_eq!(trace.source_text_reads, o.reads);
    assert_eq!(trace.content_checksum, o.checksum);
    let t = corpus::serializer_failure_thing();
    let o = oracle::thing(
        &t,
        &[serde_json::json!("https://example.org/extension-only")],
    );
    let trace = validate(&t, GatewayDefaultV1::limits(), 4096)
        .unwrap()
        .trace();
    assert_eq!(trace.content as u64, o.document);
    assert_eq!(trace.source_text_reads, o.reads);
    assert_eq!(trace.content_checksum, o.checksum);
}
#[test]
fn all_container_primitives_and_common_prefix_bytes_have_paid_monotonic_observations() {
    let mut t = source();
    let prefix = "shared-prefix-".repeat(80);
    let original = t.security_definitions.remove("none").unwrap();
    let a = format!("{prefix}a");
    let z = format!("{prefix}z");
    t.security_definitions.insert(a, original.clone());
    t.security_definitions.insert(z.clone(), original);
    t.security = vec![z];
    // The largest native map is not an affordance/definition root.
    let schemas = t.schema_definitions.get_or_insert_default();
    for n in 0..23 {
        schemas.insert(
            format!("s{n:02}"),
            serde_json::from_str(r#"{"type":"null"}"#).unwrap(),
        );
    }
    let fast = validate(&t, GatewayDefaultV1::limits(), 100_000)
        .unwrap()
        .trace();
    let chunked = validate(&t, GatewayDefaultV1::limits(), 4096)
        .unwrap()
        .trace();
    assert_eq!(fast, chunked);
    assert!(fast.compared_bytes > 2000);
    assert!(fast.next_calls > 100);
    assert!(fast.source_text_reads > 2000);
    let mut l = GatewayDefaultV1::LIMITS.clone();
    assert!(l.set(R::JsonMembersPerObjectMax, Some(2)));
    let mut t = source();
    t._extra_fields
        .insert("opaque".into(), serde_json::json!({"a":1,"b":2,"c":3}));
    assert!(
        matches!(validate(&t,&l,4096).err(),Some(ValidatedThingCause::Limit(x)) if x.kind()==R::JsonMembersPerObjectMax)
    );
}
#[test]
fn effective_uri_and_security_semantic_resource_boundaries_are_separate_from_basic() {
    let l = GatewayDefaultV1::limits();
    let mut t = source();
    t.base = Some(BaseUri::parse("https://example.test/a/b/").unwrap());
    let supplied = validate(&t, l, 4096).unwrap().trace().content as u64;
    let resolved = "https://example.test/a/b/target".len() as u64;
    for (r, n) in [
        (R::GeneratedEffectiveDocumentBytesMax, supplied + resolved),
        (R::UriTemplateSourceBytesMax, resolved),
        (R::SecurityExpressionDepthMax, 1),
        (R::SecurityBranchesPerPlanMax, 1),
    ] {
        let at = l.clone().with_limit(r, Some(n));
        let mut read = validate(&t, &at, 4096).unwrap().into_property_read();
        drive(&mut read, 4).unwrap();
        let below = l.clone().with_limit(r, Some(n - 1));
        let mut read = validate(&t, &below, 4096).unwrap().into_property_read();
        let e = drive(&mut read, 4).unwrap_err();
        assert!(
            matches!(e,ValidatedThingCause::Limit(x) if x.kind()==r),
            "{r:?}: {e:?}"
        );
        assert_eq!(read.step(&mut budget(0), true).err(), Some(e));
    }
    // Supplied content remains part of the effective-document ceiling even
    // when no readable Form would emit derived text.
    t.properties
        .as_mut()
        .unwrap()
        .get_mut("p")
        .unwrap()
        ._interaction
        .forms[0]
        .op = Some(vec![]);
    let below = l
        .clone()
        .with_limit(R::GeneratedEffectiveDocumentBytesMax, Some(0));
    assert!(
        matches!(validate(&t,&below,4096).err(),Some(ValidatedThingCause::Limit(x)) if x.kind()==R::GeneratedEffectiveDocumentBytesMax)
    );
}
