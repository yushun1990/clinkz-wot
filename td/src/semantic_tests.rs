use super::*;
use alloc::vec;
use serde_json::json;

fn source() -> Value {
    json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"semantic", "id":"urn:semantic",
        "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
        "properties":{"p":{"forms":[{"href":"http://127.0.0.1/p"}]}}})
}
#[test]
fn resolves_aliases_scoped_contexts_and_keeps_structured_unknown_values() {
    let mut td = source();
    td["@context"] = json!(["https://www.w3.org/2022/wot/td/v1.1", {
        "ex":"urn:outside:", "command":{"@id":"urn:command:", "@context":{"ex":"urn:inside:"}},
        "identifier":{"@id":"urn:identifier:", "@type":"@id"}
    }]);
    td["@type"] = json!("ex:Sensor");
    td["ex:values"] = json!([null, {}, [], {"ex:value":7}]);
    td["command"] = json!({"ex:value":"inner"});
    td["identifier"] = json!("urn:device:7");
    td["properties"]["p"]["@context"] = json!({"verb":"htv:methodName"});
    td["properties"]["p"]["forms"][0]["verb"] = json!("POST");
    td["properties"]["p"]["forms"][0]["response"] =
        json!({"contentType":"application/json", "ex:response":true});
    let thing = serde_json::from_value(td).unwrap();
    let document = ResolvedSemanticDocument::resolve(&thing).unwrap();
    let root = document.scope(SemanticScope::Thing).unwrap();
    let array = root.field("urn:outside:values").unwrap();
    assert_eq!(
        array.children().map(|n| n.value()).collect::<Vec<_>>(),
        [
            SemanticValue::Null,
            SemanticValue::Object,
            SemanticValue::Array,
            SemanticValue::Object
        ]
    );
    assert_eq!(
        root.field("urn:command:")
            .unwrap()
            .field("urn:inside:value")
            .unwrap()
            .value(),
        SemanticValue::Text("inner")
    );
    assert!(
        matches!(root.field("urn:identifier:").unwrap().value(), SemanticValue::Iri(i) if i.is("urn:device:7"))
    );
    let tags = root
        .children()
        .find(|n| n.name() == SemanticName::Keyword("@type"))
        .unwrap();
    assert!(
        matches!(tags.children().next().unwrap().value(), SemanticValue::Iri(i) if i.is("urn:outside:Sensor"))
    );
    assert_eq!(
        document
            .scope(SemanticScope::Form {
                property: "p",
                index: 0
            })
            .unwrap()
            .field("http://www.w3.org/2011/http#methodName")
            .unwrap()
            .value(),
        SemanticValue::Text("POST")
    );
    assert_eq!(
        document
            .scope(SemanticScope::Response {
                property: "p",
                index: 0
            })
            .unwrap()
            .field("urn:outside:response")
            .unwrap()
            .value(),
        SemanticValue::Bool(true)
    );
    assert!(
        root.field("http://www.w3.org/2011/http#methodName")
            .is_none()
    );
}

#[test]
fn preserves_nested_schema_and_security_extension_scope() {
    let mut td = source();
    td["properties"]["p"]["type"] = json!("object");
    td["properties"]["p"]["properties"] =
        json!({"inner":{"type":"string", "urn:schema:unit":"Kelvin"}});
    td["securityDefinitions"]["none"]["urn:security:annotation"] = json!("descriptive");
    let thing = serde_json::from_value(td).unwrap();
    let document = ResolvedSemanticDocument::resolve(&thing).unwrap();
    let schema = document
        .scope(SemanticScope::Property("p"))
        .unwrap()
        .field("https://www.w3.org/2019/wot/json-schema#properties")
        .unwrap();
    let inner = schema
        .children()
        .find(|n| n.name() == SemanticName::Unmapped("inner"))
        .unwrap();
    assert_eq!(
        inner.field("urn:schema:unit").unwrap().value(),
        SemanticValue::Text("Kelvin")
    );
    assert_eq!(
        document
            .scope(SemanticScope::Security("none"))
            .unwrap()
            .field("urn:security:annotation")
            .unwrap()
            .value(),
        SemanticValue::Text("descriptive")
    );
}

#[test]
fn unsupported_context_semantics_are_refused_instead_of_partially_expanded() {
    for extension in [
        json!("https://example.org/unprepared-context"),
        json!({"x":{"@id":"urn:x:", "@container":"@set"}}),
        json!({"@base":"https://example.org/"}),
        json!({"x":{"@reverse":"urn:x:"}}),
        json!({"x":"htv:", "verb":"x:methodName"}),
    ] {
        let mut td = source();
        td["@context"] = json!(["https://www.w3.org/2022/wot/td/v1.1", extension]);
        td["properties"]["p"]["forms"][0]["verb"] = json!("POST");
        let thing = serde_json::from_value(td).unwrap();
        assert!(matches!(
            ResolvedSemanticDocument::resolve(&thing),
            Err(SemanticError::UnsupportedContext)
        ));
    }
}

#[test]
fn cyclic_aliases_and_arena_overflow_have_finite_refusal() {
    let mut td = source();
    td["@context"] = json!(["https://www.w3.org/2022/wot/td/v1.1", {"a":"b", "b":"a"}]);
    td["a"] = json!("cycle");
    let thing = serde_json::from_value(td).unwrap();
    assert!(matches!(
        ResolvedSemanticDocument::resolve(&thing),
        Err(SemanticError::Limit)
    ));
    let mut td = source();
    td["urn:lots:"] = json!(vec![0; NODES]);
    let thing = serde_json::from_value(td).unwrap();
    assert!(matches!(
        ResolvedSemanticDocument::resolve(&thing),
        Err(SemanticError::Limit)
    ));
}

#[test]
fn prefix_flags_and_absolute_iris_follow_the_supported_json_ld_rules() {
    let mut td = source();
    td["@context"] = json!(["https://www.w3.org/2022/wot/td/v1.1", {
        "enabled":{"@id":"urn:enabled:", "@prefix":true},
        "disabled":{"@id":"urn:disabled:", "@prefix":false},
        "term":"urn:not-a-prefix", "http":"urn:other:"
    }]);
    td["enabled:key"] = json!(1);
    td["disabled:key"] = json!(2);
    td["term:key"] = json!(3);
    td["http://example.org/key"] = json!(4);
    let thing = serde_json::from_value(td).unwrap();
    let document = ResolvedSemanticDocument::resolve(&thing).unwrap();
    let root = document.scope(SemanticScope::Thing).unwrap();
    for iri in [
        "urn:enabled:key",
        "disabled:key",
        "term:key",
        "http://example.org/key",
    ] {
        assert!(root.field(iri).is_some(), "missing {iri}");
    }
}
