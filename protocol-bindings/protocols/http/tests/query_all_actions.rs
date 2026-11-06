#![cfg(feature = "prepare")]
//! One native aggregate query over qualified original Action invocations.
include!("support/action_collection.rs");

#[test]
fn consumer_result_views_keep_original_allocation_funded_through_final_owner_disposal() {
    io().block_on(async {
        let mut server = Server::new();
        server.invoke("run", 1);
        let bank = accounts();
        let instance =
            ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(2), Generation::INITIAL));
        let policy = Arc::new(ConsumerPolicy::default());
        let client = consumer(&server.td, bank.clone(), &instance, policy.clone());
        let mut call = query(&client);
        let (settlement, _) = server.exchange(&mut call).await;
        let original = policy.last_body.lock().unwrap().take().unwrap();
        let output = settlement.result().unwrap();
        let body = output.data().unwrap().body;
        assert_eq!(body.as_ptr(), original.as_ptr() as *const u8);
        let ConsumerNativeOutcome::Returned(Ok(native)) = settlement.native_outcome() else {
            panic!("native success must retain its original evidence");
        };
        assert_eq!(native.data().unwrap().body.as_ptr(), body.as_ptr());
        assert_eq!(native.metadata(), output.metadata());
        // An explicit copy owns a different allocation and may outlive the
        // Consumer. Sharing the original allocation shares the settlement.
        let copied = Arc::<[u8]>::from(body);
        assert_ne!(copied.as_ptr(), body.as_ptr());
        let settlement = Rc::new(settlement);
        let held = settlement.clone();
        drop(call);
        drop(client);
        server.close();
        let retained = ResourceKind::ALL.map(|k| bank.account(k).used());
        for k in [
            ResourceKind::EngineLiveBytesGlobalMax,
            ResourceKind::CleanupBytesMax,
            ResourceKind::CleanupItemsMax,
            ResourceKind::PendingHandlerCallsGlobalMax,
            ResourceKind::BindingResponseBufferBytesGlobalMax,
        ] {
            assert!(bank.account(k).used() > 0, "original {k:?} funding");
        }
        assert!(original.upgrade().is_some());
        drop(settlement);
        assert_eq!(retained, ResourceKind::ALL.map(|k| bank.account(k).used()));
        assert_eq!(held.result().unwrap().data().unwrap().body, &*copied);
        drop(held);
        clean(&bank);
        assert!(original.upgrade().is_none());
        assert!(!copied.is_empty());
    });
}

#[test]
fn consumer_late_native_views_retain_original_funding_after_cancellation_or_revocation() {
    io().block_on(async {
        for revoke in [false, true] {
            let mut server = Server::new();
            server.invoke("run", 1);
            let bank = accounts();
            let instance =
                ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(2), Generation::INITIAL));
            let policy = Arc::new(ConsumerPolicy::default());
            let client = consumer(&server.td, bank.clone(), &instance, policy.clone());
            let mut call = query(&client);
            let access_work = HttpConsumerConfig::for_cleartext_ip()
                .unwrap()
                .consumer_services()
                .access
                .work()
                + 1;
            let end = Instant::now() + Duration::from_secs(5);
            // Pay the progress check and native/validation work, but withhold
            // the final authorization quantum. The observer proves that the
            // original HTTP body was accepted before cancellation/revocation.
            while policy.last_body.lock().unwrap().is_none() {
                assert!(
                    call.poll(
                        &mut cx(),
                        now(),
                        &mut work().with_remaining(WorkClass::ProviderProbes, access_work)
                    )
                    .is_pending()
                );
                let native = server
                    .exposure
                    .participant_mut::<HttpActionRuntime>(0)
                    .unwrap();
                let _ = native.step(&mut server.owner, &mut work()).unwrap();
                if let Some(delivery) = native.take_delivery() {
                    assert!(delivery.locally_written);
                }
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            let original = policy.last_body.lock().unwrap().take().unwrap();
            if revoke {
                instance.revoke();
            } else {
                call.cancel();
            }
            let Poll::Ready(result) = call.poll(&mut cx(), now(), &mut work()) else {
                panic!("the validated native result requires only the final current check");
            };
            if revoke {
                assert!(matches!(result.result(), Err(CoreError::StaleHandle(_))));
            } else {
                assert!(matches!(result.result(), Err(CoreError::Cancelled(_))));
            }
            let ConsumerNativeOutcome::Returned(Ok(output)) = result.native_outcome() else {
                panic!("fencing application success must preserve original native success");
            };
            assert_eq!(
                output.data().unwrap().body.as_ptr(),
                original.as_ptr() as *const u8
            );
            assert_eq!(
                output.metadata().binding_response().unwrap().status_code(),
                200
            );
            let copied = output.data().unwrap().body.to_vec();
            let held = Rc::new(result);
            let shared = held.clone();
            drop(call);
            drop(client);
            server.close();
            assert!(original.upgrade().is_some());
            assert!(
                bank.account(ResourceKind::BindingResponseBufferBytesGlobalMax)
                    .used()
                    > 0
            );
            assert!(bank.account(ResourceKind::CleanupBytesMax).used() > 0);
            drop(held);
            let ConsumerNativeOutcome::Returned(Ok(output)) = shared.native_outcome() else {
                panic!()
            };
            assert_eq!(output.data().unwrap().body, copied.as_slice());
            assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
            drop(shared);
            clean(&bank);
            assert!(original.upgrade().is_none());
        }
    });
}

#[test]
fn engine_query_keeps_concurrent_instances_independent_without_leaf_queries() {
    io().block_on(async {
        let mut server = Server::new();
        let first = server.invoke("run", 11);
        let second = server.invoke("run", 22);
        let signal = server.invoke("signal", 0);
        let bank = accounts();
        let instance =
            ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(2), Generation::INITIAL));
        let client = consumer(
            &server.td,
            bank.clone(),
            &instance,
            Arc::new(ConsumerPolicy::default()),
        );
        let mut call = query(&client);
        let (result, delivery) = server.exchange(&mut call).await;
        assert!(delivery.locally_written);
        assert!(delivery.invocation.is_none());
        assert_eq!(result.operation(), Operation::QueryAllActions);
        assert_eq!(
            result.affordance(),
            clinkz_wot_core::thing::AffordanceSite::Thing
        );
        assert_eq!(result.consumer(), &instance.reference());
        assert!(
            result
                .result()
                .unwrap()
                .metadata()
                .action_invocation()
                .is_none()
        );
        let v = values(&result);
        assert_eq!(v["run"].as_array().unwrap().len(), 2);
        assert!(v["run"][0]["href"].as_str().unwrap().ends_with(&second));
        assert!(v["run"][1]["href"].as_str().unwrap().ends_with(&first));
        assert!(v["signal"][0]["href"].as_str().unwrap().ends_with(&signal));
        assert_eq!(v["run"][0]["status"], "running");
        assert_eq!(server.app.roots.get(), 1);
        assert_eq!(server.app.members.get(), 3);
        assert_eq!(server.app.leaf_queries.get(), 0);
        assert_eq!(server.app.invoked.get(), 3);
        assert!(
            client
                .query_all_actions(Deadline::NONE, now(), &mut work())
                .is_err()
        );
        drop(result);
        drop(call);
        server.app.complete.set(true);
        server.progress();
        let mut call = query(&client);
        let (result, _) = server.exchange(&mut call).await;
        let v = values(&result);
        assert_eq!(v["run"][0]["status"], "completed");
        assert_eq!(v["run"][0]["output"], 22);
        assert_eq!(v["run"][1]["output"], 11);
        assert_eq!(v["signal"][0]["output"], "ready");
        assert!(v["run"][0]["href"].as_str().unwrap().ends_with(&second));
        assert_eq!(server.app.invoked.get(), 3);
        drop(result);
        drop(call);
        drop(client);
        clean(&bank);
        server.close();
    });
}

#[test]
fn query_reports_failed_invocations_and_cancel_request_does_not_claim_termination() {
    let mut server = Server::new();
    let token = server.invoke("run", 7);
    let (s, v, _) = server.send("DELETE", &format!("/run/stop?id={token}"), "");
    assert_eq!(s, 200);
    assert_eq!(v["status"], "cancelling");
    server.progress();
    assert_eq!(server.app.cancelled.get(), 1);
    let (s, v, _) = server.send("GET", "/actions", "");
    assert_eq!(s, 200);
    assert_eq!(v["run"][0]["status"], "running");
    server.app.invalid.set(true);
    server.app.complete.set(true);
    server.progress();
    let (s, v, _) = server.send("GET", "/actions", "");
    assert_eq!(s, 200);
    assert_eq!(v["run"][0]["status"], "failed");
    assert!(v["run"][0].get("output").is_none());
    assert!(v["run"][0]["href"].as_str().unwrap().ends_with(&token));
    server.close();
}

#[test]
fn fresh_root_and_member_authorization_filter_records_without_reinvocation() {
    let mut server = Server::new();
    server.invoke("run", 1);
    server.invoke("signal", 2);
    server.app.deny_root.set(true);
    let (s, _, d) = server.send("GET", "/actions", "");
    assert_eq!(s, 403);
    assert!(matches!(d.request_error, Some(CoreError::Security { .. })));
    assert_eq!(server.app.members.get(), 0);
    server.app.deny_root.set(false);
    server.app.deny_member.set(true);
    let (s, v, _) = server.send("GET", "/actions", "");
    assert_eq!(s, 200);
    assert_eq!(v["signal"], json!([]));
    assert_eq!(v["run"].as_array().unwrap().len(), 1);
    server.app.deny_member.set(false);
    let (s, v, _) = server.send("GET", "/actions", "");
    assert_eq!(s, 200);
    assert_eq!(v["signal"].as_array().unwrap().len(), 1);
    assert_eq!(server.app.invoked.get(), 2);
    assert_eq!(server.app.leaf_queries.get(), 0);
    server.close();
}

#[test]
fn retained_terminal_records_use_capacity_until_acknowledged_without_truncation() {
    let mut server = Server::new();
    let first = server.invoke("run", 0);
    for n in 1..8 {
        server.invoke("run", n);
    }
    let (s, _, _) = server.send("POST", "/run", "9");
    assert_eq!(s, 503);
    server.app.complete.set(true);
    server.progress();
    let (s, v, _) = server.send("GET", "/actions", "");
    assert_eq!(s, 200);
    assert_eq!(v["run"].as_array().unwrap().len(), 8);
    let id = first.split_once('-').unwrap().1.parse().unwrap();
    let native = server
        .exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap();
    drop(native.acknowledge(&mut server.owner, id).unwrap().unwrap());
    let (s, v, _) = server.send("GET", "/actions", "");
    assert_eq!(s, 200);
    assert_eq!(v["run"].as_array().unwrap().len(), 7);
    assert!(
        v["run"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| !v["href"].as_str().unwrap().ends_with(&first))
    );
    server.invoke("run", 9);
    server.close();
}

#[test]
fn complete_producer_qualification_requires_root_policy_all_forms_and_original_schema() {
    let addr = address();
    let td = description(addr);
    let app = Rc::new(App::default());
    let missing = logical(authority(Generation::INITIAL), &td, app.clone(), false);
    assert!(prepare(&td, &missing, addr).is_err());
    for bad in [
        "method",
        "schema",
        "extra",
        "root-semantic",
        "duplicate-output",
        "security",
        "second-listener",
    ] {
        let mut altered = td.clone();
        match bad {
            "method" => altered["forms"][0]["htv:methodName"] = json!("POST"),
            "schema" => altered["actions"]["signal"]["output"]["minLength"] = json!(1),
            "extra" => altered["forms"]
                .as_array_mut()
                .unwrap()
                .push(json!({"op":"readallproperties","href":format!("http://{addr}/properties")})),
            "root-semantic" => {
                altered["@context"] =
                    json!(["https://www.w3.org/2022/wot/td/v1.1",{"x":"urn:test:"}]);
                altered["forms"][0]["x:required"] = json!(true);
            }
            "duplicate-output" => {
                altered["actions"]["signal"]["https://www.w3.org/2019/wot/td#output"] =
                    json!({"type":"boolean"})
            }
            "security" => altered["forms"][0]["security"] = json!(["missing"]),
            _ => {
                altered["actions"]["signal"]["forms"][0]["href"] =
                    json!(format!("http://{}/signal", address()))
            }
        }
        let instance = logical(authority(Generation::INITIAL), &altered, app.clone(), true);
        assert!(prepare(&altered, &instance, addr).is_err(), "{bad}");
    }
    let owner = owner();
    let instance = logical(authority(Generation::INITIAL), &td, app, true);
    let (prepared, _) = prepare(&td, &instance, addr).unwrap();
    let all: Vec<_> = prepared
        .operations()
        .iter()
        .map(|o| o.definition::<HttpActionDefinition>().unwrap().clone())
        .collect();
    // A root definition cannot qualify a participant containing only one
    // Action's records even when another participant could serve that Action.
    let missing: Vec<_> = prepared
        .operations()
        .iter()
        .filter(|o| !matches!(&o.site,FormSite::Action(n) if n=="signal"))
        .map(|o| o.definition::<HttpActionDefinition>().unwrap().clone())
        .collect();
    assert!(HttpActionRuntime::try_new(&owner, instance.key(), missing, 8).is_err());
    assert!(HttpActionRuntime::try_new(&owner, instance.key(), all, 8).is_ok());
    TcpListener::bind(addr).unwrap();
}

#[test]
fn consumer_rejects_malformed_or_misattributed_status_evidence_and_keeps_native_bytes() {
    io().block_on(async {
        let base="http://127.0.0.1/status?id=a";
        let valid=json!({"run":[{"status":"completed","href":base,"output":7}],"signal":[]}).to_string();
        let cases=vec![
            (200,valid,true),(200,"{}".into(),true),
            (200,json!({"run":[{"status":"pending","href":base}]}).to_string(),true),
            (200,json!({"signal":[{"status":"completed","href":base,"output":"ok"}]}).to_string(),true),
            (200,json!({"run":[{"status":"failed","href":base}]}).to_string(),true),
            (200,json!({"other":[]}).to_string(),false),
            (200,json!({"run":{"status":"running","href":base}}).to_string(),false),
            (200,json!({"run":[{"status":"running"}]}).to_string(),false),
            (200,json!({"run":[{"status":"cancelled","href":base}]}).to_string(),false),
            (200,json!({"run":[{"status":"completed","href":base}]}).to_string(),false),
            (200,json!({"run":[{"status":"completed","href":base,"output":true}]}).to_string(),false),
            (200,json!({"signal":[{"status":"completed","href":base,"output":7}]}).to_string(),false),
            (200,json!({"run":[{"status":"running","href":base,"output":7}]}).to_string(),false),
            (200,json!({"run":[{"status":"failed","href":base,"output":7}]}).to_string(),false),
            (200,json!({"run":[{"status":"running","href":base,"error":{"title":"hidden"}}]}).to_string(),false),
            (200,json!({"run":[{"status":"running","href":"http://user:secret@example.com/status"}]}).to_string(),false),
            (200,json!({"run":[{"status":"running","href":"/unbound"}]}).to_string(),false),
            (200,json!({"run":[{"status":"running","href":base},{"status":"running","href":base}]}).to_string(),false),
            (200,json!({"run":[{"status":"running","href":base}],"signal":[{"status":"running","href":base}]}).to_string(),false),
            (200,"{\"run\":[],\"\\u0072un\":[]}".to_string(),false),
            (200,format!("{{\"run\":[{{\"status\":\"running\",\"status\":\"failed\",\"href\":\"{base}\"}}]}}"),false),
            (206,json!({"run":[]}).to_string(),false),(500,json!({"run":[]}).to_string(),false),
            (200,"{\"run\":[]}{}".into(),false),
        ];
        for(status,body,valid)in cases {
            let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();let response=body.clone();
            let peer=std::thread::spawn(move||{let(mut socket,_)=listener.accept().unwrap();socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut bytes=Vec::new();let mut buf=[0;1024];while !bytes.ends_with(b"\r\n\r\n"){let n=socket.read(&mut buf).unwrap();assert!(n>0);bytes.extend_from_slice(&buf[..n]);}
                assert!(bytes.starts_with(b"GET /actions HTTP/1.1\r\n"));
                socket.write_all(format!("HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",response.len()).as_bytes()).unwrap();});
            let bank=accounts();let instance=ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(2),Generation::INITIAL));
            let policy=Arc::new(ConsumerPolicy::default());
            let client=consumer(&description(addr),bank.clone(),&instance,policy.clone());let mut call=query(&client);let end=Instant::now()+Duration::from_secs(5);
            let result=loop{if let Poll::Ready(r)=call.poll(&mut cx(),now(),&mut work()){break r;}assert!(Instant::now()<end);tokio::time::sleep(Duration::from_millis(1)).await;};
            assert_eq!(result.result().is_ok(),valid,"{body}");
            if !valid && status == 200 {
                let original = policy.last_body.lock().unwrap().take().unwrap();
                let output = result.rejected_output().unwrap();
                assert_eq!(output.data().unwrap().body, body.as_bytes());
                assert_eq!(output.data().unwrap().body.as_ptr(), original.as_ptr() as *const u8);
                assert_eq!(output.metadata().binding_response().unwrap().status_code(), 200);
                let result = Rc::new(result);
                let held = result.clone();
                drop(call);
                drop(client);
                peer.join().unwrap();
                assert!(original.upgrade().is_some());
                assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
                assert!(bank.account(ResourceKind::BindingResponseBufferBytesGlobalMax).used() > 0);
                drop(result);
                assert_eq!(held.rejected_output().unwrap().data().unwrap().body, body.as_bytes());
                assert!(bank.account(ResourceKind::CleanupBytesMax).used() > 0);
                drop(held);
                clean(&bank);
                assert!(original.upgrade().is_none());
            } else {
                drop(result);drop(call);drop(client);clean(&bank);peer.join().unwrap();
            }
        }
    });
}

#[test]
fn source_free_validation_service_and_fresh_authority_remain_required_through_handoff() {
    io().block_on(async {
        for mode in [
            "cancel",
            "replace",
            "policy",
            "service",
            "callback",
            "short-credit",
        ] {
            let mut server = Server::new();
            server.invoke("run", 1);
            let bank = accounts();
            let instance =
                ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(2), Generation::INITIAL));
            let policy = Arc::new(ConsumerPolicy::default());
            let client = consumer(&server.td, bank.clone(), &instance, policy.clone());
            let mut call = query(&client);
            // Admit and write the original native response while withholding
            // Consumer progress: the response cannot become cached permission.
            let end = Instant::now() + Duration::from_secs(5);
            let mut delivery = None;
            while delivery.is_none() {
                assert!(
                    call.poll(
                        &mut cx(),
                        now(),
                        &mut work().with_remaining(WorkClass::CodecInputBytes, 0)
                    )
                    .is_pending()
                );
                let native = server
                    .exposure
                    .participant_mut::<HttpActionRuntime>(0)
                    .unwrap();
                let _ = native.step(&mut server.owner, &mut work()).unwrap();
                delivery = native.take_delivery();
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            match mode {
                "cancel" => call.cancel(),
                "replace" => instance.revoke(),
                "policy" => policy.denied.store(true, Ordering::Relaxed),
                "service" => policy.unavailable.store(true, Ordering::Relaxed),
                "callback" => policy.lose_on_validate.store(true, Ordering::Relaxed),
                _ => {}
            }
            assert!(
                call.poll(&mut cx(), now(), &mut WorkBudget::new())
                    .is_pending()
            );
            let result = loop {
                if let Poll::Ready(r) = call.poll(&mut cx(), now(), &mut work()) {
                    break r;
                }
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            assert_eq!(result.result().is_ok(), mode == "short-credit", "{mode}");
            assert_eq!(result.consumer(), &instance.reference());
            assert_eq!(result.operation(), Operation::QueryAllActions);
            assert_eq!(server.app.invoked.get(), 1);
            assert_eq!(server.app.cancelled.get(), 0);
            assert!(
                client
                    .query_all_actions(Deadline::NONE, now(), &mut work())
                    .is_err()
            );
            drop(result);
            drop(call);
            drop(client);
            clean(&bank);
            server.close();
        }
    });
}

#[test]
fn pre_effect_consumer_cancellation_and_policy_refusal_create_no_root_or_member_queries() {
    io().block_on(async {
        let server = Server::new();
        let bank = accounts();
        let instance =
            ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(2), Generation::INITIAL));
        let policy = Arc::new(ConsumerPolicy::default());
        let client = consumer(&server.td, bank.clone(), &instance, policy.clone());
        policy.denied.store(true, Ordering::Relaxed);
        assert!(
            client
                .query_all_actions(Deadline::NONE, now(), &mut work())
                .is_err()
        );
        policy.denied.store(false, Ordering::Relaxed);
        let mut call = query(&client);
        call.cancel();
        let end = Instant::now() + Duration::from_secs(5);
        let result = loop {
            if let Poll::Ready(r) = call.poll(&mut cx(), now(), &mut work()) {
                break r;
            }
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        assert!(matches!(result.result(), Err(CoreError::Cancelled(_))));
        assert_eq!(server.app.roots.get(), 0);
        assert_eq!(server.app.members.get(), 0);
        drop(result);
        drop(call);
        drop(client);
        clean(&bank);
        server.close();
    });
}

#[test]
fn producer_replacement_cannot_retarget_old_status_links_or_original_pending_reply() {
    let mut old = Server::new();
    let token = old.invoke("run", 9);
    let old_key = old.exposure.key();
    let mut peer = TcpStream::connect(old.addr).unwrap();
    peer.set_nonblocking(true).unwrap();
    peer.write_all(
        format!(
            "GET /actions HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            old.addr
        )
        .as_bytes(),
    )
    .unwrap();
    // Receive+format the original aggregate, retaining its separate reply.
    assert!(
        old.exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .step(&mut old.owner, &mut work())
            .unwrap()
            .is_pending()
    );
    let next_addr = address();
    let next_td = description(next_addr);
    let app = Rc::new(App::default());
    let next_bank = accounts();
    let mut next = exposure(
        &old.owner,
        authority(Generation::INITIAL.checked_next().unwrap()),
        &next_td,
        next_addr,
        app.clone(),
        Some(old_key),
        next_bank.clone(),
    );
    next.participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    next.publish(&mut old.owner, &mut work()).unwrap();
    let native = old
        .exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let _ = native.step(&mut old.owner, &mut work()).unwrap();
        if let Some(d) = native.take_delivery() {
            assert!(d.locally_written);
            break;
        }
        assert!(Instant::now() < end);
    }
    let mut bytes = Vec::new();
    loop {
        let mut buf = [0; 1024];
        match peer.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => bytes.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("{e}"),
        };
        assert!(Instant::now() < end);
    }
    assert!(std::str::from_utf8(&bytes).unwrap().contains(&token));
    assert_eq!(app.invoked.get(), 0);
    assert_eq!(old.app.roots.get(), 1);
    let mut successor = Server {
        owner: old.owner,
        exposure: next,
        app,
        bank: next_bank,
        addr: next_addr,
        td: next_td,
    };
    let (s, _, _) = successor.send("GET", &format!("/run/status?id={token}"), "");
    assert_eq!(s, 404);
    let (s, v, _) = successor.send("GET", "/actions", "");
    assert_eq!(s, 200);
    assert_eq!(v, json!({"run":[],"signal":[]}));
    old.exposure.begin_close();
    assert!(
        old.exposure
            .retire(&mut successor.owner, &mut work())
            .is_err()
    );
    let native = old
        .exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap();
    let id = token.split_once('-').unwrap().1.parse().unwrap();
    let handle = native.handle(id).unwrap();
    successor.owner.close_raw_action(&handle).unwrap();
    assert!(
        successor
            .owner
            .step_raw_action(&handle, &mut work())
            .unwrap()
            .is_ready()
    );
    let settlement = native
        .acknowledge(&mut successor.owner, id)
        .unwrap()
        .unwrap();
    assert_eq!(settlement.instance, old_key);
    drop(settlement);
    old.exposure
        .retire(&mut successor.owner, &mut work())
        .unwrap();
    drop(old.exposure);
    clean(&old.bank);
    TcpListener::bind(old.addr).unwrap();
    successor.close();
}

#[test]
fn final_retained_td_mismatch_refuses_before_managed_handoff_without_native_effects() {
    let addr = address();
    let original = description(addr);
    let app = Rc::new(App::default());
    let owner = owner();
    let bank = accounts();
    let instance = logical(authority(Generation::INITIAL), &original, app.clone(), true);
    let (prepared, providers) = prepare(&original, &instance, addr).unwrap();
    let definitions = prepared
        .operations()
        .iter()
        .map(|o| o.definition::<HttpActionDefinition>().unwrap().clone())
        .collect();
    let participant = HttpActionRuntime::try_new(&owner, instance.key(), definitions, 8).unwrap();
    let mut altered = original;
    altered["actions"]["signal"]["output"]["type"] = json!("boolean");
    let instance = instance.with_description(Arc::from(altered.to_string()));
    let refused = ComposedProducerExposure::try_new(
        &owner,
        ComposedProducerInput {
            instance,
            prepared,
            providers,
            participants: vec![Box::new(participant)],
            expected: None,
            handler_backing_bytes: 256,
        },
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        bank.clone(),
        &mut work(),
    );
    assert!(refused.is_err());
    drop(refused);
    assert_eq!(app.roots.get(), 0);
    clean(&bank);
    TcpListener::bind(addr).unwrap();
}

#[test]
fn original_aggregate_reply_identity_survives_native_close_without_inventing_delivery() {
    let mut server = Server::new();
    server.invoke("run", 3);
    let mut peer = TcpStream::connect(server.addr).unwrap();
    peer.write_all(
        format!(
            "GET /actions HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            server.addr
        )
        .as_bytes(),
    )
    .unwrap();
    let native = server
        .exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap();
    assert!(
        native
            .step(&mut server.owner, &mut work())
            .unwrap()
            .is_pending()
    );
    native.begin_close();
    let report = native.take_delivery().unwrap();
    assert!(!report.locally_written);
    assert!(!report.write_started);
    let original = report.exchange.unwrap();
    assert_eq!(original.instance, server.exposure.key());
    assert_eq!(original.target, AffordanceTarget::Thing);
    assert_eq!(original.operation, Operation::QueryAllActions);
    assert!(matches!(report.cause, Some(CoreError::Cancelled(_))));
    assert_eq!(server.app.invoked.get(), 1);
    assert_eq!(server.app.cancelled.get(), 0);
    drop(peer);
    server.close();
}

#[test]
fn missing_status_service_or_unsupported_description_never_falls_back_to_leaf_queries() {
    let addr = address();
    let original = description(addr);
    for bad in [
        "absent",
        "method-default",
        "method",
        "schema",
        "duplicate-output",
        "security",
        "response",
        "variables",
    ] {
        let mut td = original.clone();
        match bad {
            "absent" => {
                td.as_object_mut().unwrap().remove("forms");
            }
            "method-default" => {
                td["forms"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("htv:methodName");
            }
            "method" => td["forms"][0]["htv:methodName"] = json!("POST"),
            "schema" => td["actions"]["signal"]["output"]["minLength"] = json!(1),
            "duplicate-output" => {
                td["actions"]["signal"]["https://www.w3.org/2019/wot/td#output"] =
                    json!({"type":"boolean"})
            }
            "security" => {
                td["securityDefinitions"]["b"] = json!({"scheme":"basic"});
                td["forms"][0]["security"] = json!(["b"]);
            }
            "response" => td["forms"][0]["response"]["htv:statusCodeValue"] = json!(207),
            _ => {
                td["uriVariables"] = json!({"q":{"type":"string"}});
            }
        }
        let bank = accounts();
        if let Ok(p) = prepare_consumer(&td, bank.clone(), Arc::new(ConsumerPolicy::default())) {
            assert!(
                !p.support()
                    .iter()
                    .any(|s| s.operation == Operation::QueryAllActions && s.selected),
                "{bad}"
            );
        }
        clean(&bank);
    }
    struct Scalar(Arc<dyn ConsumerDataService>);
    impl ConsumerDataService for Scalar {
        fn content_type(&self) -> &str {
            "application/json"
        }
        fn policy(&self) -> ConsumerDataPolicy {
            self.0.policy()
        }
        fn validate(&self, s: value_type::TypeSchema, p: &Payload) -> CoreResult<()> {
            self.0.validate(s, p)
        }
    }
    let mut r = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(90),
        BindingGeneration::INITIAL,
        0,
    )
    .unwrap();
    r.services.data = Arc::new(Scalar(r.services.data));
    let bank = accounts();
    let registry = ConsumerRegistry::try_new(vec![r], &bank).ok().unwrap();
    let thing: Thing = serde_json::from_value(original).unwrap();
    assert!(
        registry
            .prepare(
                &thing,
                GatewayDefaultV1::LIMITS,
                bank.clone(),
                ConsumerSelector::default(),
                &mut work(),
                |j| while !j.is_finished() {
                    j.step(&mut work(), false);
                }
            )
            .is_err()
    );
    drop(registry);
    clean(&bank);
    TcpListener::bind(addr).unwrap();
}

#[test]
fn aggregate_read_checks_original_principal_route_and_paid_work_before_any_member_view() {
    let mut server = Server::new();
    let token = server.invoke("run", 1);
    let id = token.split_once('-').unwrap().1.parse().unwrap();
    let handle = server.native().handle(id).unwrap();
    let route = ActionRoute {
        binding: (BindingId::new(61), BindingGeneration::INITIAL),
        mapping: ActionMappingKey(1),
    };
    let key = server.exposure.key();
    let observed = Cell::new(0);
    assert!(
        server
            .owner
            .query_raw_actions(
                key,
                route,
                [&handle].into_iter(),
                &InteractionInput::empty(),
                &mut WorkBudget::new(),
                |_, _, _| {
                    observed.set(observed.get() + 1);
                    Ok(())
                }
            )
            .is_err()
    );
    assert_eq!(server.app.roots.get(), 0);
    assert_eq!(observed.get(), 0);
    let forged = ActionRoute {
        mapping: ActionMappingKey(999),
        ..route
    };
    assert!(
        server
            .owner
            .query_raw_actions(
                key,
                forged,
                [&handle].into_iter(),
                &InteractionInput::empty(),
                &mut work(),
                |_, _, _| {
                    observed.set(observed.get() + 1);
                    Ok(())
                }
            )
            .is_err()
    );
    let mut input = InteractionInput::empty();
    input.principal = Some(Principal {
        id: PrincipalId::from("another"),
        scopes: Default::default(),
    });
    server
        .owner
        .query_raw_actions(
            key,
            route,
            [&handle].into_iter(),
            &input,
            &mut work(),
            |_, _, _| {
                observed.set(observed.get() + 1);
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(observed.get(), 0);
    assert_eq!(server.app.members.get(), 0);
    server
        .owner
        .query_raw_actions(
            key,
            route,
            [&handle].into_iter(),
            &InteractionInput::empty(),
            &mut work(),
            |name, original, _| {
                assert_eq!(name, "run");
                assert_eq!(original.reference(), handle.reference());
                observed.set(observed.get() + 1);
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(observed.get(), 1);
    server.close();
}

#[test]
fn oversized_native_aggregate_refuses_without_truncating_or_releasing_original_records() {
    let mut server = Server::new();
    server.app.long.set(true);
    server.invoke("signal", 1);
    server.invoke("signal", 2);
    server.app.complete.set(true);
    server.progress();
    let (s, _, report) = server.send("GET", "/actions", "");
    assert_eq!(s, 503);
    assert!(matches!(
        report.request_error,
        Some(CoreError::Backpressure(_))
    ));
    assert_eq!(server.native().records(), 2);
    let ids: Vec<_> = server.native().action_ids().collect();
    let native = server
        .exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap();
    let settlement = native
        .acknowledge(&mut server.owner, ids[0])
        .unwrap()
        .unwrap();
    drop(settlement);
    let (s, v, _) = server.send("GET", "/actions", "");
    assert_eq!(s, 200);
    assert_eq!(v["signal"].as_array().unwrap().len(), 1);
    assert_eq!(v["signal"][0]["output"].as_str().unwrap().len(), 3000);
    server.close();
}

#[test]
fn pending_application_retirement_and_unread_settlement_pin_original_funding_after_socket_close() {
    let mut server = Server::new();
    let token = server.invoke("run", 1);
    let id = token.split_once('-').unwrap().1.parse().unwrap();
    server.app.retained.set(true);
    server.exposure.begin_close();
    let native = server
        .exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap();
    let handle = native.handle(id).unwrap();
    server.owner.close_raw_action(&handle).unwrap();
    assert!(
        server
            .owner
            .step_raw_action(&handle, &mut work())
            .unwrap()
            .is_pending()
    );
    assert!(native.acknowledge(&mut server.owner, id).unwrap().is_none());
    assert!(
        server
            .exposure
            .retire(&mut server.owner, &mut work())
            .is_err()
    );
    assert!(
        server
            .bank
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used()
            > 0
    );
    TcpListener::bind(server.addr).unwrap();
    server.app.retained.set(false);
    assert!(
        server
            .owner
            .step_raw_action(&handle, &mut work())
            .unwrap()
            .is_ready()
    );
    let settlement = server
        .exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .acknowledge(&mut server.owner, id)
        .unwrap()
        .unwrap();
    server
        .exposure
        .retire(&mut server.owner, &mut work())
        .unwrap();
    drop(server.exposure);
    drop(server.owner);
    assert!(
        server
            .bank
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used()
            > 0
    );
    assert_eq!(settlement.instance.slot().slot(), SlotIndex::new(1));
    drop(settlement);
    assert_eq!(server.app.disposed.get(), 1);
    clean(&server.bank);
}

#[test]
fn native_invocation_identity_cannot_cross_action_control_routes() {
    let mut server = Server::new();
    let token = server.invoke("run", 1);
    for (method, path) in [("GET", "/signal/status"), ("DELETE", "/signal/stop")] {
        let (s, _, _) = server.send(method, &format!("{path}?id={token}"), "");
        assert_eq!(s, 404);
    }
    assert_eq!(server.app.leaf_queries.get(), 0);
    assert_eq!(server.app.cancelled.get(), 0);
    let (s, v, _) = server.send("GET", "/actions", "");
    assert_eq!(s, 200);
    assert!(
        v["run"][0]["href"]
            .as_str()
            .unwrap()
            .contains("/run/status?id=")
    );
    server.close();
}

#[test]
fn closed_native_setup_cannot_publish_after_actual_capability_disappears() {
    struct Toggle {
        enabled: Rc<Cell<bool>>,
        inner: HttpActionProducer,
    }
    impl ProducerCapability for Toggle {
        fn identity(&self) -> (BindingId, BindingGeneration) {
            self.inner.identity()
        }
        fn definition_bytes(&self, d: &dyn std::any::Any) -> Option<usize> {
            self.inner.definition_bytes(d)
        }
        fn prepare(
            &self,
            i: ProducerOperationInput<'_>,
            b: &mut WorkBudget,
        ) -> CoreResult<Option<Box<dyn std::any::Any>>> {
            self.inner.prepare(i, b)
        }
        fn supports(&self, i: ProducerOperationInput<'_>, b: &mut WorkBudget) -> CoreResult<bool> {
            if self.enabled.get() {
                self.inner.supports(i, b)
            } else {
                Ok(false)
            }
        }
        fn final_check_temporary_bytes(&self) -> Option<usize> {
            self.inner.final_check_temporary_bytes()
        }
        fn check_final(
            &self,
            i: ProducerOperationInput<'_>,
            d: &dyn std::any::Any,
            b: &mut WorkBudget,
        ) -> CoreResult<bool> {
            if self.enabled.get() {
                self.inner.check_final(i, d, b)
            } else {
                Ok(false)
            }
        }
    }
    let addr = address();
    let td = description(addr);
    let app = Rc::new(App::default());
    let mut owner = owner();
    let bank = accounts();
    let instance = logical(authority(Generation::INITIAL), &td, app.clone(), true);
    let (prepared, _) = prepare(&td, &instance, addr).unwrap();
    let enabled = Rc::new(Cell::new(true));
    let providers: Vec<Rc<dyn ProducerCapability>> = vec![Rc::new(Toggle {
        enabled: enabled.clone(),
        inner: HttpActionProducer::new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            BindingId::new(61),
            BindingGeneration::INITIAL,
        ),
    })];
    let definitions = prepared
        .operations()
        .iter()
        .map(|o| o.definition::<HttpActionDefinition>().unwrap().clone())
        .collect();
    let participant = HttpActionRuntime::try_new(&owner, instance.key(), definitions, 8).unwrap();
    let mut exposure = ComposedProducerExposure::try_new(
        &owner,
        ComposedProducerInput {
            instance,
            prepared,
            providers,
            participants: vec![Box::new(participant)],
            expected: None,
            handler_backing_bytes: 256,
        },
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        bank.clone(),
        &mut work(),
    )
    .ok()
    .unwrap();
    exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .install_closed()
        .unwrap();
    let mut peer = TcpStream::connect(addr).unwrap();
    peer.write_all(format!("GET /actions HTTP/1.1\r\nHost: {addr}\r\n\r\n").as_bytes())
        .unwrap();
    assert!(
        exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .step(&mut owner, &mut work())
            .unwrap()
            .is_pending()
    );
    assert_eq!(app.roots.get(), 0);
    enabled.set(false);
    assert!(exposure.publish(&mut owner, &mut work()).is_err());
    assert!(owner.description(exposure.key()).is_none());
    assert_eq!(app.roots.get(), 0);
    drop(peer);
    exposure.begin_close();
    exposure.retire(&mut owner, &mut work()).unwrap();
    drop(exposure);
    clean(&bank);
    TcpListener::bind(addr).unwrap();
}

#[test]
fn native_response_loss_reports_failure_and_disposes_the_original_connection_without_replay() {
    io().block_on(async {
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();
        let peer=std::thread::spawn(move||{let(mut socket,_)=listener.accept().unwrap();socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut bytes=Vec::new();let mut buf=[0;1024];while !bytes.ends_with(b"\r\n\r\n"){let n=socket.read(&mut buf).unwrap();assert!(n>0);bytes.extend_from_slice(&buf[..n]);}
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 99\r\nConnection: close\r\n\r\n{\"run\":[").unwrap();drop(socket);
            listener.set_nonblocking(true).unwrap();listener});
        let bank=accounts();let instance=ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(2),Generation::INITIAL));let client=consumer(&description(addr),bank.clone(),&instance,Arc::new(ConsumerPolicy::default()));let mut call=query(&client);let end=Instant::now()+Duration::from_secs(5);
        let result=loop{if let Poll::Ready(r)=call.poll(&mut cx(),now(),&mut work()){break r;}assert!(Instant::now()<end);tokio::time::sleep(Duration::from_millis(1)).await;};
        assert!(result.result().is_err());assert_eq!(result.operation(),Operation::QueryAllActions);drop(result);drop(call);drop(client);clean(&bank);
        let listener=peer.join().unwrap();assert!(matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
    });
}

#[test]
fn root_query_refuses_an_application_input_without_authorizing_or_progressing_actions() {
    let mut server = Server::new();
    server.invoke("run", 1);
    let mut peer = TcpStream::connect(server.addr).unwrap();
    peer.write_all(format!("GET /actions HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n1",server.addr).as_bytes()).unwrap();
    let native = server
        .exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap();
    assert!(
        native
            .step(&mut server.owner, &mut work())
            .unwrap()
            .is_ready()
    );
    assert_eq!(server.app.roots.get(), 0);
    assert_eq!(server.app.members.get(), 0);
    assert_eq!(server.app.invoked.get(), 1);
    assert!(native.take_delivery().is_none());
    let mut byte = [0; 1];
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    assert_eq!(peer.read(&mut byte).unwrap(), 0);
    server.close();
}

#[test]
fn status_output_absence_and_present_json_null_are_distinct_schema_roles() {
    use clinkz_wot_core::value_type::{TypeSchema, ValueType};
    let schema = ActionCollectionSchema::try_new(vec![
        ActionMemberSchema {
            name: "none".into(),
            output: None,
        },
        ActionMemberSchema {
            name: "nil".into(),
            output: Some(TypeSchema::Type(ValueType::Null)),
        },
    ])
    .unwrap();
    let data = HttpConsumerConfig::for_cleartext_ip()
        .unwrap()
        .consumer_services()
        .data;
    for (name, output, valid) in [
        ("none", None, true),
        ("none", Some(Value::Null), false),
        ("nil", None, false),
        ("nil", Some(Value::Null), true),
    ] {
        let mut record = json!({"status":"completed","href":"http://127.0.0.1/status?id=1"});
        if let Some(output) = output {
            record["output"] = output;
        }
        assert_eq!(
            data.validate_action_statuses(
                &schema,
                &Payload::new(
                    json!({name:[record]}).to_string().into_bytes(),
                    "application/json"
                )
            )
            .is_ok(),
            valid
        );
    }
}

#[test]
fn td_prepared_continuing_invocations_and_root_query_share_one_consumer_authority() {
    use clinkz_wot_planning::open_consumer::ConsumerActionRegistration;
    use clinkz_wot_servient_finite::consumer_action::{ConsumerActionRuntime, ConsumerActionState};
    io().block_on(async {
        let mut server = Server::new();
        let bank = accounts();
        let instance =
            ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(2), Generation::INITIAL));
        let config = HttpConsumerConfig::for_cleartext_ip().unwrap();
        let registry = ConsumerActionRegistry::try_new(vec![ConsumerActionRegistration {
            preparation: Box::new(HttpActionConsumer::new(
                config,
                BindingId::new(91),
                BindingGeneration::INITIAL,
                0,
            )),
            services: config.consumer_services(),
        }])
        .unwrap();
        let thing: Thing = serde_json::from_value(server.td.clone()).unwrap();
        let mut limits = GatewayDefaultV1::LIMITS.clone();
        assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
        let definitions = ["run", "signal"]
            .iter()
            .map(|name| {
                registry
                    .prepare_action(
                        &thing,
                        name,
                        ConsumerActionSelectors::default(),
                        &limits,
                        bank.clone(),
                        &mut work(),
                    )
                    .unwrap()
            })
            .collect();
        drop(thing);
        let runtime = ConsumerActionRuntime::try_new(instance.reference(), definitions, 2)
            .ok()
            .unwrap();
        let group = consumer(
            &server.td,
            bank.clone(),
            &instance,
            Arc::new(ConsumerPolicy::default()),
        );
        let a = runtime
            .invoke_action(
                "run",
                Some(Payload::new(b"12".to_vec(), "application/json")),
                Deadline::NONE,
                now(),
                &mut work(),
            )
            .ok()
            .unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        while a.progress(&mut cx(), now(), &mut work()).unwrap() != ConsumerActionState::Pending {
            let native = server
                .exposure
                .participant_mut::<HttpActionRuntime>(0)
                .unwrap();
            let _ = native.step(&mut server.owner, &mut work()).unwrap();
            native.take_delivery();
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let b = runtime
            .invoke_action(
                "signal",
                Some(Payload::new(b"1".to_vec(), "application/json")),
                Deadline::NONE,
                now(),
                &mut work(),
            )
            .ok()
            .unwrap();
        while b.progress(&mut cx(), now(), &mut work()).unwrap() != ConsumerActionState::Pending {
            let native = server
                .exposure
                .participant_mut::<HttpActionRuntime>(0)
                .unwrap();
            let _ = native.step(&mut server.owner, &mut work()).unwrap();
            native.take_delivery();
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let before = [a.identity().unwrap(), b.identity().unwrap()];
        let mut call = query(&group);
        let (result, _) = server.exchange(&mut call).await;
        let v = values(&result);
        assert_eq!(v["run"][0]["status"], "running");
        assert_eq!(v["signal"][0]["status"], "running");
        assert_eq!([a.identity().unwrap(), b.identity().unwrap()], before);
        assert_eq!(server.app.invoked.get(), 2);
        drop(result);
        drop(call);
        a.cancel_local();
        b.cancel_local();
        for h in [&a, &b] {
            while h.progress(&mut cx(), now(), &mut work()).unwrap() != ConsumerActionState::Closed
            {
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            drop(h.take_settlement(now(), &mut work()).unwrap());
        }
        drop(a);
        drop(b);
        drop(runtime);
        drop(group);
        clean(&bank);
        assert_eq!(server.app.cancelled.get(), 0);
        server.close();
    });
}

#[test]
fn review_borrowed_settlement_body_cannot_outlive_original_funding() {
    let mut server = Server::new();
    let token = server.invoke("run", 1);
    let id = token.split_once('-').unwrap().1.parse().unwrap();
    server.app.complete.set(true);
    server.progress();
    let settlement = server
        .exposure
        .participant_mut::<HttpActionRuntime>(0)
        .unwrap()
        .acknowledge(&mut server.owner, id)
        .unwrap()
        .unwrap();
    let original = server.app.last_body.borrow().as_ref().unwrap().clone();
    let root = Rc::new(settlement);
    let shared = root.clone();
    let pointer = shared.result().unwrap().data().unwrap().body.as_ptr();
    assert_eq!(pointer, original.as_ptr().cast::<u8>());
    drop(root);
    let bank = server.retire();
    assert_eq!(shared.result().unwrap().data().unwrap().body, b"1");
    assert!(original.strong_count() > 0);
    assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
    // Slice copies are caller-owned allocations. Sharing the original bytes
    // requires sharing their result owner and its original capacity.
    let copied = shared.result().unwrap().data().unwrap().body.to_vec();
    assert_ne!(copied.as_ptr(), pointer);
    drop(shared);
    assert_eq!(original.strong_count(), 0);
    assert_eq!(copied, b"1");
    drop(original);
    clean(&bank);
}

#[test]
fn review_root_consumer_refuses_unhandled_response_assertion() {
    for predicate in [
        "x:statusEnvelope",
        "urn:review:statusEnvelope",
        "http://www.w3.org/2011/http#unknown",
    ] {
        let addr = address();
        let mut td = description(addr);
        td["@context"] = json!(["https://www.w3.org/2022/wot/td/v1.1", {"x":"urn:review:"}]);
        td["forms"][0]["response"][predicate] = json!("wrapped");
        td["properties"] = json!({"p":{"type":"integer","readOnly":true,"forms":[{"href":format!("http://{addr}/p")}]}});
        let bank = accounts();
        let prepared =
            prepare_consumer(&td, bank.clone(), Arc::new(ConsumerPolicy::default())).unwrap();
        assert!(
            !prepared
                .support()
                .iter()
                .any(|s| s.operation == Operation::QueryAllActions && s.selected),
            "{predicate}"
        );
        assert!(
            prepared
                .support()
                .iter()
                .any(|s| s.operation == Operation::ReadProperty && s.selected)
        );
        drop(prepared);
        clean(&bank);
    }
    let mut supported = description(address());
    supported["forms"][0]["response"]["@type"] = json!("urn:review:ExpectedResponse");
    let bank = accounts();
    let prepared = prepare_consumer(
        &supported,
        bank.clone(),
        Arc::new(ConsumerPolicy::default()),
    )
    .unwrap();
    assert!(
        prepared
            .support()
            .iter()
            .any(|s| s.operation == Operation::QueryAllActions && s.selected)
    );
    drop(prepared);
    clean(&bank);
}

#[test]
fn review_acknowledged_results_still_use_bounded_result_capacity() {
    let mut server = Server::new();
    server.app.complete.set(true);
    server.app.long.set(true);
    let bank = server.bank.clone();
    let reserved = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    let mut settlements = Vec::new();
    let mut last_charge = reserved;
    for _ in 0..64 {
        let (status, value, report) = server.send("POST", "/signal", "1");
        if status == 503 {
            assert_eq!(settlements.len(), 8);
            assert!(matches!(
                report.request_error,
                Some(CoreError::Backpressure(_))
            ));
            break;
        }
        assert_eq!(status, 202);
        let token = value["id"].as_str().unwrap();
        let id = token.split_once('-').unwrap().1.parse().unwrap();
        server.progress();
        let settlement = server
            .exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .acknowledge(&mut server.owner, id)
            .unwrap()
            .unwrap();
        settlements.push(settlement);
        let charged = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
        assert!(
            charged > last_charge,
            "newly admitted output must own additional backing"
        );
        last_charge = charged;
    }
    assert_eq!(settlements.len(), 8);
    assert_eq!(server.app.invoked.get(), 8);
    assert_eq!(server.native().records(), 0);
    let retained: usize = settlements
        .iter()
        .map(|s| s.result().unwrap().data().unwrap().body.len())
        .sum();
    assert!(last_charge >= reserved + retained as u64);
    drop(settlements.pop());
    assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() < last_charge);
    let token = server.invoke("signal", 1);
    server.progress();
    let id = token.split_once('-').unwrap().1.parse().unwrap();
    settlements.push(
        server
            .exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .acknowledge(&mut server.owner, id)
            .unwrap()
            .unwrap(),
    );
    let (status, _, _) = server.send("POST", "/signal", "1");
    assert_eq!(status, 503);
    assert_eq!(server.app.invoked.get(), 9);
    let bank = server.retire();
    assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
    drop(settlements);
    clean(&bank);
}

#[test]
fn review_result_funding_refusal_is_pre_effect_and_rolls_back_every_original_account() {
    for kind in [
        ResourceKind::EngineLiveBytesGlobalMax,
        ResourceKind::CleanupBytesMax,
        ResourceKind::CleanupItemsMax,
    ] {
        let mut server = Server::new();
        let bank = server.bank.clone();
        let original: Vec<_> = ResourceKind::ALL
            .into_iter()
            .map(|k| bank.account(k).used())
            .collect();
        let account = bank.account(kind);
        let occupied = bank
            .reserve(kind, account.limit() - account.used())
            .unwrap();
        let (status, _, report) = server.send("POST", "/run", "1");
        assert_eq!(status, 503, "{kind:?}");
        assert!(matches!(
            report.request_error,
            Some(CoreError::Backpressure(_))
        ));
        assert_eq!(server.app.invoked.get(), 0);
        assert_eq!(server.native().records(), 0);
        drop(occupied);
        for (k, before) in ResourceKind::ALL.into_iter().zip(original) {
            assert_eq!(bank.account(k).used(), before, "{k:?}");
        }
        server.invoke("run", 1);
        server.close();
    }
}
