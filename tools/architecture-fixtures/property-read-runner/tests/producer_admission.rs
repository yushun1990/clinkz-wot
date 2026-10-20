//! Admission diagnostics: these assert existing defects, not safe exposure.
//! Retire/invert the defect assertions when the admitted replacement lands.
#![cfg(feature = "std")]

use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingCompilerBounds, BindingCompilerExtension,
    BindingCompilerInput, BindingCompilerStep, CoreResult, HandlerContext, HandlerFootprint,
    InteractionInput, InteractionOutput, LogicalInteractionPlan, Payload, PlanId,
    PlanSetGeneration, ReadPropertyHandler, StaticBindingCompilerRegistration, StepStatus,
};
use clinkz_wot_foundation::{
    GatewayDefaultV1, Generation, SlotIndex, StaticResourceProfile, WorkBudget, WorkClass,
};
use clinkz_wot_planning::{PlanBuildInput, PlanBuildStep, PlanCompiler, PropertyReadPlanCompiler};
use clinkz_wot_property_read_binding_fixture::{
    MockCompiler, host_property_read_fixture, static_property_read_fixture,
};
use clinkz_wot_servient::{Servient, ServientBuilder};
use clinkz_wot_td::{
    data_type::Operation,
    td_defaults::{FormContext, effective_form_operations},
    thing::Thing,
    validate::Validate,
};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    task::{Context, Waker},
};

fn document() -> Value {
    json!({"@context":["https://www.w3.org/2022/wot/td/v1.1",
        {"htv":"http://www.w3.org/2011/http#"}],
        "title":"Producer admission", "id":"urn:admission:producer",
        "securityDefinitions":{"none":{"scheme":"nosec"},"basic":{"scheme":"basic"}},
        "security":["none"],"properties":{"level":{"type":"number","readOnly":true,
        "forms":[{"href":"http://127.0.0.1:38080/level","op":["readproperty"]}]}}})
}

// Observe the complete lawful compiler input, rather than only output equality.
struct Inspect(MockCompiler);
impl BindingCompilerExtension for Inspect {
    type Cursor = <MockCompiler as BindingCompilerExtension>::Cursor;
    type Artifact = <MockCompiler as BindingCompilerExtension>::Artifact;
    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.0.compatibility()
    }
    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        assert!(
            input.resolved().is_none(),
            "Producer unexpectedly gained a semantic loan"
        );
        self.0.bounds(input)
    }
    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        self.0.start(input)
    }
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        assert!(input.resolved().is_none());
        self.0.step(input, cursor, budget)
    }
    fn abort(&self, cursor: Self::Cursor) {
        self.0.abort(cursor)
    }
}

fn plan(td: &Thing) -> LogicalInteractionPlan {
    let (registration, _) = static_property_read_fixture();
    let compiler = PropertyReadPlanCompiler::producer_route(
        PlanId::new(SlotIndex::new(0), Generation::INITIAL),
        registration.identity(),
        0,
        0,
    );
    let registrations = [StaticBindingCompilerRegistration::new(Inspect(
        MockCompiler::new(registration.identity().artifact_compatibility()),
    ))];
    let input = PlanBuildInput::new(
        td,
        registrations.as_slice(),
        PlanSetGeneration::new(Generation::INITIAL),
    );
    let mut cursor = compiler.start(&input).unwrap();
    for _ in 0..16 {
        match compiler.step(
            &input,
            cursor,
            &mut WorkBudget::new().with_remaining(WorkClass::BindingPolls, 8),
        ) {
            PlanBuildStep::Pending(next) => cursor = next,
            PlanBuildStep::Complete(output) => return output.logical_plans()[0].clone(),
            PlanBuildStep::Failed(failure) => panic!("{:?}", failure.error()),
        }
    }
    panic!("planning did not finish")
}

struct Handler(Arc<AtomicU32>);
impl ReadPropertyHandler for Handler {
    fn handle(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<InteractionOutput> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(InteractionOutput::with_data(Payload::new(
            b"42".to_vec(),
            "application/json",
        )))
    }
}
fn drive(servient: &Servient) {
    let mut cx = Context::from_waker(Waker::noop());
    for _ in 0..64 {
        let mut budget = WorkBudget::new()
            .with_remaining(WorkClass::BindingPolls, 8)
            .with_remaining(WorkClass::HandlerSteps, 1)
            .with_remaining(WorkClass::CleanupItems, 8);
        if matches!(servient.step(&mut cx, &mut budget), StepStatus::Idle) {
            return;
        }
    }
    panic!("lifecycle did not become idle")
}

#[test]
fn current_producer_loses_semantics_and_exposes_only_first_advertised_route() {
    let baseline: Thing = serde_json::from_value(document()).unwrap();
    let baseline_plan = plan(&baseline);
    for (label, field, value) in [
        ("explicit-post", "htv:methodName", json!("POST")),
        (
            "unhandled-instruction",
            "https://vendor.example/requireConfirmation",
            json!(true),
        ),
        (
            "response-media",
            "response",
            json!({"contentType":"text/plain"}),
        ),
        (
            "response-status",
            "response",
            json!({"contentType":"application/json","htv:statusCodeValue":201}),
        ),
        ("content-coding", "contentCoding", json!("gzip")),
        ("empty-scopes", "scopes", json!([])),
        ("form-basic", "security", json!(["basic"])),
        ("root-basic", "root-security", json!(["basic"])),
        ("second-form", "second-form", json!(null)),
        ("second-property", "second-property", json!(null)),
        ("default-readwrite", "default-readwrite", json!(null)),
        ("default-writeonly", "default-writeonly", json!(null)),
        ("profile", "profile", json!("urn:unsupported:profile")),
    ] {
        let mut value_td = document();
        match field {
            "root-security" => value_td["security"] = value,
            "profile" => value_td["profile"] = value,
            "second-property" => {
                value_td["properties"]["shadow"] = json!({"type":"number","readOnly":true,
                "forms":[{"href":"http://127.0.0.1:38080/shadow","op":["readproperty"]}]})
            }
            "second-form" => value_td["properties"]["level"]["forms"]
                .as_array_mut()
                .unwrap()
                .push(json!({"href":"http://127.0.0.1:38080/other","op":["readproperty"]})),
            "default-readwrite" | "default-writeonly" => {
                value_td["properties"]["level"]
                    .as_object_mut()
                    .unwrap()
                    .remove("readOnly");
                value_td["properties"]["level"]["forms"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("op");
                if field == "default-writeonly" {
                    value_td["properties"]["level"]["writeOnly"] = json!(true);
                }
            }
            _ => value_td["properties"]["level"]["forms"][0][field] = value,
        }
        let td: Thing = serde_json::from_value(value_td).unwrap();
        td.validate().unwrap();
        let property = td.properties.as_ref().unwrap().get("level").unwrap();
        let operations = effective_form_operations(
            FormContext::Property(property),
            &property._interaction.forms[0],
        );
        if field == "default-writeonly" {
            assert!(!operations.contains(&Operation::ReadProperty));
        }
        if field == "default-readwrite" {
            assert!(operations.contains(&Operation::WriteProperty));
        }
        assert_eq!(
            plan(&td),
            baseline_plan,
            "{label}: compiler distinctions lost"
        );
        let (registration, probe) = host_property_read_fixture();
        let servient = ServientBuilder::new()
            .resource_limits(GatewayDefaultV1::LIMITS.clone())
            .binding_registration(registration)
            .build()
            .unwrap();
        let exposed = servient.produce_td(td).unwrap();
        let calls = Arc::new(AtomicU32::new(0));
        exposed
            .set_read_property_handler(
                "level",
                Handler(calls.clone()),
                HandlerFootprint::new(1, 0, 0),
            )
            .unwrap();
        exposed.begin_expose().unwrap();
        drive(&servient);
        assert_eq!(probe.preparation_side_effects(), 1, "{label}");
        probe.enqueue_property_read("level", InteractionInput::empty());
        drive(&servient);
        assert_eq!(calls.load(Ordering::SeqCst), 1, "{label}");
        assert_eq!(probe.delivered_responses(), 1, "{label}");
        exposed.begin_destroy().unwrap();
        drive(&servient);
        assert_eq!(probe.outstanding_counts(), (0, 0, 0, 0));
        assert_eq!(probe.route_state_drops(), 1);
        println!(
            "{label}: Basic-valid, semantic loan absent, equal plan, prepare=handler=response=1; cleanup complete"
        );
    }
}
