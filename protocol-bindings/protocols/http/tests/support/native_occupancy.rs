use super::*;

#[test]
fn retained_results_do_not_occupy_or_release_a_successor_native_exchange() {
    io().block_on(async {
        let td = description(address());
        let mut server = Fixture::new(td.clone());
        let backing = bank();
        let definition = mapping(&td, &backing, None);
        let accounts = [bank(), bank(), bank()];
        let owners = [
            consumer_instance(),
            consumer_instance(),
            consumer_instance(),
        ];
        let clients: Vec<_> = owners
            .iter()
            .zip(&accounts)
            .map(|(owner, account)| {
                bind(
                    definition.clone(),
                    owner,
                    account,
                    Arc::new(AtomicBool::new(false)),
                )
            })
            .collect();

        let mut first = start_read(&clients[0]);
        let (retained, _) = server.exchange(&mut first).await;
        retained.result().unwrap();
        let first_charge = live(&accounts[0]);
        let mut second = start_read(&clients[1]);
        assert!(
            clients[2]
                .0
                .read_all_properties(&clients[2].1, Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        assert_eq!(
            live(&accounts[0]),
            first_charge,
            "native availability cannot refund retained results"
        );
        drop(retained);
        drop(first);
        assert!(
            clients[2]
                .0
                .read_all_properties(&clients[2].1, Deadline::NONE, now(0), &mut work())
                .is_err(),
            "dropping an old result must not unlock the successor's exchange"
        );
        let (retained_second, _) = server.exchange(&mut second).await;
        retained_second.result().unwrap();

        let mut cancelled = start_read(&clients[2]);
        cancelled.cancel();
        let result = cancelled.poll(&mut cx(), now(0), &mut work());
        let Poll::Ready(cancelled_result) = result else {
            panic!("local native cancellation must complete")
        };
        assert!(matches!(
            cancelled_result.first_cause(),
            Some(CoreError::Cancelled(_))
        ));
        // Its socket/future is retired, while the original cancellation receipt
        // and second result still retain all original backing and authority.
        let mut successor = start_read(&clients[0]);
        drop(cancelled_result);
        drop(cancelled);
        drop(retained_second);
        drop(second);
        assert!(
            clients[2]
                .0
                .read_all_properties(&clients[2].1, Deadline::NONE, now(0), &mut work())
                .is_err(),
            "old cancellation/result Drop must not release a new occupancy lease"
        );
        let (result, _) = server.exchange(&mut successor).await;
        result.result().unwrap();
        drop(result);
        drop(successor);

        // An accepted call abandoned before first poll still owns the lane until
        // the runtime actually retires it, independently of logical handle Drop.
        let abandoned = start_read(&clients[1]);
        drop(abandoned);
        assert!(
            clients[2]
                .0
                .read_all_properties(&clients[2].1, Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        clients[1].1.step(&mut cx(), now(0), &mut work());
        let mut final_call = start_read(&clients[2]);
        let (result, _) = server.exchange(&mut final_call).await;
        result.result().unwrap();
        drop(result);
        drop(final_call);
        assert_eq!(
            server.controls.requests.get(),
            4,
            "cancelled/unpolled calls perform no I/O"
        );
        drop(clients);
        drop(owners);
        drop(definition);
        for account in accounts.iter().chain([&backing]) {
            clean(account);
        }
        server.close();
    });
}

#[test]
fn failed_native_exchange_releases_occupancy_while_error_settlement_stays_funded() {
    io().block_on(async {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let td = description(listener.local_addr().unwrap());
        let peer = std::thread::spawn(move || {
            for bad in [true, false] {
                let (mut socket, _) = listener.accept().unwrap();
                socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut bytes = Vec::new(); let mut buf = [0; 1024];
                while !bytes.ends_with(b"\r\n\r\n") {
                    let n = socket.read(&mut buf).unwrap(); assert!(n > 0); bytes.extend_from_slice(&buf[..n]);
                }
                assert!(bytes.starts_with(b"GET /properties HTTP/1.1\r\n"));
                if bad {
                    socket.write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n").unwrap();
                } else {
                    let body = br#"{"level":7,"on":true}"#;
                    write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", body.len()).unwrap();
                    socket.write_all(body).unwrap();
                }
            }
        });
        let backing = bank(); let accounts = [bank(), bank()];
        let owners = [consumer_instance(), consumer_instance()];
        let definition = mapping(&td, &backing, None);
        let clients: Vec<_> = owners.iter().zip(&accounts).map(|(owner, account)| {
            bind(definition.clone(), owner, account, Arc::new(AtomicBool::new(false)))
        }).collect();
        let mut first = start_read(&clients[0]);
        let retained = tokio::time::timeout(Duration::from_secs(5), std::future::poll_fn(|cx| first.poll(cx, now(0), &mut work()))).await.unwrap();
        assert!(retained.result().is_err()); assert!(live(&accounts[0]) > 0);
        let original_charge = live(&accounts[0]);
        let mut second = start_read(&clients[1]);
        let result = tokio::time::timeout(Duration::from_secs(5), std::future::poll_fn(|cx| second.poll(cx, now(0), &mut work()))).await.unwrap();
        result.result().unwrap(); assert_eq!(live(&accounts[0]), original_charge);
        drop(result); drop(second); drop(clients); drop(owners); drop(definition);
        assert!(live(&accounts[0]) > 0 && live(&backing) > 0);
        drop(first); drop(retained);
        for account in accounts.iter().chain([&backing]) { clean(account); }
        peer.join().unwrap();
    });
}
