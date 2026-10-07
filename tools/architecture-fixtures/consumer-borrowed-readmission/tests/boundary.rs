#![cfg(feature = "validated-thing")]
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, ResourceKind as R, ResourceLimits, SlotIndex,
    StaticResourceProfile, WorkBudget, WorkClass as W,
};
use consumer_borrowed_readmission_probe::{
    data_type::{BaseUri, FormHref, resolve_form_href},
    td::*,
    thing::Thing,
};
use td_candidate as td_crate;
#[path = "../../../../td/tests/support/basic_corpus_shared.rs"]
#[allow(dead_code)]
mod corpus;
fn budget(n: u64) -> WorkBudget {
    let mut b = WorkBudget::new();
    for c in W::ALL {
        b.set_remaining(c, n);
    }
    b
}
fn ledger(l: &ResourceLimits) -> AdmissionLedger {
    let heap = ValidatedThingAdmissionConfig::try_from_limits(l)
        .unwrap()
        .controlled_heap_envelope();
    AdmissionLedger::new(
        SlotIndex::new(0),
        Generation::INITIAL,
        0,
        l.get(R::AdmissionTemporaryBytesPerOperationMax)
            .unwrap()
            .min(heap),
        0,
        0,
        0,
        0,
    )
}
fn validate<'a>(
    t: &'a Thing,
    l: &ResourceLimits,
    q: u64,
) -> Result<ValidatedThing<'a>, ValidatedThingCause> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(l).unwrap();
    let mut c = ValidatedThingCursor::from_thing(t, &config, ledger(l));
    for _ in 0..1_000_000 {
        match c.step(&mut budget(q), false) {
            ValidatedThingProgress::Pending(n) => c = n,
            ValidatedThingProgress::Complete(v) => return Ok(v),
            ValidatedThingProgress::Failed(e) => return Err(e),
        }
    }
    panic!("stalled")
}
fn source() -> Thing {
    serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"probe","id":"urn:probe","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},"properties":{"p":{"type":"null","forms":[{"href":"target"}]}}}"#).unwrap()
}
#[test]
fn shared_basic_and_first_rule_corpus() {
    let limits = GatewayDefaultV1::limits();
    for case in corpus::cases() {
        let expected = td_candidate::basic_kernel::validate(
            &td_candidate::basic_typed::TypedBasicAccess(Some(&case.thing)),
            &td_candidate::basic_kernel::InlineSink,
        );
        let actual = validate(&case.thing, limits, 4096);
        match (expected, actual) {
            (Ok(()), Ok(v)) => assert_eq!(v.trace().live, 0, "{}", case.label),
            (Err(e), Err(ValidatedThingCause::Invalid(v))) => assert_eq!(
                v.diagnostic_for_fixture(),
                Cause::Invalid(e),
                "{}",
                case.label
            ),
            (expected, actual) => panic!(
                "{} expected {expected:?}, actual {:?}",
                case.label,
                actual.err()
            ),
        }
    }
}
#[test]
fn typed_byte_oracle_counts_operation_vocabulary() {
    let limits = GatewayDefaultV1::limits();
    let mut t = source();
    let without = validate(&t, limits, 4096).unwrap().trace();
    t.properties
        .as_mut()
        .unwrap()
        .get_mut("p")
        .unwrap()
        ._interaction
        .forms[0]
        .op = Some(vec![td_candidate::data_type::Operation::ReadProperty]);
    let with = validate(&t, limits, 4096).unwrap().trace();
    assert_eq!(with.content - without.content, "readproperty".len());
    assert_eq!(
        with.text_bytes - without.text_bytes,
        "readproperty".len() as u64
    );
}
fn resolved<'a>(t: &'a Thing, q: u64) -> Result<String, ValidatedThingCause> {
    let v = validate(t, GatewayDefaultV1::limits(), 4096)?;
    let mut read = v.into_property_read();
    for _ in 0..1_000_000 {
        match read.step(&mut budget(q), false)? {
            ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Property { .. }) => {
                read.acknowledge()
            }
            ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Form(f)) => {
                return Ok(f.resolved_href().to_string());
            }
            ValidatedPropertyReadStep::Pending => {}
            ValidatedPropertyReadStep::Done => panic!("no form"),
        }
    }
    panic!("URI stalled")
}
#[test]
fn resumable_uri_matches_public_td_for_adversarial_components() {
    let bases = [
        None,
        Some("https://example.test/a/b/c?old"),
        Some("https://example.test"),
        Some("urn:foo"),
        Some("foo:"),
        Some("foo:?old"),
        Some("foo:#fragment"),
        Some("foo:/a/.."),
        Some("foo:/"),
        Some("https://example.test/a#fragment"),
        Some("https://{host}/"),
    ];
    let references = [
        "",
        "#f",
        "?q",
        "g",
        "../g",
        "./g",
        "../../g",
        "/./g/../x",
        "//other/x",
        "//other",
        "http://other/a/../g",
        "%2e/%2E%2e/g",
        "/a//b/../../c",
        "///x",
        "./g?x#y",
        "g/.",
        "g/..",
        "/..",
        "/{value}",
    ];
    for base in bases {
        for raw in references {
            let mut t = source();
            t.base = base.map(|b| BaseUri::parse(b).unwrap());
            let href = FormHref::parse(raw).unwrap();
            t.properties
                .as_mut()
                .unwrap()
                .get_mut("p")
                .unwrap()
                ._interaction
                .forms[0]
                .href = href.clone();
            let expected =
                resolve_form_href(t.base.as_ref(), &href).map(|v| v.as_str().to_string());
            let actual = resolved(&t, 4);
            match (expected, actual) {
                (Ok(e), Ok(a)) => assert_eq!(e, a, "base={base:?}, raw={raw:?}"),
                (Err(_), Err(ValidatedThingCause::Invalid(_))) => {}
                (e, a) => panic!("base={base:?} raw={raw:?}: expected={e:?}, actual={a:?}"),
            }
        }
    }
}
#[test]
fn uri_segment_cross_product_matches_existing_semantic_owner() {
    let segments = [
        "", ".", "..", "%2e", ".%2e", "%2e.", "%2E%2e", "x", "a:b", "x%2Fy",
    ];
    for base in [
        "https://h/a/b",
        "https://h/a/..",
        "foo:/a/b/",
        "foo://h/a//b/",
        "foo:/",
    ] {
        for left in segments {
            for right in segments {
                for prefix in ["", "/", "./"] {
                    for suffix in ["", "/", "/end?q#f", "?q#f"] {
                        let raw = format!("{prefix}{left}/{right}{suffix}");
                        let Ok(href) = FormHref::parse(&raw) else {
                            continue;
                        };
                        let mut t = source();
                        t.base = Some(BaseUri::parse(base).unwrap());
                        t.properties
                            .as_mut()
                            .unwrap()
                            .get_mut("p")
                            .unwrap()
                            ._interaction
                            .forms[0]
                            .href = href.clone();
                        let expected = resolve_form_href(t.base.as_ref(), &href)
                            .map(|v| v.as_str().to_string());
                        let actual = resolved(&t, 4);
                        match (expected, actual) {
                            (Ok(e), Ok(a)) => assert_eq!(e, a, "base={base} raw={raw}"),
                            (Err(_), Err(ValidatedThingCause::Invalid(_))) => {}
                            (e, a) => panic!("base={base} raw={raw}: {e:?} {a:?}"),
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn large_normalization_cancellation_matches_public_td_without_a_target_fallback() {
    let bases = [
        format!("https://h/{}/", "a".repeat(8000)),
        format!("https://h/{}", "a/".repeat(4096)),
        format!("foo:/{}", "a/".repeat(4096)),
    ];
    let references = [
        "bbbbbbbbbbbbbbbbbbbb/../../x".into(),
        "bbbbbbbbbbbbbbbbbbbb/../x?q#f".into(),
        "bbbbbbbbbbbbbbbbbbbb/.%2e/%2E%2e/x?query#fragment".into(),
        "a//b/../../../x".into(),
        "./a/./b/../../x/.".into(),
        "/a//b/../../x".into(),
        "//other/a/../../x".into(),
        format!("{}x?q#f", "../".repeat(4096)),
    ];
    for base in &bases {
        for raw in &references {
            let mut t = source();
            t.base = Some(BaseUri::parse(base).unwrap());
            let href = FormHref::parse(raw).unwrap();
            t.properties
                .as_mut()
                .unwrap()
                .get_mut("p")
                .unwrap()
                ._interaction
                .forms[0]
                .href = href.clone();
            let expected = resolve_form_href(t.base.as_ref(), &href).unwrap();
            assert!(expected.as_str().len() <= 16384);
            assert_eq!(
                resolved(&t, 4).unwrap(),
                expected.as_str(),
                "base bytes={} raw={raw}",
                base.len()
            );
        }
    }
}
