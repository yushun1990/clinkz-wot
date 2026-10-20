use super::*;
use alloc::{format, vec};
use serde_json::json;

fn source() -> Value {
    json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"semantic", "id":"urn:semantic",
        "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
        "properties":{"p":{"forms":[{"href":"http://127.0.0.1/p"}]}}})
}
#[test]
fn compact_vocabularies_expand_standard_prefixes_without_protocol_interpretation() {
    for (prefix, namespace) in [
        ("htv", "http://www.w3.org/2011/http#"),
        ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
        ("xsd", "http://www.w3.org/2001/XMLSchema#"),
        ("dct", "http://purl.org/dc/terms/"),
        ("tm", "https://www.w3.org/2019/wot/tm#"),
        ("td", "https://www.w3.org/2019/wot/td#"),
        ("hctl", "https://www.w3.org/2019/wot/hypermedia#"),
        ("jsonschema", "https://www.w3.org/2019/wot/json-schema#"),
        ("wotsec", "https://www.w3.org/2019/wot/security#"),
    ] {
        let mut td = source();
        td["properties"]["p"]["forms"][0]["@context"] = json!({"@vocab":format!("{prefix}:")});
        td["properties"]["p"]["forms"][0]["annotation"] = json!("preserved");
        let thing = serde_json::from_value(td).unwrap();
        let document = ResolvedSemanticDocument::resolve(&thing, "p", 0).unwrap();
        assert_eq!(
            document
                .scope(SemanticScope::Form {
                    property: "p",
                    index: 0
                })
                .unwrap()
                .field(&format!("{namespace}annotation"))
                .unwrap()
                .value(),
            SemanticValue::Text("preserved")
        );
    }
}

#[test]
fn security_scope_and_metadata_are_preserved_and_share_the_source_work_limit() {
    for proxy in [None, Some("http://127.0.0.1:32199/")] {
        let mut td = source();
        let security = &mut td["securityDefinitions"]["none"];
        security["description"] = json!("access");
        security["descriptions"] = json!({"en":"access"});
        security["urn:security:annotation"] = json!("preserved");
        if let Some(proxy) = proxy {
            security["proxy"] = json!(proxy);
        }
        let thing = serde_json::from_value(td.clone()).unwrap();
        let (result, used) = ResolvedSemanticDocument::resolve_metered(&thing, "p", 0);
        let document = result.unwrap();
        let facts = document.security("none").unwrap();
        assert_eq!(facts.scheme, "nosec");
        assert_eq!(facts.proxy, proxy);
        assert_eq!(facts.description, Some("access"));
        assert_eq!(facts.descriptions.unwrap().as_map()["en"], "access");
        assert_eq!(
            facts
                .semantics
                .field("urn:security:annotation")
                .unwrap()
                .value(),
            SemanticValue::Text("preserved")
        );
        assert!(matches!(
            document.security("unused"),
            Err(SemanticAccessError::OutsidePreparedScope)
        ));
        drop(document);

        assert!(used < RESOLUTION_WORK);
        for (field, value) in [
            ("description", json!("x".repeat(RESOLUTION_WORK as usize))),
            (
                "descriptions",
                json!({"en":"x".repeat(RESOLUTION_WORK as usize)}),
            ),
            (
                "proxy",
                json!(format!(
                    "http://127.0.0.1/{}",
                    "x".repeat(RESOLUTION_WORK as usize)
                )),
            ),
        ] {
            let mut large = td.clone();
            large["securityDefinitions"]["none"][field] = value;
            let thing = serde_json::from_value(large).unwrap();
            let (result, exhausted) = ResolvedSemanticDocument::resolve_metered(&thing, "p", 0);
            assert!(matches!(result, Err(SemanticError::Limit)));
            assert!(exhausted <= RESOLUTION_WORK);
        }
    }
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
    let document = ResolvedSemanticDocument::resolve(&thing, "p", 0).unwrap();
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
    let document = ResolvedSemanticDocument::resolve(&thing, "p", 0).unwrap();
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
        json!({"base":"http://www.w3.org/2011/http#", "x":"base", "verb":"x:methodName"}),
    ] {
        let mut td = source();
        td["@context"] = json!(["https://www.w3.org/2022/wot/td/v1.1", extension]);
        td["properties"]["p"]["forms"][0]["verb"] = json!("POST");
        let thing = serde_json::from_value(td).unwrap();
        assert!(matches!(
            ResolvedSemanticDocument::resolve(&thing, "p", 0),
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
        ResolvedSemanticDocument::resolve(&thing, "p", 0),
        Err(SemanticError::Limit)
    ));
    let mut td = source();
    td["urn:lots:"] = json!(vec![0; NODES]);
    let thing = serde_json::from_value(td).unwrap();
    assert!(matches!(
        ResolvedSemanticDocument::resolve(&thing, "p", 0),
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
    let document = ResolvedSemanticDocument::resolve(&thing, "p", 0).unwrap();
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

#[test]
fn schema_handles_distinguish_absent_empty_and_extension_collisions() {
    let mut td = source();
    td["properties"]["p"]["type"] = json!("object");
    td["properties"]["p"]["https://www.w3.org/2019/wot/json-schema#properties"] =
        json!({"fake":{}});
    let thing = serde_json::from_value(td.clone()).unwrap();
    let document = ResolvedSemanticDocument::resolve(&thing, "p", 0).unwrap();
    assert!(document.property().schema.properties().is_none());
    td["properties"]["p"]["properties"] = json!({});
    td["properties"]["p"]["oneOf"] = json!([]);
    let thing = serde_json::from_value(td.clone()).unwrap();
    let document = ResolvedSemanticDocument::resolve(&thing, "p", 0).unwrap();
    assert_eq!(document.property().schema.properties().unwrap().count(), 0);
    assert_eq!(document.property().schema.one_of().unwrap().count(), 0);
    assert!(document.property().schema.items().is_none());
    td["properties"]["p"]["properties"] = json!({"actual":{"type":"integer"}});
    let thing = serde_json::from_value(td).unwrap();
    let document = ResolvedSemanticDocument::resolve(&thing, "p", 0).unwrap();
    let (name, child) = document
        .property()
        .schema
        .properties()
        .unwrap()
        .next()
        .unwrap();
    assert_eq!(name, "actual");
    assert_eq!(child.facts().declared_type, Some("integer"));
}

#[test]
fn common_literal_and_schema_work_share_the_resolver_envelope() {
    let mut td = source();
    td["properties"]["p"]["const"] = json!(vec![Value::Null; RESOLUTION_WORK as usize]);
    let thing = serde_json::from_value(td).unwrap();
    let (result, used) = ResolvedSemanticDocument::resolve_metered(&thing, "p", 0);
    assert!(matches!(result, Err(SemanticError::Limit)));
    assert_eq!(used, RESOLUTION_WORK);
}

#[test]
fn uniform_and_single_tuple_items_keep_their_meaning_through_td_serde() {
    for (items, expected) in [
        (json!({"type":"integer"}), SchemaItemsMode::Uniform),
        (json!([{"type":"integer"}]), SchemaItemsMode::Tuple),
    ] {
        let mut td = source();
        td["properties"]["p"]["type"] = json!("array");
        td["properties"]["p"]["items"] = items.clone();
        let thing: Thing = serde_json::from_value(td).unwrap();
        assert_eq!(
            serde_json::to_value(&thing).unwrap()["properties"]["p"]["items"],
            items
        );
        let document = ResolvedSemanticDocument::resolve(&thing, "p", 0).unwrap();
        assert_eq!(document.property().schema.items_mode(), Some(expected));
        assert_eq!(
            document
                .property()
                .schema
                .items()
                .unwrap()
                .next()
                .unwrap()
                .facts()
                .declared_type,
            Some("integer")
        );
    }
}

#[test]
fn unsupported_generic_schema_children_are_not_disguised_as_absent() {
    let mut td = source();
    td["properties"]["p"]["items"] = json!({"type":"integer"});
    let thing = serde_json::from_value(td).unwrap();
    assert!(matches!(
        ResolvedSemanticDocument::resolve(&thing, "p", 0),
        Err(SemanticError::UnsupportedDependency)
    ));
}

#[test]
fn semantic_arena_backing_and_node_scope_depth_work_boundaries() {
    let mut td = source();
    // Four scope roots plus the extension array occupy five nodes.
    td["urn:values:"] = json!(vec![0; NODES - 5]);
    let thing = serde_json::from_value(td.clone()).unwrap();
    let (result, used) = ResolvedSemanticDocument::resolve_metered(&thing, "p", 0);
    let document = result.unwrap();
    assert_eq!(document.nodes.len(), NODES);
    assert_eq!(document.nodes.capacity(), NODES);
    assert_eq!(document.scopes.capacity(), SCOPES);
    assert_eq!(
        ResolvedSemanticDocument::capacity_bytes(),
        (document.nodes.capacity() * size_of::<Node<'_>>()
            + document.scopes.capacity() * size_of::<Scope<'_>>()) as u64
    );
    assert!(used > NODES as u64 && used < RESOLUTION_WORK);
    std::println!(
        "semantic arena: {} bytes, {} nodes, {} scopes; saturated-node closure work {used}",
        ResolvedSemanticDocument::capacity_bytes(),
        document.nodes.capacity(),
        document.scopes.capacity()
    );
    td["urn:values:"].as_array_mut().unwrap().push(json!(0));
    let thing = serde_json::from_value(td).unwrap();
    assert!(matches!(
        ResolvedSemanticDocument::resolve(&thing, "p", 0),
        Err(SemanticError::Limit)
    ));

    let mut td = source();
    let form = &mut td["properties"]["p"]["forms"][0];
    form["additionalResponses"] = json!(vec![json!({"success":true}); SCOPES - 4]);
    let thing = serde_json::from_value(td.clone()).unwrap();
    assert_eq!(
        ResolvedSemanticDocument::resolve(&thing, "p", 0)
            .unwrap()
            .scopes
            .len(),
        SCOPES
    );
    td["properties"]["p"]["forms"][0]["additionalResponses"]
        .as_array_mut()
        .unwrap()
        .push(json!({"success":false}));
    let thing = serde_json::from_value(td).unwrap();
    assert!(matches!(
        ResolvedSemanticDocument::resolve(&thing, "p", 0),
        Err(SemanticError::Limit)
    ));

    for (depth, succeeds) in [(DEPTH - 1, true), (DEPTH, false)] {
        let mut td = source();
        let mut value = json!(0);
        for _ in 0..depth {
            value = json!([value]);
        }
        td["urn:nested:"] = value;
        let thing = serde_json::from_value(td).unwrap();
        assert_eq!(
            ResolvedSemanticDocument::resolve(&thing, "p", 0).is_ok(),
            succeeds
        );
    }
    let mut td = source();
    td["urn:text:"] = json!("x".repeat(RESOLUTION_WORK as usize));
    let thing = serde_json::from_value(td).unwrap();
    let (result, used) = ResolvedSemanticDocument::resolve_metered(&thing, "p", 0);
    assert!(matches!(result, Err(SemanticError::Limit)));
    assert!(used <= RESOLUTION_WORK);
}
