use super::*;

#[test]
fn qualified_dispatch_still_rejects_wrong_operation_and_foreign_artifact() {
    let backing = bank();
    let td = json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"sealed read",
        "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
        "properties":{"level":{"type":"integer","readOnly":true,"forms":[{"href":"http://127.0.0.1:1/level"}]}}});
    let definition = definition(&td, &backing).unwrap();
    let id = definition.artifact().identity();
    let input = payload(b"original");
    let request = OutboundRequest::finite(
        BindingArtifactRef::new(id, SlotIndex::new(0)),
        Operation::WriteProperty,
        Some(input),
        Default::default(),
        None,
    )
    .unwrap();
    let rejected = definition.start_finite(request).err().unwrap();
    assert_eq!(&*rejected.input().input().unwrap().body, b"original");
    drop(rejected);
    let foreign = BindingArtifactIdentity::new(
        id.plan_set_generation(),
        id.plan_id(),
        BindingId::new(9999),
        id.binding_generation(),
        id.configuration(),
        id.compatibility(),
        id.role(),
    );
    let request = OutboundRequest::finite(
        BindingArtifactRef::new(foreign, SlotIndex::new(0)),
        Operation::ReadProperty,
        None,
        Default::default(),
        None,
    )
    .unwrap();
    assert!(definition.start_finite(request).is_err());
    drop(definition);
    clean(&backing);
}

#[test]
fn fixed_bound_http_reads_keep_live_permission_and_original_results_after_owner_loss() {
    io().block_on(async {
        let peer = Peer::new();
        let backing = bank();
        let td = json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"bound read",
            "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
            "properties":{"level":{"type":"integer","readOnly":true,"forms":[{"href":format!("http://{}/level",peer.address)}]}}});
        let definition = definition(&td, &backing).unwrap();
        let accounts = bank();
        let owner = consumer_instance();
        let denied = Arc::new(AtomicBool::new(false));
        let client = bind_dynamic(definition, &owner, &accounts,
            vec![format!("http://{}/level",peer.address)], denied.clone());
        let read = client.0.bind_property_read("level").unwrap();
        drop(td);
        denied.store(true, Ordering::Release);
        assert!(read.read(&client.1, Deadline::NONE, now(0)).is_err());
        assert_eq!(peer.count(), 0);
        denied.store(false, Ordering::Release);
        let mut last = None;
        for _ in 0..2 {
            let mut call = read.read(&client.1, Deadline::NONE, now(0)).unwrap();
            let result = finish(&mut call).await;
            assert_eq!(result.result().unwrap().data().unwrap().body, b"7");
            assert_eq!(result.destination(), format!("http://{}/level",peer.address));
            assert_eq!(result.consumer(), &owner.reference());
            drop(call);
            last = Some(result);
        }
        assert_eq!(peer.count(), 2);
        // A passive bound view does not keep its removed association usable.
        client.0.begin_close();
        assert!(matches!(read.read(&client.1, Deadline::NONE, now(0)), Err(CoreError::StaleHandle(_))));
        drop((read, client, owner));
        assert!(accounts.account(ResourceKind::CleanupBytesMax).used() > 0);
        assert!(backing.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
        drop(last);
        clean(&accounts);
        clean(&backing);
    });
}

#[test]
fn bound_variable_mapping_resolves_each_actual_destination_and_checks_each_operand() {
    io().block_on(async {
        let (a, b) = (Peer::new(), Peer::new());
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let accounts = bank();
        let owner = consumer_instance();
        let denied = Arc::new(AtomicBool::new(false));
        let client = bind_dynamic(
            definition,
            &owner,
            &accounts,
            vec![
                format!("http://{}/v/x?q=q", a.address),
                format!("http://{}/v/x?q=q", b.address),
            ],
            denied.clone(),
        );
        let read = client.0.bind_property_read("level").unwrap();
        for peer in [&a, &b] {
            let port = peer.address.port().to_string();
            assert!(
                read.read_with_variables(
                    &client.1,
                    &[("port", &port), ("item", ""), ("q", "q")],
                    Deadline::NONE,
                    now(0)
                )
                .is_err()
            );
            let mut call = read
                .read_with_variables(
                    &client.1,
                    &[("port", &port), ("item", "x"), ("q", "q")],
                    Deadline::NONE,
                    now(0),
                )
                .unwrap();
            let result = finish(&mut call).await;
            assert_eq!(result.result().unwrap().data().unwrap().body, b"7");
            assert_eq!(
                result.destination(),
                format!("http://{}/v/x?q=q", peer.address)
            );
            drop((result, call));
        }
        denied.store(true, Ordering::Release);
        let port = a.address.port().to_string();
        assert!(
            read.read_with_variables(
                &client.1,
                &[("port", &port), ("item", "x"), ("q", "q")],
                Deadline::NONE,
                now(0)
            )
            .is_err()
        );
        assert_eq!((a.count(), b.count()), (1, 1));
        drop((read, client, owner));
        clean(&accounts);
        clean(&backing);
    });
}
