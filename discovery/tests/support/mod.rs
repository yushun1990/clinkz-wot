//! Independent wire transcript peers. No ClinkZ Directory/Producer handler.
#![allow(dead_code)]
use clinkz_wot_discovery::http::*;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde_json::{Value, json};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpListener,
    sync::Semaphore,
    task::JoinHandle,
};

pub fn td(id: &str, target: &str) -> Value {
    json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "id":id,
        "title":"independent peer", "security":"n", "securityDefinitions":{"n":{"scheme":"nosec"}},
        "properties":{"level":{"type":"integer","readOnly":true,"forms":[{"href":target}]}}})
}
pub fn reply(body: &str, media: &str, headers: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {media}\r\nContent-Length: {}\r\n{headers}\r\n{body}",
        body.len()
    )
}
pub struct Step {
    pub path: String,
    pub response: String,
}
impl Step {
    pub fn new(path: &str, response: String) -> Self {
        Self {
            path: path.into(),
            response,
        }
    }
}
pub struct Keys {
    pub cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
}
impl Keys {
    pub fn new(name: &str) -> Self {
        let rcgen::CertifiedKey { cert, signing_key } =
            rcgen::generate_simple_self_signed(vec![name.into()]).unwrap();
        Self {
            cert: cert.der().clone(),
            key: PrivatePkcs8KeyDer::from(signing_key.serialize_der()).into(),
        }
    }
    pub fn expired() -> Self {
        let mut params = rcgen::CertificateParams::new(vec!["127.0.0.1".into()]).unwrap();
        params.not_before = rcgen::date_time_ymd(2000, 1, 1);
        params.not_after = rcgen::date_time_ymd(2001, 1, 1);
        let key = rcgen::KeyPair::generate().unwrap();
        let cert = params.self_signed(&key).unwrap();
        Self {
            cert: cert.der().clone(),
            key: PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
        }
    }
    fn server(&self) -> Arc<rustls::ServerConfig> {
        Arc::new(
            rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(vec![self.cert.clone()], self.key.clone_key())
            .unwrap(),
        )
    }
    pub fn transport(&self) -> DiscoveryTransport {
        DiscoveryTransport::tls(vec![self.cert.clone()]).unwrap()
    }
}
pub struct Peer {
    pub addr: SocketAddr,
    secure: bool,
    pub requests: Arc<Mutex<Vec<String>>>,
    received: Arc<Semaphore>,
    handle: JoinHandle<usize>,
}
async fn exchange<S: AsyncRead + AsyncWrite + Unpin>(
    mut socket: S,
    step: Step,
    requests: &Mutex<Vec<String>>,
    received: &Semaphore,
) {
    let mut request = Vec::new();
    while !request.ends_with(b"\r\n\r\n") {
        let mut b = [0];
        let n = socket.read(&mut b).await.unwrap();
        if n == 0 {
            break;
        }
        request.push(b[0]);
        assert!(request.len() < 8192);
    }
    let request = String::from_utf8(request).unwrap();
    assert!(
        request.starts_with(&format!("GET {} HTTP/1.1\r\n", step.path)),
        "{request}"
    );
    requests.lock().unwrap().push(request);
    received.add_permits(1);
    let _ = socket.write_all(step.response.as_bytes()).await;
    // Keep the server side alive, including on partial bodies, until the client
    // physically closes it. A successful finish proves no detached/pool socket.
    let mut scratch = [0; 1024];
    loop {
        match socket.read(&mut scratch).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}
impl Peer {
    pub async fn start(steps: Vec<Step>, keys: Option<&Keys>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::new(Semaphore::new(0));
        let saved = requests.clone();
        let signal = received.clone();
        let tls = keys.map(Keys::server);
        let handle = tokio::spawn(async move {
            let mut releases = 0;
            for step in steps {
                let (socket, _) = listener.accept().await.unwrap();
                if let Some(tls) = &tls {
                    if let Ok(socket) = tokio_rustls::TlsAcceptor::from(tls.clone())
                        .accept(socket)
                        .await
                    {
                        exchange(socket, step, &saved, &signal).await;
                    }
                } else {
                    exchange(socket, step, &saved, &signal).await;
                }
                releases += 1;
            }
            releases
        });
        Self {
            addr,
            secure: keys.is_some(),
            requests,
            received,
            handle,
        }
    }
    pub fn url(&self, path: &str, tls: bool) -> String {
        format!(
            "{}://{}{path}",
            if tls { "https" } else { "http" },
            self.addr
        )
    }
    pub fn endpoint(&self, path: &str, trust: DiscoveryTransport) -> TrustedDiscoveryEndpoint {
        TrustedDiscoveryEndpoint::new(&self.url(path, self.secure), self.addr, trust).unwrap()
    }
    pub async fn wait_request(&self) {
        tokio::time::timeout(Duration::from_secs(2), self.received.acquire())
            .await
            .unwrap()
            .unwrap()
            .forget();
    }
    pub async fn finish(self, expected: usize) {
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), self.handle)
                .await
                .expect("client socket leaked")
                .unwrap(),
            expected
        );
    }
}
pub fn limits() -> DiscoveryLimits {
    DiscoveryLimits {
        page_size: 2,
        request_timeout: Duration::from_secs(2),
        session_timeout: Duration::from_secs(10),
        ..DiscoveryLimits::default()
    }
}
