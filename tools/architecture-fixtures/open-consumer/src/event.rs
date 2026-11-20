//! Two independent test transports for a dual-listening Event source. The stream
//! uses one-byte frames and a connection per subscriber. The datagram protocol
//! registers the recipient's ephemeral port and can hex-code occurrence bytes.
use clinkz_wot_core::{
    consumer::{busy, invalid},
    notification::*,
    *,
};
use std::{
    cell::Cell,
    convert::Infallible,
    future::Future,
    net::SocketAddr,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, ready},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpStream, UdpSocket},
};

#[cfg(feature = "prepare")]
mod prepare;

pub enum SelectedEvent {
    Stream(consumer::ConsumerNotificationDefinition<Stream>),
    Datagram(consumer::ConsumerNotificationDefinition<Datagram>),
}

pub fn alarm() -> AffordanceTarget {
    AffordanceTarget::Event(Arc::from("alarm"))
}

fn occurrence(
    id: &NotificationIdentity,
    digit: u8,
    allowance: usize,
) -> CoreResult<NotificationSignal<Infallible>> {
    let target = alarm();
    let data = Payload::new([digit].as_slice(), "application/json");
    if notification_item_backing(&target, Some(&data)).is_none_or(|n| n > allowance) {
        return Err(busy());
    }
    Ok(NotificationSignal::Data(NativeNotification {
        recipient: id.clone(),
        target,
        data: Some(data),
        response: None,
    }))
}

#[derive(Clone)]
pub struct Stream {
    pub peer: SocketAddr,
    pub compilations: Rc<Cell<usize>>,
}
pub struct StreamSubscription {
    recipient: NotificationIdentity,
    connect: Pin<Box<dyn Future<Output = std::io::Result<TcpStream>>>>,
    stream: Option<TcpStream>,
}
impl ConsumerNotificationBinding for Stream {
    type Configuration = SocketAddr;
    type Program = SocketAddr;
    type Participation = StreamSubscription;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(71), BindingGeneration::INITIAL)
    }
    fn configuration(&self) -> &SocketAddr {
        &self.peer
    }
    fn support(&self, p: &SocketAddr, kind: NotificationKind) -> CoreResult<NotificationSupport> {
        if *p != self.peer || kind != NotificationKind::Event {
            return Err(invalid());
        }
        Ok(NotificationSupport {
            event_responses: false,
            cancellation: true,
        })
    }
    fn prepare(
        &mut self,
        peer: SocketAddr,
        recipient: &NotificationIdentity,
    ) -> Result<StreamSubscription, BindingInputRejection<SocketAddr>> {
        if let Err(error) = self.support(&peer, NotificationKind::Event) {
            return Err(BindingInputRejection::new(
                peer,
                BindingOperationalError::new(error),
            ));
        }
        Ok(StreamSubscription {
            recipient: recipient.clone(),
            connect: Box::pin(TcpStream::connect(peer)),
            stream: None,
        })
    }
}
impl NotificationParticipation for StreamSubscription {
    type Response = Infallible;
    fn poll_start(&mut self, cx: &mut Context<'_>, _: &SubscriptionInput) -> Poll<CoreResult<()>> {
        if self.stream.is_none() {
            self.stream = Some(ready!(self.connect.as_mut().poll(cx)).map_err(|_| invalid())?);
        }
        let stream = self.stream.as_mut().unwrap();
        match ready!(Pin::new(stream).poll_write(cx, b"+")) {
            Ok(1) => Poll::Ready(Ok(())),
            _ => Poll::Ready(Err(invalid())),
        }
    }
    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
        max: usize,
    ) -> Poll<CoreResult<NotificationSignal<Infallible>>> {
        let mut byte = [0];
        let mut buf = ReadBuf::new(&mut byte);
        ready!(Pin::new(self.stream.as_mut().unwrap()).poll_read(cx, &mut buf))
            .map_err(|_| invalid())?;
        let [digit] = buf.filled() else {
            return Poll::Ready(Err(invalid()));
        };
        Poll::Ready(occurrence(&self.recipient, *digit, max))
    }
    fn poll_close(
        &mut self,
        cx: &mut Context<'_>,
        cancel: Option<&CancellationInput>,
    ) -> Poll<CoreResult<RemoteCancellation>> {
        let outcome = if let (Some(stream), Some(_)) = (&mut self.stream, cancel) {
            if !matches!(ready!(Pin::new(stream).poll_write(cx, b"-")), Ok(1)) {
                return Poll::Ready(Err(invalid()));
            }
            RemoteCancellation::Unknown
        } else {
            RemoteCancellation::NotAttempted
        };
        self.stream.take();
        Poll::Ready(Ok(outcome))
    }
    fn abort_local(self) {}
}

#[derive(Clone, PartialEq, Eq)]
pub struct DatagramConfig {
    pub peer: SocketAddr,
    pub require_hex: bool,
}
#[derive(Clone)]
pub struct Datagram {
    pub config: DatagramConfig,
    pub compilations: Rc<Cell<usize>>,
}
#[derive(Clone)]
pub struct DatagramProgram {
    pub peer: SocketAddr,
    pub hex: bool,
}
pub struct DatagramSubscription {
    program: DatagramProgram,
    recipient: NotificationIdentity,
    socket: Option<UdpSocket>,
}
impl ConsumerNotificationBinding for Datagram {
    type Configuration = DatagramConfig;
    type Program = DatagramProgram;
    type Participation = DatagramSubscription;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(72), BindingGeneration::INITIAL)
    }
    fn configuration(&self) -> &DatagramConfig {
        &self.config
    }
    fn support(
        &self,
        p: &DatagramProgram,
        kind: NotificationKind,
    ) -> CoreResult<NotificationSupport> {
        if kind != NotificationKind::Event
            || p.peer != self.config.peer
            || self.config.require_hex && !p.hex
        {
            return Err(CoreError::UnsupportedOperation(consumer::context()));
        }
        Ok(NotificationSupport {
            event_responses: false,
            cancellation: true,
        })
    }
    fn prepare(
        &mut self,
        program: DatagramProgram,
        recipient: &NotificationIdentity,
    ) -> Result<DatagramSubscription, BindingInputRejection<DatagramProgram>> {
        if let Err(error) = self.support(&program, NotificationKind::Event) {
            return Err(BindingInputRejection::new(
                program,
                BindingOperationalError::new(error),
            ));
        }
        Ok(DatagramSubscription {
            program,
            recipient: recipient.clone(),
            socket: None,
        })
    }
}
impl NotificationParticipation for DatagramSubscription {
    type Response = Infallible;
    fn poll_start(&mut self, cx: &mut Context<'_>, _: &SubscriptionInput) -> Poll<CoreResult<()>> {
        if self.socket.is_none() {
            let socket = std::net::UdpSocket::bind("127.0.0.1:0").map_err(|_| busy())?;
            socket.connect(self.program.peer).map_err(|_| invalid())?;
            socket.set_nonblocking(true).map_err(|_| invalid())?;
            self.socket = Some(UdpSocket::from_std(socket).map_err(|_| busy())?);
        }
        ready!(self.socket.as_ref().unwrap().poll_send(cx, b"LISTEN")).map_err(|_| invalid())?;
        Poll::Ready(Ok(()))
    }
    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
        max: usize,
    ) -> Poll<CoreResult<NotificationSignal<Infallible>>> {
        // A third byte (including a truncated longer packet) is never admissible.
        let mut bytes = [0; 3];
        let mut buf = ReadBuf::new(&mut bytes);
        ready!(self.socket.as_ref().unwrap().poll_recv(cx, &mut buf)).map_err(|_| invalid())?;
        let digit = match (self.program.hex, buf.filled()) {
            (false, [b]) => *b,
            (true, [hi, lo]) => {
                let nibble = |b: u8| (b as char).to_digit(16).ok_or_else(invalid);
                (nibble(*hi)? * 16 + nibble(*lo)?) as u8
            }
            _ => return Poll::Ready(Err(invalid())),
        };
        Poll::Ready(occurrence(&self.recipient, digit, max))
    }
    fn poll_close(
        &mut self,
        cx: &mut Context<'_>,
        cancel: Option<&CancellationInput>,
    ) -> Poll<CoreResult<RemoteCancellation>> {
        let outcome = if let (Some(socket), Some(_)) = (&self.socket, cancel) {
            ready!(socket.poll_send(cx, b"UNLISTEN")).map_err(|_| invalid())?;
            RemoteCancellation::Unknown
        } else {
            RemoteCancellation::NotAttempted
        };
        self.socket.take();
        Poll::Ready(Ok(outcome))
    }
    fn abort_local(self) {}
}
