//! Downstream test Binding for modern caller-owned Property Read. The same
//! Binding/codec/state owners compile without std or atomics and execute with
//! actual UDP datagrams in tests. This is not a production protocol mapping.
#![no_std]
use clinkz_wot_core::{
    BindingGeneration, BindingId, BindingResponseMetadata, CoreError, CoreResult, Deadline,
    ErrorContext, ErrorPhase, InteractionOutputMetadata, InteractionOutputView, InteractionStatus,
    Operation, PayloadView, PlanId, ResponsePayloadRole, RetryClass,
    data_schema::{NumberValue, PreparedSchema, SchemaViolation, ValueAccess, ValueType},
    execution::{ValueExecution, read::*},
};
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use core::{
    cell::Cell,
    task::{Context, Poll},
};

/// Native interface of this test datagram protocol only. The Engine knows
/// neither this interface nor its datagram format. Drop closes local I/O.
pub trait TestDatagrams {
    fn send(&mut self, bytes: &[u8]) -> CoreResult<()>;
    fn receive(&mut self, buffer: &mut [u8]) -> CoreResult<Option<usize>>;
}
#[derive(Default)]
pub struct Statistics {
    pub starts: Cell<usize>,
    pub polls: Cell<usize>,
    pub cancel_starts: Cell<usize>,
    pub cancel_polls: Cell<usize>,
    pub cleanup_acks: Cell<usize>,
    pub calls_dropped: Cell<usize>,
    pub outputs_dropped: Cell<usize>,
    pub cleanup_error: Cell<bool>,
    pub late: Cell<bool>,
}
fn bump(n: &Cell<usize>) {
    n.set(n.get() + 1);
}
pub struct Program<'a> {
    pub target: &'a str,
    pub plan: PlanId,
}
pub struct TestBinding<'a, S> {
    pub open: &'a dyn Fn() -> CoreResult<S>,
    pub stats: &'a Statistics,
    pub configuration: u32,
}
impl<'a, S: TestDatagrams> ReadBinding for TestBinding<'a, S> {
    type Program = Program<'a>;
    type Configuration = u32;
    type Call = TestCall<'a, S>;
    fn configuration(&self) -> &u32 {
        &self.configuration
    }
    fn mapping<'b>(&'b self, p: &'b Self::Program) -> ReadMapping<'b> {
        ReadMapping {
            binding: BindingId::new(17),
            generation: BindingGeneration::INITIAL,
            plan: p.plan,
            target: p.target,
        }
    }
    fn supports(&self, _: &Self::Program, meaning: ValueExecution<'_>, media: &str) -> bool {
        meaning.operation() == Operation::ReadProperty && media == "application/x-test-number"
    }
    fn backing_bytes(&self) -> u64 {
        0
    } // Entire Engine-visible state/output is inline.
    fn start(&self, p: &Self::Program, _: Deadline) -> CoreResult<Self::Call> {
        let socket = (self.open)()?;
        bump(&self.stats.starts);
        Ok(TestCall {
            socket: Some(socket),
            stats: self.stats,
            sent: false,
            mapping: (BindingId::new(17), BindingGeneration::INITIAL, p.plan),
        })
    }
}
pub struct TestCall<'a, S> {
    socket: Option<S>,
    stats: &'a Statistics,
    sent: bool,
    mapping: (BindingId, BindingGeneration, PlanId),
}
impl<S> Drop for TestCall<'_, S> {
    fn drop(&mut self) {
        bump(&self.stats.calls_dropped);
    }
}
pub struct InlineOutput<'a> {
    body: [u8; 64],
    len: usize,
    flags: u8,
    metadata: InteractionOutputMetadata,
    stats: &'a Statistics,
}
impl Drop for InlineOutput<'_> {
    fn drop(&mut self) {
        bump(&self.stats.outputs_dropped);
    }
}
impl ReadOutput for InlineOutput<'_> {
    fn view(&self) -> InteractionOutputView<'_> {
        InteractionOutputView::new(
            (self.flags != 4).then_some(PayloadView {
                body: &self.body[..self.len],
                content_type: if self.flags == 3 {
                    "wrong/media"
                } else {
                    "application/x-test-number"
                },
                content_coding: (self.flags == 6).then_some("unsupported"),
            }),
            if self.flags == 2 {
                InteractionStatus::Accepted
            } else {
                InteractionStatus::Ok
            },
            &self.metadata,
        )
    }
}
fn native_credit(b: &mut WorkBudget) {
    for class in WorkClass::ALL {
        assert_eq!(
            b.remaining(class),
            if class == WorkClass::BindingPolls {
                1
            } else {
                0
            }
        );
    }
    b.consume(WorkClass::BindingPolls, 1).unwrap();
}
impl<'a, S: TestDatagrams> TestCall<'a, S> {
    fn receive(&mut self) -> Poll<CoreResult<InlineOutput<'a>>> {
        let mut wire = [0; 65];
        let length = match self.socket.as_mut().unwrap().receive(&mut wire) {
            Ok(None) => return Poll::Pending,
            Ok(Some(n)) if n > 0 && n <= wire.len() => n,
            _ => return Poll::Ready(Err(failed())),
        };
        let flags = wire[0];
        let (binding, generation, plan) = self.mapping;
        let plan = if flags == 1 {
            PlanId::new(
                clinkz_wot_foundation::SlotIndex::new(plan.slot().get() + 1),
                plan.generation(),
            )
        } else {
            plan
        };
        let mut metadata = InteractionOutputMetadata::default().with_untrusted_binding_response(
            BindingResponseMetadata::primary(binding, generation, plan, 0),
        );
        if flags == 5 {
            metadata = metadata.with_payload_role(ResponsePayloadRole::OperationStatus);
        }
        let mut body = [0; 64];
        body[..length - 1].copy_from_slice(&wire[1..length]);
        Poll::Ready(Ok(InlineOutput {
            body,
            len: length - 1,
            flags,
            metadata,
            stats: self.stats,
        }))
    }
}
impl<'a, S: TestDatagrams> ReadNativeCall for TestCall<'a, S> {
    type Output = InlineOutput<'a>;
    fn poll_result(
        &mut self,
        _: &mut Context<'_>,
        b: &mut WorkBudget,
    ) -> Poll<CoreResult<Self::Output>> {
        native_credit(b);
        bump(&self.stats.polls);
        if !self.sent {
            if self.socket.as_mut().unwrap().send(b"read-value").is_err() {
                return Poll::Ready(Err(failed()));
            }
            self.sent = true;
        }
        self.receive()
    }
    fn start_cancel(
        &mut self,
        _: &CoreError,
        _: &mut Context<'_>,
        b: &mut WorkBudget,
    ) -> CoreResult<Poll<ReadNativeSettlement<Self::Output>>> {
        native_credit(b);
        bump(&self.stats.cancel_starts);
        Ok(Poll::Pending)
    }
    fn poll_cancel(
        &mut self,
        _: &mut Context<'_>,
        b: &mut WorkBudget,
    ) -> Poll<CoreResult<ReadNativeSettlement<Self::Output>>> {
        native_credit(b);
        bump(&self.stats.cancel_polls);
        if self.stats.cleanup_error.replace(false) {
            return Poll::Ready(Err(failed()));
        }
        if self.stats.late.get() {
            return self
                .receive()
                .map(|r| Ok(ReadNativeSettlement::Returned(r)));
        }
        drop(self.socket.take());
        bump(&self.stats.cleanup_acks);
        Poll::Ready(Ok(ReadNativeSettlement::Cancelled))
    }
}

/// This codec only supplies numeric evidence. Core evaluates the same prepared
/// constraints used by Host execution, including mathematically integral decimals.
#[derive(Default)]
pub struct NumberCodec<'a> {
    pub validations: Cell<usize>,
    pub available: Cell<bool>,
    pub callback: Option<&'a dyn Fn()>,
}
impl NumberCodec<'_> {
    pub fn ready() -> Self {
        Self {
            available: Cell::new(true),
            ..Self::default()
        }
    }
}
impl ReadValidator for NumberCodec<'_> {
    fn content_type(&self) -> &str {
        "application/x-test-number"
    }
    fn supports(&self, schema: &PreparedSchema) -> bool {
        matches!(
            schema,
            PreparedSchema::Integer { .. } | PreparedSchema::Type(ValueType::Integer)
        )
    }
    fn body_bytes(&self) -> usize {
        32
    }
    fn work(&self) -> u64 {
        32
    }
    fn available(&self) -> bool {
        self.available.get()
    }
    fn validate(&self, schema: &PreparedSchema, p: PayloadView<'_>) -> CoreResult<()> {
        bump(&self.validations);
        if let Some(callback) = self.callback {
            callback();
        }
        let number = core::str::from_utf8(p.body).map_err(|_| failed())?;
        if !number.parse::<f64>().is_ok_and(f64::is_finite) {
            return Err(failed());
        }
        schema
            .validate(&Number(number), ())
            .map_err(|_| CoreError::Validation(context()))
    }
}
struct Number<'a>(&'a str);
impl ValueAccess for Number<'_> {
    type Node = ();
    fn kind(&self, _: ()) -> Result<ValueType, SchemaViolation> {
        Ok(if NumberValue::Decimal(self.0).is_integer() {
            ValueType::Integer
        } else {
            ValueType::Number
        })
    }
    fn number(&self, _: ()) -> Result<NumberValue<'_>, SchemaViolation> {
        Ok(NumberValue::Decimal(self.0))
    }
    fn string_length(&self, _: ()) -> Result<usize, SchemaViolation> {
        Err(SchemaViolation::Value)
    }
    fn property(&self, _: (), _: &str) -> Result<Option<()>, SchemaViolation> {
        Err(SchemaViolation::Value)
    }
}
#[derive(Default)]
pub struct Permission<'a> {
    pub checks: Cell<usize>,
    pub denied: Cell<bool>,
    pub revision: Cell<u64>,
    pub revoke: Option<&'a LocalConsumerInstance>,
    pub rotate_on_check: Cell<bool>,
}
impl ReadAccess for Permission<'_> {
    fn revision(&self) -> u64 {
        self.revision.get()
    }
    fn work(&self) -> u64 {
        1
    }
    fn authorize(&self, _: LocalConsumerRef<'_>, _: ReadMapping<'_>) -> CoreResult<()> {
        bump(&self.checks);
        if let Some(instance) = self.revoke {
            instance.revoke();
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
pub fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
}
pub fn failed() -> CoreError {
    CoreError::Binding(context())
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests;
