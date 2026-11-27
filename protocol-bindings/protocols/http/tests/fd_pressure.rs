#![cfg(feature = "prepare")]
//! RLIMIT_NOFILE is process-wide: lower it only in a disposable Linux child.
#![cfg(target_os = "linux")]
include!("support/application.rs");

fn exhaust() -> Vec<std::fs::File> {
    let mut files = Vec::new();
    loop {
        match std::fs::File::open("/dev/null") {
            Ok(file) => files.push(file),
            Err(error) => {
                assert_eq!(error.raw_os_error(), Some(24), "expected Linux EMFILE");
                return files;
            }
        }
    }
}

#[test]
fn descriptor_pressure_recovers_input_and_accepted_call_then_reuses_runtime() {
    const CHILD: &str = "CLINKZ_HTTP_FD_PRESSURE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new("bash")
            .args(["-c", "ulimit -n 128 && exec \"$@\"", "http-fd-pressure"])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "descriptor_pressure_recovers_input_and_accepted_call_then_reuses_runtime",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    recipe().block_on(async {
        let (owner, handle, calls, parent) = engine(port);
        let baseline = usage(&parent);
        let fd_baseline = std::fs::read_dir("/proc/self/fd").unwrap().count();
        let files = exhaust();
        let rejected = handle.read_property(&calls, "a", after(321), now()).err().expect("probe must refuse EMFILE");
        drop(files);
        let error = rejected;
        assert_eq!(error.context().resource_failure().unwrap().1, ResourceFailureDomain::DeploymentFailure);
        assert!(error.context().plan().is_some());
        assert_eq!(usage(&parent), baseline);

        let mut call = handle.read_property(&calls, "a", Deadline::NONE, now()).unwrap();
        let files = exhaust();
        let _ = call.progress(&mut Context::from_waker(Waker::noop()), now());
        let result = take_ready(&mut call).expect("socket creation failure must settle without reactor or network progress");
        drop(files);
        let error = result.err().unwrap();
        assert!(matches!(error, CoreError::Binding(_)), "{error:?}");
        assert_eq!(error.context().cause_code(), Some(604));
        assert_eq!(error.context().resource_failure().unwrap().1, ResourceFailureDomain::DeploymentFailure, "{error:?}");
        assert_eq!(error.context().binding(), Some((BindingId::new(600), BindingGeneration::INITIAL)));
        assert!(error.context().plan().is_some());
        drop(call);
        assert_eq!(usage(&parent), baseline);
        assert_eq!(std::fs::read_dir("/proc/self/fd").unwrap().count(), fd_baseline);
        assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock, "failed calls connected");

        listener.set_nonblocking(false).unwrap();
        let (connected, connection) = std::sync::mpsc::channel();
        let (retired, retirement) = std::sync::mpsc::channel();
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            assert!(request(&mut socket).starts_with("GET /a?x=1 HTTP/1.1\r\n"));
            connected.send(()).unwrap();
            assert_retired(&mut socket); // Silent peer: no EOF or response supplied.
            drop(socket);
            retired.send(()).unwrap();
            let (mut socket, _) = listener.accept().unwrap();
            assert!(request(&mut socket).starts_with("GET /b HTTP/1.1\r\n"));
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n29").unwrap();
            assert_retired(&mut socket);
        });
        let mut cancelled = handle.read_property(&calls, "a", Deadline::NONE, now()).unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            let _ = cancelled.progress(&mut Context::from_waker(Waker::noop()), now());
            if connection.try_recv().is_ok() { break; }
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let files = exhaust();
        cancelled.cancel();
        let _ = cancelled.progress(&mut Context::from_waker(Waker::noop()), now());
        let result = take_ready(&mut cancelled).expect("cancellation under FD pressure waited for I/O");
        drop(files);
        assert!(matches!(result, Err(CoreError::Cancelled(_))));
        drop(cancelled);
        retirement.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(usage(&parent), baseline);
        let mut retry = handle.read_property(&calls, "b", Deadline::NONE, now()).unwrap();
        assert_eq!(take(&mut retry).await.unwrap().result().unwrap().data().unwrap().body, b"29");
        drop(retry);
        peer.join().unwrap();
        // Ordinary connection refusal shares the deployment-availability domain;
        // no automatic retry or relabeling as an ingress-policy failure.
        let mut refused = handle.read_property(&calls, "b", Deadline::NONE, now()).unwrap();
        let error = take(&mut refused).await.err().unwrap();
        assert_eq!(error.context().cause_code(), Some(604));
        assert_eq!(error.context().resource_failure().unwrap().1, ResourceFailureDomain::DeploymentFailure);
        drop(refused);
        assert_eq!(usage(&parent), baseline);
        release((owner, handle, calls, parent));
    });
}
