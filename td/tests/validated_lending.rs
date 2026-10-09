//! Exercise only the admitted production proof/lending boundary as a caller.
#![cfg(feature = "validated-thing")]
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, ResourceKind as R, SlotIndex,
    StaticResourceProfile, WorkBudget, WorkClass as W,
};
use clinkz_wot_td::{
    ValidatedPropertyReadCursor, ValidatedPropertyReadEvent as Event,
    ValidatedPropertyReadStep as Step, ValidatedThingAdmissionConfig, ValidatedThingCause as Cause,
    ValidatedThingCursor, ValidatedThingPhase, ValidatedThingProgress as Progress, thing::Thing,
};
fn budget(n: u64) -> WorkBudget {
    W::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
fn proof<'a>(
    thing: &'a Thing,
    limits: &clinkz_wot_foundation::ResourceLimits,
) -> clinkz_wot_td::ValidatedThing<'a> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(limits).unwrap();
    let ledger = AdmissionLedger::new(
        SlotIndex::new(1),
        Generation::new(1).unwrap(),
        0,
        8 << 20,
        0,
        0,
        0,
        0,
    );
    let mut c = ValidatedThingCursor::from_thing(thing, &config, ledger);
    for _ in 0..300_000 {
        match c.step(&mut budget(10_000), false) {
            Progress::Pending(next) => c = next,
            Progress::Complete(p) => return p,
            Progress::Failed(cause) => panic!("{cause:?}"),
        }
    }
    panic!("validation failed to progress")
}
fn input() -> Thing {
    serde_json::from_str(r#"{
      "@context":"https://www.w3.org/2022/wot/td/v1.1","title":"lending", "id":"urn:lending",
      "base":"http://example/a/b/", "security":["none"],
      "securityDefinitions":{"aaa":{"scheme":"basic"},"none":{"scheme":"nosec"}},
      "properties":{
        "z-empty":{"type":"null","forms":[]},
        "a":{"type":"string","forms":[
          {"href":"ignored","op":[]},
          {"href":"../value?x#f","contentType":"text/plain","contentCoding":"gzip","subprotocol":"sub","scopes":["one","二",""]},
          {"href":"http://absolute/a/../b","security":[]},
          {"href":"{template}","security":["none","aaa"]}
        ]},
        "write":{"type":"string","writeOnly":true,"forms":[{"href":"ignored"}]}
      }
    }"#).unwrap()
}
#[derive(Debug, PartialEq)]
enum Fact {
    Property(u32, String),
    Form {
        property: u32,
        name: String,
        index: u32,
        raw: String,
        resolved: String,
        content: String,
        coding: Option<String>,
        subprotocol: Option<String>,
        scopes: Vec<String>,
        scope_bytes: u64,
        security: u64,
        security_name: Option<String>,
        scheme: Option<String>,
        copy: u64,
    },
}
fn next(c: &mut ValidatedPropertyReadCursor<'_>) -> Option<Fact> {
    for _ in 0..400_000 {
        match c.step(&mut budget(1000), false).unwrap() {
            Step::Pending => {}
            Step::Done => return None,
            Step::Ready(Event::Property { ordinal, name }) => {
                return Some(Fact::Property(ordinal, name.to_owned()));
            }
            Step::Ready(Event::Form(f)) => {
                assert!(f.readable());
                return Some(Fact::Form {
                    property: f.property_ordinal(),
                    name: f.property_name().to_owned(),
                    index: f.original_index(),
                    raw: f.href().to_owned(),
                    resolved: f.resolved_href().to_owned(),
                    content: f.content_type().to_owned(),
                    coding: f.content_coding().map(str::to_owned),
                    subprotocol: f.subprotocol().map(str::to_owned),
                    scopes: f.scopes().iter().map(str::to_owned).collect(),
                    scope_bytes: f.scopes().byte_len(),
                    security: f.security_count(),
                    security_name: f.security_name().map(str::to_owned),
                    scheme: f.security_scheme().map(str::to_owned),
                    copy: f.copy_bytes(),
                });
            }
        }
    }
    panic!("semantic progress")
}
fn pass(c: &mut ValidatedPropertyReadCursor<'_>) -> Vec<Fact> {
    let mut facts = Vec::new();
    while let Some(fact) = next(c) {
        facts.push(fact);
        c.acknowledge();
    }
    facts
}
#[test]
fn original_coordinates_defaults_security_scopes_copy_and_rewind() {
    let t = input();
    let mut c = proof(&t, GatewayDefaultV1::LIMITS).into_property_read();
    assert_eq!(c.id(), Some("urn:lending"));
    c.acknowledge(); // no-op outside Ready
    c = c.rewind(); // no-op outside Done
    assert!(matches!(
        c.step(&mut budget(0), false).unwrap(),
        Step::Pending
    ));
    let first = pass(&mut c);
    assert_eq!(first.len(), 6); // one event for every property, three readable Forms
    assert_eq!(first[0], Fact::Property(0, "a".into()));
    assert_eq!(first[4], Fact::Property(1, "write".into()));
    assert_eq!(first[5], Fact::Property(2, "z-empty".into()));
    for (position, index) in [(1, 1), (2, 2), (3, 3)] {
        let Fact::Form {
            property,
            name,
            index: actual,
            raw,
            resolved,
            content,
            coding,
            subprotocol,
            scopes,
            scope_bytes,
            security,
            security_name,
            scheme,
            copy,
        } = &first[position]
        else {
            panic!()
        };
        assert_eq!((*property, name.as_str(), *actual), (0, "a", index));
        let expected = clinkz_wot_td::data_type::resolve_form_href(
            t.base.as_ref(),
            &t.properties.as_ref().unwrap()["a"]._interaction.forms[index as usize].href,
        )
        .unwrap();
        assert_eq!(resolved, expected.as_str());
        assert_eq!(
            *scope_bytes,
            scopes.iter().map(|s| s.len() as u64).sum::<u64>()
        );
        assert_eq!(
            *copy,
            ("urn:lending".len()
                + name.len()
                + raw.len()
                + resolved.len()
                + content.len()
                + coding.as_ref().map_or(0, String::len)
                + subprotocol.as_ref().map_or(0, String::len)) as u64
                + scope_bytes
        );
        match index {
            1 => {
                assert_eq!(*security, 1);
                assert_eq!(security_name.as_deref(), Some("none"));
                assert_eq!(scheme.as_deref(), Some("nosec"));
            }
            2 => {
                assert_eq!(*security, 0);
                assert!(security_name.is_none());
                assert!(scheme.is_none());
            }
            3 => {
                assert_eq!(*security, 2);
                assert!(security_name.is_none());
                assert!(scheme.is_none());
            }
            _ => unreachable!(),
        }
    }
    assert!(matches!(c.step(&mut budget(0), true).unwrap(), Step::Done));
    c = c.rewind();
    assert_eq!(pass(&mut c), first);
    // Completion remains stable even if later polls request cancellation.
    assert!(matches!(c.step(&mut budget(0), false).unwrap(), Step::Done));
}
#[test]
fn ready_retries_never_spend_credit_or_change_facts() {
    let t = input();
    let mut c = proof(&t, GatewayDefaultV1::LIMITS).into_property_read();
    for _ in 0..4 {
        let expected = next(&mut c).unwrap();
        for _ in 0..10 {
            let mut b = budget(0);
            let fact = match c.step(&mut b, false).unwrap() {
                Step::Ready(Event::Property { ordinal, name }) => {
                    Fact::Property(ordinal, name.into())
                }
                Step::Ready(Event::Form(f)) => {
                    assert_eq!(f.scopes().len(), f.scopes().iter().len());
                    // getters must be safe on a zero-credit re-loan of scratch
                    assert!(!f.resolved_href().is_empty());
                    expected_form(&f)
                }
                _ => panic!("Ready lost"),
            };
            assert_eq!(fact, expected);
            assert!(b.is_exhausted());
        }
        c.acknowledge();
    }
}
fn expected_form(f: &clinkz_wot_td::ValidatedPropertyReadForm<'_>) -> Fact {
    Fact::Form {
        property: f.property_ordinal(),
        name: f.property_name().into(),
        index: f.original_index(),
        raw: f.href().into(),
        resolved: f.resolved_href().into(),
        content: f.content_type().into(),
        coding: f.content_coding().map(str::to_owned),
        subprotocol: f.subprotocol().map(str::to_owned),
        scopes: f.scopes().iter().map(str::to_owned).collect(),
        scope_bytes: f.scopes().byte_len(),
        security: f.security_count(),
        security_name: f.security_name().map(str::to_owned),
        scheme: f.security_scheme().map(str::to_owned),
        copy: f.copy_bytes(),
    }
}
#[test]
fn first_semantic_failure_is_stable_and_no_id_is_synthesized() {
    let mut t = input();
    t.id = None;
    t.base = Some(clinkz_wot_td::data_type::BaseUri::Template(
        "http://{host}/".into(),
    ));
    let mut c = proof(&t, GatewayDefaultV1::LIMITS).into_property_read();
    assert_eq!(c.id(), None);
    assert!(matches!(next(&mut c), Some(Fact::Property(..))));
    c.acknowledge();
    let cause = loop {
        match c.step(&mut budget(1000), false) {
            Err(cause) => break cause,
            Ok(Step::Pending) => {}
            _ => panic!(),
        }
    };
    let Cause::Invalid(diagnostic) = cause else {
        panic!("{cause:?}")
    };
    assert_eq!(
        diagnostic.kind(),
        clinkz_wot_td::ValidatedThingInvalidKind::InvalidUri
    );
    assert_eq!(diagnostic.phase(), ValidatedThingPhase::Semantics);
    for _ in 0..3 {
        let mut b = budget(1000);
        assert!(matches!(c.step(&mut b,true),Err(c) if c==cause));
        assert_eq!(b.remaining(W::UriBytes), 1000);
    }
    c = c.rewind();
    c.acknowledge();
    assert!(matches!(c.step(&mut budget(0),false),Err(c) if c==cause));
}
#[test]
fn finished_resolved_limit_rejects_before_ready() {
    let t = input();
    for (resource, limit) in [(R::UriTemplateSourceBytesMax, 23)] {
        let limits = GatewayDefaultV1::LIMITS
            .clone()
            .with_limit(resource, Some(limit));
        // The input itself must be admitted so the failure witnesses semantics.
        let mut c = proof(&t, &limits).into_property_read();
        let cause = loop {
            match c.step(&mut budget(1000), false) {
                Err(cause) => break cause,
                Ok(Step::Ready(_)) => c.acknowledge(),
                Ok(Step::Pending) => {}
                Ok(Step::Done) => panic!("expected {resource:?} rejection"),
            }
        };
        let Cause::Limit(l) = cause else {
            panic!("{cause:?}")
        };
        assert_eq!(l.kind(), resource);
        assert_eq!(l.phase(), ValidatedThingPhase::Semantics);
        assert!(l.observed() > l.configured());
    }
}

#[test]
fn charged_uri_path_matches_shared_meaning_through_shift_pop_and_utf8() {
    use clinkz_wot_td::data_type::{BaseUri, FormHref, resolve_form_href};
    for (base, raw) in [
        ("foo:/a/b", "/a/../..//x"),
        ("http://a/a/%2e%2E", "../next"),
        ("http://a/", "a/b/../../c"),
        ("http://a/base?old", "?new#fragment"),
        ("urn:opaque", "#fragment"),
        ("http://a/base", "//other/a/../value"),
        ("http://a/base", "{变量}"),
        ("http://a/base", "http://other/a/../b"),
    ] {
        let mut t = input();
        t.base = Some(BaseUri::parse(base).unwrap());
        let forms = &mut t
            .properties
            .as_mut()
            .unwrap()
            .get_mut("a")
            .unwrap()
            ._interaction
            .forms;
        forms.truncate(1);
        forms[0].op = None;
        forms[0].href = FormHref::parse(raw).unwrap();
        let expected = resolve_form_href(t.base.as_ref(), &forms[0].href)
            .unwrap()
            .as_str()
            .to_owned();
        let mut c = proof(&t, GatewayDefaultV1::LIMITS).into_property_read();
        assert!(matches!(next(&mut c), Some(Fact::Property(..))));
        c.acknowledge();
        let Fact::Form { resolved, .. } = next(&mut c).unwrap() else {
            panic!()
        };
        assert_eq!(resolved, expected, "{base} + {raw}");
    }
}

fn review_input() -> Thing {
    serde_json::from_str(
        r#"{
      "@context":"https://www.w3.org/2022/wot/td/v1.1",
      "id":"urn:test","title":"test","base":"https://example.com/a/b/",
      "security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},
      "properties":{"p":{"type":"string","forms":[{"href":"../value"}]}}
    }"#,
    )
    .unwrap()
}
#[test]
fn effective_security_limits_reject_disabled_and_excess_roots_before_ready() {
    for (resource, ceiling, roots) in [
        (R::SecurityBranchesPerPlanMax, 0, 1),
        (R::SecurityExpressionDepthMax, 0, 1),
        (R::SecurityBranchesPerPlanMax, 1, 2),
    ] {
        let mut t = review_input();
        if roots == 2 {
            t.security_definitions
                .insert("other".into(), t.security_definitions["none"].clone());
            t.properties
                .as_mut()
                .unwrap()
                .get_mut("p")
                .unwrap()
                ._interaction
                .forms[0]
                .security = Some(vec!["none".into(), "other".into()]);
        }
        let limits = GatewayDefaultV1::LIMITS
            .clone()
            .with_limit(resource, Some(ceiling));
        let mut c = proof(&t, &limits).into_property_read();
        let cause = loop {
            match c.step(&mut budget(1000), false) {
                Err(cause) => break cause,
                Ok(Step::Pending) => {}
                Ok(Step::Ready(Event::Property { .. })) => c.acknowledge(),
                _ => panic!("resource-inadmissible security was lent"),
            }
        };
        let Cause::Limit(limit) = cause else {
            panic!("{cause:?}")
        };
        assert_eq!(limit.kind(), resource);
        assert_eq!(limit.configured(), ceiling);
        assert_eq!(limit.observed(), roots);
        assert_eq!(limit.phase(), ValidatedThingPhase::Semantics);
        for _ in 0..3 {
            assert!(matches!(c.step(&mut budget(0), true), Err(same) if same == cause));
        }
    }
}
#[test]
fn effective_security_limits_are_per_form_and_preserve_empty_overrides() {
    let mut t = review_input();
    for name in ["other", "third"] {
        t.security_definitions
            .insert(name.into(), t.security_definitions["none"].clone());
    }
    t.security_definitions.insert(
        "combo".into(),
        serde_json::from_str(r#"{"scheme":"combo","allOf":["none","other"]}"#).unwrap(),
    );
    t.security = vec!["none".into(), "other".into(), "third".into()];
    let forms = &mut t
        .properties
        .as_mut()
        .unwrap()
        .get_mut("p")
        .unwrap()
        ._interaction
        .forms;
    let original = forms[0].clone();
    forms[0].op = Some(vec![]); // unreadable inherited roots do not form a plan
    for names in [
        vec![],
        vec!["none"],
        vec!["none", "other"],
        vec!["none", "other"],
        vec!["combo"],
    ] {
        let mut form = original.clone();
        form.security = Some(names.into_iter().map(str::to_owned).collect());
        forms.push(form);
    }
    for (branches, depth) in [(2, 1), (2, 2), (3, 1)] {
        let limits = GatewayDefaultV1::LIMITS
            .clone()
            .with_limit(R::SecurityBranchesPerPlanMax, Some(branches))
            .with_limit(R::SecurityExpressionDepthMax, Some(depth));
        let mut c = proof(&t, &limits).into_property_read();
        let forms: Vec<_> = pass(&mut c)
            .into_iter()
            .filter_map(|f| match f {
                Fact::Form {
                    index,
                    security,
                    scheme,
                    ..
                } => Some((index, security, scheme)),
                _ => None,
            })
            .collect();
        assert_eq!(
            forms.iter().map(|(i, n, _)| (*i, *n)).collect::<Vec<_>>(),
            [(1, 0), (2, 1), (3, 2), (4, 2), (5, 1)]
        );
        // Lending direct roots does not expand combo expressions or decide
        // Planning's exactly-one-NoSec eligibility.
        assert_eq!(forms[4].2.as_deref(), Some("combo"));
    }
    t.properties
        .as_mut()
        .unwrap()
        .get_mut("p")
        .unwrap()
        ._interaction
        .forms
        .truncate(2);
    let disabled = GatewayDefaultV1::LIMITS
        .clone()
        .with_limit(R::SecurityBranchesPerPlanMax, Some(0))
        .with_limit(R::SecurityExpressionDepthMax, Some(0));
    let mut c = proof(&t, &disabled).into_property_read();
    let facts = pass(&mut c);
    assert_eq!(facts.len(), 2);
    assert!(matches!(
        &facts[1],
        Fact::Form {
            index: 1,
            security: 0,
            ..
        }
    ));
}
#[test]
fn derived_uri_stalls_without_output_credit_and_resumes_without_partial_debits() {
    let t = review_input();
    let mut c = proof(&t, GatewayDefaultV1::LIMITS).into_property_read();
    for _ in 0..1000 {
        let mut b = budget(1000).with_remaining(W::CodecOutputBytes, 0);
        match c.step(&mut b, false).unwrap() {
            Step::Pending => {}
            Step::Ready(Event::Property { .. }) => c.acknowledge(),
            _ => panic!("derived bytes were produced without output credit"),
        }
    }
    for _ in 0..10 {
        let mut b = budget(1000).with_remaining(W::CodecOutputBytes, 0);
        assert!(matches!(c.step(&mut b, false).unwrap(), Step::Pending));
        for class in W::ALL {
            assert_eq!(
                b.remaining(class),
                if class == W::CodecOutputBytes {
                    0
                } else {
                    1000
                }
            );
        }
    }
    for _ in 0..1000 {
        let mut b = budget(1000).with_remaining(W::CodecOutputBytes, 1);
        match c.step(&mut b, false).unwrap() {
            Step::Pending => {}
            Step::Ready(Event::Form(form)) => {
                assert_eq!(form.resolved_href(), "https://example.com/a/value");
                return;
            }
            _ => panic!("lost current coordinate"),
        }
    }
    panic!("URI failed to resume with copy credit")
}
