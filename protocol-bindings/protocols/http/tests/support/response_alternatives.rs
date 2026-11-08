use super::*;

// Response alternatives use the same actual Consumer/Producer fixture and
// lifecycle as the existing primary Read/Write exchanges above.
pub(super) fn response_output(
    mode: u8,
    context: HandlerContext<'_>,
) -> CoreResult<InteractionOutput> {
    if mode == 11 {
        use clinkz_wot_core::{data_schema::PreparedSchema, execution::responses::ResponseData};
        let responses = context
            .response_contract()
            .expect("the original accepted native contract");
        let (i, response) = responses
            .additional()
            .iter()
            .enumerate()
            .find(|(_, r)| !r.success)
            .unwrap();
        let has_code = matches!(&response.data, ResponseData::Value(PreparedSchema::Object(s))
            if s.properties().iter().any(|p| p.name.as_ref() == "code"));
        let body = if has_code {
            br#"{"code":7}"#.as_slice()
        } else {
            br#"{"reason":"busy"}"#.as_slice()
        };
        return Ok(InteractionOutput::with_data(Payload::new(
            body,
            response.content_type.as_ref(),
        ))
        .with_status(InteractionStatus::ApplicationError)
        .try_with_metadata(
            InteractionOutputMetadata::default()
                .with_response_selection(ResponseSelection::Additional(i as u16)),
        )
        .unwrap());
    }
    let (branch, bytes, status): (u16, &[u8], _) = match mode {
        4 => (0, br#""ready""#, InteractionStatus::Ok),
        5 | 10 => (
            1,
            br#"{"reason":"busy"}"#,
            InteractionStatus::ApplicationError,
        ),
        6 => (1, br#"{"reason":1}"#, InteractionStatus::ApplicationError),
        7 => (20, b"7", InteractionStatus::Ok),
        8 => (1, br#"{"reason":"busy"}"#, InteractionStatus::Ok),
        9 => (2, b"7", InteractionStatus::ApplicationError),
        _ => unreachable!(),
    };
    let data = if mode == 10 {
        Payload::new(bytes, "application/json; charset=utf-8")
    } else {
        payload(bytes)
    };
    Ok(InteractionOutput::with_data(data)
        .with_status(status)
        .try_with_metadata(
            InteractionOutputMetadata::default()
                .with_response_selection(ResponseSelection::Additional(branch)),
        )
        .unwrap())
}
fn response_description(addr: SocketAddr, write: bool) -> Value {
    let mut td = description(addr);
    let p = &mut td["properties"]["level"];
    p[if write { "writeOnly" } else { "readOnly" }] = json!(true);
    p["forms"][0]["additionalResponses"] = json!([
        {"success":true,"schema":"ready","htv:statusCodeValue":201},
        {"schema":"failure","htv:statusCodeValue":422}
    ]);
    td["schemaDefinitions"] = json!({
        "ready":{"type":"string","minLength":2,"maxLength":5},
        "failure":{"type":"object","required":["reason"],"properties":{"reason":{"type":"string","minLength":3}}}
    });
    td
}
#[test]
fn td_responses_execute_primary_additional_success_and_application_failure_in_both_roles() {
    io().block_on(async {
        for write in [false, true] {
            let addr = address();
            let td = response_description(addr, write);
            let mut server = Fixture::new(td.clone());
            let accounts = bank();
            let instance = consumer_instance();
            let client = client(
                td,
                Arc::new(AtomicBool::new(false)),
                accounts.clone(),
                &instance,
            );
            for mode in [0, 4, 5] {
                server.app.output.set(mode);
                let mut call = if write {
                    start_write(&client, b"7")
                } else {
                    client
                        .read_property("level", Deadline::NONE, now(0), &mut work())
                        .unwrap()
                };
                let (local, remote) = server.exchange(&mut call).await;
                let output = local.result().unwrap();
                assert_eq!(
                    output.status(),
                    if mode == 5 {
                        InteractionStatus::ApplicationError
                    } else {
                        InteractionStatus::Ok
                    }
                );
                let expected = match mode {
                    0 => ResponseSelection::Primary,
                    4 => ResponseSelection::Additional(0),
                    _ => ResponseSelection::Additional(1),
                };
                assert_eq!(output.metadata().response_selection(), expected);
                assert_eq!(
                    remote
                        .result
                        .as_ref()
                        .unwrap()
                        .metadata()
                        .response_selection(),
                    expected
                );
                assert_eq!(remote.delivery, FiniteDelivery::Delivered);
                if write {
                    assert_eq!(
                        local.effect(),
                        if mode != 0 {
                            ConsumerEffect::Unknown
                        } else {
                            ConsumerEffect::Completed
                        }
                    );
                    assert_eq!(
                        remote.effect,
                        if mode == 5 {
                            FiniteEffect::Unknown
                        } else {
                            FiniteEffect::Completed
                        }
                    );
                    assert_eq!(*server.app.value.borrow(), b"7");
                }
                if mode != 0 {
                    assert_eq!(
                        output.data().unwrap().body.as_ref(),
                        if mode == 4 {
                            br#""ready""#.as_slice()
                        } else {
                            br#"{"reason":"busy"}"#.as_slice()
                        }
                    );
                }
            }
            assert_eq!(
                if write {
                    server.app.writes.get()
                } else {
                    server.app.reads.get()
                },
                3
            );
            drop(client);
            clean(&accounts);
            server.close();
        }
    });
}
#[test]
fn td_response_schema_branch_and_success_lies_refuse_without_erasing_write_effects() {
    io().block_on(async {
        let addr = address();
        let td = response_description(addr, true);
        let mut server = Fixture::new(td.clone());
        let accounts = bank();
        let instance = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &instance,
        );
        for mode in [6, 7, 8] {
            server.app.output.set(mode);
            let mut call = start_write(&client, b"19");
            let (local, remote) = server.exchange(&mut call).await;
            assert!(local.result().is_err());
            assert!(matches!(
                remote.result,
                Err(CoreError::Validation(_)) | Err(CoreError::UnsupportedOperation(_))
            ));
            assert_eq!(local.effect(), ConsumerEffect::Unknown);
            assert_eq!(remote.effect, FiniteEffect::Unknown);
            assert_eq!(*server.app.value.borrow(), b"19");
        }
        assert_eq!(server.app.writes.get(), 3);
        drop(client);
        clean(&accounts);
        server.close();
    });
}
#[test]
fn additional_response_without_schema_inherits_operation_output_and_own_form_media_default() {
    io().block_on(async {
        let addr = address();
        let mut td = response_description(addr, false);
        td["properties"]["level"]["forms"][0]["response"] =
            json!({"contentType":"application/json; charset=utf-8"});
        td["properties"]["level"]["forms"][0]["additionalResponses"]
            .as_array_mut()
            .unwrap()
            .push(json!({"htv:statusCodeValue":409}));
        let mut server = Fixture::new(td.clone());
        server.app.output.set(9);
        let accounts = bank();
        let instance = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &instance,
        );
        let mut call = client
            .read_property("level", Deadline::NONE, now(0), &mut work())
            .unwrap();
        let (local, remote) = server.exchange(&mut call).await;
        assert_eq!(
            local.result().unwrap().status(),
            InteractionStatus::ApplicationError
        );
        assert_eq!(local.result().unwrap().data().unwrap().body.as_ref(), b"7");
        assert_eq!(remote.delivery, FiniteDelivery::Delivered);
        drop(local);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}
#[test]
fn unused_request_media_and_primary_response_media_are_independent() {
    io().block_on(async {
        let addr = address();
        let mut td = description(addr);
        td["properties"]["level"]["forms"][0]["additionalResponses"] = json!([]);
        td["properties"]["level"]["readOnly"] = json!(true);
        td["properties"]["level"]["forms"][0]["contentType"] = json!("text/plain");
        td["properties"]["level"]["forms"][0]["response"] =
            json!({"contentType":"application/json"});
        let mut server = Fixture::new(td.clone());
        let accounts = bank();
        let instance = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &instance,
        );
        let mut call = client
            .read_property("level", Deadline::NONE, now(0), &mut work())
            .unwrap();
        let (local, remote) = server.exchange(&mut call).await;
        assert_eq!(
            local.result().unwrap().data().unwrap().content_type,
            "application/json"
        );
        assert_eq!(remote.delivery, FiniteDelivery::Delivered);
        drop(local);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn response_qualification_covers_every_branch_before_consumer_selection_or_producer_setup() {
    for case in 0..8 {
        let addr = address();
        let mut td = response_description(addr, false);
        match case {
            0 => {
                td["properties"]["level"]["forms"][0]["additionalResponses"][1]["contentType"] =
                    json!("application/cbor")
            }
            1 => {
                td["schemaDefinitions"]["failure"]["properties"]["reason"]["pattern"] =
                    json!("busy")
            }
            2 => {
                td["properties"]["level"]["forms"][0]["additionalResponses"][1]["htv:statusCodeValue"] =
                    json!(200)
            }
            3 => {
                td["properties"]["level"]["forms"][0]["additionalResponses"][1]
                    .as_object_mut()
                    .unwrap()
                    .remove("htv:statusCodeValue");
            }
            4 => {
                td["properties"]["level"]["forms"][0]["additionalResponses"][1]["htv:statusCodeValue"] =
                    json!(204)
            }
            5 => {
                td["properties"]["level"]["forms"][0]["additionalResponses"][1]["htv:headers"] =
                    json!([])
            }
            6 => {
                td["properties"]["level"]["forms"][0]["additionalResponses"][1]["htv:statusCodeValue"] =
                    json!("422")
            }
            7 => {
                td["properties"]["level"]["forms"][0]["additionalResponses"][0]["htv:statusCodeValue"] =
                    json!(202)
            }
            _ => unreachable!(),
        }
        let accounts = bank();
        let registry = ConsumerRegistry::try_new(
            vec![
                try_consumer_registration(
                    HttpConsumerConfig::for_cleartext_ip().unwrap(),
                    BindingId::new(98),
                    BindingGeneration::INITIAL,
                    0,
                )
                .unwrap(),
            ],
            &accounts,
        )
        .unwrap_or_else(|_| panic!());
        let thing: Thing = serde_json::from_value(td.clone()).unwrap();
        let result = registry.prepare(
            &thing,
            GatewayDefaultV1::LIMITS,
            accounts.clone(),
            ConsumerSelector::default(),
            &mut work(),
            |job| {
                while !job.is_finished() {
                    job.step(&mut work(), false);
                }
            },
        );
        match result {
            Ok(prepared) => assert!(
                prepared.support().iter().all(|s| !s.selected),
                "case {case}"
            ),
            Err(error) => assert!(matches!(
                error,
                CoreError::UnsupportedOperation(_) | CoreError::Validation(_)
            )),
        }
        drop(registry);
        clean(&accounts);
        let provider = HttpFiniteProducer::try_new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            BindingId::new(97),
            BindingGeneration::INITIAL,
            1000,
        )
        .unwrap();
        assert!(
            prepare_thing(
                &td.to_string(),
                authority(Generation::INITIAL).key(),
                GatewayDefaultV1::LIMITS,
                &mut work(),
                &[&provider],
                |_, _| true
            )
            .is_err(),
            "case {case}"
        );
        drop(TcpListener::bind(addr).unwrap());
    }
}

#[test]
fn consumer_checks_wire_selected_response_schema_and_rejects_undeclared_native_outcomes() {
    io().block_on(async {
        for (status, body, validation) in [
            (201, br#""overlong""#.as_slice(), true),
            (422, br#"{"reason":1}"#.as_slice(), true),
            (503, b"7".as_slice(), false),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let peer = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut head = Vec::new();
                while !head.ends_with(b"\r\n\r\n") { let mut b=[0]; stream.read_exact(&mut b).unwrap(); head.push(b[0]); }
                write!(stream, "HTTP/1.1 {status} response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                stream.write_all(body).unwrap();
            });
            let accounts = bank(); let instance = consumer_instance();
            let client = client(response_description(addr, false), Arc::new(AtomicBool::new(false)), accounts.clone(), &instance);
            let mut call = client.read_property("level", Deadline::NONE, now(0), &mut work()).unwrap();
            let start = Instant::now();
            let result = loop {
                if let Poll::Ready(s) = call.poll(&mut cx(), now(0), &mut work()) { break s; }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            assert_eq!(matches!(result.result(), Err(CoreError::Validation(_))), validation);
            assert!(result.result().is_err());
            if validation {
                let rejected = result.rejected_output().unwrap();
                assert_eq!(rejected.data().unwrap().body, body);
                assert_eq!(rejected.metadata().response_selection(), if status == 201 {ResponseSelection::Additional(0)} else {ResponseSelection::Additional(1)});
            }
            drop(result); drop(call); drop(client); clean(&accounts); peer.join().unwrap();
        }
    });
}

#[test]
fn http_native_status_class_does_not_override_explicit_td_response_success() {
    io().block_on(async {
        let addr = address();
        let mut td = response_description(addr, false);
        td["properties"]["level"]["forms"][0]["additionalResponses"][0]["htv:statusCodeValue"] =
            json!(503);
        td["properties"]["level"]["forms"][0]["additionalResponses"][1]["htv:statusCodeValue"] =
            json!(201);
        let mut server = Fixture::new(td.clone());
        let accounts = bank();
        let instance = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &instance,
        );
        for mode in [4, 5] {
            server.app.output.set(mode);
            let mut call = client
                .read_property("level", Deadline::NONE, now(0), &mut work())
                .unwrap();
            let (local, remote) = server.exchange(&mut call).await;
            assert_eq!(
                local.result().unwrap().status(),
                if mode == 4 {
                    InteractionStatus::Ok
                } else {
                    InteractionStatus::ApplicationError
                }
            );
            assert_eq!(
                local
                    .result()
                    .unwrap()
                    .metadata()
                    .binding_response()
                    .unwrap()
                    .status_code(),
                Some(if mode == 4 { 503 } else { 201 })
            );
            assert_eq!(remote.delivery, FiniteDelivery::Delivered);
        }
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn cancelled_write_keeps_original_declared_failure_and_unknown_effect_through_native_retirement() {
    io().block_on(async {
        let addr = address();
        let td = response_description(addr, true);
        let mut server = Fixture::new(td.clone());
        server.app.output.set(5);
        server.controls.hold_send.set(true);
        let accounts = bank();
        let instance = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &instance,
        );
        let mut call = start_write(&client, b"29");
        let start = Instant::now();
        while server.app.writes.get() == 0 {
            server.step();
            assert!(call.poll(&mut cx(), now(0), &mut work()).is_pending());
            assert!(start.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        call.cancel();
        let local = loop {
            if let Poll::Ready(s) = call.poll(&mut cx(), now(0), &mut work()) {
                break s;
            }
            assert!(start.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        assert!(matches!(local.result(), Err(CoreError::Cancelled(_))));
        assert_eq!(local.consumer(), &instance.reference());
        assert_eq!(local.effect(), ConsumerEffect::Unknown);
        server.controls.hold_close.set(true);
        server.exposure.begin_close();
        server.step();
        assert_eq!(server.server().state(), RawFiniteState::Closing);
        assert!(TcpListener::bind(addr).is_err());
        server.controls.hold_close.set(false);
        server.step();
        let remote = server.server().take_settlement().unwrap();
        assert_eq!(
            remote.result.unwrap().status(),
            InteractionStatus::ApplicationError
        );
        assert_eq!(remote.effect, FiniteEffect::Unknown);
        assert_eq!(remote.delivery, FiniteDelivery::NotDelivered);
        assert_eq!(*server.app.value.borrow(), b"29");
        assert!(accounts.account(ResourceKind::CleanupBytesMax).used() > 0);
        drop(local);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn write_request_and_declared_error_response_use_independently_qualified_media_parameters() {
    io().block_on(async {
        let addr = address();
        let mut td = response_description(addr, true);
        td["properties"]["level"]["forms"][0]["additionalResponses"][1]["contentType"] =
            json!("application/json; charset=utf-8");
        let failure = &mut td["properties"]["level"]["forms"][0]["additionalResponses"][1];
        failure
            .as_object_mut()
            .unwrap()
            .remove("htv:statusCodeValue");
        failure["@context"] = json!({"wireStatus":"htv:statusCodeValue"});
        failure["wireStatus"] = json!(422);
        failure["@type"] = json!("https://www.w3.org/2019/wot/td#AdditionalExpectedResponse");
        let mut server = Fixture::new(td.clone());
        server.app.output.set(10);
        let accounts = bank();
        let instance = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &instance,
        );
        let mut call = start_write(&client, b"37");
        let (local, remote) = server.exchange(&mut call).await;
        let output = local.result().unwrap();
        assert_eq!(output.status(), InteractionStatus::ApplicationError);
        assert_eq!(
            output.data().unwrap().content_type,
            "application/json; charset=utf-8"
        );
        assert_eq!(output.data().unwrap().body, br#"{"reason":"busy"}"#);
        assert_eq!(remote.delivery, FiniteDelivery::Delivered);
        assert_eq!(*server.app.value.borrow(), b"37");
        assert_eq!(server.app.writes.get(), 1);
        drop(local);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn response_content_reuse_and_final_producer_checks_compare_their_exact_semantic_boundaries() {
    let addr = address();
    let td = response_description(addr, false);
    let accounts = bank();
    let registry = ConsumerRegistry::try_new(
        vec![
            try_consumer_registration(
                HttpConsumerConfig::for_cleartext_ip().unwrap(),
                BindingId::new(98),
                BindingGeneration::INITIAL,
                0,
            )
            .unwrap(),
        ],
        &accounts,
    )
    .unwrap_or_else(|_| panic!());
    let thing: Thing = serde_json::from_value(td.clone()).unwrap();
    let prepared = registry
        .prepare(
            &thing,
            GatewayDefaultV1::LIMITS,
            accounts.clone(),
            ConsumerSelector::default(),
            &mut work(),
            |job| {
                while !job.is_finished() {
                    job.step(&mut work(), false);
                }
            },
        )
        .unwrap();
    let (bindings, funding) = prepared.into_execution();
    let content = bindings[0].definition().content().clone();
    assert!(core::ptr::eq(
        content.value_execution().unwrap().responses().unwrap(),
        content.responses().unwrap()
    ));
    let provider = HttpFiniteProducer::try_new(
        HttpProducerConfig::for_loopback(addr).unwrap(),
        BindingId::new(97),
        BindingGeneration::INITIAL,
        1000,
    )
    .unwrap();
    let key = authority(Generation::INITIAL).key();
    let original = prepare_thing(
        &td.to_string(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[&provider],
        |_, _| true,
    )
    .unwrap();
    check_final_thing(
        &td.to_string(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[&provider],
        &original,
        |_, _| true,
    )
    .unwrap();
    for case in 0..7 {
        let mut changed = td.clone();
        match case {
            0 => {
                changed["properties"]["level"]["forms"][0]["additionalResponses"][1]["success"] =
                    json!(true)
            }
            1 => {
                changed["properties"]["level"]["forms"][0]["additionalResponses"][1]["contentType"] =
                    json!("application/json; charset=utf-8")
            }
            2 => {
                changed["schemaDefinitions"]["failure"]["properties"]["reason"]["minLength"] =
                    json!(4)
            }
            3 => changed["properties"]["level"]["forms"][0]["additionalResponses"]
                .as_array_mut()
                .unwrap()
                .swap(0, 1),
            4 => changed["properties"]["level"]["forms"][0]["additionalResponses"]
                .as_array_mut()
                .unwrap()
                .push(json!({"htv:statusCodeValue":409})),
            5 => {
                changed["properties"]["level"]["forms"][0]["additionalResponses"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            // A native discriminator changes the specialized mapping, while
            // the common response content remains eligible for exact reuse.
            6 => {
                changed["properties"]["level"]["forms"][0]["additionalResponses"][1]["htv:statusCodeValue"] =
                    json!(423)
            }
            _ => unreachable!(),
        }
        assert!(
            check_final_thing(
                &changed.to_string(),
                key,
                GatewayDefaultV1::LIMITS,
                &mut work(),
                &[&provider],
                &original,
                |_, _| true
            )
            .is_err(),
            "case {case}"
        );
        let thing: Thing = serde_json::from_value(changed).unwrap();
        let reused = registry.prepare_with_content(
            &thing,
            GatewayDefaultV1::LIMITS,
            accounts.clone(),
            ConsumerSelector::default(),
            &content,
            &mut work(),
            |job| {
                while !job.is_finished() {
                    job.step(&mut work(), false);
                }
            },
        );
        if case == 6 {
            let reused = reused.unwrap();
            assert_eq!(reused.content_constructions(), 0);
            let (mappings, retained) = reused.into_execution();
            assert!(Arc::ptr_eq(mappings[0].definition().content(), &content));
            drop(mappings);
            drop(retained);
        } else {
            assert!(reused.is_err(), "case {case}");
        }
    }
    drop(original);
    drop(content);
    drop(bindings);
    drop(funding);
    drop(registry);
    clean(&accounts);
    drop(TcpListener::bind(addr).unwrap());
}

fn two_form_response_description(addr: SocketAddr) -> Value {
    let mut td = response_description(addr, false);
    let mut second = td["properties"]["level"]["forms"][0].clone();
    second["href"] = json!(format!("http://{addr}/other"));
    second["additionalResponses"][1]["schema"] = json!("failure2");
    second["additionalResponses"][1]["htv:statusCodeValue"] = json!(423);
    td["schemaDefinitions"]["failure2"] =
        json!({"type":"object","required":["code"],"properties":{"code":{"type":"integer"}}});
    td["properties"]["level"]["forms"]
        .as_array_mut()
        .unwrap()
        .push(second);
    td
}
#[test]
fn producer_validates_the_original_native_form_instead_of_a_sibling_response_schema() {
    io().block_on(async {
        let addr = address(); let td = two_form_response_description(addr);
        let mut server = Fixture::new(td.clone()); server.app.output.set(5);
        let accounts = bank(); let instance = consumer_instance();
        let client = client_with_selector(td, Arc::new(AtomicBool::new(false)), accounts.clone(), &instance,
            ConsumerSelector {form:Some(1), ..Default::default()});
        let mut call = client.read_property("level", Deadline::NONE, now(0), &mut work()).unwrap();
        let (local, remote) = server.exchange(&mut call).await;
        assert!(local.result().is_err());
        assert!(matches!(remote.result, Err(CoreError::Validation(_))),
            "the first Form's valid reason payload must not certify the selected Form's required code");
        drop(local); drop(call); drop(client); clean(&accounts); server.close();
    });
}

#[test]
fn one_behavior_borrows_each_original_forms_response_contract_for_scoped_branch_choice() {
    io().block_on(async {
        let addr = address();
        let mut td = two_form_response_description(addr);
        td["properties"]["level"]["forms"][1]["additionalResponses"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        let mut server = Fixture::new(td.clone());
        server.app.output.set(11);
        for form in [0, 1] {
            let accounts = bank();
            let instance = consumer_instance();
            let client = client_with_selector(
                td.clone(),
                Arc::new(AtomicBool::new(false)),
                accounts.clone(),
                &instance,
                ConsumerSelector {
                    form: Some(form),
                    ..Default::default()
                },
            );
            let mut call = client
                .read_property("level", Deadline::NONE, now(0), &mut work())
                .unwrap();
            let (local, remote) = server.exchange(&mut call).await;
            let output = local.result().unwrap();
            assert_eq!(output.status(), InteractionStatus::ApplicationError);
            assert_eq!(
                output.metadata().response_selection(),
                ResponseSelection::Additional(if form == 0 { 1 } else { 0 })
            );
            assert_eq!(
                output.metadata().binding_response().unwrap().status_code(),
                Some(if form == 0 { 422 } else { 423 })
            );
            assert_eq!(
                output.data().unwrap().body,
                if form == 0 {
                    br#"{"reason":"busy"}"#.as_slice()
                } else {
                    br#"{"code":7}"#.as_slice()
                }
            );
            assert_eq!(remote.delivery, FiniteDelivery::Delivered);
            assert_eq!(
                remote.result.unwrap().status(),
                InteractionStatus::ApplicationError
            );
            drop(local);
            drop(call);
            drop(client);
            clean(&accounts);
        }
        assert_eq!(server.app.reads.get(), 2);
        server.close();
    });
}
