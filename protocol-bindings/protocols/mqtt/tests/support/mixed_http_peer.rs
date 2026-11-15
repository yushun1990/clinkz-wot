//! A scripted HTTP peer with actual sockets, separate from Engine authority.
use std::{
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Default)]
pub struct Observations {
    pub reads: AtomicUsize,
    pub invokes: AtomicUsize,
    pub queries: AtomicUsize,
    pub cancels: AtomicUsize,
    pub closed: AtomicUsize,
    pub hold_reads: AtomicBool,
    stopping: AtomicBool,
}
pub struct HttpPeer {
    pub address: SocketAddr,
    pub observed: Arc<Observations>,
    thread: Option<JoinHandle<()>>,
}
impl HttpPeer {
    pub fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let observed = Arc::new(Observations::default());
        let state = observed.clone();
        let thread = thread::spawn(move || {
            let mut exchanges = Vec::new();
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                if state.stopping.load(Ordering::Acquire) {
                    break;
                }
                let state = state.clone();
                exchanges.push(thread::spawn(move || exchange(stream, &state)));
            }
            for exchange in exchanges {
                exchange.join().unwrap();
            }
        });
        Self {
            address,
            observed,
            thread: Some(thread),
        }
    }
}
impl Drop for HttpPeer {
    fn drop(&mut self) {
        self.observed.stopping.store(true, Ordering::Release);
        self.observed.hold_reads.store(false, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        let result = self.thread.take().unwrap().join();
        if !thread::panicking() {
            result.unwrap();
        }
    }
}
fn exchange(mut stream: TcpStream, state: &Observations) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut request = Vec::new();
    let end = loop {
        let mut byte = [0];
        if stream.read(&mut byte).unwrap() == 0 {
            return;
        }
        request.push(byte[0]);
        assert!(request.len() <= 8192);
        if request.ends_with(b"\r\n\r\n") {
            break request.len();
        }
    };
    let headers = std::str::from_utf8(&request).unwrap();
    let length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    let line = headers.lines().next().unwrap().to_owned();
    request.resize(end + length, 0);
    stream.read_exact(&mut request[end..]).unwrap();
    let (code, body) = if line == "GET /value HTTP/1.1" {
        state.reads.fetch_add(1, Ordering::Release);
        stream.set_nonblocking(true).unwrap();
        while state.hold_reads.load(Ordering::Acquire) && !state.stopping.load(Ordering::Acquire) {
            match stream.peek(&mut [0]) {
                Ok(0) => {
                    state.closed.fetch_add(1, Ordering::Release);
                    return;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                other => panic!("unexpected data while holding response: {other:?}"),
            }
            thread::sleep(Duration::from_millis(1));
        }
        stream.set_nonblocking(false).unwrap();
        (200, "7")
    } else if line == "POST /run HTTP/1.1" {
        state.invokes.fetch_add(1, Ordering::Release);
        (202, r#"{"status":"accepted","id":"0000000000000001-1"}"#)
    } else if line == "GET /status?id=0000000000000001-1 HTTP/1.1" {
        state.queries.fetch_add(1, Ordering::Release);
        (200, r#"{"status":"completed","output":24}"#)
    } else if line == "DELETE /stop?id=0000000000000001-1 HTTP/1.1" {
        state.cancels.fetch_add(1, Ordering::Release);
        (200, r#"{"status":"cancelling"}"#)
    } else {
        panic!("unexpected request: {line}")
    };
    let response = format!(
        "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    // A cancelled client may have closed before this original response arrives.
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.shutdown(Shutdown::Write);
    match stream.read(&mut [0]) {
        Ok(0) => {}
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {}
        other => panic!("client did not physically retire its socket: {other:?}"),
    }
    state.closed.fetch_add(1, Ordering::Release);
}
