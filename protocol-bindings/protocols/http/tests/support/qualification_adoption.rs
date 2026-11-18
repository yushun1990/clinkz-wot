//! On 5d7796e these providers inherit revision zero and remain source-compatible
//! while withdrawn support is silently accepted. Each test must reject it.
use super::*;

struct MutableData {
    actual: Arc<dyn ConsumerDataService>,
    revision: Arc<AtomicU64>,
}
impl ConsumerDataService for MutableData {
    fn content_type(&self) -> &str { self.actual.content_type() }
    fn policy(&self) -> ConsumerDataPolicy { self.actual.policy() }
    fn supports_schema(&self, schema: &PreparedSchema) -> bool {
        self.revision.load(Ordering::Acquire) == 0 && self.actual.supports_schema(schema)
    }
    fn validate(&self, schema: &PreparedSchema, value: &Payload) -> CoreResult<()> {
        self.actual.validate(schema, value)
    }
}
struct MutableAccess {
    actual: Arc<dyn ConsumerAccessService>,
    revision: Arc<AtomicU64>,
}
impl ConsumerAccessService for MutableAccess {
    fn supports_security(&self, requirement: &security::SecurityRequirement) -> bool {
        self.revision.load(Ordering::Acquire) == 0 && self.actual.supports_security(requirement)
    }
    fn authorize(&self, b: BindingRegistrationIdentity, t: &str, o: Operation) -> CoreResult<()> {
        self.actual.authorize(b,t,o)
    }
    fn authorize_instance(&self, c: &ConsumerInstanceRef, b: BindingRegistrationIdentity, t: &str, o: Operation) -> CoreResult<()> {
        self.actual.authorize_instance(c,b,t,o)
    }
    fn retained_bytes(&self) -> u64 { self.actual.retained_bytes() }
    fn work(&self) -> u64 { self.actual.work() }
}
struct MutableClient {
    actual: LocalClient,
    revision: Arc<AtomicU64>,
}
impl binding::ClientBinding for MutableClient {
    fn supports_operation(&self, op: Operation) -> bool {
        self.revision.load(Ordering::Acquire) == 0 && self.actual.supports_operation(op)
    }
    fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> { self.actual.consumer_configuration() }
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility { self.actual.artifact_compatibility() }
    fn invoke(&self, request: OutboundRequest, artifact: &BindingArtifactEnvelope<HostBindingArtifact>) -> Result<HostBindingCallBox<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>> {
        self.actual.invoke(request, artifact)
    }
}

fn withdrawn_support(dependency: &str) {
    for unread in [false, true] {
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let owner = consumer_instance();
        let probe = Arc::new(Probe::default());
        let revision = Arc::new(AtomicU64::new(0));
        let mut input = try_consumer_registration(HttpConsumerConfig::for_cleartext_ip().unwrap(), BindingId::new(106), BindingGeneration::INITIAL,106).unwrap();
        if dependency == "data" {
            input.services.data = Arc::new(MutableData { actual: input.services.data, revision: revision.clone() });
        }
        if dependency == "security" {
            input.services.access = Arc::new(MutableAccess { actual: input.services.access, revision: revision.clone() });
        }
        let (runtime, compiler) = input.preparation.into_parts();
        let client = MutableClient { actual: LocalClient {
            configuration: runtime.consumer_configuration().unwrap(),
            compatibility: runtime.identity().artifact_compatibility(),
            probe: probe.clone(),
        }, revision: if dependency == "native" {revision.clone()} else {Arc::new(AtomicU64::new(0))} };
        let execution = HostConsumerBindingRegistration::new(HostConsumerBindingRegistrationInput::new(runtime.identity(), runtime.execution(), Box::new(client), runtime.resources(), runtime.status())).unwrap_or_else(|_| panic!());
        input.preparation = HostConsumerBindingPreparation::try_pair(execution, compiler).unwrap_or_else(|_| panic!());
        let registry = ConsumerRegistry::try_new(vec![input], &bank).unwrap_or_else(|_| panic!());
        let td = serde_json::from_value(json!({"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"revision adoption", "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},"properties":{"level":{"type":"integer","readOnly":true,"forms":[{"href":"http://127.0.0.1:10600/level"}]}}})).unwrap();
        let prepared = registry.prepare(&td, GatewayDefaultV1::LIMITS, bank.clone(), ConsumerSelector::default(), &mut work(), |job| { while !job.is_finished() {job.step(&mut work(), false);} }).unwrap();
        drop((td,registry));
        let (definitions,funding) = prepared.into_execution();
        let association = StandaloneConsumerAssociation::try_new(owner.reference(), definitions, funding, bank.clone()).unwrap_or_else(|_| panic!());
        let read = association.bind_property_read("level", &mut work()).unwrap();
        let calls = ConsumerCalls::try_new(bank.clone(),1).unwrap();
        if unread {
            let mut call = read.read(&calls,Deadline::NONE,now(),&mut work()).unwrap();
            calls.step(&mut cx(), now(), &mut work());
            revision.fetch_add(1,Ordering::AcqRel);
            let result = settle(&mut call);
            assert!(result.result().is_err(), "withdrawn {dependency} support delivered unread success");
            drop((result,call));
        } else {
            revision.fetch_add(1,Ordering::AcqRel);
            assert!(read.read(&calls,Deadline::NONE,now(),&mut work()).is_err(), "withdrawn {dependency} support admitted native work");
            assert_eq!(probe.starts.load(Ordering::Relaxed),0);
        }
        drop((read,association,calls,owner));
        clean(&bank);
    }
}
fn consumer_instance() -> ConsumerInstance {
    ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0),Generation::INITIAL))
}
#[test]
fn withdrawn_data_support_refuses_admission_and_unread_result() { withdrawn_support("data"); }
#[test]
fn withdrawn_native_support_refuses_admission_and_unread_result() { withdrawn_support("native"); }
#[test]
fn withdrawn_security_support_refuses_admission_and_unread_result() { withdrawn_support("security"); }
