//! Qualify a complete TD, then emit to an independently subscribed local peer.
use clinkz_wot_binding_mqtt::*;
use clinkz_wot_core::{producer::ProducerInstanceAuthority, publication::PublicationData, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::producer_composition::*;
use clinkz_wot_servient::{host_accounts::HostResourceAccounts, producer_composition::*};
use clinkz_wot_servient_finite::*;
use clinkz_wot_td::ValidatedProducerScope;
use std::{
    net::SocketAddr,
    rc::Rc,
    sync::Arc,
    task::{Context, Waker},
    time::{Duration, Instant},
};

fn budget() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, c| b.with_remaining(c, 1_000_000))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let address: SocketAddr = args
        .next()
        .unwrap_or_else(|| "127.0.0.1:1883".into())
        .parse()?;
    let topic = args.next().unwrap_or_else(|| "things/alarm".into());
    let value = args.next().unwrap_or_else(|| "42".into());
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let authority=ProducerInstanceAuthority::try_new(ThingSlotId::new(SlotIndex::new(1),Generation::INITIAL))?;
        let key=authority.key();
        let description=serde_json::json!({
            "@context":["https://www.w3.org/2022/wot/td/v1.1",{"mqv":"https://www.w3.org/2019/wot/mqtt#"}],
            "title":"Alarm source", "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
            "events":{"alarm":{"data":{"type":"integer"},"forms":[{
                "href":format!("mqtt://{address}"),"mqv:filter":topic,"mqv:qos":"0","mqv:retain":false
            }]}}
        }).to_string();
        let instance=RawProducerInstance::new(authority,vec![])
            .with_notification_sources(vec![ProducerNotificationSource::event("alarm")]).with_description(Arc::from(description));
        let provider=Rc::new(MqttNotificationProducer::new(MqttProducerConfig::for_loopback(address,8192)?,BindingId::new(1),BindingGeneration::INITIAL));
        let service=MqttNotificationService::new(&provider)?;
        let prepared=prepare_thing(instance.description().expect("staged TD"),key,GatewayDefaultV1::LIMITS,
            &mut budget(),&[provider.as_ref()],|s,op| matches!(s,ValidatedProducerScope::Event(n) if instance.supports_notification(&AffordanceTarget::Event(Arc::from(n)),op)))?;
        let definitions=prepared.operations().iter().map(|o| o.definition::<MqttNotificationDefinition>().expect("selected MQTT").clone()).collect();
        let mut owner=RawProducerServient::try_empty(RawProducerRegistryLimits {instances:1,behaviors_per_instance:1,target_name_bytes:64})?;
        let native=service.participation(&owner,key,definitions)?;
        let accounts=Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let mut exposure=ComposedProducerExposure::try_new(&owner,ComposedProducerInput {instance,prepared,
            providers:vec![provider],participants:vec![Box::new(native)],expected:None,handler_backing_bytes:0},
            Arc::new(GatewayDefaultV1::LIMITS.clone()),accounts,&mut budget()).map_err(|r| r.error().error().clone())?;
        let mut cx=Context::from_waker(Waker::noop());
        let end=Instant::now()+Duration::from_secs(3);
        loop {
            let native=exposure.participant_mut::<MqttNotificationRuntime>(0).expect("owned MQTT");
            let _=native.step(&mut owner,&mut cx,&mut budget())?;
            if let Some(error)=native.first_failure() { return Err(error.clone().into()); }
            match exposure.publish(&mut owner,&mut budget()) {
                Ok(())=>break,
                Err(CoreError::Backpressure(_)) if Instant::now()<end=>{},
                Err(error)=>return Err(error.into()),
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let now=MonotonicInstant::new(ClockId::new(1),0);
        let native=exposure.participant_mut::<MqttNotificationRuntime>(0).expect("owned MQTT");
        let mut emission=owner.publish_notification(native,key,AffordanceTarget::Event(Arc::from("alarm")),PublicationData::EventData(Payload::new(value.into_bytes(),"application/json")),
            Deadline::NONE,now,&mut budget()).map_err(|r| r.error().error().clone())?;
        while emission.step(&mut cx,&mut budget(),now).is_pending() {
            if Instant::now()>=end { emission.cancel(); }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let settlement=emission.acknowledge().ok().expect("terminal Event acknowledgement");
        println!("Local Event publication: {:?}; subscriber delivery is observed separately",settlement.native.effect);
        exposure.begin_close();
        let native=exposure.participant_mut::<MqttNotificationRuntime>(0).expect("owned MQTT");
        let _=native.step(&mut owner,&mut cx,&mut budget())?;
        exposure.retire(&mut owner,&mut budget())?;
        if let Some(error)=settlement.cause { return Err(error.into()); }
        Ok(())
    })
}
