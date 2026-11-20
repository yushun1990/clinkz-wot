#![cfg(all(feature = "std", feature = "validated-thing"))]
//! Public preparation accounting; this test Binding performs no native I/O.
use clinkz_wot_core::{
    consumer::*, consumer_action::*, data_schema::PreparedSchema, host_accounts::*, *,
};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_td::thing::Thing;
use std::{
    any::Any,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

fn identity() -> BindingRegistrationIdentity {
    BindingRegistrationIdentity::new(
        BindingId::new(1),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([1; 32]),
        BindingArtifactCompatibility::new([1; 16]),
        0,
    )
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never))
}
struct Data(Box<str>);
impl ConsumerDataService for Data {
    fn qualification_revision(&self) -> u64 {
        0
    }

    fn content_type(&self) -> &str {
        &self.0
    }
    fn policy(&self) -> ConsumerDataPolicy {
        ConsumerDataPolicy {
            retained_bytes: (size_of::<Self>() + self.0.len()) as u64,
            temporary_bytes: 0,
            body_bytes: 1,
            encoded_selection_bytes: 0,
            validation_work: 1,
        }
    }
    fn validate(&self, _: &PreparedSchema, _: &Payload) -> CoreResult<()> {
        Err(unsupported())
    }
}
struct Access;
impl ConsumerAccessService for Access {
    fn qualification_revision(&self) -> u64 {
        0
    }

    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
        Ok(())
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        _: BindingRegistrationIdentity,
        _: &str,
        _: Operation,
    ) -> CoreResult<()> {
        Ok(())
    }
    fn work(&self) -> u64 {
        1
    }
    fn retained_bytes(&self) -> u64 {
        0
    }
}
struct Compiler {
    calls: Arc<AtomicUsize>,
    temporary_bytes: u64,
}
impl ConsumerActionPreparation for Compiler {
    fn identity(&self) -> BindingRegistrationIdentity {
        identity()
    }
    fn bounds(&self) -> ConsumerPreparationBounds {
        ConsumerPreparationBounds {
            work: 1,
            temporary_bytes: self.temporary_bytes,
            retained_bytes: 0,
        }
    }
    fn compile_action(
        &self,
        input: ConsumerActionPreparationInput<'_>,
    ) -> CoreResult<Option<Box<dyn ConsumerActionMapping>>> {
        assert_eq!(input.operation, Operation::InvokeAction);
        self.calls.fetch_add(1, Ordering::Relaxed);
        let scratch = vec![0xa5_u8; self.temporary_bytes as usize];
        std::hint::black_box(&scratch);
        Ok(Some(Box::new(Mapping)))
    }
}
struct Mapping;
impl ConsumerActionMapping for Mapping {
    fn identity(&self) -> BindingRegistrationIdentity {
        identity()
    }
    fn operation(&self) -> Operation {
        Operation::InvokeAction
    }
    fn bounds(&self) -> ConsumerActionBounds {
        ConsumerActionBounds {
            exchange_bytes: 1,
            temporary_bytes: 0,
            continuation_bytes: 1,
            response_bytes: 1,
            poll_work: 1,
        }
    }
    fn native_definition(&self) -> &dyn Any {
        self
    }
    fn supports_continuation(&self, _: &dyn ConsumerActionMapping) -> CoreResult<()> {
        Err(unsupported())
    }
    fn destination(&self, _: Option<&dyn Any>) -> CoreResult<Box<str>> {
        Ok("test://action/run".into())
    }
    fn start(
        &self,
        _: Option<&Payload>,
        _: Option<&dyn Any>,
    ) -> CoreResult<Box<dyn ConsumerActionExchange>> {
        Err(unsupported())
    }
}
fn registry(
    media: &str,
    bank: &Arc<HostResourceAccounts>,
    count: &Arc<AtomicUsize>,
    temporary_bytes: u64,
) -> (ConsumerActionRegistry, RetainedConsumerServices) {
    let services = RetainedConsumerServices {
        data: ConsumerServiceOwner::retain_data(Arc::new(Data(media.into())), bank)
            .ok()
            .unwrap(),
        access: ConsumerServiceOwner::retain_access(Arc::new(Access), bank)
            .ok()
            .unwrap(),
    };
    let registry = ConsumerActionRegistry::try_new_fixed(vec![ConsumerActionRegistration {
        preparation: Box::new(Compiler {
            calls: count.clone(),
            temporary_bytes,
        }),
        services: services.clone(),
    }])
    .unwrap();
    (registry, services)
}
fn prepare(
    registry: &ConsumerActionRegistry,
    media: &str,
    limits: &ResourceLimits,
    bank: Arc<HostResourceAccounts>,
) -> CoreResult<ConsumerActionDefinition> {
    let thing: Thing = serde_json::from_value(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"media backing",
        "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "actions":{"run":{"synchronous":false,"forms":[{
            "href":"test://action/run", "op":"invokeaction", "contentType":media
        }]}}
    }))
    .unwrap();
    let mut work = WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000));
    registry.prepare_action(
        &thing,
        "run",
        ConsumerActionSelectors::default(),
        limits,
        bank,
        &mut work,
    )
}
fn large_media() -> String {
    format!(
        "application/json;profile=\"https://example.com/{}\"",
        "a".repeat(4096)
    )
}
fn live(bank: &HostResourceAccounts) -> u64 {
    bank.account(ResourceKind::EngineLiveBytesGlobalMax).used()
}
fn clean(bank: &HostResourceAccounts) {
    for k in ResourceKind::ALL {
        assert_eq!(bank.account(k).used(), 0, "{k:?}");
    }
}
fn measure(media: &str) -> u64 {
    let service_bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let definition_bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let count = Arc::new(AtomicUsize::new(0));
    let (registry, services) = registry(media, &service_bank, &count, 0);
    let original_service_charge = live(&service_bank);
    let definition = prepare(
        &registry,
        media,
        GatewayDefaultV1::LIMITS,
        definition_bank.clone(),
    )
    .unwrap();
    assert_eq!(&*definition.input().invoke.content_type, media);
    assert_ne!(
        definition.input().invoke.content_type.as_ptr(),
        services.data.content_type().as_ptr()
    );
    assert_eq!(count.load(Ordering::Relaxed), 1);
    assert_eq!(
        live(&service_bank),
        original_service_charge,
        "the copy belongs to the definition, not the physical service"
    );
    let charge = live(&definition_bank);
    drop((registry, services));
    assert!(
        live(&service_bank) > 0,
        "definition keeps the original data owner funded"
    );
    drop(definition);
    clean(&service_bank);
    clean(&definition_bank);
    charge
}
#[test]
fn copied_media_snapshot_has_independent_definition_funding() {
    let short = "application/json";
    let large = large_media();
    assert!(large.len() > 4096);
    let small_charge = measure(short);
    let large_charge = measure(&large);
    assert_eq!(
        large_charge - small_charge,
        (large.len() - short.len()) as u64
    );
    assert!(large_charge >= large.len() as u64);
}
#[test]
fn unfunded_media_copy_refuses_before_compilation_and_refunds_partial_preparation() {
    let short = "application/json";
    let large = large_media();
    let required = measure(short) + (large.len() - short.len()) as u64;
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    let scratch = limits
        .get(ResourceKind::AdmissionTemporaryBytesPerOperationMax)
        .unwrap();
    assert!(limits.set(
        ResourceKind::EngineLiveBytesGlobalMax,
        Some(scratch + required - 1)
    ));
    let service_bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let definition_bank = Arc::new(HostResourceAccounts::new(&limits));
    let count = Arc::new(AtomicUsize::new(0));
    let (registry, services) = registry(&large, &service_bank, &count, 0);
    let original_service_charge = live(&service_bank);
    let result = prepare(&registry, &large, &limits, definition_bank.clone());
    assert!(matches!(result, Err(CoreError::LimitExceeded {
        resource: ResourceKind::EngineLiveBytesGlobalMax, requested: Some(bytes), ..
    }) if bytes == required));
    assert_eq!(
        count.load(Ordering::Relaxed),
        0,
        "reserve the copy before native compilation/allocation"
    );
    assert_eq!(live(&service_bank), original_service_charge);
    clean(&definition_bank);
    drop((registry, services));
    clean(&service_bank);
}

#[test]
fn native_scratch_shares_the_single_paid_allowance_with_td_and_selection() {
    let base = GatewayDefaultV1::LIMITS.clone().with_limit(
        ResourceKind::AdmissionTemporaryBytesPerOperationMax,
        Some(1 << 20),
    );
    for kind in [
        ResourceKind::AdmissionTemporaryBytesPerOperationMax,
        ResourceKind::AdmissionTemporaryBytesGlobalMax,
        ResourceKind::PeakLiveBytesPerAdmissionMax,
        ResourceKind::AdmissionPeakLiveBytesGlobalMax,
    ] {
        let ceiling = 64 * 1024;
        let limits = base.clone().with_limit(kind, Some(ceiling));
        let bank = Arc::new(HostResourceAccounts::new(&limits));
        let count = Arc::new(AtomicUsize::new(0));
        let attempt = |native_bytes| {
            let (registry, _) = registry("application/json", &bank, &count, native_bytes);
            prepare(&registry, "application/json", &limits, bank.clone())
        };
        let probe = 128 * 1024;
        let available = match attempt(probe) {
            Err(CoreError::LimitExceeded {
                resource,
                limit,
                requested: Some(bytes),
                ..
            }) => {
                assert_eq!((resource, bytes), (kind, probe));
                limit
            }
            _ => panic!(
                "compiler scratch must fit beside the TD and selection storage under {kind:?}"
            ),
        };
        assert!(available > 0 && available < ceiling);
        assert!(
            matches!(attempt(available + 1), Err(CoreError::LimitExceeded {resource, limit, ..}) if resource == kind && limit == available)
        );
        assert_eq!(count.load(Ordering::Relaxed), 0);
        clean(&bank);
        drop(attempt(available).unwrap());
        assert_eq!(count.load(Ordering::Relaxed), 1);
        clean(&bank);
    }
}
