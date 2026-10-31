#![allow(dead_code)] // Shared by distinct Producer and Consumer integration crates.
use super::*;
use std::{
    fs::{self, File},
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
// Reserve-a-free-port and broker bind must not race another fixture startup.
static START: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A real independently implemented broker process, never a test stand-in.
pub struct Broker {
    child: Option<Child>,
    pub address: SocketAddr,
    dir: PathBuf,
}
impl Broker {
    pub fn start() -> Self {
        let _startup = START.lock().unwrap();
        let address = address();
        let dir = std::env::temp_dir().join(format!(
            "clinkz-mqtt-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let config = dir.join("mosquitto.conf");
        fs::write(&config,format!("listener {} 127.0.0.1\nallow_anonymous true\npersistence false\nlog_dest stderr\nlog_type all\n",address.port())).unwrap();
        let child =
            Command::new(std::env::var_os("MOSQUITTO_BIN").unwrap_or_else(|| "mosquitto".into()))
                .args(["-c"])
                .arg(&config)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::from(File::create(dir.join("broker.log")).unwrap()))
                .spawn()
                .expect(
                    "broker-tests require Mosquitto 2.x on PATH or MOSQUITTO_BIN; no silent skip",
                );
        let mut broker = Self {
            child: Some(child),
            address,
            dir,
        };
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            if TcpStream::connect(address).is_ok() {
                return broker;
            }
            assert!(
                broker.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "broker exited: {}",
                broker.logs()
            );
            assert!(
                Instant::now() < end,
                "broker startup timed out: {}",
                broker.logs()
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    pub fn logs(&self) -> String {
        fs::read_to_string(self.dir.join("broker.log")).unwrap()
    }
    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            c.wait().unwrap();
        }
    }
}
impl Drop for Broker {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("broker {}: {}", self.address, self.logs());
        }
        self.stop();
        let _ = fs::remove_dir_all(&self.dir);
    }
}
pub struct Peer {
    stream: TcpStream,
}
impl Peer {
    pub fn new(broker: &Broker) -> Self {
        let mut p = Self {
            stream: TcpStream::connect(broker.address).unwrap(),
        };
        p.stream
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        p.stream
            .set_write_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let client = format!("peer{}", NEXT.fetch_add(1, Ordering::Relaxed));
        let mut connect = b"\0\x04MQTT\x04\x02\0\0".to_vec();
        string(&mut connect, &client);
        p.send(0x10, &connect);
        assert_eq!(p.packet(), (0x20, vec![0, 0]));
        p
    }
    fn send(&mut self, header: u8, body: &[u8]) {
        let mut frame = vec![header];
        let mut n = body.len();
        loop {
            let mut b = (n % 128) as u8;
            n /= 128;
            if n != 0 {
                b |= 128;
            }
            frame.push(b);
            if n == 0 {
                break;
            }
        }
        frame.extend_from_slice(body);
        self.stream.write_all(&frame).unwrap();
    }
    fn packet(&mut self) -> (u8, Vec<u8>) {
        let mut byte = [0];
        self.stream.read_exact(&mut byte).unwrap();
        let header = byte[0];
        let mut len = 0;
        let mut multiplier = 1;
        loop {
            self.stream.read_exact(&mut byte).unwrap();
            len += usize::from(byte[0] & 127) * multiplier;
            assert!(len < 8192);
            if byte[0] & 128 == 0 {
                break;
            }
            multiplier *= 128;
            assert!(multiplier <= 2097152);
        }
        let mut body = vec![0; len];
        self.stream.read_exact(&mut body).unwrap();
        (header, body)
    }
    pub fn subscribe(&mut self, filter: &str) {
        let mut body = vec![0, 1];
        string(&mut body, filter);
        body.push(0);
        self.send(0x82, &body);
        assert_eq!(self.packet(), (0x90, vec![0, 1, 0]));
    }
    pub fn unsubscribe(&mut self, filter: &str) {
        let mut body = vec![0, 2];
        string(&mut body, filter);
        self.send(0xA2, &body);
        assert_eq!(self.packet(), (0xB0, vec![0, 2]));
    }
    pub fn publish_raw(&mut self, topic: &str, bytes: &[u8], retained: bool) {
        let mut body = Vec::new();
        string(&mut body, topic);
        body.extend_from_slice(bytes);
        self.send(if retained { 0x31 } else { 0x30 }, &body);
    }
    pub fn event(&mut self, topic: &str, value: &[u8]) {
        let (header, body) = self.packet();
        assert_eq!(header, 0x30, "no replay/retain/duplicate/QoS is promised");
        let len = usize::from(u16::from_be_bytes([body[0], body[1]]));
        assert_eq!(&body[2..2 + len], topic.as_bytes());
        assert_eq!(&body[2 + len..], value);
    }
    pub fn absent(&mut self) {
        let mut byte = [0];
        let error = self
            .stream
            .read(&mut byte)
            .expect_err("unexpected Event or closure");
        assert!(matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
        ));
    }
}
fn string(body: &mut Vec<u8>, value: &str) {
    body.extend_from_slice(&(value.len() as u16).to_be_bytes());
    body.extend_from_slice(value.as_bytes());
}
pub fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
pub async fn publish(owner: &mut RawProducerServient, exposure: &mut ComposedProducerExposure) {
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        let _ = native(exposure)
            .step(owner, &mut cx(), &mut budget())
            .unwrap();
        match exposure.publish(owner, &mut budget()) {
            Ok(()) => return,
            Err(CoreError::Backpressure(_)) => {}
            Err(e) => panic!("publish failed: {e:?}"),
        }
        assert!(
            Instant::now() < end,
            "publication readiness timed out: {:?}",
            native(exposure).first_failure()
        );
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
pub async fn finish<P: NativeNotificationPublication>(
    emission: &mut ProducerNotificationPublication<P>,
) {
    let end = Instant::now() + Duration::from_secs(3);
    while emission.step(&mut cx(), &mut budget(), now()).is_pending() {
        assert!(Instant::now() < end, "emission timed out");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
pub fn close(owner: &mut RawProducerServient, exposure: &mut ComposedProducerExposure) {
    exposure.begin_close();
    assert!(
        native(exposure)
            .step(owner, &mut cx(), &mut budget())
            .unwrap()
            .is_ready()
    );
    exposure.retire(owner, &mut budget()).unwrap();
}
