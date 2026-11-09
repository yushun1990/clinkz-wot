#![cfg(feature = "prepare")]
#![allow(dead_code)] // Share the existing restricted-runtime/socket fixture.
include!("support/application.rs");

#[test]
fn restricted_read_skips_templates_before_selecting_a_static_alternative() {
    recipe().block_on(async {
        for thing_scope in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            listener.set_nonblocking(true).unwrap();
            let peer = std::thread::spawn(move || {
                for _ in 0..2 {
                    let end = Instant::now() + Duration::from_secs(5);
                    let mut socket = loop {
                        match listener.accept() {
                            Ok((socket, _)) => break socket,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                assert!(Instant::now() < end, "static Form never executed");
                                std::thread::sleep(Duration::from_millis(1));
                            }
                            Err(error) => panic!("{error}"),
                        }
                    };
                    let head = request(&mut socket);
                    assert!(head.starts_with("GET /static HTTP/1.1\r\n"), "{head}");
                    socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n17").unwrap();
                    assert_retired(&mut socket);
                }
                assert!(matches!(listener.accept(),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock));
            });

            let policy = policy();
            let parent = Arc::new(
                clinkz_wot_servient::host_accounts::HostResourceAccounts::try_from_accounts(
                    policy.parent_accounts(),
                ).unwrap(),
            );
            let registration = try_inline_consumer_registration(
                HttpConsumerConfig::for_cleartext_ip().unwrap(),
                BindingId::new(600),
                BindingGeneration::INITIAL,
                600,
            ).unwrap();
            let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
                registration,
                policy,
                Arc::new(Clock(Instant::now())),
                parent.clone(),
            ).unwrap_or_else(|error| panic!("{:?}", error.error()));
            let mut description = serde_json::json!({
                "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"literal Read alternative",
                "id":"urn:restricted-form-selection",
                "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
                "properties":{"a":{"type":"integer","readOnly":true,
                    "forms":[{"href":format!("http://{address}/{{item}}")},
                             {"href":format!("http://{address}/static")}]
                }}
            });
            let declarations = serde_json::json!({"item":{"type":"string"}});
            if thing_scope {
                description["uriVariables"] = declarations;
            } else {
                description["properties"]["a"]["uriVariables"] = declarations;
            }
            let thing = serde_json::from_value(description).unwrap();
            let handle = match runtime.consume(&thing, &mut work(), |build| {
                for _ in 0..10000 {
                    build.step(&mut work());
                    if build.is_finished() {
                        return;
                    }
                }
                panic!("Read preparation stalled");
            }).unwrap() {
                StartStatus::Ready(handle) => handle,
                _ => panic!("publication pending"),
            };
            drop(thing);

            for options in [
                InteractionOptions::new(),
                InteractionOptions::new().with_form_index(1),
            ] {
                let mut call = handle.read_property("a", options, &mut work()).unwrap();
                let output = take(&mut call).await.unwrap();
                assert_eq!(output.data().unwrap().body.as_ref(), b"17");
                drop((output, call));
            }
            // Neither selecting the template nor supplying variables can add
            // the actual-destination admission this restricted owner lacks.
            for options in [
                InteractionOptions::new().with_form_index(0),
                InteractionOptions::new().with_form_index(0).with_uri_variable("item", "chosen"),
            ] {
                let rejected = handle.read_property("a", options.clone(), &mut work())
                    .err().expect("template must be ineligible");
                assert_eq!(rejected.input().0, options);
                assert!(rejected.input().1.is_none(),
                    "refuse before constructing an outbound request");
            }
            peer.join().unwrap();
            drop(handle);
            shutdown(&runtime);
            drop((runtime, observer));
            assert!(parent.snapshot(ConsumerPropertyReadPolicy::PARENT_KINDS)
                .iter().all(|account| account.used() == 0));
        }
    });
}
