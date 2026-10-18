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
        let (runtime, handle, observer, parent) = engine(port);
        let baseline: Vec<_> = parent.lock().unwrap().iter().map(|a| a.used()).collect();
        let fd_baseline = std::fs::read_dir("/proc/self/fd").unwrap().count();
        let files = exhaust();
        let rejected = handle.read_property("a", InteractionOptions::new().with_timeout(Duration::from_millis(321)), &mut work()).err().expect("probe must refuse EMFILE");
        drop(files);
        let ((options, recovered), error) = rejected.into_parts();
        assert_eq!(options.timeout, Some(Duration::from_millis(321)));
        let original = recovered.expect("raw binding rejected original request");
        assert!(original.uri_variables().is_empty());
        assert_eq!(error.error().context().resource_failure().unwrap().1, ResourceFailureDomain::DeploymentFailure);
        assert_eq!(error.error().context().plan(), Some(original.plan_id()));
        drop((options, original, error));
        assert_eq!(parent.lock().unwrap().iter().map(|a| a.used()).collect::<Vec<_>>(), baseline);

        let mut call = handle.read_property("a", InteractionOptions::new(), &mut work()).unwrap();
        let files = exhaust();
        call.step(&mut Context::from_waker(Waker::noop()), &mut work());
        let result = call.try_take(&mut work()).unwrap().expect("socket creation failure must settle without reactor or network progress");
        drop(files);
        let error = result.unwrap_err();
        assert!(matches!(error, CoreError::Binding(_)), "{error:?}");
        assert_eq!(error.context().cause_code(), Some(604));
        assert_eq!(error.context().resource_failure().unwrap().1, ResourceFailureDomain::DeploymentFailure, "{error:?}");
        assert_eq!(error.context().binding(), Some((BindingId::new(600), BindingGeneration::INITIAL)));
        assert!(error.context().plan().is_some());
        drop(call);
        assert_eq!(parent.lock().unwrap().iter().map(|a| a.used()).collect::<Vec<_>>(), baseline);
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
        let mut cancelled = handle.read_property("a", InteractionOptions::new(), &mut work()).unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            cancelled.step(&mut Context::from_waker(Waker::noop()), &mut work());
            if connection.try_recv().is_ok() { break; }
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let files = exhaust();
        cancelled.cancel().unwrap();
        cancelled.step(&mut Context::from_waker(Waker::noop()), &mut work());
        let result = cancelled.try_take(&mut work()).unwrap().expect("cancellation under FD pressure waited for I/O");
        drop(files);
        assert!(matches!(result, Err(CoreError::Cancelled(_))), "{result:?}");
        drop(cancelled);
        retirement.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(parent.lock().unwrap().iter().map(|a| a.used()).collect::<Vec<_>>(), baseline);
        let mut retry = handle.read_property("b", InteractionOptions::new(), &mut work()).unwrap();
        assert_eq!(&*take(&mut retry).await.unwrap().data().unwrap().body, b"29");
        drop(retry);
        peer.join().unwrap();
        // Ordinary connection refusal shares the deployment-availability domain;
        // no automatic retry or relabeling as an ingress-policy failure.
        let mut refused = handle.read_property("b", InteractionOptions::new(), &mut work()).unwrap();
        let error = take(&mut refused).await.unwrap_err();
        assert_eq!(error.context().cause_code(), Some(604));
        assert_eq!(error.context().resource_failure().unwrap().1, ResourceFailureDomain::DeploymentFailure);
        drop(refused);
        assert_eq!(parent.lock().unwrap().iter().map(|a| a.used()).collect::<Vec<_>>(), baseline);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    });
}
