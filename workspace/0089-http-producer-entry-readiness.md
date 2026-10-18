# 0089 HTTP Producer Property Read entry readiness

Status: OPEN

## Finding and decision requested

The production HTTP Producer tranche cannot yet be admitted under ADR-0013.
The existing route/permit/response ownership topology is constructible, but the
production Producer planning path loses facts needed to reject unsupported
HTTP Forms and does not enforce its single-route coverage boundary. Correct
that narrow semantic entry boundary before implementing the HTTP server.
This is an investigation and correction recommendation, not implementation
admission or an architecture/gate verdict change.

Evidence baseline: fetched `master` at
`22047e07dbd71b97af38b5ddc5c8e3a3755e7352` (PR #203). The completed
`WP-600-HTTP-CONSUMER-PROPERTY-READ` tranche and its accepted evidence are reused
at their registered revisions. This investigation changes no completed status.

## Reconstructed architecture and implementation

| Boundary | Production owner and observed behavior |
| --- | --- |
| Registration | `core/src/binding.rs::HostBindingRegistration` captures compiler, route server, capabilities, execution, resources, ingress and status as one checked bundle. A Producer-only bundle needs no dummy client. HTTP currently supplies only the separate Consumer registration. |
| Compilation | `planning/src/property_read.rs::PropertyReadPlanCompiler::producer_route` creates an eager `ProducerRoute` artifact with its compiler-owned `RouteReservationIdentity`. `servient/src/property_read.rs::step_planning` freezes the real output, resolves the artifact envelope against registration/plan/route identities and funds first entry before `prepare`. The binding receives a scoped artifact borrow and retains derived owned state. |
| Route lifecycle | `RouteServerBinding` owns protocol preparation, readiness, activation, commit, accept, response and abort/shutdown calls. Prepared, active and committed guards move one Core-owned carrier with unchanged `PrepareInput`, footprint and pinned shared typed state. Interior protocol mutation does not replace the linear guard or retain a plan-set/TD borrow. |
| Publication and permits | Commit returns committed-closed. Servient publishes its plan set and activation record before entering Serving. Only its Serving transition borrows the exact route lease, claims the matching `ServingActivationAuthority` and passes a temporary `RouteActivationPermit` with a shared committed guard to `poll_accept`. The permit cannot be retained. Destroy closes issuance before shutdown. Binding-local network callbacks have no handler/registry authority. |
| Dispatch and response | Binding returns an owned `RouteInboundRequest` with route, target, correlation and single-use `RouteResponseOpportunity`. Servient checks route/target, retains the request across exhausted handler credit, constructs `HandlerContext` and invokes the handler. Core seals its result into `RouteInboundResponse`; the binding cannot supply unchecked handler success. `deliver_response` rejects with the full response or accepts an owned Host call. Servient keeps route/correlation through delivery and cancellation settlement. |
| Public Host composition | `ServientBuilder::resource_limits(...).binding_registration(...).build()`, `Servient::produce_td`, `ExposedThingHandle::set_read_property_handler`, `begin_expose`, `Servient::step`, and `begin_destroy` reach this target path. Legacy `with_server_binding`/`ServerBinding::serve` and the legacy async expose dispatcher are different paths and cannot establish target completion. |
| Cleanup | Each lifecycle call returns its predecessor on rejection or carries its complete successor/cancellation state. Servient cancels/joins outstanding calls before route cleanup, retains prebacked manual/transfer/status ownership and releases the plan only after terminal disposition. Real socket/SDK retirement and last-root destruction remain production evidence obligations. |

The passed [Producer gate](../docs/work-packages/property-read-architecture-gate.toml)
registers the completed handler, planning, route/reservation, binding, sealing
and Servient dependencies. Its external static/Host fixtures and the real
Zenoh feedback remain architecture evidence within their own scopes, not HTTP
SDK qualification. No new route SPI or Servient executor is justified here.

## Demonstrated admission blocker

`make_property_read_plan` copies target, media and subprotocol through
`LogicalInteractionPlan::try_property_read`. That constructor initializes
`additional_form_metadata` to `None` (unknown). The Producer caller never
populates the existing field. Method/header extensions, expected responses,
content coding and scopes therefore disappear; effective security is also
absent. The Consumer aggregate's paid eligibility projection is a different
implemented path and does not repair the Producer leaf.

The same leaf loops properties/Forms and returns the first read candidate.
It neither builds the remaining advertised routes nor rejects a TD outside
its one-route envelope. Servient checks that first plan against its one handler;
it does not prove coverage of the other advertised Forms before preparation.

The reproducible public-API probe below establishes all of the following:

- The baseline is a Basic-valid read-only number property with one
  `readproperty` Form, default JSON, default method, NoSec and an IP HTTP target.
- Each of eight mutations remains Basic-valid: explicit GET, explicit POST,
  primary response override, gzip coding, present-empty scopes, Form Basic
  security, inherited root Basic security, and a second read property.
- Every mutation produces an exactly equal `LogicalInteractionPlan` under the
  same registration/plan/generation, with metadata still `None`. The compiler
  input contains only that plan, candidate and role, so a real HTTP compiler
  cannot distinguish those cases using its legal input.
- The existing public Host path with the existing mock registration prepares a
  route, dispatches one injected read and settles its response for every case;
  destroy releases the route once. There is no earlier generic eligibility or
  whole-TD coverage barrier in this path.

The mock is deliberately unchanged and has no HTTP interpretation claim. This
is not an HTTP exploit, SDK result, or invalidation of its accepted ownership
proof. Equality of the production semantic projection is the discriminator.
A strict HTTP compiler would have to reject even the valid baseline, or treat
unknown as default and silently accept unsupported declarations. Checking the
application TD in an HTTP factory, passing a caller promise, rescanning it in
the binding, or synthesizing a test-local plan would bypass accepted ownership.

This intersects `PLAN-ARTIFACT-001`, `PLAN-COST-003`, `FORM-COVERAGE-001` and
Planning's explicit rule that omitted protocol-significant facts are never
defaults. It prevents ADR-0013's stable/closed implementation boundary and
pre-code evidence conditions from being met for the production tranche.

## Smallest correction recommended for scoped admission

Retain the existing Producer route lifecycle and response contracts. Amend only
the bounded Producer semantic entry path, with TD owning effective/default
meaning, Planning preserving the needed facts and the binding compiler owning
HTTP interpretation:

1. Establish and enforce the narrow whole-draft coverage envelope before any
   binding side effect. For the first tranche, reject extra properties/Forms,
   actions/events/root operations or additional effective operations that this
   one-route implementation cannot cover. Do not silently keep the first Form.
   A correct rejection is smaller than implementing multi-route scheduling.
2. Populate the existing owned Form-metadata fact from the actual selected
   Form, including present-empty cases. Preserve unknown-versus-absent and
   compiler refusal; reject unsupported metadata instead of silently defaulting.
   Reuse TD's semantics and paid work discipline; do not add an HTTP branch to
   neutral crates or expose raw TD data to the compiler.
3. Prove the selected Form's effective NoSec eligibility, including inheritance
   and Form overrides, before route preparation. Retain the minimal immutable
   protocol-neutral fact or checked eligibility proof needed by this entry.
   An application-supplied security assertion is insufficient. Determine the
   smallest carrier/API amendment with an executable discriminator before
   recording correction admission; no new security framework is needed.
4. Bound and charge validation/projection work, retained facts and rollback;
   zero/short budget and cancellation must not reach route effects. Preserve
   source-independent artifacts, exact configuration/generation checks and
   the existing resource/profile numeric authority.

Merely setting metadata to `Some(false)` does not fix security or coverage.
A binding-specific preflight wrapper would leave the ordinary public Producer
entry unchecked. Broad borrowed-Consumer migration, generalized TD ingestion,
Contracts extraction and multi-route expansion are disproportionate remedies.

Likely affected owners are TD effective semantics, Planning's Producer leaf,
its minimal Core projection if necessary, and the public Servient entry's
coverage admission. The later correction admission must name exact paths,
requirements, impacted completed evidence and API/resource/work projections.
Prefer an additive fact/rejection boundary; only demonstrated shared layout or
semantic changes require renewing dependent evidence. Consumer production
qualification is not automatically reopened.

## Smallest viable production successor after correction

The useful first HTTP Producer tranche is one explicit cleartext Host IP
listener, one produced Thing with exactly one read-only property/Form, one
complete Producer-only registration, one route and one request/response
opportunity at a time, reused for sequential reads. Use metadata-free default
GET/NoSec/JSON and one fresh HTTP/1.1 connection per read, with bounded ingress
and response payload, explicit application-owned runtime progress and finite
shutdown. Use a configured nonzero endpoint known before freeze; ephemeral
port assignment/form generation and shared listener routing are not required.
Preserve immutable endpoint reservation and collision identity.

Supply the HTTP server/compiler inside the existing optional HTTP crate,
using the public Host binding and Servient APIs above. Choose and pin the
server SDK/features/configuration during that implementation; enabling Hyper's
server feature would require its own real evidence. The current client-only
SDK qualification does not qualify a server. No neutral crate takes an HTTP,
Tokio or TLS dependency. A dual-role binding bundle is unnecessary: the accepted
Consumer and the new Producer can be independently registered and manually
progressed by the application over actual loopback TCP.

The existing Producer registration accepts only `StrictV1` resource semantics
(`HostBindingRegistration::new`); schema-2 owned contracts are expressly
Consumer-only. Do not copy `ConsumerOwnedHostV2` into the Producer bundle or
transfer the Consumer SDK disposal relaxation implicitly. Producer
implementation must meet the current strict contract. If a selected server
SDK demonstrates a conflicting allocation/disposal/resource requirement,
preserve that counterexample and seek the smallest separately reviewed
Producer resource amendment before effects. This is an explicit constraint,
not a demonstrated universal HTTP SDK incompatibility.

Dependencies for that future admission are the Producer gate's registered
completed dependencies, completed/current WP-000 and
`WP-600-HTTP-CONSUMER-PROPERTY-READ`, plus the completed semantic-entry correction
above. WP-600 retains its WP-300 package-completion dependency; broad WP-300 or
WP-400 completion, an SDK repair package, or a repeated Consumer acceptance
cycle is not an additional predecessor.

Topics 0076 and 0081's broader composition/reuse proposals are disjoint from
this one-Thing/one-route composition. Topic 0082 concerns subscriptions and 0083
is deferred Zenoh ownership. This new topic is the concrete intersecting entry
finding. The existing multi-owner checkpoint remains separate. HTTPS/trust,
explicit HTTP vocabulary including explicit GET, DNS, additional operations,
subscriptions, embedded qualification and release readiness remain successors.

## Required evidence for the later admission and implementation

These are proposed completion boundaries for review, not registered admission
keys or produced completion results. A later ADR-0013 record must freeze exact
keys/output paths and executable prechecks once the correction is resolved.

| Proposed key | Falsifiable completion boundary |
| --- | --- |
| `http-producer-form-eligibility` | Production default-only positive; method/header/response/coding/subprotocol/scopes/security/coverage negatives before any route effect. Include inherited/override security, present-empty and unknown facts, second property/Form/operation and late bad input. Zero/short budgets, cancellation, funded rejection/rollback and static/Host projection parity. |
| `http-producer-property-read` | Production compiler-derived reservation and artifact reach real prepare/readiness/activate/commit-closed; no pre-publication handler dispatch; fresh permits match exact generations; handler called once; Core-sealed success/failure delivered once on the original correlation. Exercise stale/mismatched permit refusal with no state changes and explicit prepared-ingress policy. |
| `http-public-property-read-round-trip` | Real HTTP Consumer registration plus real HTTP Producer registration through public Runtime/Servient, actual loopback TCP and application handler; destroy Consumer TD/build inputs after consumption. At least 100 sequential reads with distinct changing payloads, saturation/reuse and terminal-account/FD checks. Independently implemented raw HTTP peer verifies server method/target/status/media/framing; a raw client alone does not close the public bidirectional loop. |
| `http-producer-resource-lifecycle` | Exact/one-over request line/header/target/body/connection/response limits, malformed/fragmented/truncated/slow input and declared no-body/unsupported-method policy, disconnect before/during response, bind conflict, readiness/activation/commit failure, cancel/destroy races and last-root loss. No stale correlation/replay, hidden ingress, background handler dispatch or detached cleanup. Actual SDK backing/growth/overlap/native-stack/disposal and engine Layout/funding/physical-retirement measurements at exact dependency versions under the admitted Producer resource contract; truthful failures/residuals and original-account release. |
| `http-producer-authoring-feature-conformance` | External application constructs only the required roles, supplies runtime/clock driving and documented cleartext permission/deployment limits; normal dependency graph is registry-resolvable. Focused shared Producer tests, portable projection cells and existing authority/performance harness pass. Preserve Consumer completion and rerun its focused integration only if the new server feature/dependency unification or shared change actually affects it. |

Risk-based pre-code checks are the documented counterexample, current authority
checks, focused Core guard/sealing tests, Planning Producer tests, public
Servient Property Read tests and the existing external architecture runner.
Portable cells apply to changed shared contracts, not an embedded HTTP server.
Reuse registered workloads for compiler/metadata, owned calls, plan lifecycle,
activation and ingress (`PERF-GW-009/010/028/029/030/031/032` and
`PERF-CS-020/021/022/023`) where affected. Their existing harness excludes
network I/O and establishes no HTTP throughput budget. Measure the new real
transport separately without inventing a benchmark framework.

## Review and disposition

Independent scoped review must reconstruct the exact PR diff and baseline,
rerun the counterexample and applicable pre-code checks, challenge the proposed
minimum correction, and post validation results and unresolved issues directly
in PR comments. It must distinguish support for the blocker diagnosis from
implementation admission. The author does not self-approve, merge or change
package/tranche/gate states. This topic remains OPEN until a reviewed correction
is migrated into its authoritative owners under ADR-0013.

## Reproduce the public-path discriminator

Run from the repository root at the recorded baseline. This creates only a
temporary external diagnostic crate using existing production code and the
existing mock registration; it writes no repository source/test/Cargo files.
It uses the baseline workspace lock as the dependency seed, generates the
external crate's lock offline, then runs with `--locked`. No HTTP listener or
production server is implemented by this recipe.

```sh
set -eu
producer_repo=$(pwd)
producer_probe=$(mktemp -d /tmp/clinkz-http-producer-entry.XXXXXX)
trap 'rm -rf "$producer_probe"' EXIT
mkdir -p "$producer_probe/src"
cat > "$producer_probe/Cargo.toml" <<EOF_MANIFEST
[package]
name = "http-producer-entry-probe"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
clinkz-wot-core = { path = "$producer_repo/core" }
clinkz-wot-foundation = { path = "$producer_repo/foundation" }
clinkz-wot-planning = { path = "$producer_repo/planning" }
clinkz-wot-servient = { path = "$producer_repo/servient" }
clinkz-wot-td = { path = "$producer_repo/td" }
clinkz-wot-property-read-binding-fixture = { path = "$producer_repo/tools/architecture-fixtures/property-read-binding" }
serde_json = "=1.0.149"
EOF_MANIFEST
cat > "$producer_probe/src/main.rs" <<'EOF_RUST'
use clinkz_wot_core::{CoreResult, HandlerContext, HandlerFootprint, InteractionInput,
    InteractionOutput, LogicalInteractionPlan, Payload, PlanId, PlanSetGeneration,
    ReadPropertyHandler, StepStatus};
use clinkz_wot_foundation::{GatewayDefaultV1, Generation, SlotIndex, StaticResourceProfile,
    WorkBudget, WorkClass};
use clinkz_wot_planning::{PlanBuildInput, PlanBuildStep, PlanCompiler, PropertyReadPlanCompiler};
use clinkz_wot_property_read_binding_fixture::{host_property_read_fixture, static_property_read_fixture};
use clinkz_wot_servient::{Servient, ServientBuilder};
use clinkz_wot_td::{thing::Thing, validate::Validate};
use serde_json::{Value, json};
use std::{sync::{Arc, atomic::{AtomicU32, Ordering}}, task::{Context, Waker}};

fn document() -> Value {
    json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "id":"urn:probe:producer",
        "title":"Producer entry", "securityDefinitions":{"none":{"scheme":"nosec"},
        "basic":{"scheme":"basic","in":"header"}}, "security":["none"],
        "properties":{"level":{"type":"number","readOnly":true,
        "forms":[{"href":"http://127.0.0.1:38080/level","op":["readproperty"]}]}}})
}
fn plan(td: &Thing) -> LogicalInteractionPlan {
    let (registration, _) = static_property_read_fixture();
    let compiler = PropertyReadPlanCompiler::producer_route(
        PlanId::new(SlotIndex::new(0), Generation::INITIAL), registration.identity(), 0, 0);
    let registrations = [registration.compiler()];
    let input = PlanBuildInput::new(td, registrations.as_slice(),
        PlanSetGeneration::new(Generation::INITIAL));
    let mut cursor = compiler.start(&input).unwrap();
    for _ in 0..16 {
        match compiler.step(&input, cursor,
            &mut WorkBudget::new().with_remaining(WorkClass::BindingPolls, 8)) {
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
        Ok(InteractionOutput::with_data(Payload::new(b"42".to_vec(), "application/json")))
    }
}
fn drive(servient: &Servient, cx: &mut Context<'_>) {
    for _ in 0..64 {
        let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 8)
            .with_remaining(WorkClass::HandlerSteps, 1).with_remaining(WorkClass::CleanupItems, 8);
        match servient.step(cx, &mut budget) {
            StepStatus::Idle => return,
            _ => {}
        }
    }
    panic!("lifecycle did not become idle")
}
fn main() {
    let base: Thing = serde_json::from_value(document()).unwrap();
    base.validate().unwrap();
    let baseline = plan(&base);
    assert_eq!(baseline.has_additional_form_metadata(), None);
    for (label, field, value) in [
        ("explicit-get", "htv:methodName", json!("GET")),
        ("explicit-post", "htv:methodName", json!("POST")),
        ("response-override", "response", json!({"contentType":"text/plain"})),
        ("content-coding", "contentCoding", json!("gzip")),
        ("empty-scopes", "scopes", json!([])),
        ("form-basic", "security", json!(["basic"])),
        ("root-basic", "root-security", json!(["basic"])),
        ("second-read-property", "second-property", json!(null)),
    ] {
        let mut value_td = document();
        if field == "second-property" {
            value_td["properties"]["shadow"] = json!({"type":"number","readOnly":true,
                "forms":[{"href":"http://127.0.0.1:38080/shadow","op":["readproperty"]}]});
        } else if field == "root-security" { value_td["security"] = value; }
        else { value_td["properties"]["level"]["forms"][0][field] = value; }
        let td: Thing = serde_json::from_value(value_td).unwrap();
        td.validate().unwrap();
        assert_eq!(plan(&td), baseline, "{label} should demonstrate lost facts");
        let (registration, probe) = host_property_read_fixture();
        let servient = ServientBuilder::new().resource_limits(GatewayDefaultV1::LIMITS.clone())
            .binding_registration(registration).build().unwrap();
        let exposed = servient.produce_td(td).unwrap();
        let calls = Arc::new(AtomicU32::new(0));
        exposed.set_read_property_handler("level", Handler(calls.clone()), HandlerFootprint::new(1,0,0)).unwrap();
        exposed.begin_expose().unwrap();
        let mut cx = Context::from_waker(Waker::noop());
        drive(&servient, &mut cx);
        assert_eq!(probe.preparation_side_effects(), 1);
        probe.enqueue_property_read("level", InteractionInput::empty());
        drive(&servient, &mut cx);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(probe.delivered_responses(), 1);
        exposed.begin_destroy().unwrap();
        drive(&servient, &mut cx);
        assert_eq!(probe.outstanding_counts(), (0,0,0,0));
        assert_eq!(probe.route_state_drops(), 1);
        println!("{label}: basic-valid; plan=baseline; metadata=None; prepare=1; handler=1; response=1; route-drops=1");
    }
}
EOF_RUST
cp "$producer_repo/Cargo.lock" "$producer_probe/Cargo.lock"
cargo generate-lockfile --offline --manifest-path "$producer_probe/Cargo.toml"
cargo run --locked --offline --manifest-path "$producer_probe/Cargo.toml"
```

Expected: eight lines, each reporting `basic-valid; plan=baseline; metadata=None;
prepare=1; handler=1; response=1; route-drops=1`. These are successful
counterexample assertions, not successful production admission checks.
