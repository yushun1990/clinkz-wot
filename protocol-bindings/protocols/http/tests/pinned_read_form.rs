#![cfg(feature = "prepare")]
#![allow(dead_code)] // Shared peer and driver helpers.
include!("support/application.rs");

#[test]
fn a_pinned_literal_read_stays_callable_beside_a_dynamic_form() {
    recipe().block_on(async {
        for thing_scope in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let peer = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let head = request(&mut socket);
                assert!(head.starts_with("GET /static HTTP/1.1\r\n"), "{head}");
                socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n17").unwrap();
                assert_retired(&mut socket);
            });

            let mut description = serde_json::json!({
                "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"literal Read alternative",
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
            let engine = consumer(
                serde_json::from_value(description).unwrap(),
                http_registration(),
                ConsumerSelector { form: Some(1), ..Default::default() },
            ).unwrap();
            let (_, handle, calls, _) = &engine;
            assert!(handle.read_property_with_variables(
                calls, "a", &[("item", "chosen")], Deadline::NONE, now()
            ).is_err());
            let mut call = handle.read_property(calls, "a", Deadline::NONE, now()).unwrap();
            let output = take(&mut call).await.unwrap();
            assert_eq!(output.result().unwrap().data().unwrap().body, b"17");
            drop((output, call));
            peer.join().unwrap();
            release(engine);
        }
    });
}
