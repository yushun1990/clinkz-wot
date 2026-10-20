//! Pinned native-server feasibility only. No Engine exposure, permission or
//! production Producer implementation is supplied by this test.
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use std::{
    convert::Infallible,
    future::Future,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    os::fd::AsRawFd,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    task::{Context, Poll, Waker},
    time::Duration,
};

fn listener(address: SocketAddr) -> std::io::Result<TcpListener> {
    let socket = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)?;
    socket.set_reuse_address(true)?;
    socket.bind(&address.into())?;
    socket.listen(1)?;
    socket.set_nonblocking(true)?;
    Ok(socket.into())
}

#[test]
fn native_listener_can_backpressure_before_poll_and_retain_one_close_owner() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let socket = listener("127.0.0.1:0".parse().unwrap()).unwrap();
    let address = socket.local_addr().unwrap();
    // Binding a socket is observable before application publication. It cannot
    // be described as hidden preparation or undone at a remote observer.
    assert!(listener(address).is_err());
    let (observed, wait) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        stream
            .write_all(b"GET /level HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut byte = [0];
        let error = stream.read(&mut byte).unwrap_err();
        assert!(matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
        ));
        observed.send(()).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert!(response.ends_with(b"42"));
    });
    wait.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::from_std(socket).unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        let service_calls = calls.clone();
        let reply = Arc::new(Mutex::new((None, None::<Waker>)));
        let service_reply = reply.clone();
        let service = service_fn(move |request: hyper::Request<hyper::body::Incoming>| {
            assert_eq!(request.method(), "GET");
            assert_eq!(request.uri().path(), "/level");
            service_calls.fetch_add(1, Ordering::SeqCst);
            let reply = service_reply.clone();
            std::future::poll_fn(move |cx| {
                let mut slot = reply.lock().unwrap();
                if let Some(response) = slot.0.take() {
                    Poll::Ready(Ok::<_, Infallible>(response))
                } else {
                    slot.1 = Some(cx.waker().clone());
                    Poll::Pending
                }
            })
        });
        let mut connection = Box::pin(
            http1::Builder::new()
                .keep_alive(false)
                .max_headers(16)
                .max_buf_size(8192)
                .writev(false)
                .serve_connection(TokioIo::new(stream), service),
        );
        // Production must obtain its real Engine permit before converting a
        // native request into an Engine event. This poll is only native proof.
        std::future::poll_fn(|cx| {
            assert!(connection.as_mut().poll(cx).is_pending());
            if calls.load(Ordering::SeqCst) == 1 {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
        // A request can suspend in Hyper while an outer Engine owns the unique
        // reply opportunity. Returning the reply later must wake and complete
        // the original connection, without a spawned driver or reentrant handler.
        let mut slot = reply.lock().unwrap();
        slot.0 = Some(
            Response::builder()
                .header("content-type", "application/json")
                .header("connection", "close")
                .body(Full::new(Bytes::from_static(b"42")))
                .unwrap(),
        );
        slot.1.take().unwrap().wake();
        drop(slot);
        connection.await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // Cancellation/root loss while a real request head is unfinished.
        let partial = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            stream.write_all(b"GET /level HTTP/1.1\r\nHost:").unwrap();
            let mut byte = [0];
            match stream.read(&mut byte) {
                Ok(0) => (),
                Err(error) if matches!(error.kind(), std::io::ErrorKind::ConnectionReset) => (),
                result => panic!("connection was not retired: {result:?}"),
            }
        });
        let (stream, _) = listener.accept().await.unwrap();
        stream.readable().await.unwrap();
        let descriptor = stream.as_raw_fd();
        let service_calls = calls.clone();
        let service = service_fn(move |_: hyper::Request<hyper::body::Incoming>| {
            service_calls.fetch_add(1, Ordering::SeqCst);
            async { Ok::<_, Infallible>(Response::new(Full::new(Bytes::new()))) }
        });
        let mut connection = Box::pin(
            http1::Builder::new()
                .keep_alive(false)
                .max_headers(16)
                .max_buf_size(8192)
                .writev(false)
                .serve_connection(TokioIo::new(stream), service),
        );
        assert!(matches!(
            connection
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        drop(connection);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        #[cfg(target_os = "linux")]
        assert!(!std::path::Path::new(&format!("/proc/self/fd/{descriptor}")).exists());
        partial.join().unwrap();
        drop(listener);
    });
    peer.join().unwrap();
    drop(runtime);
    drop(listener(address).expect("no retained listener after terminal close"));
}
