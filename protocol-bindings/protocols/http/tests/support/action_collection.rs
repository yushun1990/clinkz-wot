use clinkz_wot_binding_http::*;
use clinkz_wot_core::{action::*, consumer::*, host_accounts::*, producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::{open_consumer::*, producer_composition::*};
use clinkz_wot_servient::producer_composition::*;
use clinkz_wot_servient_finite::{consumer::*, *};
use clinkz_wot_td::{ValidatedProducerScope, thing::Thing};
use serde_json::{Value, json};
use std::{cell::{Cell, RefCell}, io::{Read, Write}, net::{SocketAddr, TcpListener, TcpStream}, rc::Rc,
    sync::{Arc, atomic::{AtomicBool, Ordering}}, task::{Context, Poll, Waker}, time::{Duration, Instant}};
fn work() -> WorkBudget { WorkClass::ALL.into_iter().fold(WorkBudget::new(), |b,c| b.with_remaining(c,1_000_000)) }
fn cx() -> Context<'static> { Context::from_waker(Waker::noop()) }
fn now() -> MonotonicInstant { MonotonicInstant::new(ClockId::new(66),0) }
fn address() -> SocketAddr { TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap() }
fn accounts() -> Arc<HostResourceAccounts> { Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS)) }
fn clean(bank: &HostResourceAccounts) { for k in ResourceKind::ALL { assert_eq!(bank.account(k).used(),0,"{k:?}"); } }
fn io() -> tokio::runtime::Runtime { tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().unwrap() }
fn denied() -> CoreError { CoreError::Security { reason: SecurityFailureReason::AuthorizationDenied,
    context: ErrorContext::new(ErrorPhase::Handler,RetryClass::Never) } }
fn authority(g: Generation) -> ProducerInstanceAuthority { ProducerInstanceAuthority::try_new(ThingSlotId::new(SlotIndex::new(1),g)).unwrap() }
fn description(addr: SocketAddr) -> Value {
    let mut td = json!({"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"Action inventory",
        "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
        "forms":[{"op":"queryallactions","href":format!("http://{addr}/actions"),"htv:methodName":"GET",
            "response":{"contentType":"application/json","htv:statusCodeValue":200}}],"actions":{}});
    for (name,kind) in [("run","integer"),("signal","string")] {
        td["actions"][name] = json!({"synchronous":false,"input":{"type":"integer"},"output":{"type":kind},
            "uriVariables":{"id":{"type":"string"}},"forms":[
                {"op":"invokeaction","href":format!("http://{addr}/{name}"),"htv:methodName":"POST"},
                {"op":"queryaction","href":format!("http://{addr}/{name}/status{{?id}}"),"htv:methodName":"GET"},
                {"op":"cancelaction","href":format!("http://{addr}/{name}/stop{{?id}}"),"htv:methodName":"DELETE"}]});
    }
    td
}
#[derive(Default)]
struct App {
    last_body: RefCell<Option<std::sync::Weak<[u8]>>>, invoked: Cell<usize>, roots: Cell<usize>, members: Cell<usize>, leaf_queries: Cell<usize>,
    deny_root: Cell<bool>, deny_member: Cell<bool>, complete: Cell<bool>, invalid: Cell<bool>,
    cancelled: Cell<usize>, long: Cell<bool>, disposed: Cell<usize>, retained: Cell<bool> }
struct Root(Rc<App>);
impl ActionQueryPolicy for Root { fn authorize(&self,_: &InteractionInput)->CoreResult<()> {
    self.0.roots.set(self.0.roots.get()+1); if self.0.deny_root.get() { Err(denied()) } else { Ok(()) }
} }
struct Behavior { app: Rc<App>, signal: bool }
impl RawActionHandler for Behavior {
    fn authorize(&self,op:Operation,_:&InteractionInput)->CoreResult<()> {
        if op == Operation::QueryAllActions { self.app.members.set(self.app.members.get()+1);
            if self.signal && self.app.deny_member.get() { return Err(denied()); } }
        if op == Operation::QueryAction { self.app.leaf_queries.set(self.app.leaf_queries.get()+1); }
        Ok(())
    }
    fn invoke(&self,_:HandlerContext<'_>,input:&InteractionInput)->CoreResult<RawActionStart> {
        self.app.invoked.set(self.app.invoked.get()+1);
        let value = serde_json::from_slice::<u32>(&input.data.as_ref().unwrap().body).unwrap();
        Ok(RawActionStart::Continue(Box::new(Job { app: self.app.clone(), signal:self.signal, value })))
    }
}
struct Job { app:Rc<App>, signal:bool, value:u32 }
impl Drop for Job { fn drop(&mut self) { self.app.disposed.set(self.app.disposed.get()+1); } }
impl RawActionWork for Job {
    fn poll(&mut self)->Poll<CoreResult<InteractionOutput>> {
        if !self.app.complete.get() { return Poll::Pending; }
        let value = if self.app.invalid.get() { b"false".to_vec() }
            else if self.signal && self.app.long.get() { serde_json::to_vec(&"x".repeat(3000)).unwrap() }
            else if self.signal { b"\"ready\"".to_vec() } else { self.value.to_string().into_bytes() };
        let payload = Payload::new(value,"application/json");
        *self.app.last_body.borrow_mut() = Some(Arc::downgrade(&payload.body));
        Poll::Ready(Ok(InteractionOutput::with_data(payload)))
    }
    fn request_cancel(&mut self)->CoreResult<()> { self.app.cancelled.set(self.app.cancelled.get()+1); Ok(()) }
    fn poll_close(&mut self)->Poll<CoreResult<()>> { if self.app.retained.get() { Poll::Pending } else { Poll::Ready(Ok(())) } }
    fn abort_local(self: Box<Self>) {}
}
fn logical(auth:ProducerInstanceAuthority,td:&Value,app:Rc<App>,root:bool)->RawProducerInstance {
    let instance = RawProducerInstance::new(auth,vec![]).with_actions(vec![
        RawActionBehavior::new(Arc::from("run"),Behavior{app:app.clone(),signal:false},0,true,true),
        RawActionBehavior::new(Arc::from("signal"),Behavior{app:app.clone(),signal:true},0,true,true)])
        .with_description(Arc::from(td.to_string()));
    if root { instance.with_action_query(Root(app)) } else { instance }
}
fn owner()->RawProducerServient {
    let mut owner=RawProducerServient::try_empty(RawProducerRegistryLimits { instances:2,behaviors_per_instance:4,target_name_bytes:64 }).unwrap();
    owner.enable_raw_actions(RawActionLimits { invocations:8,application_work_bytes:64,input_fields:8,input_bytes:4160,
        principal_id_bytes:32,output_body_bytes:4096,output_media_capacity:64,output_retained_bytes:8192 }).unwrap(); owner
}
fn prepare(td:&Value,instance:&RawProducerInstance,addr:SocketAddr)->CoreResult<(PreparedThing,Vec<Rc<dyn ProducerCapability>>)> {
    let providers:Vec<Rc<dyn ProducerCapability>>=vec![Rc::new(HttpActionProducer::new(HttpProducerConfig::for_loopback(addr).unwrap(),BindingId::new(61),BindingGeneration::INITIAL))];
    let borrowed:Vec<_>=providers.iter().map(|p|p.as_ref()).collect();
    let prepared=prepare_thing(&td.to_string(),instance.key(),GatewayDefaultV1::LIMITS,&mut work(),&borrowed,|scope,op|match scope {
        ValidatedProducerScope::Thing=>instance.supports(&AffordanceTarget::Thing,op),
        ValidatedProducerScope::Action(n)=>instance.supports_action(n,op),_=>false })?;
    Ok((prepared,providers))
}
fn exposure(owner:&RawProducerServient,auth:ProducerInstanceAuthority,td:&Value,addr:SocketAddr,app:Rc<App>,expected:Option<ProducerInstanceKey>,bank:Arc<HostResourceAccounts>)->ComposedProducerExposure {
    let instance=logical(auth,td,app,true);
    let (prepared,providers)=prepare(td,&instance,addr).unwrap();
    let definitions=prepared.operations().iter().map(|o|o.definition::<HttpActionDefinition>().unwrap().clone()).collect();
    let participant=HttpActionRuntime::try_new(owner,instance.key(),definitions,8).unwrap();
    ComposedProducerExposure::try_new(owner,ComposedProducerInput { instance,prepared,providers,
        participants:vec![Box::new(participant)],expected,handler_backing_bytes:256 },
        Arc::new(GatewayDefaultV1::LIMITS.clone()),bank,&mut work()).ok().unwrap()
}
struct Server { owner:RawProducerServient, exposure:ComposedProducerExposure, app:Rc<App>, bank:Arc<HostResourceAccounts>, addr:SocketAddr, td:Value }
impl Server {
    fn new()->Self {
        let addr=address(); let td=description(addr); let mut owner=owner(); let app=Rc::new(App::default()); let bank=accounts();
        let mut exposure=exposure(&owner,authority(Generation::INITIAL),&td,addr,app.clone(),None,bank.clone());
        exposure.participant_mut::<HttpActionRuntime>(0).unwrap().install_closed().unwrap();
        exposure.publish(&mut owner,&mut work()).unwrap();
        Self {owner,exposure,app,bank,addr,td}
    }
    fn native(&mut self)->&mut HttpActionRuntime { self.exposure.participant_mut::<HttpActionRuntime>(0).unwrap() }
    fn send(&mut self,method:&str,path:&str,body:&str)->(u16,Value,HttpActionDelivery) {
        let mut peer=TcpStream::connect(self.addr).unwrap(); peer.set_nonblocking(true).unwrap();
        peer.write_all(format!("{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n{}\r\n{body}",self.addr,body.len(),if body.is_empty(){""}else{"Content-Type: application/json\r\n"}).as_bytes()).unwrap();
        let end=Instant::now()+Duration::from_secs(5); let mut bytes=Vec::new();
        let delivery=loop {
            let native=self.exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
            let _ = native.step(&mut self.owner,&mut work()).unwrap();
            if let Some(d)=native.take_delivery(){break d;}
            assert!(Instant::now()<end);
        };
        loop { let mut buf=[0;512];match peer.read(&mut buf) {Ok(0)=>break,Ok(n)=>bytes.extend_from_slice(&buf[..n]),Err(e)if e.kind()==std::io::ErrorKind::WouldBlock=>{},Err(e)=>panic!("{e}")};assert!(Instant::now()<end); }
        let text=std::str::from_utf8(&bytes).unwrap();let status=text[9..12].parse().unwrap();
        (status,serde_json::from_str(text.split_once("\r\n\r\n").unwrap().1).unwrap(),delivery)
    }
    fn invoke(&mut self,name:&str,value:u32)->String { let (s,v,d)=self.send("POST",&format!("/{name}"),&value.to_string());assert_eq!(s,202);assert!(d.locally_written);v["id"].as_str().unwrap().into() }
    fn progress(&mut self) { let native=self.exposure.participant_mut::<HttpActionRuntime>(0).unwrap();for _ in 0..8 { let _ = native.progress_actions(&mut self.owner,&mut work()).unwrap(); } }
    async fn exchange(&mut self,call:&mut ConsumerCall)->(ConsumerSettlement,HttpActionDelivery) {
        let end=Instant::now()+Duration::from_secs(5);let mut delivered=None;
        loop {
            if let Poll::Ready(result)=call.poll(&mut cx(),now(),&mut work()) { return(result,delivered.unwrap()); }
            let native=self.exposure.participant_mut::<HttpActionRuntime>(0).unwrap();let _ = native.step(&mut self.owner,&mut work()).unwrap();
            if let Some(d)=native.take_delivery(){assert!(delivered.replace(d).is_none());}
            assert!(Instant::now()<end);tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    fn close(self) { clean(&self.retire()); }
    fn retire(mut self) -> Arc<HostResourceAccounts> {
        self.exposure.begin_close();assert!(self.exposure.retire(&mut self.owner,&mut work()).is_err()||self.native().records()==0);
        let native=self.exposure.participant_mut::<HttpActionRuntime>(0).unwrap();
        let ids:Vec<_>=native.action_ids().collect();
        for id in ids { let h=native.handle(id).unwrap();self.owner.close_raw_action(&h).unwrap();
            for _ in 0..8 { if self.owner.step_raw_action(&h,&mut work()).unwrap().is_ready(){break;} }
            drop(native.acknowledge(&mut self.owner,id).unwrap().unwrap()); }
        native.take_delivery();self.exposure.retire(&mut self.owner,&mut work()).unwrap();
        drop(self.exposure);drop(self.owner);TcpListener::bind(self.addr).unwrap();self.bank
    }
}
#[derive(Default)]
struct ConsumerPolicy { denied:AtomicBool, unavailable:AtomicBool, lose_on_validate:AtomicBool }
struct Data(Arc<ConsumerPolicy>,Arc<dyn ConsumerDataService>);
impl ConsumerDataService for Data {
    fn content_type(&self)->&str{"application/json"}
    fn policy(&self)->ConsumerDataPolicy{self.1.policy()}
    fn available(&self)->bool{!self.0.unavailable.load(Ordering::Relaxed)}
    fn validate(&self,s:value_type::TypeSchema,p:&Payload)->CoreResult<()>{self.1.validate(s,p)}
    fn supports_action_statuses(&self,s:&ActionCollectionSchema)->bool{self.1.supports_action_statuses(s)}
    fn validate_action_statuses(&self,s:&ActionCollectionSchema,p:&Payload)->CoreResult<()>{let r=self.1.validate_action_statuses(s,p);
        if self.0.lose_on_validate.swap(false,Ordering::Relaxed){self.0.unavailable.store(true,Ordering::Relaxed);}r}
}
struct Access(Arc<ConsumerPolicy>,Arc<dyn ConsumerAccessService>);
impl ConsumerAccessService for Access {
    fn authorize(&self,b:BindingRegistrationIdentity,t:&str,o:Operation)->CoreResult<()>{self.1.authorize(b,t,o)}
    fn authorize_instance(&self,c:&ConsumerInstanceRef,b:BindingRegistrationIdentity,t:&str,o:Operation)->CoreResult<()>{
        self.1.authorize_instance(c,b,t,o)?; if self.0.denied.load(Ordering::Relaxed){Err(denied())}else{Ok(())}}
    fn retained_bytes(&self)->u64{self.1.retained_bytes()+64}
    fn work(&self)->u64{self.1.work()+1}
}
fn prepare_consumer(td:&Value,bank:Arc<HostResourceAccounts>,policy:Arc<ConsumerPolicy>)->CoreResult<PreparedConsumer>{
    let config=HttpConsumerConfig::for_cleartext_ip().unwrap();let services=config.consumer_services();
    let mut registration=try_consumer_registration(config,BindingId::new(90),BindingGeneration::INITIAL,0)?;
    registration.services=ConsumerServices{data:Arc::new(Data(policy.clone(),services.data)),
        access:Arc::new(Access(policy,services.access))};
    let registry=ConsumerRegistry::try_new(vec![registration]).map_err(|e|e.error().error().clone())?;let thing:Thing=serde_json::from_value(td.clone()).unwrap();
    registry.prepare(&thing,GatewayDefaultV1::LIMITS,bank,ConsumerSelector::default(),&mut work(),|j|{
        while !j.is_finished(){j.step(&mut work(),false);}
    })
}
fn consumer(td:&Value,bank:Arc<HostResourceAccounts>,instance:&ConsumerInstance,policy:Arc<ConsumerPolicy>)->ConsumerRuntime{
    let p=prepare_consumer(td,bank.clone(),policy).unwrap();
    assert!(p.support().iter().any(|s|s.operation==Operation::QueryAllActions&&s.selected));
    let(d,f)=p.into_execution();ConsumerRuntime::try_new(instance.reference(),d,f,bank,1).ok().unwrap()
}
fn query(client:&ConsumerRuntime)->ConsumerCall{client.query_all_actions(Deadline::NONE,now(),&mut work()).unwrap()}
fn values(result:&ConsumerSettlement)->Value{serde_json::from_slice(&result.result().unwrap().data().unwrap().body).unwrap()}
