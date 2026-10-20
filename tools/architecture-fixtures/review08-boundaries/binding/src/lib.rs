//! Non-production native datagram sketch. ONE socket, at most two Producer routes.
//! No handler capability, compiler dependency in runtime builds, or worker thread.
#[cfg(feature = "prepare")]
pub mod prepare;

use review08_core::*;
use std::{
    cell::{Cell, RefCell},
    io,
    net::{SocketAddr, UdpSocket},
    rc::Rc,
    task::Poll,
};

#[derive(Default)]
pub struct Counts {
    pub starts: Cell<usize>,
    pub compiles: Cell<usize>,
    pub program_drops: Cell<usize>,
    pub opens: Cell<usize>,
    pub closes: Cell<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MappingPolicy {
    TextBoolean,
    DigitBoolean,
}

/// Owned, non-Copy, source-free native program. No credentials or instance id.
pub struct Program {
    prefix: Box<str>,
    policy: MappingPolicy,
    counts: Rc<Counts>,
}
impl Drop for Program {
    fn drop(&mut self) {
        self.counts
            .program_drops
            .set(self.counts.program_drops.get() + 1);
    }
}
pub struct Operands {
    endpoint: Endpoint,
}

struct Native {
    policy: MappingPolicy,
    socket: Option<UdpSocket>,
    routes: usize,
    observations: usize,
    deliveries: usize,
    jobs: usize,
    retiring: bool,
    fail_send: bool,
    counts: Rc<Counts>,
}

/// Physical owner and retirement progress object; Rc count is NOT user count.
#[derive(Clone)]
pub struct NativeOwner(Rc<RefCell<Native>>);
impl NativeOwner {
    pub fn new(counts: Rc<Counts>) -> Self {
        Self::with_policy(counts, MappingPolicy::TextBoolean)
    }
    pub fn with_policy(counts: Rc<Counts>, policy: MappingPolicy) -> Self {
        Self(Rc::new(RefCell::new(Native {
            policy,
            socket: None,
            routes: 0,
            observations: 0,
            deliveries: 0,
            jobs: 0,
            retiring: false,
            fail_send: false,
            counts,
        })))
    }
    /// Pure construction. No route participation or native acquisition yet.
    pub fn route(&self, bound: Bound<Program, Operands>) -> Route {
        Route {
            owner: self.clone(),
            bound,
            gate: None,
            installed: false,
        }
    }
    pub fn address(&self) -> SocketAddr {
        self.0
            .borrow()
            .socket
            .as_ref()
            .unwrap()
            .local_addr()
            .unwrap()
    }
    /// Bounded native I/O may happen before publication. This grants no request.
    pub fn observe(&self) -> io::Result<Option<Observation>> {
        let mut native = self.0.borrow_mut();
        if native.retiring || native.observations == 1 {
            return Ok(None);
        }
        let mut bytes = [0u8; 33];
        let Some(socket) = &native.socket else {
            return Ok(None);
        };
        match socket.recv_from(&mut bytes) {
            Ok((n, peer)) if n <= 32 => {
                let Ok(key) = std::str::from_utf8(&bytes[..n]) else {
                    return Ok(None);
                };
                let endpoint = Endpoint(key.into());
                native.observations += 1;
                Ok(Some(Observation {
                    owner: self.clone(),
                    endpoint,
                    peer,
                }))
            }
            Ok(_) => Ok(None),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }
    /// Simulates a retained SDK callback/job, not an actual background worker.
    pub fn hold_native_job(&self) -> NativeJob {
        let mut native = self.0.borrow_mut();
        assert!(native.socket.is_some() && !native.retiring && native.jobs == 0);
        native.jobs += 1;
        NativeJob(self.clone())
    }
    pub fn fail_next_reply(&self) {
        self.0.borrow_mut().fail_send = true;
    }
    /// Pending keeps this same physical owner addressable. No detached closer.
    pub fn poll_retire(&self) -> Poll<()> {
        let mut native = self.0.borrow_mut();
        native.retiring = true;
        if native.routes + native.observations + native.deliveries + native.jobs != 0 {
            return Poll::Pending;
        }
        if native.socket.take().is_some() {
            native.counts.closes.set(native.counts.closes.get() + 1);
        }
        Poll::Ready(())
    }
    pub fn users(&self) -> (usize, usize, usize, usize) {
        let n = self.0.borrow();
        (n.routes, n.observations, n.deliveries, n.jobs)
    }
}

impl Drop for Native {
    fn drop(&mut self) {
        // This tiny SDK has no asynchronous destructor or native worker tail.
        if self.socket.take().is_some() {
            self.counts.closes.set(self.counts.closes.get() + 1);
        }
    }
}
pub struct NativeJob(NativeOwner);
impl Drop for NativeJob {
    fn drop(&mut self) {
        self.0.0.borrow_mut().jobs -= 1;
    }
}

pub struct Observation {
    owner: NativeOwner,
    endpoint: Endpoint,
    peer: SocketAddr,
}
impl ObservedRead for Observation {
    fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
}
impl Drop for Observation {
    fn drop(&mut self) {
        self.owner.0.borrow_mut().observations -= 1;
    }
}

pub struct Route {
    owner: NativeOwner,
    bound: Bound<Program, Operands>,
    gate: Option<Gate>,
    installed: bool,
}
impl ReadRoute for Route {
    type Observation = Observation;
    type Delivery = Delivery;
    fn endpoint(&self) -> &Endpoint {
        &self.bound.operands.endpoint
    }
    fn install(&mut self, gate: Gate) -> Result<(), &'static str> {
        let mut native = self.owner.0.borrow_mut();
        if self.bound.definition.native.policy != native.policy
            || !self
                .bound
                .operands
                .endpoint
                .0
                .starts_with(self.bound.definition.native.prefix.as_ref())
        {
            return Err("native program/operand/executor mismatch");
        }
        if self.installed || native.retiring || native.routes == 2 {
            return Err("native route capacity/phase");
        }
        if native.socket.is_none() {
            let socket = UdpSocket::bind("127.0.0.1:0").map_err(|_| "bind")?;
            socket.set_nonblocking(true).map_err(|_| "nonblocking")?;
            native.socket = Some(socket);
            native.counts.opens.set(native.counts.opens.get() + 1);
        }
        native.routes += 1;
        self.installed = true;
        self.gate = Some(gate);
        Ok(())
    }
    fn accept(
        &mut self,
        permit: AcceptPermit<'_>,
        input: Observation,
    ) -> Result<Delivery, Observation> {
        if !self.installed
            || self.endpoint() != input.endpoint()
            || !Rc::ptr_eq(&self.owner.0, &input.owner.0)
            || !permit.authorizes(self.gate.as_ref().unwrap())
            || self.owner.0.borrow().deliveries == 2
        {
            return Err(input);
        }
        self.owner.0.borrow_mut().deliveries += 1;
        Ok(Delivery {
            owner: self.owner.clone(),
            definition: self.bound.definition.clone(),
            peer: input.peer,
        })
    }
    fn close(&mut self) {
        if self.installed {
            let mut native = self.owner.0.borrow_mut();
            native.routes -= 1;
            if native.routes == 0 {
                native.retiring = true;
            }
            self.installed = false;
        }
    }
}
impl Drop for Route {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct Delivery {
    owner: NativeOwner,
    definition: Rc<Definition<Program>>,
    peer: SocketAddr,
}
impl ReadDelivery for Delivery {
    fn poll_reply(&mut self, result: bool) -> Result<bool, &'static str> {
        assert_eq!(self.definition.result, ResultShape::Boolean);
        let mut native = self.owner.0.borrow_mut();
        if std::mem::take(&mut native.fail_send) {
            return Err("injected send failure; owner retained");
        }
        let bytes: &[u8] = match (self.definition.native.policy, result) {
            (MappingPolicy::TextBoolean, true) => b"true",
            (MappingPolicy::TextBoolean, false) => b"false",
            (MappingPolicy::DigitBoolean, true) => b"1",
            (MappingPolicy::DigitBoolean, false) => b"0",
        };
        match native
            .socket
            .as_ref()
            .ok_or("lost native owner")?
            .send_to(bytes, self.peer)
        {
            Ok(n) if n == bytes.len() => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(false),
            _ => Err("native send failure; owner retained"),
        }
    }
}
impl Drop for Delivery {
    fn drop(&mut self) {
        self.owner.0.borrow_mut().deliveries -= 1;
    }
}
