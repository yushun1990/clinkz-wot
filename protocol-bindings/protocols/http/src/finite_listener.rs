//! Optional native infrastructure for independent finite HTTP participants.
//! Routing and bounded buffering stay here; no logical publication or behavior
//! handle enters this owner.
use crate::{HttpProducerConfig, HttpProducerProgram, producer_transport::NativeConnection};
use clinkz_wot_core::{AffordanceTarget, CoreResult, Deadline};
use clinkz_wot_foundation::MonotonicInstant;
use std::{
    collections::VecDeque,
    io,
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    task::Context,
};

const CAPACITY: usize = 32;
type Mappings = Arc<[(AffordanceTarget, HttpProducerProgram)]>;

/// A caller-provisioned, Binding-owned cleartext listener. Construction is pure;
/// the first participant's funded start binds the socket. Clones share only
/// native infrastructure and confer no publication or whole-service shutdown.
/// The last native participation releases the socket even if handles remain.
#[derive(Clone)]
pub struct HttpFiniteListener(Arc<Mutex<Listener>>);

impl PartialEq for HttpFiniteListener {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for HttpFiniteListener {}
impl std::fmt::Debug for HttpFiniteListener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpFiniteListener").finish_non_exhaustive()
    }
}

struct Listener {
    config: HttpProducerConfig,
    timeout: u64,
    socket: Option<TcpListener>,
    next: u64,
    leases: Vec<Routes>,
    pending: VecDeque<Head>,
    last_now: Option<MonotonicInstant>,
    failed: bool,
}
struct Routes {
    id: u64,
    mappings: Mappings,
    queued: Option<SharedConnection>,
    busy: bool,
}
struct Head {
    stream: TcpStream,
    deadline: Deadline,
    // A later route registration can never receive this original input.
    last_lease: u64,
}
pub(super) struct SharedConnection {
    pub connection: NativeConnection,
    pub deadline: Deadline,
}

impl HttpFiniteListener {
    pub fn new(config: HttpProducerConfig, request_timeout_ticks: u64) -> CoreResult<Self> {
        if config.secure || request_timeout_ticks == 0 {
            return Err(crate::producer::producer_unsupported());
        }
        Ok(Self(Arc::new(Mutex::new(Listener {
            config,
            timeout: request_timeout_ticks,
            socket: None,
            next: 0,
            leases: Vec::with_capacity(CAPACITY),
            pending: VecDeque::with_capacity(CAPACITY),
            last_now: None,
            failed: false,
        }))))
    }

    pub(super) fn compatible(&self, config: HttpProducerConfig, timeout: u64) -> bool {
        let owner = self.0.lock().unwrap();
        owner.config == config && owner.timeout == timeout
    }

    pub(super) fn bound_address(&self) -> Option<SocketAddr> {
        let owner = self.0.lock().unwrap();
        if owner.failed {
            return None;
        }
        owner.socket.as_ref()?.local_addr().ok()
    }

    pub(super) fn available(&self) -> bool {
        !self.0.lock().unwrap().failed
    }

    pub(super) fn acquire(&self, mappings: Mappings) -> CoreResult<u64> {
        let mut owner = self.0.lock().unwrap();
        // Reserve the whole route set atomically. Identity, Property name and
        // behavior do not weaken HTTP's native method/path collision rule.
        if owner.failed
            || owner.leases.iter().map(|r| r.mappings.len()).sum::<usize>() + mappings.len()
                > CAPACITY
            || owner.leases.iter().any(|routes| {
                routes.mappings.iter().any(|(_, old)| {
                    mappings.iter().any(|(_, new)| {
                        old.method() == new.method() && old.path_and_query() == new.path_and_query()
                    })
                })
            })
        {
            return Err(crate::producer::producer_unsupported());
        }
        let next = owner
            .next
            .checked_add(1)
            .ok_or_else(crate::producer::producer_unsupported)?;
        if owner.socket.is_none() {
            let address = owner.config.listen_address();
            let bind = || -> io::Result<TcpListener> {
                let socket = socket2::Socket::new(
                    socket2::Domain::for_address(address),
                    socket2::Type::STREAM,
                    Some(socket2::Protocol::TCP),
                )?;
                socket.set_reuse_address(true)?;
                socket.bind(&address.into())?;
                socket.listen(128)?;
                socket.set_nonblocking(true)?;
                Ok(socket.into())
            };
            owner.socket = Some(bind().map_err(|_| crate::producer::producer_unsupported())?);
        }
        owner.next = next;
        owner.leases.push(Routes {
            id: next,
            mappings,
            queued: None,
            busy: false,
        });
        Ok(next)
    }

    pub(super) fn finish(&self, id: u64) {
        let mut owner = self.0.lock().unwrap();
        if let Some(routes) = owner.leases.iter_mut().find(|r| r.id == id) {
            routes.busy = false;
        }
    }

    pub(super) fn release(&self, id: u64) {
        let mut owner = self.0.lock().unwrap();
        owner.leases.retain(|r| r.id != id);
        if owner.leases.is_empty() {
            owner.pending.clear();
            owner.socket = None;
            owner.last_now = None;
        }
    }

    /// One accept and one round-robin head inspection per paid participant
    /// receive. Unclassified sockets retain no Thing/configuration backing.
    pub(super) fn receive(
        &self,
        id: u64,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> CoreResult<Option<SharedConnection>> {
        let mut owner = self.0.lock().unwrap();
        if owner
            .last_now
            .is_some_and(|last| now.checked_duration_since(last).is_none())
        {
            owner.failed = true;
        }
        owner.last_now = Some(now);
        if owner.failed || !owner.leases.iter().any(|r| r.id == id) {
            return Err(crate::producer::producer_unsupported());
        }
        // Closed/unpolled participants cannot retain a queued socket forever.
        for routes in &mut owner.leases {
            if routes
                .queued
                .as_ref()
                .is_some_and(|q| q.deadline.checked_is_elapsed_at(now) != Some(false))
            {
                routes.queued = None;
                routes.busy = false;
            }
        }
        if owner.pending.len() < CAPACITY {
            match owner.socket.as_ref().unwrap().accept() {
                Ok((stream, _)) => {
                    let Some(end) = now.checked_add_ticks(owner.timeout) else {
                        owner.failed = true;
                        return Err(crate::producer::producer_unsupported());
                    };
                    if stream.set_nonblocking(true).is_ok() {
                        let last_lease = owner.next;
                        owner.pending.push_back(Head {
                            stream,
                            deadline: Deadline::at(end),
                            last_lease,
                        });
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(_) => {
                    owner.failed = true;
                    return Err(crate::producer::producer_unsupported());
                }
            }
        }
        if let Some(head) = owner.pending.pop_front()
            && head.deadline.checked_is_elapsed_at(now) == Some(false)
        {
            let mut bytes = [0; 8192];
            let mut fields = [httparse::EMPTY_HEADER; 16];
            let mut request = httparse::Request::new(&mut fields);
            match head.stream.peek(&mut bytes) {
                Ok(0) => {}
                Ok(n) => match request.parse(&bytes[..n]) {
                    Ok(httparse::Status::Partial) if n < bytes.len() => {
                        owner.pending.push_back(head)
                    }
                    Ok(httparse::Status::Complete(_)) => {
                        if let Some(routes) = owner.leases.iter_mut().find(|routes| {
                            routes.id <= head.last_lease
                                && routes.mappings.iter().any(|(_, p)| {
                                    request.method == Some(p.method())
                                        && request.path == Some(p.path_and_query())
                                })
                        }) && !routes.busy
                        {
                            routes.queued = Some(SharedConnection {
                                connection: NativeConnection::from_programs(
                                    head.stream,
                                    routes.mappings.iter().map(|(_, p)| p.clone()).collect(),
                                ),
                                deadline: head.deadline,
                            });
                            routes.busy = true;
                        }
                    }
                    _ => {}
                },
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => owner.pending.push_back(head),
                Err(_) => {}
            }
            zeroize::Zeroize::zeroize(&mut bytes);
        }
        // The ordinary native parser still validates version, Host, framing,
        // credentials and body. Header peeking only selects the original lease.
        cx.waker().wake_by_ref();
        Ok(owner
            .leases
            .iter_mut()
            .find(|r| r.id == id)
            .unwrap()
            .queued
            .take())
    }
}
