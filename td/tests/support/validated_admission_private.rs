//! Production invariant tests, using the accepted corpora only as independent
//! input/expected-coordinate/content oracles.
extern crate std;
use super::*;
use crate as td_crate;
use crate::validate::Validate;
use alloc::{format, vec};
use clinkz_wot_foundation::{
    BenchmarkStaticReferenceV1, GatewayDefaultV1, Generation, SlotIndex, StaticResourceProfile,
};
#[path = "basic_corpus_shared.rs"]
mod corpus;
#[path = "../../../tools/architecture-fixtures/consumer-borrowed-readmission/tests/support/oracle.rs"]
mod oracle;
#[allow(dead_code)]
#[path = "typed_corpus_shared.rs"]
mod typed;
fn ledger() -> AdmissionLedger {
    AdmissionLedger::new(
        SlotIndex::new(1),
        Generation::new(1).unwrap(),
        0,
        8 << 20,
        0,
        0,
        0,
        0,
    )
}
fn limits() -> ResourceLimits {
    GatewayDefaultV1::LIMITS.clone()
}
fn budget(n: u64) -> WorkBudget {
    W::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
fn config(limits: &ResourceLimits) -> ValidatedThingAdmissionConfig {
    ValidatedThingAdmissionConfig::try_from_limits(limits)
        .unwrap_or_else(|e| panic!("config {e:?}"))
}
fn base() -> Thing {
    serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"Borrowed production", "security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},"properties":{"p":{"type":"null","forms":[{"href":"/p"}]}}}"#).unwrap()
}
fn drive<'a>(thing: &'a Thing, limits: &ResourceLimits) -> Result<ValidatedThing<'a>, Cause> {
    drive_with_ledger(thing, limits, ledger())
}
fn drive_with_ledger<'a>(
    thing: &'a Thing,
    limits: &ResourceLimits,
    ledger: AdmissionLedger,
) -> Result<ValidatedThing<'a>, Cause> {
    let mut cursor = ValidatedThingCursor::from_thing(thing, &config(limits), ledger);
    for _ in 0..300_000 {
        match cursor.step(&mut budget(10_000), false) {
            ValidatedThingProgress::Pending(next) => cursor = next,
            ValidatedThingProgress::Complete(proof) => return Ok(proof),
            ValidatedThingProgress::Failed(cause) => return Err(cause),
        }
    }
    panic!("no bounded progress")
}
fn checked_limit(thing: &Thing, resource: R, limit: u64) -> Cause {
    drive(thing, &limits().with_limit(resource, Some(limit)))
        .err()
        .expect("must reject")
}
#[test]
fn operation_projection_and_named_support() {
    for resource in CATALOG {
        let values = limits().with_limit(resource, None);
        let err = ValidatedThingAdmissionConfig::try_from_limits(&values)
            .err()
            .unwrap();
        assert_eq!(
            err.kind(),
            ValidatedThingConfigErrorKind::MissingAdmissionLimit
        );
        assert_eq!(err.resource_kind(), resource);
        assert_eq!(err.configured(), None);
    }
    let unrelated = limits().with_limit(R::RetainedSourceBytesPerOwnerMax, None);
    assert!(ValidatedThingAdmissionConfig::try_from_limits(&unrelated).is_ok());
    for values in [limits(), BenchmarkStaticReferenceV1::LIMITS.clone()] {
        let checked = config(&values);
        assert!(checked.policy.frames >= 8);
        assert!(
            checked.policy.atomic[W::CodecInputBytes as usize]
                >= checked.policy.get(R::NumberLexemeBytesMax)
        );
    }
    let bad = limits().with_limit(
        R::NumberLexemeBytesMax,
        Some(limits().document_validation_work_units_max().unwrap()),
    );
    let error = ValidatedThingAdmissionConfig::try_from_limits(&bad)
        .err()
        .unwrap();
    assert_eq!(error.resource_kind(), R::NumberLexemeBytesMax);
    assert_eq!(
        error.kind(),
        ValidatedThingConfigErrorKind::UnsupportedLimit
    );
    for r in [
        R::JsonNestingDepthMax,
        R::UriTemplateSourceBytesMax,
        R::AffordancesPerThingMax,
    ] {
        assert!(
            ValidatedThingAdmissionConfig::try_from_limits(&limits().with_limit(r, Some(u64::MAX)))
                .is_err()
        );
    }
}
#[test]
fn complete_basic_first_coordinate_parity() {
    for case in corpus::cases() {
        let public = case.thing.validate();
        let bounded = drive(&case.thing, &limits());
        assert_eq!(
            public.is_ok(),
            bounded.is_ok(),
            "{}: {:?}",
            case.label,
            bounded.as_ref().err()
        );
        assert_eq!(public.is_ok(), case.valid, "{}", case.label);
        if let (Some(first), Err(Cause::Invalid(invalid))) = (case.first, bounded) {
            assert_eq!(
                format!("{:?}", invalid.site.owner.kind),
                first.owner,
                "{}",
                case.label
            );
            assert_eq!(invalid.site.owner.ordinal, first.ordinal, "{}", case.label);
            assert_eq!(
                format!("{:?}", invalid.site.field),
                first.field,
                "{}",
                case.label
            );
            assert_eq!(invalid.site.index, first.index, "{}", case.label);
            assert_eq!(invalid.site.member, first.member, "{}", case.label);
        }
    }
}
#[test]
fn whole_typed_content_matches_independent_field_oracle() {
    let thing: Thing = serde_json::from_str(typed::CORPUS).unwrap();
    let wire: Value = serde_json::from_str(typed::CORPUS).unwrap();
    let context = wire["@context"].as_array().unwrap();
    let expected = oracle::thing(&thing, context);
    let proof = drive(&thing, &limits()).unwrap_or_else(|v| panic!("{v:?}"));
    assert_eq!(proof.owner.counts.content, expected.document);
    assert_eq!(proof.owner.counts.strings, expected.text);
    assert_eq!(proof.owner.counts.extensions, expected.extension);
    assert_eq!(proof.owner.trace.reads, expected.reads);
    assert_eq!(proof.owner.trace.checksum, expected.checksum);
    assert!(proof.owner.counts.schemas > 10);
    assert!(proof.owner.counts.forms > 5);
    assert_eq!(proof.owner.trace.entered, proof.owner.counts.nodes);
    assert_eq!(
        proof.owner.remaining,
        limits().document_validation_work_units_max().unwrap()
            - proof.owner.trace.work.iter().sum::<u64>()
    );
    assert!(proof.owner.trace.moves > 0);
}
#[test]
fn missing_id_and_serializer_failing_input_are_basic_valid() {
    let mut thing = base();
    thing.id = None;
    // A non-finite typed scalar is not one of the five extension Number rules.
    thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("p")
        .unwrap()
        ._schema = crate::data_schema::DataSchema::number()
        .minimum(f64::NAN)
        .into();
    assert!(thing.validate().is_ok());
    assert!(drive(&thing, &limits()).is_ok());
    let thing = typed::serializer_failure_thing();
    assert!(serde_json::to_string(&thing).is_err());
    assert!(drive(&thing, &limits()).is_ok());
    // Independently exercise the shared corpus's actual serializer failure.
    for case in corpus::cases()
        .into_iter()
        .filter(|c| c.valid && serde_json::to_string(&c.thing).is_err())
    {
        assert!(drive(&case.thing, &limits()).is_ok(), "{}", case.label);
    }
}
#[test]
fn content_and_structural_limits_precede_basic() {
    let thing = base();
    let proof = drive(&thing, &limits()).unwrap();
    let total = proof.owner.counts.content;
    let text = proof.owner.counts.strings;
    let nodes = proof.owner.counts.nodes;
    drop(proof);
    for (r, n) in [
        (R::DocumentBytesMax, total),
        (R::GeneratedEffectiveDocumentBytesMax, total),
        (R::StringBytesMax, text),
        (R::JsonValueNodesPerDocumentMax, nodes),
        (R::SchemaNodesPerDocumentMax, 1),
        (R::AffordancesPerThingMax, 1),
        (R::FormsPerContextMax, 1),
        (R::FormsPerThingMax, 1),
    ] {
        assert!(
            drive(&thing, &limits().with_limit(r, Some(n))).is_ok(),
            "{r:?}"
        );
        assert!(
            drive(&thing, &limits().with_limit(r, Some(n + 1))).is_ok(),
            "{r:?}"
        );
        assert!(
            matches!(checked_limit(&thing,r,n-1),Cause::Limit(v) if v.kind()==r&&v.phase()==Phase::Inspect),
            "{r:?}"
        );
    }
    let mut invalid = thing;
    invalid._metadata.title = None;
    assert!(
        matches!(checked_limit(&invalid,R::DocumentBytesMax,0),Cause::Limit(v) if v.phase()==Phase::Inspect)
    );
    assert!(
        matches!(drive(&invalid,&limits()),Err(Cause::Invalid(v)) if v.kind()==ValidatedThingInvalidKind::MissingRequiredField)
    );
}
#[test]
fn map_associations_count_as_admitted_json_nodes() {
    let mut thing: Thing = serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"review","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}}}"#).unwrap();
    // The supplied typed model has 15 occurrences: nine records/containers,
    // five text leaves and the security-definition association. Each added
    // association/key/null contributes three; an outer opaque entry adds three.
    assert_eq!(drive(&thing, &limits()).unwrap().owner.counts.nodes, 15);
    for (opaque, expected) in [(false, 24), (true, 27)] {
        let entries = ["a", "b", "c"]
            .into_iter()
            .map(|key| (key.into(), Value::Null));
        thing._extra_fields.clear();
        if opaque {
            thing
                ._extra_fields
                .insert("opaque".into(), Value::Object(entries.collect()));
        } else {
            thing._extra_fields.extend(entries);
        }
        let proof = drive(&thing, &limits()).unwrap();
        assert_eq!(proof.owner.counts.nodes, expected, "opaque={opaque}");
        assert_eq!(proof.owner.trace.entered, expected);
    }
}

#[test]
fn upstream_source_charge_survives_frame_growth_and_proof() {
    const SOURCE: usize = 131_072;
    const TEMPORARY: u64 = 65_536;
    let mut source = vec![b' '; SOURCE].into_boxed_slice();
    source[..typed::CORPUS.len()].copy_from_slice(typed::CORPUS.as_bytes());
    let thing: Thing = serde_json::from_slice(&source).unwrap();
    let values = limits()
        .with_limit(R::AdmissionTemporaryBytesPerOperationMax, Some(TEMPORARY))
        .with_limit(R::AdmissionTemporaryBytesGlobalMax, Some(TEMPORARY))
        .with_limit(R::PeakLiveBytesPerAdmissionMax, Some(TEMPORARY));
    let mut child = AdmissionLedger::new(
        SlotIndex::new(1),
        Generation::new(1).unwrap(),
        SOURCE as u64,
        TEMPORARY,
        0,
        0,
        0,
        0,
    );
    child
        .try_reserve_source(R::RetainedSourceBytesPerOwnerMax, SOURCE as u64)
        .unwrap()
        .commit();
    let mut cursor = ValidatedThingCursor::from_thing(&thing, &config(&values), child);
    let mut grew = false;
    loop {
        let stack = &cursor.stack;
        let old = Layout::array::<Frame<'_>>(stack.frames.capacity)
            .unwrap()
            .size() as u64;
        let new = stack.transfer.as_ref().map_or(0, |block| {
            Layout::array::<Frame<'_>>(block.capacity).unwrap().size() as u64
        });
        assert_eq!(stack.ledger.live_bytes(), SOURCE as u64 + old + new);
        grew |= old != 0 && new != 0;
        match cursor.step(&mut budget(10_000), false) {
            ValidatedThingProgress::Pending(next) => cursor = next,
            ValidatedThingProgress::Complete(proof) => {
                let stack = &proof.owner.stack;
                assert!(stack.transfer.is_none());
                let frames = Layout::array::<Frame<'_>>(stack.frames.capacity)
                    .unwrap()
                    .size() as u64;
                assert_eq!(stack.ledger.live_bytes(), SOURCE as u64 + frames);
                drop(proof);
                break;
            }
            ValidatedThingProgress::Failed(cause) => panic!("precharged source failed: {cause:?}"),
        }
    }
    assert!(grew, "must exercise old/new frame coexistence");
    assert_eq!(&source[..typed::CORPUS.len()], typed::CORPUS.as_bytes());
    assert_eq!(source.len(), SOURCE);
}
#[test]
fn numbers_have_lexical_limit_before_basic_and_projection_is_atomic() {
    use crate::data_schema::ContextHelper;
    for limit in [0usize, 64, 256] {
        for n in [limit.saturating_sub(1), limit, limit + 1]
            .into_iter()
            .filter(|&n| n > 0)
        {
            let token = format!("1{}", "0".repeat(n - 1));
            let value = serde_json::from_str::<Value>(&token).unwrap();
            let mut thing = base();
            thing._extra_fields.insert("number".into(), value);
            let result = drive(
                &thing,
                &limits().with_limit(R::NumberLexemeBytesMax, Some(limit as u64)),
            );
            if n <= limit {
                assert!(result.is_ok());
            } else {
                assert!(
                    matches!(result,Err(Cause::Limit(v)) if v.kind()==R::NumberLexemeBytesMax&&v.phase()==Phase::Inspect)
                );
            }
        }
    }
    let mut thing = base();
    let p = thing.properties.as_mut().unwrap().get_mut("p").unwrap();
    p._schema = DataSchema::null()
        .extra_field("minimum", serde_json::from_str("1e309").unwrap())
        .into();
    let mut cursor = ValidatedThingCursor::from_thing(&thing, &config(&limits()), ledger());
    for _ in 0..20_000 {
        let numeric = if cursor.stack.frames.len > 0 && cursor.stack.transfer.is_none() {
            if let Frame::Schema(f) = cursor.stack.top() {
                let mut walk = f.walk;
                walk.action(
                    TypedAccess.one_of_count(f.node),
                    TypedAccess.child_count(f.node),
                ) == s::Action::Numeric
                    && f.numbers[0].is_some()
            } else {
                false
            }
        } else {
            false
        };
        if numeric {
            let bytes = if let Frame::Schema(f) = cursor.stack.top() {
                f.numbers[0].unwrap().as_str().len() as u64
            } else {
                unreachable!()
            };
            let remaining = cursor.remaining;
            let projected = cursor.trace.projections;
            let spent = cursor.trace.work;
            for _ in 0..4 {
                let mut allowance = budget(1000);
                allowance.set_remaining(W::CodecInputBytes, bytes - 1);
                let before = allowance.remaining(W::JsonSchemaNodes);
                cursor = match cursor.step(&mut allowance, false) {
                    ValidatedThingProgress::Pending(v) => v,
                    _ => panic!("whole projection needs 5 bytes"),
                };
                assert_eq!(cursor.remaining, remaining);
                assert_eq!(cursor.trace.projections, projected);
                assert_eq!(cursor.trace.work, spent);
                assert_eq!(allowance.remaining(W::CodecInputBytes), bytes - 1);
                assert_eq!(allowance.remaining(W::JsonSchemaNodes), before);
            }
            let result = cursor.step(&mut budget(bytes), false);
            assert!(
                matches!(result,ValidatedThingProgress::Failed(Cause::Invalid(v)) if v.rule==Rule::Schema(s::Rule::FailedProjection(s::Field::Minimum)))
            );
            return;
        }
        cursor = match cursor.step(&mut budget(10_000), false) {
            ValidatedThingProgress::Pending(v) => v,
            _ => panic!("must reach numeric action"),
        };
    }
    panic!("numeric phase not reached")
}
#[test]
fn zero_credit_and_nonresettable_lifetime_across_moves() {
    let thing = base();
    let conf = config(&limits());
    let mut cursor = ValidatedThingCursor::from_thing(&thing, &conf, ledger());
    for _ in 0..10 {
        cursor = match cursor.step(&mut WorkBudget::new(), false) {
            ValidatedThingProgress::Pending(v) => v,
            _ => panic!(),
        };
        assert_eq!(cursor.trace.entered, 0);
        assert_eq!(cursor.stack.ledger.live_bytes(), 0);
        assert_eq!(
            cursor.remaining,
            conf.policy.get(R::DocumentValidationWorkUnitsMax)
        );
    }
    let small = limits().with_limit(R::JsonMembersPerObjectMax, Some(32));
    let proof = drive(&thing, &small).unwrap();
    let spent = proof.owner.trace.work.iter().sum::<u64>();
    drop(proof);
    assert!(
        drive(
            &thing,
            &small
                .clone()
                .with_limit(R::DocumentValidationWorkUnitsMax, Some(spent))
        )
        .is_ok()
    );
    assert!(
        matches!(drive(&thing,&small.clone().with_limit(R::DocumentValidationWorkUnitsMax,Some(spent-1))),Err(Cause::Limit(v)) if v.kind()==R::DocumentValidationWorkUnitsMax)
    );
}

#[test]
fn private_context_opaque_and_unread_affordance_resources_are_not_skipped() {
    let mut thing = base();
    thing.context = crate::context::Context::builder()
        .object(BTreeMap::from([(
            "long-context-key".into(),
            serde_json::json!({"supplied":["leaf"]}),
        )]))
        .build()
        .unwrap();
    thing._extra_fields.insert(
        "extension".into(),
        serde_json::json!({"child":[null,true,23,"bytes"]}),
    );
    let property = thing.properties.as_mut().unwrap().get_mut("p").unwrap();
    property._schema =
        serde_json::from_str(r#"{"type":"array","items":{"type":"string"}}"#).unwrap();
    property._interaction.uri_variables =
        Some(BTreeMap::from([("v".into(), DataSchema::null().into())]));
    property._interaction.forms[0].additional_responses = Some(vec![
        serde_json::from_str(r#"{"schema":"schema-name","success":true}"#).unwrap(),
    ]);
    let proof = drive(&thing, &limits()).unwrap();
    let extensions = proof.owner.counts.extensions;
    let schemas = proof.owner.counts.schemas;
    let edges = proof.owner.counts.edges;
    drop(proof);
    for (kind, limit) in [
        (R::JsonNestingDepthMax, 0),
        (R::JsonMembersPerObjectMax, 1),
        (R::JsonArrayItemsMax, 0),
        (R::SchemaCompositionDepthMax, 1),
        (R::UriVariablesPerFormMax, 0),
        (R::AdditionalResponsesPerFormMax, 0),
        (R::UriTemplateSourceBytesMax, 1),
        (R::ExtensionBytesMax, extensions - 1),
        (R::SchemaNodesPerDocumentMax, schemas - 1),
        (R::SchemaReferenceEdgesPerDocumentMax, edges - 1),
    ] {
        assert!(
            matches!(checked_limit(&thing,kind,limit),Cause::Limit(v) if v.kind()==kind&&v.phase()==Phase::Inspect),
            "{kind:?}"
        );
    }
    assert!(
        drive(
            &thing,
            &limits().with_limit(R::ExtensionBytesMax, Some(extensions))
        )
        .is_ok()
    );
    assert!(
        drive(
            &thing,
            &limits().with_limit(R::SchemaReferenceEdgesPerDocumentMax, Some(edges))
        )
        .is_ok()
    );
    assert!(
        drive(
            &thing,
            &limits().with_limit(R::SchemaNodesPerDocumentMax, Some(schemas))
        )
        .is_ok()
    );
}
#[test]
fn short_native_and_byte_allowances_preserve_the_exact_continuation() {
    let mut thing = base();
    let prefix = "repeated-long-prefix-".repeat(32);
    thing.security_definitions.clear();
    thing
        .security_definitions
        .insert(format!("{prefix}a"), SecurityScheme::nosec());
    thing
        .security_definitions
        .insert(format!("{prefix}b"), SecurityScheme::nosec());
    thing.security = vec![format!("{prefix}b")];
    let mut cursor = ValidatedThingCursor::from_thing(&thing, &config(&limits()), ledger());
    let mut frozen_native = false;
    let mut frozen_text = false;
    let mut frozen_comparison = false;
    for _ in 0..30_000 {
        if cursor.stack.transfer.is_none()
            && cursor.stack.frames.len > 0
            && cursor.stack.frames.len + 2 <= cursor.stack.frames.capacity
        {
            let freeze = match cursor.stack.top() {
                Frame::Structure(f) if matches!(f.node, Node::Map(_)) && !frozen_native => {
                    frozen_native = true;
                    Some((W::DocumentNodes, 0))
                }
                Frame::Text(text, pos) if *pos < text.len() && !frozen_text => {
                    frozen_text = true;
                    Some((W::CodecInputBytes, 0))
                }
                Frame::Lookup(f)
                    if f.current.is_some_and(|v| f.byte < v.len()) && !frozen_comparison =>
                {
                    frozen_comparison = true;
                    Some((W::CodecInputBytes, 1))
                }
                _ => None,
            };
            if let Some((class, n)) = freeze {
                let before = cursor.trace;
                let remaining = cursor.remaining;
                for _ in 0..3 {
                    let mut small = budget(10_000);
                    small.set_remaining(class, n);
                    cursor = match cursor.step(&mut small, false) {
                        ValidatedThingProgress::Pending(v) => v,
                        _ => panic!("short action must remain pending"),
                    };
                    assert_eq!(cursor.trace, before);
                    assert_eq!(cursor.remaining, remaining);
                    assert_eq!(small.remaining(class), n);
                }
            }
        }
        match cursor.step(&mut budget(10_000), false) {
            ValidatedThingProgress::Pending(next) => cursor = next,
            ValidatedThingProgress::Complete(proof) => {
                assert!(frozen_native && frozen_text && frozen_comparison);
                assert!(proof.owner.trace.compared >= 2 * prefix.len() as u64);
                return;
            }
            ValidatedThingProgress::Failed(cause) => panic!("{cause:?}"),
        }
    }
    panic!("no complete proof")
}

fn semantic_input() -> Thing {
    let mut t = base();
    t.base = Some(crate::data_type::BaseUri::parse("http://example/a/b/").unwrap());
    t.properties
        .as_mut()
        .unwrap()
        .get_mut("p")
        .unwrap()
        ._interaction
        .forms[0]
        .href = crate::data_type::FormHref::parse("../../value?x#f").unwrap();
    t
}
fn semantics(c: &mut ValidatedPropertyReadCursor<'_>) -> Result<(), Cause> {
    for _ in 0..1_000_000 {
        match c.step(&mut budget(10_000), false)? {
            ValidatedPropertyReadStep::Pending => {}
            ValidatedPropertyReadStep::Ready(_) => c.acknowledge(),
            ValidatedPropertyReadStep::Done => return Ok(()),
        }
    }
    panic!("semantic progress")
}
#[test]
fn semantic_work_remains_linear_across_ready_rewind_and_exact_exhaustion() {
    let t = semantic_input();
    let policy = limits().with_limit(R::JsonMembersPerObjectMax, Some(32));
    let p = drive(&t, &policy).unwrap();
    let validation = limits().document_validation_work_units_max().unwrap() - p.owner.remaining;
    let mut c = p.into_property_read();
    let before = c.owner.remaining;
    semantics(&mut c).unwrap();
    let one_pass = before - c.owner.remaining;
    assert!(one_pass > 0);
    let after = c.owner.remaining;
    let trace = c.owner.trace;
    for _ in 0..10 {
        assert!(matches!(
            c.step(&mut budget(0), true).unwrap(),
            ValidatedPropertyReadStep::Done
        ));
    }
    assert_eq!(c.owner.remaining, after);
    assert_eq!(c.owner.trace, trace);
    c = c.rewind();
    assert_eq!(c.owner.remaining, after);
    semantics(&mut c).unwrap();
    assert_eq!(c.owner.remaining, after - one_pass);
    drop(c);
    let exact = policy.clone().with_limit(
        R::DocumentValidationWorkUnitsMax,
        Some(validation + one_pass),
    );
    let mut c = drive(&t, &exact).unwrap().into_property_read();
    semantics(&mut c).unwrap();
    assert_eq!(c.owner.remaining, 0);
    c = c.rewind();
    let cause = semantics(&mut c).unwrap_err();
    assert!(
        matches!(cause, Cause::Limit(l) if l.kind()==R::DocumentValidationWorkUnitsMax && l.phase()==Phase::Semantics)
    );
    let trace = c.owner.trace;
    for _ in 0..5 {
        assert!(matches!(c.step(&mut budget(10_000),true),Err(e) if e==cause));
    }
    assert_eq!(trace, c.owner.trace);
    let short = policy.with_limit(
        R::DocumentValidationWorkUnitsMax,
        Some(validation + one_pass - 1),
    );
    let mut c = drive(&t, &short).unwrap().into_property_read();
    assert!(
        matches!(semantics(&mut c),Err(Cause::Limit(l)) if l.kind()==R::DocumentValidationWorkUnitsMax)
    );
}
#[test]
fn semantic_short_credit_has_no_traversal_allocation_or_carried_credit() {
    let t = semantic_input();
    let mut c = drive(&t, &limits()).unwrap().into_property_read();
    for _ in 0..10_000 {
        if c.phase == LendingPhase::Uri {
            break;
        }
        match c.step(&mut budget(10_000), false).unwrap() {
            ValidatedPropertyReadStep::Ready(_) => c.acknowledge(),
            _ => {}
        }
    }
    assert!(c.phase == LendingPhase::Uri);
    let required = c.resolver.as_ref().unwrap().work();
    let trace = c.owner.trace;
    let remaining = c.owner.remaining;
    let live = c.owner.stack.ledger.live_bytes();
    for _ in 0..100 {
        let mut short = budget(10_000).with_remaining(W::UriBytes, required - 1);
        assert!(matches!(
            c.step(&mut short, false).unwrap(),
            ValidatedPropertyReadStep::Pending
        ));
        assert_eq!(short.remaining(W::UriBytes), required - 1);
        assert_eq!(c.scratch.len, 0);
        assert_eq!(c.owner.remaining, remaining);
        assert_eq!(c.owner.trace, trace);
        assert_eq!(c.owner.stack.ledger.live_bytes(), live);
    }
    semantics(&mut c).unwrap();
}
#[test]
fn semantic_content_and_resolved_uri_exact_boundaries() {
    let t = semantic_input();
    let mut c = drive(&t, &limits()).unwrap().into_property_read();
    semantics(&mut c).unwrap();
    let effective = c.effective;
    let resolved = c.scratch.len as u64;
    assert!(effective > c.owner.counts.content);
    drop(c);
    for (resource, observed) in [
        (R::GeneratedEffectiveDocumentBytesMax, effective),
        (R::UriTemplateSourceBytesMax, resolved),
    ] {
        for limit in [observed - 1, observed, observed + 1] {
            let policy = limits().with_limit(resource, Some(limit));
            let mut c = drive(&t, &policy).unwrap().into_property_read();
            let result = semantics(&mut c);
            if limit < observed {
                assert!(
                    matches!(result,Err(Cause::Limit(l)) if l.kind()==resource && l.observed()==observed && l.phase()==Phase::Semantics)
                );
            } else {
                result.unwrap();
            }
        }
    }
}
#[test]
fn semantic_upstream_charge_and_preallocation_thresholds() {
    const SOURCE: u64 = 131072;
    let t = semantic_input();
    let source = vec![0u8; SOURCE as usize].into_boxed_slice();
    let mut child = AdmissionLedger::new(
        SlotIndex::new(1),
        Generation::new(1).unwrap(),
        SOURCE,
        8 << 20,
        0,
        0,
        0,
        0,
    );
    child
        .try_reserve_source(R::RetainedSourceBytesPerOwnerMax, SOURCE)
        .unwrap()
        .commit();
    let p = drive_with_ledger(&t, &limits(), child).unwrap();
    let frame_bytes = p.owner.stack.ledger.live_bytes() - SOURCE;
    let mut c = p.into_property_read();
    semantics(&mut c).unwrap();
    assert_eq!(
        c.owner.stack.ledger.live_bytes(),
        SOURCE + frame_bytes + c.scratch.capacity as u64
    );
    let temporary = frame_bytes + c.scratch.capacity as u64 + inline_bytes();
    let request = c.scratch.capacity as u64;
    // Drop does not release a physically live upstream owner on our behalf.
    c.scratch.release(&mut c.owner.stack.ledger);
    assert_eq!(c.owner.stack.ledger.live_bytes(), SOURCE + frame_bytes);
    drop(c);
    assert_eq!(source.len() as u64, SOURCE);
    for resource in [
        R::AdmissionTemporaryBytesPerOperationMax,
        R::AdmissionTemporaryBytesGlobalMax,
        R::PeakLiveBytesPerAdmissionMax,
        R::EngineLiveBytesGlobalMax,
        R::AdmissionPeakLiveBytesGlobalMax,
        R::LargestContiguousAllocationBytesMax,
    ] {
        // Isolate this semantic request after a production proof. No proof or
        // allowance is fabricated; test-only mutation tightens one policy row.
        let mut c = drive(&t, &limits()).unwrap().into_property_read();
        let observed = if resource == R::LargestContiguousAllocationBytesMax {
            request
        } else {
            temporary
        };
        c.owner.policy.values[CATALOG.iter().position(|r| *r == resource).unwrap()] = observed - 1;
        assert!(
            matches!(semantics(&mut c),Err(Cause::Limit(l)) if l.kind()==resource && l.observed()==observed && l.phase()==Phase::Semantics)
        );
        assert_eq!(c.scratch.capacity, 0);
    }
}

#[test]
fn named_uri_lengths_fit_linear_production_support() {
    // The static work ceiling is deliberately much smaller than gateway's.
    // These real named policies exercise a full-sized composite URI without
    // replacing the work/temporary ceilings with fixture-specific allowances.
    for policy in [GatewayDefaultV1::LIMITS, BenchmarkStaticReferenceV1::LIMITS] {
        let mut t = base();
        let prefix = "http://a/";
        let maximum = policy.uri_template_source_bytes_max().unwrap() as usize;
        t.base = Some(crate::data_type::BaseUri::parse(prefix).unwrap());
        let raw = "a".repeat(maximum - prefix.len());
        t.properties
            .as_mut()
            .unwrap()
            .get_mut("p")
            .unwrap()
            ._interaction
            .forms[0]
            .href = crate::data_type::FormHref::parse(&raw).unwrap();
        let mut c = drive(&t, policy).unwrap().into_property_read();
        semantics(&mut c).unwrap();
        assert_eq!(c.scratch.len, maximum);
        assert!(c.owner.remaining > 0);
    }
}

#[test]
fn semantic_native_and_common_prefix_search_never_replay_on_short_credit() {
    let mut t = semantic_input();
    let prefix = "common-prefix-".repeat(32);
    t.security_definitions.clear();
    t.security_definitions
        .insert(format!("{prefix}a"), SecurityScheme::nosec());
    t.security_definitions
        .insert(format!("{prefix}b"), SecurityScheme::nosec());
    t.security = vec![format!("{prefix}b")];
    let mut c = drive(&t, &limits()).unwrap().into_property_read();
    let mut native = false;
    let mut compare = false;
    for _ in 0..50_000 {
        let freeze = if !native && c.phase == LendingPhase::Property {
            native = true;
            Some((W::DocumentNodes, 1))
        } else if !compare && c.phase == LendingPhase::Compare {
            compare = true;
            Some((W::CodecInputBytes, 1))
        } else {
            None
        };
        if let Some((class, credit)) = freeze {
            let trace = c.owner.trace;
            let remaining = c.owner.remaining;
            let pos = c.compare;
            for _ in 0..10 {
                let mut short = budget(10_000).with_remaining(class, credit);
                assert!(matches!(
                    c.step(&mut short, false).unwrap(),
                    ValidatedPropertyReadStep::Pending
                ));
                assert_eq!(c.owner.trace, trace);
                assert_eq!(c.owner.remaining, remaining);
                assert_eq!(c.compare, pos);
                assert_eq!(short.remaining(class), credit);
            }
        }
        match c.step(&mut budget(10_000), false).unwrap() {
            ValidatedPropertyReadStep::Pending => {}
            ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Property { .. }) => {
                c.acknowledge()
            }
            ValidatedPropertyReadStep::Ready(ValidatedPropertyReadEvent::Form(f)) => {
                assert_eq!(f.security_scheme(), Some("nosec"));
                assert!(native && compare);
                return;
            }
            ValidatedPropertyReadStep::Done => panic!("no readable Form"),
        }
    }
    panic!("no semantic progress")
}
