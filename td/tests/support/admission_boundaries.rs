//! Fixed public-input threshold oracles, shared by host tests and the booted
//! no_std runtime. Expected counts come from these literals, never TD traces.
extern crate alloc;
use alloc::{format, vec};
use clinkz_wot_foundation::{
    AdmissionLedger, ResourceKind as R, ResourceLimits, WorkBudget, WorkClass,
};
use clinkz_wot_td::{
    ValidatedPropertyReadStep, ValidatedThingAdmissionConfig, ValidatedThingCause as Cause,
    ValidatedThingCursor, ValidatedThingPhase as Phase, ValidatedThingProgress, thing::Thing,
};

const BASE: &str = r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"x","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}}}"#;

pub struct Case {
    pub thing: Thing,
    pub kind: R,
    pub maximum: u64,
    pub phase: Phase,
}
fn base() -> Thing {
    serde_json::from_str(BASE).unwrap()
}
fn with_fields(fields: &str) -> Thing {
    serde_json::from_str(&format!("{},{} }}", &BASE[..BASE.len() - 1], fields)).unwrap()
}
pub fn for_each(mut check: impl FnMut(Case)) {
    // Provision one caller-owned input at a time in the fixed runtime arena.
    let mut add = |thing: Thing, kind, maximum, phase| {
        check(Case {
            thing,
            kind,
            maximum,
            phase,
        })
    };
    // Fifteen supplied typed nodes: nine containers, five text leaves, one
    // security-definition association. Empty option slots/field labels add none.
    let text = ("https://www.w3.org/2022/wot/td/v1.1".len() + 1 + 4 + 4 + 5) as u64;
    for (kind, maximum) in [
        (R::DocumentBytesMax, text),
        (R::StringBytesMax, text),
        (R::GeneratedEffectiveDocumentBytesMax, text),
        (R::JsonValueNodesPerDocumentMax, 15),
    ] {
        add(base(), kind, maximum, Phase::Inspect);
    }
    // Extension keys opaque/v, null/bool scalars, UTF-8 text, AP Number text.
    let opaque = with_fields(r#""opaque":{"v":[null,true,"é",123]}"#);
    add(
        opaque,
        R::ExtensionBytesMax,
        6 + 1 + 8 + 8 + 2 + 3,
        Phase::Inspect,
    );
    add(
        with_fields(r#""opaque":123"#),
        R::NumberLexemeBytesMax,
        3,
        Phase::Inspect,
    );
    let mut deep = base();
    let mut value = serde_json::Value::Null;
    for _ in 0..8 {
        value = serde_json::Value::Array(vec![value]);
    }
    deep._extra_fields.insert("deep".into(), value);
    // Thing + extension map + eight nested arrays; associations add no depth.
    add(deep, R::JsonNestingDepthMax, 10, Phase::Inspect);
    let mut members = base();
    members
        ._extra_fields
        .extend((0..32).map(|i| (format!("k{i}"), serde_json::Value::Null)));
    add(members, R::JsonMembersPerObjectMax, 32, Phase::Inspect);
    let mut items = base();
    items._extra_fields.insert(
        "items".into(),
        serde_json::Value::Array(vec![serde_json::Value::Null; 32]),
    );
    add(items, R::JsonArrayItemsMax, 32, Phase::Inspect);
    let interactions = r#""properties":{"a":{"type":"null","forms":[{"href":"a"},{"href":"b"},{"href":"c"}]},"b":{"type":"null","forms":[]}},"actions":{"a":{"forms":[{"href":"a"}]}},"events":{"e":{"forms":[{"href":"a"},{"href":"b"}]}},"forms":[{"href":"a"},{"href":"b"}]"#;
    for (kind, maximum) in [
        (R::AffordancesPerThingMax, 4),
        (R::FormsPerContextMax, 3),
        (R::FormsPerThingMax, 8),
    ] {
        add(with_fields(interactions), kind, maximum, Phase::Inspect);
    }
    add(
        with_fields(
            r#""actions":{"a":{"forms":[{"href":"a","additionalResponses":[{"schema":"absent"},{"schema":"absent"},{"schema":"absent"}]}]}}"#,
        ),
        R::AdditionalResponsesPerFormMax,
        3,
        Phase::Inspect,
    );
    add(
        with_fields(
            r#""events":{"e":{"forms":[],"uriVariables":{"a":{"type":"null"},"b":{"type":"null"},"c":{"type":"null"},"d":{"type":"null"}}}}"#,
        ),
        R::UriVariablesPerFormMax,
        4,
        Phase::Inspect,
    );
    let schema = r#""schemaDefinitions":{"s":{"type":"array","items":{"type":"array","items":{"type":"null"}}}}"#;
    for (kind, maximum) in [
        (R::SchemaNodesPerDocumentMax, 3),
        (R::SchemaCompositionDepthMax, 3),
        (R::SchemaReferenceEdgesPerDocumentMax, 2),
    ] {
        add(with_fields(schema), kind, maximum, Phase::Inspect);
    }
    add(
        with_fields(r#""properties":{"p":{"type":"null","forms":[{"href":"/original/value"}]}}"#),
        R::UriTemplateSourceBytesMax,
        "/original/value".len() as u64,
        Phase::Inspect,
    );
    // The original href fits both short ceilings. Only the completed derived
    // target/effective content exceeds them, so inspection cannot mask a
    // missing semantic-phase check on either execution target.
    let relative = r#""base":"http://a/","properties":{"p":{"type":"null","observable":false,"forms":[{"href":"p"}]}}"#;
    let resolved = "http://a/p".len() as u64;
    // Added leaves: base, property key/type, readOnly/writeOnly/observable
    // scalars, raw href and default content type.
    let supplied = text
        + "http://a/".len() as u64
        + 1
        + "null".len() as u64
        + 8
        + 8
        + 8
        + 1
        + "application/json".len() as u64;
    add(
        with_fields(relative),
        R::UriTemplateSourceBytesMax,
        resolved,
        Phase::Semantics,
    );
    add(
        with_fields(relative),
        R::GeneratedEffectiveDocumentBytesMax,
        supplied + resolved,
        Phase::Semantics,
    );
    let security = r#""properties":{"p":{"type":"null","forms":[{"href":"/p","security":["none","none","none"]}]}}"#;
    for (kind, maximum) in [
        (R::SecurityBranchesPerPlanMax, 3),
        (R::SecurityExpressionDepthMax, 1),
    ] {
        add(with_fields(security), kind, maximum, Phase::Semantics);
    }
}

pub fn execute(
    t: &Thing,
    limits: &ResourceLimits,
    ledger: AdmissionLedger,
    credit: u64,
) -> Result<(), Cause> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(limits).unwrap();
    let budget = || {
        WorkClass::ALL
            .into_iter()
            .fold(WorkBudget::new(), |b, w| b.with_remaining(w, credit))
    };
    let mut cursor = ValidatedThingCursor::from_thing(t, &config, ledger);
    let mut polls = 0;
    let mut read = loop {
        polls += 1;
        assert!(polls < 300_000, "bounded inspection stalled");
        match cursor.step(&mut budget(), false) {
            ValidatedThingProgress::Pending(next) => cursor = next,
            ValidatedThingProgress::Complete(proof) => break proof.into_property_read(),
            ValidatedThingProgress::Failed(cause) => return Err(cause),
        }
    };
    for _ in 0..300_000 {
        match read.step(&mut budget(), false)? {
            ValidatedPropertyReadStep::Pending => {}
            ValidatedPropertyReadStep::Ready(_) => read.acknowledge(),
            ValidatedPropertyReadStep::Done => return Ok(()),
        }
    }
    panic!("bounded semantic progress stalled")
}
