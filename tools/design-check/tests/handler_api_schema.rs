#![allow(dead_code)]

use core::future::{Future, Ready};
use core::pin::Pin;
use core::task::{Context, Poll};
use std::rc::Rc;

type CoreResult<T> = Result<T, ()>;

#[derive(Clone, Copy)]
struct HandlerContext<'a>(&'a ());

struct InteractionInput;
struct InteractionOutput;
struct SubscriptionAcceptance;
struct WorkBudget;
struct HandlerSlotId;
struct HandlerFootprint;
struct ExposedThingHandle;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
enum PendingWorkClass {
    BindingInput = 1 << 0,
    ResponseDelivery = 1 << 1,
    OutboundRequest = 1 << 2,
    SubscriptionData = 1 << 3,
    Timer = 1 << 4,
    Cleanup = 1 << 5,
    EmissionFanOut = 1 << 6,
    BindingPublication = 1 << 7,
    SubscriptionCancellation = 1 << 8,
    RouteReadiness = 1 << 9,
    RouteCleanup = 1 << 10,
    HandlerCall = 1 << 11,
    ProducerSubscriptionSetup = 1 << 12,
    ProducerSubscriptionTeardown = 1 << 13,
}

enum HandlerStep<R> {
    Pending,
    Ready(CoreResult<R>),
}

macro_rules! handler_family {
    ($sync:ident, $async:ident, $step:ident, $result:ty) => {
        trait $sync {
            fn handle(
                &self,
                context: HandlerContext<'_>,
                input: &InteractionInput,
            ) -> CoreResult<$result>;
        }

        trait $async {
            type Future<'a>: Future<Output = CoreResult<$result>> + 'a
            where
                Self: 'a;

            fn handle<'a>(
                &'a self,
                context: HandlerContext<'a>,
                input: &'a InteractionInput,
            ) -> Self::Future<'a>;
        }

        trait $step {
            type State;

            fn start(
                &self,
                context: HandlerContext<'_>,
                input: &InteractionInput,
                budget: &mut WorkBudget,
            ) -> CoreResult<Self::State>;

            fn step(
                &self,
                context: HandlerContext<'_>,
                input: &InteractionInput,
                state: &mut Self::State,
                budget: &mut WorkBudget,
            ) -> HandlerStep<$result>;

            fn cancel(
                &self,
                context: HandlerContext<'_>,
                input: &InteractionInput,
                state: &mut Self::State,
                budget: &mut WorkBudget,
            ) -> HandlerStep<()>;
        }
    };
}

handler_family!(
    ReadPropertyHandler,
    AsyncReadPropertyHandler,
    StepReadPropertyHandler,
    InteractionOutput
);
handler_family!(
    WritePropertyHandler,
    AsyncWritePropertyHandler,
    StepWritePropertyHandler,
    InteractionOutput
);
handler_family!(
    ObservePropertyHandler,
    AsyncObservePropertyHandler,
    StepObservePropertyHandler,
    SubscriptionAcceptance
);
handler_family!(
    UnobservePropertyHandler,
    AsyncUnobservePropertyHandler,
    StepUnobservePropertyHandler,
    InteractionOutput
);
handler_family!(
    InvokeActionHandler,
    AsyncInvokeActionHandler,
    StepInvokeActionHandler,
    InteractionOutput
);
handler_family!(
    QueryActionHandler,
    AsyncQueryActionHandler,
    StepQueryActionHandler,
    InteractionOutput
);
handler_family!(
    CancelActionHandler,
    AsyncCancelActionHandler,
    StepCancelActionHandler,
    InteractionOutput
);
handler_family!(
    SubscribeEventHandler,
    AsyncSubscribeEventHandler,
    StepSubscribeEventHandler,
    SubscriptionAcceptance
);
handler_family!(
    UnsubscribeEventHandler,
    AsyncUnsubscribeEventHandler,
    StepUnsubscribeEventHandler,
    InteractionOutput
);
handler_family!(
    ReadAllPropertiesHandler,
    AsyncReadAllPropertiesHandler,
    StepReadAllPropertiesHandler,
    InteractionOutput
);
handler_family!(
    WriteAllPropertiesHandler,
    AsyncWriteAllPropertiesHandler,
    StepWriteAllPropertiesHandler,
    InteractionOutput
);
handler_family!(
    ReadMultiplePropertiesHandler,
    AsyncReadMultiplePropertiesHandler,
    StepReadMultiplePropertiesHandler,
    InteractionOutput
);
handler_family!(
    WriteMultiplePropertiesHandler,
    AsyncWriteMultiplePropertiesHandler,
    StepWriteMultiplePropertiesHandler,
    InteractionOutput
);
handler_family!(
    ObserveAllPropertiesHandler,
    AsyncObserveAllPropertiesHandler,
    StepObserveAllPropertiesHandler,
    SubscriptionAcceptance
);
handler_family!(
    UnobserveAllPropertiesHandler,
    AsyncUnobserveAllPropertiesHandler,
    StepUnobserveAllPropertiesHandler,
    InteractionOutput
);
handler_family!(
    QueryAllActionsHandler,
    AsyncQueryAllActionsHandler,
    StepQueryAllActionsHandler,
    InteractionOutput
);
handler_family!(
    SubscribeAllEventsHandler,
    AsyncSubscribeAllEventsHandler,
    StepSubscribeAllEventsHandler,
    SubscriptionAcceptance
);
handler_family!(
    UnsubscribeAllEventsHandler,
    AsyncUnsubscribeAllEventsHandler,
    StepUnsubscribeAllEventsHandler,
    InteractionOutput
);

struct StaticHandlerRegistration<'h, H> {
    slot_id: HandlerSlotId,
    handler: &'h H,
}

type HostHandlerFuture<'a, R> = Pin<Box<dyn Future<Output = CoreResult<R>> + Send + 'a>>;

struct PortableNonSendHandler;

struct NonSendFuture {
    marker: Rc<()>,
}

impl Future for NonSendFuture {
    type Output = CoreResult<InteractionOutput>;

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(Ok(InteractionOutput))
    }
}

impl AsyncReadPropertyHandler for PortableNonSendHandler {
    type Future<'a> = NonSendFuture;

    fn handle<'a>(
        &'a self,
        _context: HandlerContext<'a>,
        _input: &'a InteractionInput,
    ) -> Self::Future<'a> {
        NonSendFuture {
            marker: Rc::new(()),
        }
    }
}

struct HostSendHandler;

impl AsyncReadPropertyHandler for HostSendHandler {
    type Future<'a> = Ready<CoreResult<InteractionOutput>>;

    fn handle<'a>(
        &'a self,
        _context: HandlerContext<'a>,
        _input: &'a InteractionInput,
    ) -> Self::Future<'a> {
        core::future::ready(Ok(InteractionOutput))
    }
}

fn erase_host_future<'a, H>(
    handler: &'a H,
    context: HandlerContext<'a>,
    input: &'a InteractionInput,
) -> HostHandlerFuture<'a, InteractionOutput>
where
    H: AsyncReadPropertyHandler,
    H::Future<'a>: Send,
{
    Box::pin(handler.handle(context, input))
}

impl ExposedThingHandle {
    fn set_async_read_property_handler<H>(
        &self,
        _name: &str,
        _handler: H,
        _footprint: HandlerFootprint,
    ) -> CoreResult<HandlerSlotId>
    where
        H: AsyncReadPropertyHandler + Send + Sync + 'static,
        for<'a> <H as AsyncReadPropertyHandler>::Future<'a>: Send,
    {
        Ok(HandlerSlotId)
    }
}

struct ObjectSafeSyncHandler;

impl ReadPropertyHandler for ObjectSafeSyncHandler {
    fn handle(
        &self,
        _context: HandlerContext<'_>,
        _input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        Ok(InteractionOutput)
    }
}

#[test]
fn portable_async_does_not_require_send_but_host_erasure_does() {
    fn accepts_portable<H: AsyncReadPropertyHandler>() {}

    accepts_portable::<PortableNonSendHandler>();
    let identity = ();
    let context = HandlerContext(&identity);
    let input = InteractionInput;
    let _future = erase_host_future(&HostSendHandler, context, &input);
    assert!(
        ExposedThingHandle
            .set_async_read_property_handler("temperature", HostSendHandler, HandlerFootprint)
            .is_ok()
    );
}

#[test]
fn pending_work_class_discriminants_are_exact() {
    assert_eq!(PendingWorkClass::HandlerCall as u16, 1 << 11);
    assert_eq!(PendingWorkClass::ProducerSubscriptionSetup as u16, 1 << 12);
    assert_eq!(
        PendingWorkClass::ProducerSubscriptionTeardown as u16,
        1 << 13,
    );
}

#[test]
fn synchronous_trait_is_object_safe() {
    let handler = ObjectSafeSyncHandler;
    let erased: &dyn ReadPropertyHandler = &handler;
    let identity = ();
    let input = InteractionInput;
    assert!(erased.handle(HandlerContext(&identity), &input).is_ok());
}

#[test]
fn static_registration_places_no_bound_on_handler_type() {
    struct Unconstrained;

    let handler = Unconstrained;
    let registration = StaticHandlerRegistration {
        slot_id: HandlerSlotId,
        handler: &handler,
    };
    let _ = registration.handler;
}
