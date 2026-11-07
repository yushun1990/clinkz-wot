//! A separate-process loopback peer. No peer allocations enter consumer meters.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering::Relaxed},
    },
    time::Duration,
};

pub fn body(id: usize, members: usize) -> Vec<u8> {
    let mut value = json!({"id": id});
    for n in 1..members {
        value[format!("calibrated_measurement_{n:02}")] = json!(n);
    }
    serde_json::to_vec(&value).unwrap()
}
pub struct Peer {
    child: Child,
    pub address: String,
}
impl Peer {
    pub fn start(members: usize, delay_us: u64) -> Self {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--peer", &members.to_string(), &delay_us.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut address = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        assert!(address.starts_with("127.0.0.1:"), "peer failed to start");
        Self {
            child,
            address: address.trim().to_owned(),
        }
    }
    pub fn stats(&self) -> Value {
        let mut socket = TcpStream::connect(&self.address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        socket
            .write_all(b"GET /stats HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
pub async fn serve(members: usize, delay_us: u64) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    println!("{}", listener.local_addr().unwrap());
    std::io::stdout().flush().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let (requests, active, peak) = (requests.clone(), active.clone(), peak.clone());
        tokio::spawn(async move {
            let mut request = Vec::new();
            loop {
                if socket.readable().await.is_err() {
                    return;
                }
                let mut bytes = [0; 2048];
                match socket.try_read(&mut bytes) {
                    Ok(0) => return,
                    Ok(n) => request.extend_from_slice(&bytes[..n]),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(_) => return,
                }
                assert!(request.len() < 8192);
                if request.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let request = std::str::from_utf8(&request).unwrap();
            let path = request.split_whitespace().nth(1).unwrap();
            let bytes = if path == "/stats" {
                serde_json::to_vec(&json!({"requests": requests.load(Relaxed), "active": active.load(Relaxed), "peak": peak.load(Relaxed)})).unwrap()
            } else {
                assert!(request.starts_with("GET /t/"));
                let id: usize = path.strip_prefix("/t/").unwrap().parse().unwrap();
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains(&format!("x-target: {id}\r\n"))
                );
                requests.fetch_add(1, Relaxed);
                let n = active.fetch_add(1, Relaxed) + 1;
                peak.fetch_max(n, Relaxed);
                if delay_us != 0 {
                    tokio::time::sleep(Duration::from_micros(delay_us)).await;
                }
                body(id, members)
            };
            let mut response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len()).into_bytes();
            response.extend_from_slice(&bytes);
            let mut sent = 0;
            while sent < response.len() {
                if socket.writable().await.is_err() {
                    break;
                }
                match socket.try_write(&response[sent..]) {
                    Ok(0) => break,
                    Ok(n) => sent += n,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(_) => break,
                }
            }
            if path != "/stats" {
                active.fetch_sub(1, Relaxed);
            }
        });
    }
}
