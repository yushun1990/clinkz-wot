use super::*;
use clinkz_wot_core::{
    consumer::{
        ConsumerDefinition, ConsumerDefinitionInput, ConsumerExecutionContent,
        ConsumerFiniteSchemas, ConsumerInstance, ConsumerServiceOwner, PreparedConsumerContent,
    },
    data_schema::{PreparedSchema, ValueSchemas, ValueType},
    host_accounts::HostResourceAccounts,
};
use clinkz_wot_servient_finite::consumer::{ConsumerCalls, StandaloneConsumerAssociation};
use std::{
    any::Any,
    io::{Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};

// This test image is already compiled native content. Its owner has no
// preparation dependencies and does not expose any arbitrary native format.
struct PreparedInstructions(HttpArtifact);
impl HostArtifactStorage for PreparedInstructions {
    fn storage_overhead(&self) -> u64 {
        0
    }
    fn payload(&self) -> &dyn Any {
        &self.0
    }
    fn take_payload(&mut self, destination: &mut dyn Any) {
        *destination.downcast_mut::<Option<HttpArtifact>>().unwrap() = Some(self.0);
    }
}

fn prepared_artifact(
    identity: BindingRegistrationIdentity,
    target: &str,
) -> BindingArtifactEnvelope<HostBindingArtifact> {
    let plan = PlanId::new(SlotIndex::new(13), Generation::INITIAL);
    let mut program = HttpArtifact::default();
    program.target[..target.len()].copy_from_slice(target.as_bytes());
    program.target_len = target.len() as u16;
    let bounds = BindingArtifactFootprint::new(1, size_of::<HttpArtifact>() as u64);
    BindingArtifactEnvelope::try_new(
        BindingArtifactIdentity::new(
            PlanSetGeneration::INITIAL,
            plan,
            identity.binding_id(),
            identity.binding_generation(),
            identity.configuration(),
            identity.artifact_compatibility(),
            BindingArtifactRole::ConsumerCall,
        ),
        bounds,
        BindingArtifact::new(
            COMPATIBILITY,
            bounds,
            HostBindingArtifact::from_storage(Box::new(PreparedInstructions(program))),
        ),
    )
    .unwrap()
}

#[test]
fn prepared_http_read_uses_the_same_application_execution_without_preparation() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let target = format!("http://{}/value", listener.local_addr().unwrap());
    let config = HttpConsumerConfig::for_cleartext_ip().unwrap();
    let id = BindingId::new(601);
    let execution =
        Arc::new(try_consumer_execution(config, id, BindingGeneration::INITIAL, 0).unwrap());
    let artifact = prepared_artifact(execution.identity(), &target);
    let plan = artifact.identity().plan_id();
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let services = config.consumer_services();
    let data =
        ConsumerServiceOwner::retain_data(services.data, &bank).unwrap_or_else(|_| panic!("data"));
    let access = ConsumerServiceOwner::retain_access(services.access, &bank)
        .unwrap_or_else(|_| panic!("access"));
    let content = ConsumerExecutionContent::try_new(
        AffordanceTarget::Property("value".into()),
        Operation::ReadProperty,
        ConsumerFiniteSchemas::Value(ValueSchemas {
            input: None,
            output: Some(PreparedSchema::Type(ValueType::Integer)),
        }),
    )
    .unwrap();
    let mut input = ConsumerDefinitionInput {
        content: PreparedConsumerContent::try_new(content, &bank)
            .unwrap_or_else(|_| panic!("content")),
        form: 1,
        target: target.into(),
        artifact,
        execution: execution.clone(),
        configuration: config.configuration,
        data,
        content_type: "application/json".into(),
        execution_funding: Arc::new(Vec::new()),
        funding: Vec::new(),
    };
    for foreign in [
        try_consumer_execution(
            config,
            id,
            BindingGeneration::new(Generation::INITIAL.checked_next().unwrap()),
            0,
        )
        .unwrap(),
        try_consumer_execution(
            config.with_target_capacity(128).unwrap(),
            id,
            BindingGeneration::INITIAL,
            0,
        )
        .unwrap(),
    ] {
        input.execution = Arc::new(foreign);
        input = ConsumerDefinition::try_new(input)
            .err()
            .expect("foreign generation or configuration")
            .into_input();
    }
    input.execution = execution;
    let mapping =
        Arc::new(ConsumerDefinition::try_new(input).unwrap_or_else(|r| panic!("{:?}", r.error())));
    let instance = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
    let association = StandaloneConsumerAssociation::try_new(
        instance.reference(),
        vec![
            mapping
                .bind(access, bank.clone())
                .unwrap_or_else(|r| panic!("{:?}", r.error())),
        ],
        Vec::new(),
        bank.clone(),
    )
    .unwrap_or_else(|r| panic!("{:?}", r.error()));
    let read = association.handle().bind_property_read("value").unwrap();
    let calls = ConsumerCalls::try_new(bank.clone(), 1).unwrap();

    listener.set_nonblocking(true).unwrap();
    let peer = std::thread::spawn(move || {
        let end = Instant::now() + Duration::from_secs(5);
        let mut socket = loop {
            match listener.accept() {
                Ok((socket, _)) => break socket,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < end);
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => panic!("{e}"),
            }
        };
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            socket.read_exact(&mut byte).unwrap();
            head.push(byte[0]);
            assert!(head.len() < 2048);
        }
        assert!(head.starts_with(b"GET /value HTTP/1.1\r\n"));
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n7",
            )
            .unwrap();
        assert_eq!(socket.read(&mut [0]).unwrap(), 0);
    });
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let now = MonotonicInstant::new(ClockId::new(601), 0);
            let mut call = read.read(&calls, Deadline::NONE, now).unwrap();
            let result = tokio::time::timeout(
                Duration::from_secs(5),
                std::future::poll_fn(|cx| call.poll(cx, now)),
            )
            .await
            .unwrap()
            .unwrap();
            let output = result.result().unwrap();
            assert_eq!(output.data().unwrap().body, b"7");
            assert_eq!(
                output.metadata().binding_response().unwrap().plan_id(),
                plan
            );
            drop(call);
            let outstanding = ResourceKind::PendingHandlerCallsGlobalMax;
            assert_eq!(bank.account(outstanding).used(), 1);
            let pressure = bank
                .reserve(outstanding, bank.account(outstanding).limit() - 1)
                .unwrap();
            assert!(matches!(
                read.read(&calls, Deadline::NONE, now),
                Err(CoreError::LimitExceeded { resource, .. }) if resource == outstanding
            ));
            drop(result);
            let next = read.read(&calls, Deadline::NONE, now).unwrap();
            drop((next, pressure));
        });
    peer.join().unwrap();
    drop((read, association, calls, instance));
    assert!(
        ResourceKind::ALL
            .into_iter()
            .all(|kind| bank.account(kind).used() == 0)
    );
}
