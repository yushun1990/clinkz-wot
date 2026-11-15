use super::*;
use std::sync::atomic::AtomicU64;

#[derive(Default)]
struct Declarations {
    data_bytes: AtomicU64,
    data_work: AtomicU64,
    data_temporary: AtomicU64,
    body_limit: AtomicU64,
    validations: AtomicUsize,
    media: AtomicBool,
    unsupported: AtomicBool,
    native_bytes: AtomicU64,
    native_work: AtomicU64,
    native_identity: AtomicBool,
    native_operation: AtomicBool,
    access_bytes: AtomicU64,
    access_work: AtomicU64,
    revision: AtomicU64,
    compile_change: AtomicBool,
    authorization_change: AtomicBool,
    preparation_work: AtomicU64,
    validation_change: AtomicBool,
    starts: AtomicUsize,
    drops: AtomicUsize,
    last_output: Mutex<Option<std::sync::Weak<[u8]>>>,
}
struct OwnedData {
    inner: Arc<dyn ConsumerDataService>,
    backing: Vec<u8>,
    declarations: Arc<Declarations>,
    bank: Arc<HostResourceAccounts>,
}
impl ConsumerDataService for OwnedData {
    fn content_type(&self) -> &str {
        if self.declarations.media.load(Ordering::Relaxed) {
            "text/plain"
        } else {
            self.inner.content_type()
        }
    }
    fn policy(&self) -> ConsumerDataPolicy {
        let mut p = self.inner.policy();
        p.retained_bytes +=
            self.backing.len() as u64 + self.declarations.data_bytes.load(Ordering::Relaxed);
        p.validation_work += self.declarations.data_work.load(Ordering::Relaxed);
        p.temporary_bytes += self.declarations.data_temporary.load(Ordering::Relaxed);
        let limit = self.declarations.body_limit.load(Ordering::Relaxed);
        if limit != 0 {
            p.body_bytes = limit;
        }
        p
    }
    fn available(&self) -> bool {
        self.inner.available()
    }
    fn supports_schema(&self, s: &PreparedSchema) -> bool {
        !self.declarations.unsupported.load(Ordering::Relaxed) && self.inner.supports_schema(s)
    }
    fn validate(&self, s: &PreparedSchema, p: &Payload) -> CoreResult<()> {
        self.declarations
            .validations
            .fetch_add(1, Ordering::Relaxed);
        let result = self.inner.validate(s, p);
        if p.body.as_ref() == b"24" {
            *self.declarations.last_output.lock().unwrap() = Some(Arc::downgrade(&p.body));
        }
        if p.body.as_ref() == b"24"
            && self
                .declarations
                .validation_change
                .swap(false, Ordering::Relaxed)
        {
            self.declarations.data_work.fetch_add(1, Ordering::Relaxed);
        }
        result
    }
}
impl Drop for OwnedData {
    fn drop(&mut self) {
        assert!(
            self.bank
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                >= self.backing.len() as u64
        );
        self.declarations.drops.fetch_add(1, Ordering::Relaxed);
    }
}
struct OwnedAccess {
    inner: Arc<dyn ConsumerAccessService>,
    backing: Vec<u8>,
    declarations: Arc<Declarations>,
    bank: Arc<HostResourceAccounts>,
}
impl ConsumerAccessService for OwnedAccess {
    fn authorize(
        &self,
        id: BindingRegistrationIdentity,
        target: &str,
        op: Operation,
    ) -> CoreResult<()> {
        let result = self.inner.authorize(id, target, op);
        if self
            .declarations
            .authorization_change
            .swap(false, Ordering::Relaxed)
        {
            self.declarations
                .preparation_work
                .fetch_add(1, Ordering::Relaxed);
        }
        result
    }
    fn authorize_instance(
        &self,
        consumer: &ConsumerInstanceRef,
        id: BindingRegistrationIdentity,
        target: &str,
        op: Operation,
    ) -> CoreResult<()> {
        self.inner.authorize_instance(consumer, id, target, op)
    }
    fn security_revision(&self) -> u64 {
        self.declarations.revision.load(Ordering::Relaxed)
    }
    fn work(&self) -> u64 {
        self.inner.work() + self.declarations.access_work.load(Ordering::Relaxed)
    }
    fn retained_bytes(&self) -> u64 {
        self.inner.retained_bytes()
            + self.backing.len() as u64
            + self.declarations.access_bytes.load(Ordering::Relaxed)
    }
}
impl Drop for OwnedAccess {
    fn drop(&mut self) {
        assert!(
            self.bank
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                >= self.backing.len() as u64
        );
        self.declarations.drops.fetch_add(1, Ordering::Relaxed);
    }
}
fn retained(
    b: &Arc<HostResourceAccounts>,
    declarations: &Arc<Declarations>,
    bytes: (usize, usize),
    policy: Arc<Policy>,
) -> RetainedConsumerServices {
    let actual = HttpConsumerConfig::for_cleartext_ip()
        .unwrap()
        .consumer_services();
    RetainedConsumerServices {
        data: ConsumerServiceOwner::retain_data(
            Arc::new(OwnedData {
                inner: actual.data,
                backing: vec![0; bytes.0],
                declarations: declarations.clone(),
                bank: b.clone(),
            }),
            b,
        )
        .ok()
        .unwrap(),
        access: ConsumerServiceOwner::retain_access(
            Arc::new(OwnedAccess {
                inner: Arc::new(Access(policy, actual.access)),
                backing: vec![0; bytes.1],
                declarations: declarations.clone(),
                bank: b.clone(),
            }),
            b,
        )
        .ok()
        .unwrap(),
    }
}
struct CountedPreparation {
    inner: HttpActionConsumer,
    count: Arc<AtomicUsize>,
    declarations: Arc<Declarations>,
}
impl ConsumerActionPreparation for CountedPreparation {
    fn identity(&self) -> BindingRegistrationIdentity {
        self.inner.identity()
    }
    fn bounds(&self) -> ConsumerPreparationBounds {
        let mut bounds = self.inner.bounds();
        bounds.work += self.declarations.preparation_work.load(Ordering::Relaxed);
        bounds
    }
    fn compile_action(
        &self,
        input: ConsumerActionPreparationInput<'_>,
    ) -> CoreResult<Option<Box<dyn ConsumerActionMapping>>> {
        self.count.fetch_add(1, Ordering::Relaxed);
        let result = self.inner.compile_action(input)?;
        if self
            .declarations
            .compile_change
            .swap(false, Ordering::Relaxed)
        {
            self.declarations.data_work.fetch_add(1, Ordering::Relaxed);
        }
        Ok(result.map(|inner| {
            Box::new(DeclaredMapping {
                inner,
                declarations: self.declarations.clone(),
            }) as Box<dyn ConsumerActionMapping>
        }))
    }
}
struct DeclaredMapping {
    inner: Box<dyn ConsumerActionMapping>,
    declarations: Arc<Declarations>,
}
impl ConsumerActionMapping for DeclaredMapping {
    fn identity(&self) -> BindingRegistrationIdentity {
        if self.declarations.native_identity.load(Ordering::Relaxed) {
            definition_identity(999)
        } else {
            self.inner.identity()
        }
    }
    fn operation(&self) -> Operation {
        if self.declarations.native_operation.load(Ordering::Relaxed) {
            Operation::ReadProperty
        } else {
            self.inner.operation()
        }
    }
    fn bounds(&self) -> ConsumerActionBounds {
        let mut b = self.inner.bounds();
        b.exchange_bytes += self.declarations.native_bytes.load(Ordering::Relaxed);
        b.poll_work += self.declarations.native_work.load(Ordering::Relaxed);
        b
    }
    fn native_definition(&self) -> &dyn Any {
        self.inner.native_definition()
    }
    fn supports_continuation(&self, invoke: &dyn ConsumerActionMapping) -> CoreResult<()> {
        self.inner.supports_continuation(invoke)
    }
    fn destination(&self, continuation: Option<&dyn Any>) -> CoreResult<Box<str>> {
        self.inner.destination(continuation)
    }
    fn start(
        &self,
        input: Option<&Payload>,
        continuation: Option<&dyn Any>,
    ) -> CoreResult<Box<dyn ConsumerActionExchange>> {
        self.declarations.starts.fetch_add(1, Ordering::Relaxed);
        self.inner.start(input, continuation)
    }
}
fn prepare(
    text: &str,
    services: &RetainedConsumerServices,
    definition_bank: Arc<HostResourceAccounts>,
    declarations: &Arc<Declarations>,
    count: &Arc<AtomicUsize>,
) -> CoreResult<ConsumerActionDefinition> {
    prepare_operations(text, [(services, declarations); 3], definition_bank, count)
}
fn prepare_operations(
    text: &str,
    registrations: [(&RetainedConsumerServices, &Arc<Declarations>); 3],
    definition_bank: Arc<HostResourceAccounts>,
    count: &Arc<AtomicUsize>,
) -> CoreResult<ConsumerActionDefinition> {
    let config = HttpConsumerConfig::for_cleartext_ip().unwrap();
    let registry = ConsumerActionRegistry::try_new(
        (0..3)
            .map(|i| ConsumerActionRegistration {
                preparation: Box::new(CountedPreparation {
                    inner: HttpActionConsumer::new(
                        config,
                        BindingId::new(90 + i),
                        BindingGeneration::INITIAL,
                        i,
                    ),
                    count: count.clone(),
                    declarations: registrations[i as usize].1.clone(),
                }),
                services: registrations[i as usize].0.clone(),
            })
            .collect(),
    )?;
    let select = |id| ConsumerSelector {
        binding: Some(BindingId::new(id)),
        ..Default::default()
    };
    let thing: Thing = serde_json::from_str(text).unwrap();
    registry.prepare_action(
        &thing,
        "run",
        ConsumerActionSelectors {
            invoke: select(90),
            query: select(91),
            cancel: select(92),
        },
        &limits(),
        definition_bank,
        &mut budget(),
    )
}
fn access(services: &RetainedConsumerServices) -> ConsumerActionAccess {
    ConsumerActionAccess {
        invoke: services.access.clone(),
        query: Some(services.access.clone()),
        cancel: Some(services.access.clone()),
    }
}
fn associate(
    owner: &ConsumerInstance,
    definition: &Rc<ConsumerActionDefinition>,
    services: &RetainedConsumerServices,
    b: Arc<HostResourceAccounts>,
) -> ConsumerActionAssociation {
    ConsumerActionAssociation::try_new(
        owner.reference(),
        definition.clone(),
        access(services),
        b,
        &mut budget(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn live(b: &HostResourceAccounts) -> u64 {
    b.account(ResourceKind::EngineLiveBytesGlobalMax).used()
}

#[test]
fn physical_services_are_funded_once_across_operations_definitions_and_registries() {
    let measure = |bytes| {
        let services_bank = bank();
        let definition_banks = [bank(), bank()];
        let declarations = Arc::new(Declarations::default());
        let services = retained(
            &services_bank,
            &declarations,
            bytes,
            Arc::new(Policy::default()),
        );
        let physical = live(&services_bank);
        let count = Arc::new(AtomicUsize::new(0));
        let text = td("127.0.0.1:9".parse().unwrap(), None);
        let a = prepare(
            &text,
            &services,
            definition_banks[0].clone(),
            &declarations,
            &count,
        )
        .unwrap();
        let b = prepare(
            &text,
            &services,
            definition_banks[1].clone(),
            &declarations,
            &count,
        )
        .unwrap();
        assert_eq!(count.load(Ordering::Relaxed), 6);
        assert_eq!(live(&services_bank), physical);
        assert!(Arc::ptr_eq(
            &a.input().invoke.data,
            &a.input().query.as_ref().unwrap().data
        ));
        assert!(Arc::ptr_eq(
            &a.input().invoke.data,
            &b.input().cancel.as_ref().unwrap().data
        ));
        let definition_charge = live(&definition_banks[0]);
        drop((a, b, services));
        assert_eq!(declarations.drops.load(Ordering::Relaxed), 2);
        for b in &definition_banks {
            clean(b);
        }
        clean(&services_bank);
        (physical, definition_charge)
    };
    let small = measure((32, 32));
    for bytes in [(60032, 32), (32, 60032)] {
        let large = measure(bytes);
        assert_eq!(
            large.0 - small.0,
            60000,
            "one physical owner, regardless of three operations and two registries"
        );
        assert_eq!(
            large.1, small.1,
            "mapping funding excludes physical service backing"
        );
    }
}

#[test]
fn prepared_declarations_refuse_changed_handoffs_without_refresh_or_owner_loss() {
    for case in 0..10 {
        let sb = bank();
        let db = bank();
        let ab = bank();
        let eb = bank();
        let declarations = Arc::new(Declarations::default());
        let services = retained(&sb, &declarations, (32, 32), Arc::new(Policy::default()));
        let count = Arc::new(AtomicUsize::new(0));
        let definition = Rc::new(
            prepare(
                &td("127.0.0.1:9".parse().unwrap(), None),
                &services,
                db.clone(),
                &declarations,
                &count,
            )
            .unwrap(),
        );
        let owner = instance();
        let original_association = associate(&owner, &definition, &services, ab.clone());
        match case {
            0 => declarations.data_bytes.store(123456, Ordering::Relaxed),
            1 => declarations.data_temporary.store(1, Ordering::Relaxed),
            2 => declarations.data_work.store(1, Ordering::Relaxed),
            3 => declarations.media.store(true, Ordering::Relaxed),
            4 => declarations.unsupported.store(true, Ordering::Relaxed),
            5 => declarations.native_bytes.store(123456, Ordering::Relaxed),
            6 => declarations.native_work.store(1, Ordering::Relaxed),
            7 => declarations.native_identity.store(true, Ordering::Relaxed),
            8 => declarations.native_operation.store(true, Ordering::Relaxed),
            _ => declarations.access_bytes.store(123456, Ordering::Relaxed),
        }
        let before = live(&ab);
        let refusal = ConsumerActionAssociation::try_new(
            owner.reference(),
            definition.clone(),
            access(&services),
            ab.clone(),
            &mut budget(),
        )
        .err()
        .expect("changed qualified declaration must refuse");
        let ((returned, returned_access), _) = refusal.into_parts();
        assert!(Rc::ptr_eq(&returned, &definition));
        assert!(Arc::ptr_eq(&returned_access.invoke, &services.access));
        assert_eq!(live(&ab), before);
        assert_eq!(declarations.starts.load(Ordering::Relaxed), 0);
        // Independent empty execution capacity neither consumes nor refreshes a definition.
        let capacity = ConsumerActions::try_new(1, eb.clone()).unwrap();
        assert!(capacity.is_idle());
        let input = payload(b"12").unwrap();
        let original = input.body.clone();
        let refusal = capacity
            .invoke_action(
                &original_association,
                Some(input),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .err()
            .expect("an existing association cannot refresh the definition either");
        let (input, _) = refusal.into_parts();
        assert!(Arc::ptr_eq(&original, &input.unwrap().body));
        assert!(capacity.is_idle());
        assert_eq!(declarations.starts.load(Ordering::Relaxed), 0);
        drop((
            capacity,
            original_association,
            returned,
            returned_access,
            definition,
            services,
            owner,
        ));
        for b in [&sb, &db, &ab, &eb] {
            clean(b);
        }
    }
}

#[test]
fn preparation_callbacks_cannot_replace_original_service_or_compiler_declarations() {
    for during_authorization in [false, true] {
        let sb = bank();
        let db = bank();
        let declarations = Arc::new(Declarations::default());
        let services = retained(&sb, &declarations, (32, 32), Arc::new(Policy::default()));
        let count = Arc::new(AtomicUsize::new(0));
        if during_authorization {
            declarations
                .authorization_change
                .store(true, Ordering::Relaxed);
        } else {
            declarations.compile_change.store(true, Ordering::Relaxed);
        }
        assert!(
            prepare(
                &td("127.0.0.1:9".parse().unwrap(), None),
                &services,
                db.clone(),
                &declarations,
                &count
            )
            .is_err()
        );
        assert_eq!(
            count.load(Ordering::Relaxed),
            usize::from(!during_authorization)
        );
        clean(&db);
        drop(services);
        clean(&sb);
    }
}

#[test]
fn real_http_definition_serves_independent_associations_and_execution_capacity() {
    io().block_on(async {
        let mut producer = Producer::new();
        let sb = bank();
        let db = bank();
        let ab = [bank(), bank(), bank()];
        let eb = [bank(), bank()];
        let declarations = Arc::new(Declarations::default());
        let policy_a = Arc::new(Policy::default());
        let services = retained(&sb, &declarations, (60032, 60032), policy_a.clone());
        let count = Arc::new(AtomicUsize::new(0));
        let definition = Rc::new(
            prepare(
                &td(producer.address, None),
                &services,
                db.clone(),
                &declarations,
                &count,
            )
            .unwrap(),
        );
        let service_charge = live(&sb);
        let definition_charge = live(&db);
        let owners = [instance(), instance(), instance()];
        let policy_b = Arc::new(Policy::default());
        let independent = retained(
            &ab[1],
            &Arc::new(Declarations::default()),
            (32, 32),
            policy_b.clone(),
        );
        let a = associate(&owners[0], &definition, &services, ab[0].clone());
        let b = associate(&owners[1], &definition, &independent, ab[1].clone());
        let c = associate(&owners[2], &definition, &independent, ab[2].clone());
        assert!(Rc::ptr_eq(a.definition(), b.definition()));
        assert_eq!(count.load(Ordering::Relaxed), 3);
        assert_eq!((live(&sb), live(&db)), (service_charge, definition_charge));
        assert!(
            ab.iter()
                .all(|b| b.account(ResourceKind::PendingHandlerCallsGlobalMax).used() == 0)
        );
        let capacities = [
            ConsumerActions::try_new(1, eb[0].clone()).unwrap(),
            ConsumerActions::try_new(1, eb[1].clone()).unwrap(),
        ];
        let first = capacities[0]
            .invoke_action(&a, payload(b"12"), Deadline::NONE, now(), &mut budget())
            .ok()
            .unwrap();
        let first_identity = first.identity().unwrap();
        producer.drive(&first, ConsumerActionState::Pending).await;
        let p = payload(b"12").unwrap();
        let original = p.body.clone();
        let refusal = capacities[0]
            .invoke_action(&b, Some(p), Deadline::NONE, now(), &mut budget())
            .err()
            .unwrap();
        let (p, _) = refusal.into_parts();
        assert!(Arc::ptr_eq(&original, &p.unwrap().body));
        let second = capacities[1]
            .invoke_action(&b, payload(b"12"), Deadline::NONE, now(), &mut budget())
            .ok()
            .unwrap();
        producer.drive(&second, ConsumerActionState::Pending).await;
        let second_identity = second.identity().unwrap();
        assert_ne!(first_identity, second_identity);
        assert_eq!(first_identity.consumer(), &owners[0].reference());
        assert_eq!(second_identity.consumer(), &owners[1].reference());
        drop(b); // A view never revokes the admitted work.
        policy_a.denied.store(true, Ordering::Relaxed);
        capacities[0].begin_close();
        producer.drive(&first, ConsumerActionState::Closed).await;
        second.request_cancel(now(), &mut budget()).unwrap();
        producer.drive(&second, ConsumerActionState::Pending).await;
        assert_eq!(
            second.remote_cancellation().unwrap(),
            ConsumerActionCancellation::Requested
        );
        producer.counts.complete.set(true);
        second.query(now(), &mut budget()).unwrap();
        producer
            .drive(&second, ConsumerActionState::Completed)
            .await;
        let first_result = first.take_settlement(now(), &mut budget()).unwrap();
        let second_result = second.take_settlement(now(), &mut budget()).unwrap();
        assert!(first_result.result().is_err());
        assert_eq!(second_result.result().unwrap().data().unwrap().body, b"24");
        assert_eq!(second_result.identity(), &second_identity);
        assert_eq!(
            second_result.selected_form(Operation::QueryAction).unwrap(),
            (definition_identity(91), 1)
        );
        assert_eq!(
            second_result
                .selected_form(Operation::CancelAction)
                .unwrap(),
            (definition_identity(92), 2)
        );
        assert!(
            policy_b
                .log
                .lock()
                .unwrap()
                .iter()
                .any(|(_, op, target)| *op == Operation::CancelAction
                    && target.contains("/stop?id="))
        );
        // Settlement frees a slot, while a distinct retained quota stays with the result.
        let third = capacities[1]
            .invoke_action(&c, payload(b"12"), Deadline::NONE, now(), &mut budget())
            .ok()
            .unwrap();
        producer.drive(&third, ConsumerActionState::Pending).await;
        third.query(now(), &mut budget()).unwrap();
        producer.drive(&third, ConsumerActionState::Completed).await;
        let third_result = third.take_settlement(now(), &mut budget()).unwrap();
        assert!(third_result.result().is_ok());
        let output = declarations
            .last_output
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .clone();
        let third_result = Rc::new(third_result);
        let shared_result = third_result.clone();
        assert_eq!(output.strong_count(), 1);
        assert_eq!(count.load(Ordering::Relaxed), 3);
        assert_eq!(producer.counts.invoked.get(), 3);
        drop((
            first,
            second,
            third,
            capacities,
            a,
            c,
            definition,
            services,
            independent,
            owners,
        ));
        assert!(live(&sb) > 0 && live(&db) > 0);
        assert!(
            ab.iter()
                .all(|b| b.account(ResourceKind::PendingHandlerCallsGlobalMax).used() == 1)
        );
        drop((first_result, second_result, third_result));
        assert_eq!(output.strong_count(), 1);
        assert_eq!(shared_result.result().unwrap().data().unwrap().body, b"24");
        assert!(live(&sb) > 0 && live(&db) > 0);
        drop(shared_result);
        assert_eq!(output.strong_count(), 0);
        assert_eq!(declarations.drops.load(Ordering::Relaxed), 2);
        for b in [&sb, &db, &ab[0], &ab[1], &ab[2], &eb[0], &eb[1]] {
            clean(b);
        }
        producer.finish();
    });
}

#[test]
fn real_http_observed_dependency_change_is_irreversible_and_retains_native_output() {
    io().block_on(async {
        for case in 0..7 {
            let mut producer = Producer::new();
            let sb = bank();
            let db = bank();
            let ab = bank();
            let eb = bank();
            let declarations = [0, 1, 2].map(|_| Arc::new(Declarations::default()));
            let services = [0, 1, 2]
                .map(|i| retained(&sb, &declarations[i], (32, 32), Arc::new(Policy::default())));
            let definition = Rc::new(
                prepare_operations(
                    &td(producer.address, None),
                    [
                        (&services[0], &declarations[0]),
                        (&services[1], &declarations[1]),
                        (&services[2], &declarations[2]),
                    ],
                    db.clone(),
                    &Arc::new(AtomicUsize::new(0)),
                )
                .unwrap(),
            );
            let owner = instance();
            let association = ConsumerActionAssociation::try_new(
                owner.reference(),
                definition.clone(),
                ConsumerActionAccess {
                    invoke: services[0].access.clone(),
                    query: Some(services[1].access.clone()),
                    cancel: Some(services[2].access.clone()),
                },
                ab.clone(),
                &mut budget(),
            )
            .ok()
            .unwrap();
            let calls = ConsumerActions::try_new(1, eb.clone()).unwrap();
            let h = calls
                .invoke_action(
                    &association,
                    payload(b"12"),
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .ok()
                .unwrap();
            let identity = h.identity().unwrap();
            producer.drive(&h, ConsumerActionState::Pending).await;
            producer.counts.complete.set(true);
            h.query(now(), &mut budget()).unwrap();
            if case == 0 {
                declarations[1]
                    .validation_change
                    .store(true, Ordering::Relaxed);
            } else {
                producer.drive(&h, ConsumerActionState::Completed).await;
                match case {
                    1 => declarations[1].revision.store(1, Ordering::Relaxed),
                    2 => declarations[1].native_bytes.store(1, Ordering::Relaxed),
                    3 => declarations[1].data_work.store(1, Ordering::Relaxed),
                    4 => declarations[1].access_work.store(1, Ordering::Relaxed),
                    5 => declarations[0].revision.store(1, Ordering::Relaxed),
                    _ => declarations[0].native_bytes.store(1, Ordering::Relaxed),
                }
            }
            producer.drive(&h, ConsumerActionState::Closed).await;
            declarations[1].revision.store(0, Ordering::Relaxed);
            declarations[1].data_work.store(0, Ordering::Relaxed);
            declarations[1].native_bytes.store(0, Ordering::Relaxed);
            declarations[1].access_work.store(0, Ordering::Relaxed);
            declarations[0].revision.store(0, Ordering::Relaxed);
            declarations[0].native_bytes.store(0, Ordering::Relaxed);
            assert!(h.query(now(), &mut budget()).is_err());
            let s = h.take_settlement(now(), &mut budget()).unwrap();
            assert_eq!(s.identity(), &identity);
            assert!(s.result().is_err());
            assert_eq!(
                s.native_result()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .data()
                    .unwrap()
                    .body,
                b"24"
            );
            drop((h, calls, association, definition, services, owner));
            assert!(live(&sb) > 0 && live(&db) > 0);
            drop(s);
            for b in [&sb, &db, &ab, &eb] {
                clean(b);
            }
            producer.finish();
        }
    });
}

#[test]
fn real_http_association_quota_spans_independent_capacities_and_retained_settlements() {
    io().block_on(async {
        let mut producer = Producer::new();
        let sb = bank();
        let db = bank();
        let mut quota_limits = limits();
        assert!(quota_limits.set(ResourceKind::PendingHandlerCallsGlobalMax, Some(1)));
        let ab = Arc::new(HostResourceAccounts::new(&quota_limits));
        let eb = [bank(), bank()];
        let declarations = Arc::new(Declarations::default());
        let services = retained(&sb, &declarations, (32, 32), Arc::new(Policy::default()));
        let definition = Rc::new(
            prepare(
                &td(producer.address, None),
                &services,
                db.clone(),
                &declarations,
                &Arc::new(AtomicUsize::new(0)),
            )
            .unwrap(),
        );
        let owner = instance();
        let association = associate(&owner, &definition, &services, ab.clone());
        let capacities = [
            ConsumerActions::try_new(1, eb[0].clone()).unwrap(),
            ConsumerActions::try_new(1, eb[1].clone()).unwrap(),
        ];
        let first = capacities[0]
            .invoke_action(
                &association,
                payload(b"12"),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .ok()
            .unwrap();
        producer.drive(&first, ConsumerActionState::Pending).await;
        let refuse = || {
            let before = live(&eb[1]);
            let p = payload(b"12").unwrap();
            let original = p.body.clone();
            let refusal = capacities[1]
                .invoke_action(&association, Some(p), Deadline::NONE, now(), &mut budget())
                .err()
                .unwrap();
            let (p, _) = refusal.into_parts();
            assert!(Arc::ptr_eq(&original, &p.unwrap().body));
            assert!(capacities[1].is_idle());
            assert_eq!(live(&eb[1]), before);
        };
        refuse();
        producer.counts.complete.set(true);
        first.query(now(), &mut budget()).unwrap();
        producer.drive(&first, ConsumerActionState::Completed).await;
        let settlement = first.take_settlement(now(), &mut budget()).unwrap();
        assert!(capacities[0].is_idle());
        refuse(); // A free slot cannot refund retained result quota.
        assert_eq!(producer.counts.invoked.get(), 1);
        drop(settlement);
        let second = capacities[1]
            .invoke_action(
                &association,
                payload(b"12"),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .ok()
            .unwrap();
        producer.drive(&second, ConsumerActionState::Pending).await;
        assert!(first.identity().is_err()); // Stale handle cannot address the new capacity.
        second.query(now(), &mut budget()).unwrap();
        producer
            .drive(&second, ConsumerActionState::Completed)
            .await;
        let settlement = second.take_settlement(now(), &mut budget()).unwrap();
        assert!(settlement.result().is_ok());
        assert_eq!(producer.counts.invoked.get(), 2);
        drop((
            first,
            second,
            capacities,
            association,
            definition,
            services,
            owner,
            settlement,
        ));
        for b in [&sb, &db, &ab, &eb[0], &eb[1]] {
            clean(b);
        }
        producer.finish();
    });
}

#[test]
fn real_http_unused_cancel_dependency_cannot_revoke_original_work_or_query_result() {
    io().block_on(async {
        for retired_cancel in [false, true] {
            for case in 0..6 {
                let mut producer = Producer::new();
                let sb = bank();
                let db = bank();
                let ab = bank();
                let eb = bank();
                let declarations = [0, 1, 2].map(|_| Arc::new(Declarations::default()));
                let policies = [0, 1, 2].map(|_| Arc::new(Policy::default()));
                let services = [0, 1, 2]
                    .map(|i| retained(&sb, &declarations[i], (32, 32), policies[i].clone()));
                let definition = Rc::new(
                    prepare_operations(
                        &td(producer.address, None),
                        [
                            (&services[0], &declarations[0]),
                            (&services[1], &declarations[1]),
                            (&services[2], &declarations[2]),
                        ],
                        db.clone(),
                        &Arc::new(AtomicUsize::new(0)),
                    )
                    .unwrap(),
                );
                let owner = instance();
                let access = || ConsumerActionAccess {
                    invoke: services[0].access.clone(),
                    query: Some(services[1].access.clone()),
                    cancel: Some(services[2].access.clone()),
                };
                let association = ConsumerActionAssociation::try_new(
                    owner.reference(),
                    definition.clone(),
                    access(),
                    ab.clone(),
                    &mut budget(),
                )
                .ok()
                .unwrap();
                let calls = ConsumerActions::try_new(2, eb.clone()).unwrap();
                let h = calls
                    .invoke_action(
                        &association,
                        payload(b"12"),
                        Deadline::NONE,
                        now(),
                        &mut budget(),
                    )
                    .ok()
                    .unwrap();
                let identity = h.identity().unwrap();
                producer.drive(&h, ConsumerActionState::Pending).await;
                if retired_cancel {
                    h.request_cancel(now(), &mut budget()).unwrap();
                    producer.drive(&h, ConsumerActionState::Pending).await;
                }
                let cancellation = h.remote_cancellation().unwrap();
                match case {
                    0 => declarations[2].revision.store(1, Ordering::Relaxed),
                    1 => declarations[2].access_work.store(1, Ordering::Relaxed),
                    2 => declarations[2]
                        .access_bytes
                        .store(123456, Ordering::Relaxed),
                    3 => declarations[2].native_bytes.store(1, Ordering::Relaxed),
                    4 => declarations[2].native_work.store(1, Ordering::Relaxed),
                    _ => policies[2].denied.store(true, Ordering::Relaxed),
                }
                assert_eq!(
                    h.progress(&mut cx(), now(), &mut budget()).unwrap(),
                    ConsumerActionState::Pending,
                    "retired={retired_cancel}, case={case}"
                );
                assert!(h.first_cause().unwrap().is_none());
                if case != 5 {
                    let rebound = ConsumerActionAssociation::try_new(
                        owner.reference(),
                        definition.clone(),
                        access(),
                        ab.clone(),
                        &mut budget(),
                    );
                    if case <= 1 {
                        // Explicit new access binding qualifies its new revision/work.
                        // It cannot refresh the original association or definition.
                        assert!(rebound.is_ok());
                    } else {
                        assert!(rebound.is_err());
                    }
                    drop(rebound);
                    let p = payload(b"12").unwrap();
                    let original = p.body.clone();
                    let refusal = calls
                        .invoke_action(&association, Some(p), Deadline::NONE, now(), &mut budget())
                        .err()
                        .unwrap();
                    let (p, _) = refusal.into_parts();
                    assert!(Arc::ptr_eq(&original, &p.unwrap().body));
                }
                assert!(h.request_cancel(now(), &mut budget()).is_err());
                assert_eq!(h.state().unwrap(), ConsumerActionState::Pending);
                assert!(h.first_cause().unwrap().is_none());
                assert_eq!(h.remote_cancellation().unwrap(), cancellation);
                assert_eq!(
                    declarations[2].starts.load(Ordering::Relaxed),
                    usize::from(retired_cancel)
                );
                producer.counts.complete.set(true);
                h.query(now(), &mut budget()).unwrap();
                producer.drive(&h, ConsumerActionState::Completed).await;
                let result = h.take_settlement(now(), &mut budget()).unwrap();
                assert_eq!(result.identity(), &identity);
                assert_eq!(result.result().unwrap().data().unwrap().body, b"24");
                assert_eq!(result.remote_cancellation(), cancellation);
                assert_eq!(producer.counts.invoked.get(), 1);
                assert_eq!(producer.counts.cancelled.get(), usize::from(retired_cancel));
                assert!(
                    policies[0]
                        .log
                        .lock()
                        .unwrap()
                        .iter()
                        .all(|(_, op, _)| *op == Operation::InvokeAction)
                );
                assert!(
                    policies[1]
                        .log
                        .lock()
                        .unwrap()
                        .iter()
                        .all(|(_, op, target)| *op == Operation::QueryAction
                            && target.contains("/status?id="))
                );
                drop((h, calls, association, definition, services, owner));
                assert!(live(&sb) > 0 && live(&db) > 0);
                drop(result);
                for b in [&sb, &db, &ab, &eb] {
                    clean(b);
                }
                producer.finish();
            }
        }
    });
}

#[test]
fn real_http_query_output_uses_its_own_body_allowance_and_validator() {
    io().block_on(async {
        for (invoke_limit, query_limit, query_valid) in [(2, 8, true), (32, 1, false)] {
            let mut producer = Producer::new();
            let sb = bank();
            let db = bank();
            let ab = bank();
            let eb = bank();
            let declarations = [0, 1, 2].map(|_| Arc::new(Declarations::default()));
            declarations[0]
                .body_limit
                .store(invoke_limit, Ordering::Relaxed);
            declarations[1]
                .body_limit
                .store(query_limit, Ordering::Relaxed);
            let services = [0, 1, 2]
                .map(|i| retained(&sb, &declarations[i], (32, 32), Arc::new(Policy::default())));
            let definition = Rc::new(
                prepare_operations(
                    &td(producer.address, None),
                    [
                        (&services[0], &declarations[0]),
                        (&services[1], &declarations[1]),
                        (&services[2], &declarations[2]),
                    ],
                    db.clone(),
                    &Arc::new(AtomicUsize::new(0)),
                )
                .unwrap(),
            );
            let owner = instance();
            let association = ConsumerActionAssociation::try_new(
                owner.reference(),
                definition.clone(),
                ConsumerActionAccess {
                    invoke: services[0].access.clone(),
                    query: Some(services[1].access.clone()),
                    cancel: Some(services[2].access.clone()),
                },
                ab.clone(),
                &mut budget(),
            )
            .ok()
            .unwrap();
            let calls = ConsumerActions::try_new(1, eb.clone()).unwrap();
            let h = calls
                .invoke_action(
                    &association,
                    payload(b"12"),
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .ok()
                .unwrap();
            let identity = h.identity().unwrap();
            producer.drive(&h, ConsumerActionState::Pending).await;
            *producer.counts.output.borrow_mut() = Some(if query_valid {
                b"2400".to_vec()
            } else {
                b"24".to_vec()
            });
            producer.counts.complete.set(true);
            h.query(now(), &mut budget()).unwrap();
            let expected = if query_valid {
                ConsumerActionState::Completed
            } else {
                ConsumerActionState::Closed
            };
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                let state = h.progress(&mut cx(), now(), &mut budget()).unwrap();
                if matches!(
                    state,
                    ConsumerActionState::Completed | ConsumerActionState::Closed
                ) {
                    assert_eq!(
                        state, expected,
                        "Invoke limit {invoke_limit}, Query limit {query_limit}"
                    );
                    break;
                }
                producer.step();
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            let s = h.take_settlement(now(), &mut budget()).unwrap();
            assert_eq!(s.identity(), &identity);
            assert_eq!(s.result().is_ok(), query_valid);
            assert!(s.native_result().unwrap().is_ok());
            assert_eq!(
                declarations[0].validations.load(Ordering::Relaxed),
                1,
                "Invoke validates only its input"
            );
            assert_eq!(
                declarations[1].validations.load(Ordering::Relaxed),
                usize::from(query_valid)
            );
            drop((h, calls, association, definition, services, owner));
            assert!(live(&sb) > 0 && live(&db) > 0);
            drop(s);
            for b in [&sb, &db, &ab, &eb] {
                clean(b);
            }
            producer.finish();
        }
    });
}
