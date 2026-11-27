//! Downstream native continuing Action. This exact Binding/token/output code
//! compiles without std or pointer atomics; tests supply real UDP sockets.
use super::{NumberCodec, TestDatagrams, bump, context, failed};
use clinkz_wot_core::{
    BindingGeneration, BindingId, CoreError, CoreResult, Deadline, InteractionOutputMetadata,
    InteractionOutputView, InteractionStatus, Operation, PayloadView,
    data_schema::PreparedSchema,
    execution::{ValueExecution, ValueValidator, action::*, read::LocalConsumerInstance},
};
use core::{
    cell::Cell,
    task::{Context, Poll},
};

#[derive(Default)]
pub struct Statistics {
    pub starts: Cell<usize>,
    pub polls: Cell<usize>,
    pub closes: Cell<usize>,
    pub exchanges_dropped: Cell<usize>,
    pub tokens_dropped: Cell<usize>,
    pub outputs_dropped: Cell<usize>,
    pub inputs_dropped: Cell<usize>,
    pub input_views: Cell<usize>,
    pub close_blocked: Cell<bool>,
    pub close_error: Cell<bool>,
    pub poll_error_with_late: Cell<bool>,
    pub fail_start: Cell<bool>,
    pub query_mapping_changed: Cell<bool>,
    pub output_views: Cell<usize>,
    pub output_variant: Cell<OutputVariant>,
    pub select_after_validation: Cell<Option<OutputVariant>>,
}
/// Adversarial safe-Rust view selection. The original output stays owned;
/// interior mutability can select different immutable bytes or metadata later.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OutputVariant {
    #[default]
    Original,
    Bytes,
    Media,
    Coding,
    Status,
    Metadata,
}
pub struct Program<'a> {
    pub target: &'a str,
}
pub struct Binding<'a, S> {
    pub open: &'a dyn Fn() -> CoreResult<S>,
    pub stats: &'a Statistics,
    pub configuration: u32,
    pub query_media: &'static str,
}
pub struct Token<'a> {
    key: u8,
    stats: &'a Statistics,
}
impl Drop for Token<'_> {
    fn drop(&mut self) {
        bump(&self.stats.tokens_dropped);
    }
}
pub struct Input<'a> {
    pub body: &'a [u8],
    pub stats: &'a Statistics,
}
impl ActionInput for Input<'_> {
    fn view(&self) -> PayloadView<'_> {
        bump(&self.stats.input_views);
        PayloadView {
            body: self.body,
            content_type: "application/x-test-number",
            content_coding: None,
        }
    }
}
impl Drop for Input<'_> {
    fn drop(&mut self) {
        bump(&self.stats.inputs_dropped);
    }
}
pub struct Output<'a> {
    body: [u8; 64],
    len: usize,
    flags: u8,
    media: &'static str,
    metadata: InteractionOutputMetadata,
    alternate_metadata: InteractionOutputMetadata,
    stats: &'a Statistics,
}
impl ActionOutput for Output<'_> {
    fn view(&self) -> InteractionOutputView<'_> {
        bump(&self.stats.output_views);
        let variant = self.stats.output_variant.get();
        InteractionOutputView::new(
            Some(PayloadView {
                body: if variant == OutputVariant::Bytes {
                    b"99"
                } else {
                    &self.body[..self.len]
                },
                content_type: if self.flags == 1 || variant == OutputVariant::Media {
                    "wrong/media"
                } else {
                    self.media
                },
                content_coding: (self.flags == 2 || variant == OutputVariant::Coding)
                    .then_some("unsupported"),
            }),
            if self.flags == 3 || variant == OutputVariant::Status {
                InteractionStatus::Accepted
            } else {
                InteractionStatus::Ok
            },
            if variant == OutputVariant::Metadata {
                &self.alternate_metadata
            } else {
                &self.metadata
            },
        )
    }
}
impl Drop for Output<'_> {
    fn drop(&mut self) {
        bump(&self.stats.outputs_dropped);
    }
}
pub struct Exchange<'a, S> {
    socket: S,
    request: [u8; 64],
    len: usize,
    sent: bool,
    answered: bool,
    late: Option<ActionReply<Token<'a>, Output<'a>>>,
    media: &'static str,
    stats: &'a Statistics,
}
impl<S> Drop for Exchange<'_, S> {
    fn drop(&mut self) {
        bump(&self.stats.exchanges_dropped);
    }
}
impl<'a, S: TestDatagrams> ActionBinding for Binding<'a, S> {
    type Program = Program<'a>;
    type Configuration = u32;
    type Exchange = Exchange<'a, S>;
    type Destination = &'a str;
    fn configuration(&self) -> &u32 {
        &self.configuration
    }
    fn mapping<'b>(&'b self, p: &'b Self::Program, op: Operation) -> Option<ActionMapping<'b>> {
        matches!(
            op,
            Operation::InvokeAction | Operation::QueryAction | Operation::CancelAction
        )
        .then_some(ActionMapping {
            binding: BindingId::new(if op == Operation::InvokeAction {
                71
            } else {
                72 + u32::from(
                    op == Operation::QueryAction && self.stats.query_mapping_changed.get(),
                )
            }),
            generation: BindingGeneration::INITIAL,
            target: p.target,
        })
    }
    fn destination(
        &self,
        p: &Self::Program,
        _: Operation,
        _: Option<&Token<'a>>,
    ) -> CoreResult<Self::Destination> {
        Ok(p.target)
    }
    fn supports(&self, _: &Self::Program, m: ValueExecution<'_>) -> bool {
        m.operation() == Operation::InvokeAction
    }
    fn start(
        &self,
        program: &Self::Program,
        op: Operation,
        destination: &Self::Destination,
        input: Option<PayloadView<'_>>,
        token: Option<&Token<'a>>,
        _: Deadline,
    ) -> CoreResult<Self::Exchange> {
        if self.stats.fail_start.get() || *destination != program.target {
            return Err(failed());
        }
        let mut request = [0; 64];
        let len = match op {
            Operation::InvokeAction => {
                if token.is_some() {
                    return Err(failed());
                }
                let input = input.ok_or_else(failed)?;
                if input.body.len() > 62 {
                    return Err(failed());
                }
                request[0] = b'I';
                request[1..input.body.len() + 1].copy_from_slice(input.body);
                input.body.len() + 1
            }
            Operation::QueryAction | Operation::CancelAction => {
                if input.is_some() {
                    return Err(failed());
                }
                request[0] = if op == Operation::QueryAction {
                    b'Q'
                } else {
                    b'C'
                };
                request[1] = token.ok_or_else(failed)?.key;
                2
            }
            _ => return Err(failed()),
        };
        let socket = (self.open)()?;
        bump(&self.stats.starts);
        Ok(Exchange {
            socket,
            request,
            len,
            sent: false,
            answered: false,
            late: None,
            media: if op == Operation::QueryAction {
                self.query_media
            } else {
                "application/x-test-number"
            },
            stats: self.stats,
        })
    }
}
impl<'a, S: TestDatagrams> Exchange<'a, S> {
    fn receive(&mut self) -> Poll<CoreResult<ActionReply<Token<'a>, Output<'a>>>> {
        let mut bytes = [0; 66];
        let len = match self.socket.receive(&mut bytes) {
            Ok(Some(n)) if n > 0 && n <= bytes.len() => n,
            Ok(None) => return Poll::Pending,
            _ => return Poll::Ready(Err(failed())),
        };
        self.answered = true;
        Poll::Ready(match bytes[0] {
            b'A' if len == 2 => Ok(ActionReply::Accepted(Token {
                key: bytes[1],
                stats: self.stats,
            })),
            b'P' => Ok(ActionReply::Pending),
            b'C' => Ok(ActionReply::CancelRequested),
            b'T' => Ok(ActionReply::AlreadyTerminal),
            b'F' => Ok(ActionReply::Failed(failed())),
            b'O' if len >= 2 => {
                let mut body = [0; 64];
                body[..len - 2].copy_from_slice(&bytes[2..len]);
                Ok(ActionReply::Complete(Output {
                    body,
                    len: len - 2,
                    flags: bytes[1],
                    media: self.media,
                    metadata: InteractionOutputMetadata::default(),
                    alternate_metadata: InteractionOutputMetadata::default()
                        .with_response_selection(clinkz_wot_core::ResponseSelection::Additional(0)),
                    stats: self.stats,
                }))
            }
            _ => Err(failed()),
        })
    }
}
impl<'a, S: TestDatagrams> ActionExchange for Exchange<'a, S> {
    type Continuation = Token<'a>;
    type Output = Output<'a>;
    fn poll(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<CoreResult<ActionReply<Token<'a>, Output<'a>>>> {
        bump(&self.stats.polls);
        if !self.sent {
            if let Err(e) = self.socket.send(&self.request[..self.len]) {
                return Poll::Ready(Err(e));
            }
            self.sent = true;
        }
        let result = self.receive();
        if result.is_ready() && self.stats.poll_error_with_late.replace(false) {
            if let Poll::Ready(Ok(reply)) = result {
                self.late = Some(reply);
            }
            Poll::Ready(Err(failed()))
        } else {
            result
        }
    }
    fn poll_close(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<CoreResult<Option<ActionReply<Token<'a>, Output<'a>>>>> {
        bump(&self.stats.closes);
        if self.stats.close_error.replace(false) {
            return Poll::Ready(Err(failed()));
        }
        if self.stats.close_blocked.get() {
            return Poll::Pending;
        }
        if !self.sent {
            return Poll::Ready(Ok(None));
        }
        if let Some(late) = self.late.take() {
            return Poll::Ready(Ok(Some(late)));
        }
        if !self.answered {
            self.receive().map(|r| r.map(Some))
        } else {
            Poll::Ready(Ok(None))
        }
    }
}
/// Separate per-operation representations over Core's same schema evaluator.
pub struct Codec<'a> {
    pub number: NumberCodec<'a>,
    pub media: &'static str,
    pub revoke_on_validate: Cell<bool>,
    pub authority: &'a LocalConsumerInstance,
}
impl ValueValidator for Codec<'_> {
    fn content_type(&self) -> &str {
        self.media
    }
    fn supports(&self, s: &PreparedSchema) -> bool {
        self.number.supports(s)
    }
    fn body_bytes(&self) -> usize {
        self.number.body_bytes()
    }
    fn available(&self) -> bool {
        self.number.available()
    }
    fn validate(&self, s: &PreparedSchema, p: PayloadView<'_>) -> CoreResult<()> {
        if self.revoke_on_validate.replace(false) {
            self.authority.revoke();
        }
        self.number.validate(s, p)
    }
}
#[derive(Default)]
pub struct Permission<'a> {
    pub revision: Cell<u64>,
    pub checks: Cell<usize>,
    pub denied: Cell<bool>,
    pub rotate_on_check: Cell<bool>,
    pub revoke: Option<&'a LocalConsumerInstance>,
    pub callback: Option<&'a dyn Fn()>,
}
impl ActionAccess for Permission<'_> {
    fn revision(&self) -> u64 {
        self.revision.get()
    }
    fn authorize(
        &self,
        _: clinkz_wot_core::execution::read::LocalConsumerRef<'_>,
        _: Operation,
        _: ActionMapping<'_>,
    ) -> CoreResult<()> {
        bump(&self.checks);
        if let Some(c) = self.callback {
            c();
        }
        if let Some(consumer) = self.revoke {
            consumer.revoke();
        }
        if self.rotate_on_check.replace(false) {
            self.revision.set(self.revision.get() + 1);
        }
        if self.denied.get() {
            Err(CoreError::Security {
                reason: clinkz_wot_core::SecurityFailureReason::AuthorizationDenied,
                context: context(),
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
