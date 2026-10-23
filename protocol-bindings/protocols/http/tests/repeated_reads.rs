#![cfg(feature = "prepare")]
//! Separate process: process-wide FD observations must not race other tests.
include!("support/application.rs");
use std::sync::mpsc;

#[test]
fn hundred_alternating_public_reads_are_source_free_and_use_fresh_connections() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (retired, retirement) = mpsc::channel();
    let (finished, finishing) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        for i in 0..100 {
            let (mut socket, _) = listener.accept().unwrap();
            let head = request(&mut socket);
            let path = if i % 2 == 0 { "/a?x=1" } else { "/b" };
            assert!(
                head.starts_with(&format!("GET {path} HTTP/1.1\r\n")),
                "{head}"
            );
            assert!(
                head.to_lowercase()
                    .contains(&format!("host: 127.0.0.1:{port}\r\n"))
            );
            let body = if i % 2 == 0 { "17" } else { "29" };
            match i % 3 {
                0 => write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{body}").unwrap(),
                1 => {
                    assert!(!head.to_lowercase().contains("\r\nte:"));
                    write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nTrailer: X-Check\r\n\r\n2;note=ok\r\n{body}\r\n0\r\nX-Check: yes\r\n\r\n").unwrap();
                }
                _ => {
                    write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{body}").unwrap();
                    socket.shutdown(std::net::Shutdown::Write).unwrap();
                }
            }
            assert_retired(&mut socket);
            drop(socket);
            retired.send(()).unwrap();
        }
        finishing.recv_timeout(Duration::from_secs(5)).unwrap();
    });
    recipe().block_on(async {
        let (runtime, handle, observer, parent) = engine(port);
        let account_baseline: Vec<_> = parent
            .snapshot(ConsumerPropertyReadPolicy::PARENT_KINDS)
            .iter()
            .map(|a| a.used())
            .collect();
        #[cfg(target_os = "linux")]
        let fd_baseline = std::fs::read_dir("/proc/self/fd").unwrap().count();
        for i in 0..100 {
            let mut call = handle
                .read_property(
                    if i % 2 == 0 { "a" } else { "b" },
                    InteractionOptions::new(),
                    &mut work(),
                )
                .unwrap();
            assert!(
                handle
                    .read_property("a", InteractionOptions::new(), &mut work())
                    .is_err()
            );
            let output = take(&mut call).await.unwrap();
            assert_eq!(
                &*output.data().unwrap().body,
                if i % 2 == 0 { b"17" } else { b"29" }
            );
            assert_eq!(
                output.metadata().binding_response().unwrap().status_code(),
                200
            );
            drop(output);
            drop(call);
            retirement.recv_timeout(Duration::from_secs(5)).unwrap();
            assert_eq!(
                parent
                    .snapshot(ConsumerPropertyReadPolicy::PARENT_KINDS)
                    .iter()
                    .map(|a| a.used())
                    .collect::<Vec<_>>(),
                account_baseline
            );
            #[cfg(target_os = "linux")]
            assert_eq!(
                std::fs::read_dir("/proc/self/fd").unwrap().count(),
                fd_baseline,
                "fresh reads leaked a descriptor"
            );
        }
        finished.send(()).unwrap();
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(
            parent
                .snapshot(ConsumerPropertyReadPolicy::PARENT_KINDS)
                .iter()
                .all(|a| a.used() == 0)
        );
    });
    peer.join().unwrap();
}
