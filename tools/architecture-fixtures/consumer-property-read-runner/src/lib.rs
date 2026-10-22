//! Downstream application entry: only public Consumer registration and Runtime.
#![no_std]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;
use clinkz_wot_consumer_property_read_binding_fixture::StaticClient;
use clinkz_wot_core::*;
use clinkz_wot_foundation::*;
use clinkz_wot_planning::binding_registration::*;
use clinkz_wot_servient::consumer_property_read::*;
use clinkz_wot_td::thing::Thing;
use core::{
    cell::{Cell, RefCell},
    task::{Context, Waker},
};

pub fn work(n: u64) -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
pub fn policy(limits: ResourceLimits) -> ConsumerPropertyReadPolicy {
    checked_policy(BindingResourceContract::StrictV1, limits)
}
fn checked_policy(
    contract: BindingResourceContract,
    limits: ResourceLimits,
) -> ConsumerPropertyReadPolicy {
    let registration = BindingLifetimeFootprint::new(4, 4096);
    let call = BindingLifetimeFootprint::new(8, 1280);
    if contract == BindingResourceContract::StrictV1 {
        ConsumerPropertyReadPolicy::try_new(
            limits,
            registration,
            call,
            [1, 64, 64, 64],
            [64, 32],
            [2, 2, 1, 1, 1, 1, 1],
            [50, 5],
            65536,
        )
    } else {
        ConsumerPropertyReadPolicy::try_new_owned(
            contract,
            ResourceProfileId::GATEWAY_DEFAULT_V1,
            limits,
            registration,
            call,
            [1, 64, 64, 64],
            [64, 32],
            [2, 2, 1, 1, 1, 1, 1],
            [50, 5],
            65536,
        )
    }
    .unwrap()
}
/// Own the Thing, consume it, destroy it, read distinct non-first coordinates,
/// and reclaim. This function is compiled in normal no_std and ARM library IR.
pub fn application(
    thing: Thing,
    client: StaticClient,
    clock: &dyn RuntimeClock,
) -> CoreResult<[u16; 2]> {
    let contract = PollClientBinding::consumer_configuration(&client)
        .map_or(BindingResourceContract::StrictV1, |c| c.resource_contract());
    let policy = checked_policy(contract, GatewayDefaultV1::LIMITS.clone());
    let parent = RefCell::new(policy.parent_accounts());
    let mut report = (
        Cell::new((None, [0; 4])),
        Cell::new(0),
        RefCell::new(ResourceAccount::new(
            SlotIndex::new(50),
            Generation::INITIAL,
            ResourceKind::CleanupBytesMax,
            4096,
        )),
    );
    let registration =
        clinkz_wot_consumer_property_read_binding_fixture::static_registration(client);
    let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        clock,
        &parent,
        &mut report,
    )
    .map_err(|e| e.error().error().clone())?;
    runtime.consume(&thing, &mut work(100000), |b| {
        for _ in 0..100000 {
            b.step(&mut work(4096));
            if b.is_finished() {
                return;
            }
        }
        panic!("bounded fixture build stalled");
    })?;
    drop(thing);
    let mut statuses = [0; 2];
    let mut cx = Context::from_waker(Waker::noop());
    for (index, (name, form)) in [("a-read", 2), ("c-温度", 1)].into_iter().enumerate() {
        let generation = runtime
            .read_property(
                name,
                InteractionOptions::new().with_form_index(form),
                &mut work(100000),
            )
            .map_err(|e| e.error().error().clone())?;
        runtime.step(&mut cx, &mut work(4096));
        let output = runtime
            .try_take(generation, &mut work(4096))?
            .expect("finite result")?;
        statuses[index] = output
            .metadata()
            .binding_response()
            .expect("Core sealed binding result")
            .status_code();
    }
    runtime.close();
    for _ in 0..100000 {
        if let StartStatus::Ready(outcome) = runtime.shutdown(&mut cx, &mut work(4096)) {
            assert!(outcome.is_complete());
            drop(runtime);
            assert!(observer().0.unwrap().is_complete());
            drop(observer);
            assert!(parent.borrow().iter().all(|a| a.used() == 0));
            return Ok(statuses);
        }
    }
    panic!("bounded fixture shutdown stalled");
}

#[cfg(feature = "std")]
pub fn host_application(
    thing: Thing,
    client: clinkz_wot_consumer_property_read_binding_fixture::HostClient,
    clock: alloc::sync::Arc<dyn RuntimeClock + Send + Sync>,
) -> CoreResult<[u16; 2]> {
    let contract = binding::ClientBinding::consumer_configuration(&client)
        .map_or(BindingResourceContract::StrictV1, |c| c.resource_contract());
    let policy = checked_policy(contract, GatewayDefaultV1::LIMITS.clone());
    let parent = alloc::sync::Arc::new(
        clinkz_wot_servient::host_accounts::HostResourceAccounts::try_from_accounts(
            policy.parent_accounts(),
        )
        .unwrap(),
    );
    let registration = clinkz_wot_consumer_property_read_binding_fixture::host_registration(client);
    let (runtime, report) =
        ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
            .map_err(|e| e.error().error().clone())?;
    let handle = match runtime.consume(&thing, &mut work(100000), |b| {
        for _ in 0..100000 {
            b.step(&mut work(4096));
            if b.is_finished() {
                return;
            }
        }
        panic!("bounded fixture build stalled");
    })? {
        StartStatus::Ready(handle) => handle,
        StartStatus::Pending => panic!("final credit supplied"),
    };
    drop(thing);
    let mut statuses = [0; 2];
    let mut cx = Context::from_waker(Waker::noop());
    for (index, (name, form)) in [("a-read", 2), ("c-温度", 1)].into_iter().enumerate() {
        let mut call = handle
            .read_property(
                name,
                InteractionOptions::new().with_form_index(form),
                &mut work(100000),
            )
            .map_err(|e| e.error().error().clone())?;
        call.step(&mut cx, &mut work(4096));
        let output = call.try_take(&mut work(4096))?.expect("finite result")?;
        statuses[index] = output
            .metadata()
            .binding_response()
            .expect("Core sealed binding result")
            .status_code();
    }
    drop(handle);
    for _ in 0..100000 {
        if let StartStatus::Ready(outcome) = runtime.shutdown(&mut cx, &mut work(4096)) {
            assert!(outcome.is_complete());
            drop(runtime);
            assert!(report().0.unwrap().is_complete());
            drop(report);
            assert!(
                parent
                    .snapshot(ConsumerPropertyReadPolicy::PARENT_KINDS)
                    .iter()
                    .all(|a| a.used() == 0)
            );
            return Ok(statuses);
        }
    }
    panic!("bounded fixture shutdown stalled");
}

const fn layout<T>() -> [usize; 2] {
    [core::mem::size_of::<T>(), core::mem::align_of::<T>()]
}
/// Normal-library layouts, also emitted by the actual ARM feature cells.
#[used]
pub static STATIC_RUNTIME_LAYOUTS: [[usize; 2]; 8] = [
    layout::<StaticConsumerPropertyReadRuntime<'static, StaticClient>>(),
    layout::<StaticConsumerPropertyReadBuild<'static, 'static, 'static, StaticClient>>(),
    layout::<
        StaticConsumerPropertyReadSlot<clinkz_wot_consumer_property_read_binding_fixture::State>,
    >(),
    layout::<clinkz_wot_consumer_property_read_binding_fixture::State>(),
    layout::<ConsumerCompilerRegistration<StaticConsumerBindingPreparation<StaticClient>>>(),
    layout::<OutboundRequest>(),
    layout::<CoreResult<InteractionOutput>>(),
    layout::<(
        Cell<(Option<CleanupOutcome>, [u64; 4])>,
        Cell<u8>,
        RefCell<ResourceAccount>,
    )>(),
];
