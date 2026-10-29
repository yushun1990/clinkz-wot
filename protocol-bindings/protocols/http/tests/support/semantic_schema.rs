// Independently authored compiler and client, through public Consumer admission.
mod standard_schema {
    use super::*;
    use clinkz_wot_td::semantic::{
        SchemaConstraints as C, SchemaItemsMode, SemanticAccessError, UriVariableScope,
    };

    struct SchemaCompiler {
        base: Compiler,
        security_seen: Arc<AtomicUsize>,
    }
    impl HostSemanticCompiler for SchemaCompiler {
        type Artifact = Program;
        const WORK: u64 = 1;
        fn compatibility(&self) -> BindingArtifactCompatibility {
            COMPAT
        }
        fn configuration(&self) -> ConsumerBindingConfiguration {
            self.base.config
        }
        fn schemes(&self) -> &'static [&'static str] {
            &["replybus"]
        }
        fn compile(&self, input: ResolvedBindingInput<'_>) -> CoreResult<Program> {
            let document = input.document;
            assert_eq!(document.thing_id(), Some("urn:semantic-read"));
            assert_eq!(document.thing_metadata().title, Some("semantic read"));
            let property = document.property();
            assert_eq!(property.name, input.property);
            let schema = property.schema;
            let facts = schema.facts();
            let code = match facts.declared_type {
                None => {
                    let SemanticValue::Number(minimum) = schema
                        .semantics()
                        .field("https://www.w3.org/2019/wot/json-schema#minimum")
                        .unwrap()
                        .value()
                    else {
                        panic!("minimum");
                    };
                    assert_eq!(minimum.as_i64(), Some(2));
                    0
                }
                Some("string") => {
                    assert!(matches!(facts.constraints, C::String { .. }));
                    1
                }
                Some("integer") => {
                    assert!(matches!(facts.constraints, C::Integer { .. }));
                    2
                }
                Some("object") => {
                    assert!(property.observable && facts.read_only && !facts.write_only);
                    assert_eq!(facts.unit, Some("Kelvin"));
                    assert_eq!(facts.format, Some("record"));
                    assert_eq!(facts.metadata.title, Some("sample"));
                    let constant = facts.constant.unwrap();
                    assert_eq!(constant["reading"].as_i64(), Some(7));
                    assert_eq!(constant["@context"].as_str(), Some("literal data"));
                    assert_eq!(facts.default.unwrap()["reading"].as_i64(), Some(7));
                    let enumeration = facts.enumeration.unwrap();
                    assert_eq!(enumeration.len(), 1);
                    assert_eq!(enumeration[0]["reading"].as_i64(), Some(7));
                    let C::Object {
                        required: Some(required),
                    } = facts.constraints
                    else {
                        panic!("required");
                    };
                    assert_eq!(required.len(), 2);
                    assert_eq!(required[0], "reading");
                    assert_eq!(required[1], "history");
                    let mut fields = schema.properties().unwrap();
                    let (history_name, history) = fields.next().unwrap();
                    let (name_name, name) = fields.next().unwrap();
                    let (reading_name, reading) = fields.next().unwrap();
                    assert_eq!(
                        (history_name, name_name, reading_name),
                        ("history", "name", "reading")
                    );
                    assert!(fields.next().is_none());
                    let r = reading.facts();
                    assert_eq!(r.declared_type, Some("integer"));
                    assert_eq!(
                        r.constraints,
                        C::Integer {
                            minimum: Some(0),
                            exclusive_minimum: Some(-1),
                            maximum: Some(10),
                            exclusive_maximum: Some(11),
                            multiple_of: Some(1)
                        }
                    );
                    assert_eq!(r.constant.unwrap().as_i64(), Some(7));
                    assert_eq!(r.default, Some(&Value::Null));
                    let values = r.enumeration.unwrap();
                    assert_eq!(values.len(), 2);
                    assert_eq!(values[0].as_i64(), Some(7));
                    assert_eq!(values[1].as_i64(), Some(9));
                    assert_eq!(
                        reading
                            .semantics()
                            .field("urn:vendor:code")
                            .unwrap()
                            .value(),
                        SemanticValue::Text("sensor")
                    );
                    assert!(reading.properties().is_none());
                    assert_eq!(
                        history.facts().constraints,
                        C::Array {
                            min_items: Some(1),
                            max_items: Some(4)
                        }
                    );
                    let number = history.items().unwrap().next().unwrap().facts();
                    assert_eq!(history.items_mode(), Some(SchemaItemsMode::Tuple));
                    assert_eq!(
                        number.constraints,
                        C::Number {
                            minimum: Some(0.0),
                            exclusive_minimum: Some(-1.0),
                            maximum: Some(10.0),
                            exclusive_maximum: Some(11.0),
                            multiple_of: Some(0.5)
                        }
                    );
                    let mut alternatives = history.one_of().unwrap();
                    let first = alternatives.next().unwrap();
                    let second = alternatives.next().unwrap();
                    assert!(alternatives.next().is_none());
                    assert_eq!(first.items_mode(), Some(SchemaItemsMode::Uniform));
                    assert_eq!(second.items_mode(), Some(SchemaItemsMode::Tuple));
                    assert_eq!(
                        first.items().unwrap().next().unwrap().facts().declared_type,
                        Some("null")
                    );
                    assert_eq!(
                        second
                            .items()
                            .unwrap()
                            .next()
                            .unwrap()
                            .facts()
                            .declared_type,
                        Some("boolean")
                    );
                    let name = name.facts();
                    assert_eq!(
                        name.constraints,
                        C::String {
                            min_length: Some(1),
                            max_length: Some(8),
                            pattern: Some("[a-z]+"),
                            content_encoding: Some("base64"),
                            content_media_type: Some("text/plain")
                        }
                    );
                    assert_eq!(name.format, Some("date"));
                    let mut global = document.uri_variables(UriVariableScope::Thing).unwrap();
                    let (key, variable) = global.next().unwrap();
                    assert_eq!(key, "global");
                    assert_eq!(variable.facts().declared_type, Some("integer"));
                    assert!(global.next().is_none());
                    let (key, variable) = document
                        .uri_variables(UriVariableScope::Property)
                        .unwrap()
                        .next()
                        .unwrap();
                    assert_eq!(key, "local");
                    assert_eq!(variable.facts().declared_type, Some("string"));
                    assert_eq!(
                        variable
                            .semantics()
                            .field("urn:variable:scope")
                            .unwrap()
                            .value(),
                        SemanticValue::Text("property")
                    );
                    let response = document
                        .additional_response(input.property, input.form_index, 0)
                        .unwrap()
                        .unwrap();
                    assert_eq!(response.declared_content_type, None);
                    assert_eq!(response.effective_content_type, "application/json");
                    assert!(!response.success);
                    assert_eq!(response.schema, Some("Problem"));
                    assert_eq!(
                        response
                            .semantics
                            .field("urn:response:kind")
                            .unwrap()
                            .value(),
                        SemanticValue::Text("error")
                    );
                    let problem = document.schema_definition("Problem").unwrap();
                    assert_eq!(
                        problem
                            .properties()
                            .unwrap()
                            .next()
                            .unwrap()
                            .1
                            .facts()
                            .declared_type,
                        Some("string")
                    );
                    assert!(matches!(
                        document.schema_definition("Unused"),
                        Err(SemanticAccessError::OutsidePreparedScope)
                    ));
                    assert!(
                        document
                            .additional_response(input.property, input.form_index, 1)
                            .unwrap()
                            .is_none()
                    );
                    3
                }
                _ => panic!("unexpected schema"),
            };
            let form = document
                .form_facts(input.property, input.form_index)
                .unwrap();
            assert_eq!(form.content_type, "application/json");
            assert_eq!(form.subprotocol, input.subprotocol);
            assert_eq!(
                form.effective_operations[0],
                clinkz_wot_td::data_type::Operation::ReadProperty
            );
            assert_eq!(form.declared_operations.is_some(), code == 3);
            assert!(form.declared_security.is_none());
            assert_eq!(form.effective_security.len(), 1);
            assert_eq!(form.effective_security[0], "none");
            let security = document.security("none").unwrap();
            assert_eq!(security.scheme, "nosec");
            assert_eq!(security.description, Some("access"));
            assert_eq!(security.descriptions.unwrap().as_map()["en"], "access");
            assert_eq!(
                security
                    .semantics
                    .field("urn:security:annotation")
                    .unwrap()
                    .value(),
                SemanticValue::Text("preserved")
            );
            if let Some(proxy) = security.proxy {
                assert_eq!(proxy, "http://127.0.0.1:32199/");
                self.security_seen.fetch_or(2, Ordering::Relaxed);
                // This independent protocol also declines unsupported targets;
                // observing the fact does not grant proxy execution authority.
                return Err(CoreError::UnsupportedOperation(ErrorContext::new(
                    ErrorPhase::Admission,
                    RetryClass::Never,
                )));
            }
            self.security_seen.fetch_or(1, Ordering::Relaxed);
            let mut program = self.base.compile(input)?;
            // Observable source-free instructions, distinguished after TD Drop.
            program.request_topic[31] = code;
            Ok(program)
        }
    }
    struct SchemaClient {
        config: ConsumerBindingConfiguration,
        seen: Arc<AtomicUsize>,
    }
    impl binding::ClientBinding for SchemaClient {
    fn supports_operation(&self, operation: clinkz_wot_core::Operation) -> bool { operation == clinkz_wot_core::Operation::ReadProperty }
        fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
            COMPAT
        }
        fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
            Some(self.config)
        }
        fn invoke(
            &self,
            request: OutboundRequest,
            artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
        ) -> Result<
            HostBindingCallBox<CoreResult<InteractionOutput>>,
            BindingInputRejection<OutboundRequest>,
        > {
            let program = artifact.artifact().try_payload::<Program>(COMPAT).unwrap();
            self.seen
                .fetch_or(1 << program.request_topic[31], Ordering::Relaxed);
            Client(self.config).invoke(request, artifact)
        }
    }
    fn complex() -> Value {
        json!({"type":"object", "title":"sample", "readOnly":true, "observable":true, "unit":"Kelvin", "format":"record",
        "const":{"reading":7,"@context":"literal data"}, "default":{"reading":7}, "enum":[{"reading":7}], "required":["reading","history"],
        "uriVariables":{"local":{"type":"string","minLength":1,"urn:variable:scope":"property"}},
        "properties":{
            "reading":{"type":"integer","minimum":0,"exclusiveMinimum":-1,"maximum":10,"exclusiveMaximum":11,"multipleOf":1,
                "const":7,"default":null,"enum":[7,9],"urn:vendor:code":"sensor"},
            "history":{"type":"array","minItems":1,"maxItems":4,"items":[{"type":"number","minimum":0,"exclusiveMinimum":-1,"maximum":10,"exclusiveMaximum":11,"multipleOf":0.5}],
                    "oneOf":[{"type":"array","items":{"type":"null"}},{"type":"array","items":[{"type":"boolean"}]}]},
            "name":{"type":"string","minLength":1,"maxLength":8,"pattern":"[a-z]+","format":"date","contentEncoding":"base64","contentMediaType":"text/plain"}
        }})
    }
    #[test]
    fn admitted_common_facts_survive_public_consumer_compilation_and_source_drop() {
        recipe().block_on(async {
            let seen = Arc::new(AtomicUsize::new(0));
            let security_seen = Arc::new(AtomicUsize::new(0));
            for variant in 0..7 {
                let compiled = Arc::new(AtomicUsize::new(0));
                let config = ConsumerBindingConfiguration::for_semantic_host(64, BindingConfigurationDigest::new([93;32]), COMPAT,4096,16).unwrap();
                let compiler = HostBindingCompilerRegistration::try_new_inline_semantic(SchemaCompiler { base:Compiler { config, compiled:compiled.clone() }, security_seen:security_seen.clone() }).unwrap_or_else(|_| panic!("capture"));
                let owner = HostConsumerBindingPreparation::new(HostConsumerBindingPreparationInput::new(
                    BindingRegistrationIdentity::new(BindingId::new(903), BindingGeneration::INITIAL, config.configuration(),COMPAT,903),
                    BindingExecutionSupport::host_erased(), compiler, Box::new(SchemaClient { config, seen:seen.clone() }),
                    BindingResourceDeclarations::new(BindingLifetimeFootprint::new(3,1024),BindingLifetimeFootprint::new(16,8192))
                        .with_resource_contract(BindingResourceContract::ConsumerOwnedHostV2).with_consumer_result_limits(4096,16), BindingStatusPolicy::new(2,17)
                )).unwrap_or_else(|_| panic!("registration")).try_into_inline_consumer_compiler().unwrap_or_else(|_| panic!("admission"));
                let policy = policy(); let parent = Arc::new(clinkz_wot_servient::host_accounts::HostResourceAccounts::try_from_accounts(policy.parent_accounts()).unwrap());
                let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(owner,policy,Arc::new(Clock(Instant::now())),parent.clone()).unwrap_or_else(|_| panic!("runtime"));
                let mut td = source(json!([]));
                td["securityDefinitions"]["none"]["description"] = json!("access");
                td["securityDefinitions"]["none"]["descriptions"] = json!({"en":"access"});
                td["securityDefinitions"]["none"]["urn:security:annotation"] = json!("preserved");
                if variant == 6 { td["securityDefinitions"]["none"]["proxy"] = json!("http://127.0.0.1:32199/"); }
                td["properties"]["a"] = match variant { 0 => json!({"minimum":2}), 1 => json!({"type":"string"}), 2 => json!({"type":"integer"}), _ => complex() };
                td["properties"]["a"]["forms"] = json!([{"href":"replybus:temperature","urn:replybus:replyTopic":"replies"}]);
                td["urn:opaque:annotation"] = json!("retained"); td["properties"]["a"]["urn:opaque:unit"] = json!("Kelvin");
                if variant >= 3 {
                    td["uriVariables"] = json!({"global":{"type":"integer","minimum":0}});
                    td["schemaDefinitions"] = json!({"Problem":{"type":"object","properties":{"message":{"type":"string","minLength":3}}},
                        "Unused":{"@context":{"@base":"https://example.org/"}}});
                    let form = &mut td["properties"]["a"]["forms"][0];
                    form["op"] = json!(["readproperty"]);
                    form["additionalResponses"] = json!([{"success":false,"schema":"Problem","urn:response:kind":"error"}]);
                    if variant == 4 { td["schemaDefinitions"]["Problem"]["$ref"] = json!("#/schemaDefinitions/Unused"); }
                    if variant == 5 { td["properties"]["a"]["properties"]["reading"]["@context"] = json!({"reference":"tm:ref"});
                        td["properties"]["a"]["properties"]["reading"]["reference"] = json!("#/schemaDefinitions/Unused"); }
                }
                let thing = serde_json::from_value(td).unwrap();
                let StartStatus::Ready(handle) = runtime.consume(&thing,&mut work(),|build| {
                    build.step(&mut WorkBudget::new()); assert_eq!(compiled.load(Ordering::Relaxed),0);
                    for _ in 0..300_000 { build.step(&mut work()); if build.is_finished() {return;} } panic!("build stalled");
                }).unwrap() else { panic!("publication"); };
                drop(thing);
                if variant >= 4 {
                    assert_eq!(compiled.load(Ordering::Relaxed),0);
                    assert!(handle.read_property("a",InteractionOptions::new(),&mut work()).is_err());
                } else {
                    assert_eq!(compiled.load(Ordering::Relaxed),1);
                    let mut call = handle.read_property("a",InteractionOptions::new(),&mut work()).unwrap();
                    assert!(take(&mut call).await.is_ok()); drop(call);
                }
                close(runtime,handle,observer,parent);
            }
            assert_eq!(seen.load(Ordering::Relaxed),0b1111);
            assert_eq!(security_seen.load(Ordering::Relaxed),0b11);
        });
    }
}
