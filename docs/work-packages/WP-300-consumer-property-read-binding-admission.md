# WP-300 Consumer Property Read Binding Admission

Status: REOPENED under ADR-0013 impact review for design revision v5.1.

The prior admission froze only the first Consumer one-shot Binding tranche. It
refined the broad retained WP-300 client surface for the active v5.1 Consumer
Property Read proof and did not activate the later long-lived domain.

The prior admission and completion evidence are no longer current. Independent
review confirmed that Host acceptance and synchronous application-static
completion can each make the selected request unavailable before the only
admitted Core response validator can borrow it. The remaining sections record
the superseded admission baseline and impact surface; they do not authorize
production changes until Core-mediated result sealing, complete-registration
no-bypass behavior, resource accounting, and replacement evidence are amended
and independently readmitted.

## Tranche

- id: `WP-300-CONSUMER-PROPERTY-READ-BINDING`
- predecessor: `WP-200-CONSUMER-PROPERTY-READ-PLANNING`
- owner package: `clinkz-wot-core`
- feature cells: `no-default`, `async-no-std`, `std`
- completion evidence key: `consumer-property-read-binding-execution`

The affected active requirements are exactly:

- `BIND-OUT-001`;
- `BIND-REG-001`;
- `BIND-STORAGE-001`;
- `BIND-MEM-001`;
- `BIND-DELIVERY-001`;
- `BIND-IO-001`;
- `BIND-CALL-CANCEL-001`;
- `BIND-HOST-CANCEL-001`;
- `API-PAYLOAD-001`;
- `PLAN-REQUEST-001`; and
- `PLAN-ARTIFACT-001`.

`BIND-PROGRESS-001` remains inactive. No subscription start/driver API, collection behavior, emission, broad retry/fallback, or concrete protocol implementation enters this tranche.

## Permitted production paths

Production implementation may change exactly:

- `core/src/binding.rs`;
- `core/src/outbound.rs`;
- `core/src/response.rs`; and
- `core/src/lib.rs`.

Tests and the registered completion-evidence file may be added outside those production paths. A required production change elsewhere stops implementation and returns the tranche to impact review.

`core/src/security.rs` is deliberately outside this tranche. The first proof covers only a request whose security decision has already committed and requires no binding-carried security material. If implementation needs `AppliedSecurity`, credential-provider access, a security branch selector, or another security representation, this admission is insufficient and must be amended before code proceeds.

## Selected request

The target request is one owned Property Read execution envelope. Its fields are private and its public construction surface is frozen to:

```rust
impl OutboundRequest {
    pub fn property_read(
        thing_id: ThingId,
        target: AffordanceTarget,
        artifact: BindingArtifactRef,
        uri_variables: BTreeMap<String, String>,
        deadline: Option<Deadline>,
    ) -> CoreResult<Self>;

    pub const fn thing_id(&self) -> &ThingId;
    pub const fn target(&self) -> &AffordanceTarget;
    pub const fn operation(&self) -> Operation;
    pub const fn artifact(&self) -> BindingArtifactRef;
    pub const fn binding_id(&self) -> BindingId;
    pub const fn binding_generation(&self) -> BindingGeneration;
    pub const fn plan_set_generation(&self) -> PlanSetGeneration;
    pub const fn plan_id(&self) -> PlanId;
    pub fn uri_variables(&self) -> &BTreeMap<String, String>;
    pub const fn deadline(&self) -> Option<Deadline>;
}
```

`property_read` accepts only `AffordanceTarget::Property(_)` and only an artifact whose role is `BindingArtifactRole::ConsumerCall`; other targets or roles fail structurally before binding work. `operation()` is always `Operation::ReadProperty`. Binding, binding-generation, plan-set, and plan identities are derived from the captured `BindingArtifactRef`; they are not independently supplied fields that may disagree.

The request contains no TD, raw `Form`, `Thing`, credential provider, mutable `InteractionOptions`, binding-support callback, candidate list, fallback policy, or legacy `BindingRequest`. It contains no operation payload in this read-only slice. URI variables and deadline are the only admitted call-varying target/control facts. The selected form index, resolved target, content type, subprotocol, response classification, and protocol-specific compiled facts remain owned by the immutable plan/artifact selected in WP-200 rather than being recopied into every request.

A pre-acceptance `BindingInputRejection<OutboundRequest>` returns that exact owned request and never authorizes reselection or fallback.

## Consumer response validation

WP-100's private validator kernel becomes reachable through exactly one public wrapper:

```rust
pub fn validate_untrusted_binding_output(
    request: &OutboundRequest,
    output: InteractionOutput,
) -> CoreResult<InteractionOutput>;
```

The wrapper derives the expected binding id, binding generation, and plan id only from the live `OutboundRequest` and delegates to `validate_property_read_binding_output`. It adds no second validation algorithm and does not interpret protocol-native numeric status. No public caller may supply expected ids separately.

## Immutable artifact handoff

A selected client constructor receives the exact admitted artifact envelope as a scoped read-only borrow together with the owned request. This borrow is necessary to keep static target/protocol facts behind the immutable compiled artifact rather than copying them into `OutboundRequest`.

The binding must compare the envelope identity and compatibility with the request's `BindingArtifactRef` before accepting the request or causing a protocol side effect. The scoped artifact borrow must not enter a returned host call, a constrained request slot, a cleanup object, or detached work; all retained protocol-local state is derived during the constructor/start call.

This is a v5.1 one-shot refinement of the broad retained client spelling in `docs/spec/binding-spi.md`. It does not grant a client access to a plan-set registry or permit artifact lookup after construction.

## Host selected client path

The target-generation Host trait is public at `clinkz_wot_core::binding::ClientBinding` while the legacy root/outbound `ClientBinding` remains available only to unmigrated callers. The two source generations have no adapter or conversion in this tranche. The target Host trait is frozen to:

```rust
#[cfg(feature = "std")]
pub trait ClientBinding: Send + Sync {
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility;

    fn invoke(
        &self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    >;
}
```

There is no target `subscribe` method in this tranche. Constructor rejection is side-effect free and returns the exact request. Acceptance returns one owned `HostBindingCallBox` before the first protocol side effect. The existing `HostBindingCall` contract owns polling, immutable lifetime footprint, cancellation, late completion, deadline wake, settlement, and cleanup transfer; this tranche does not create a second host call state machine.

The legacy `core::outbound::ClientBinding`, `BindingRequest`, `supports`, and `supports_with_thing` remain untouched for legitimate unmigrated capabilities. Target tests must poison those paths and still pass, proving zero target edge to them. The root `clinkz_wot_core::ClientBinding` export therefore remains legacy during this tranche; target third-party authors import `clinkz_wot_core::binding::ClientBinding`. A later staged legacy-removal tranche may switch the root export without changing this target trait.

## Constrained selected client path

The active one-shot static surface uses the retained `PollClientBinding` / `ClientRequestSlot` names only for request execution. Their subscription members remain inactive and are not implemented by this tranche.

The request slot owns exactly one accepted request and its binding-private state:

```rust
pub struct ClientRequestSlot<S> { /* private */ }

impl<S> ClientRequestSlot<S> {
    pub const fn new() -> Self;
    pub fn initialize(&mut self, request: OutboundRequest, state: S);
    pub fn request(&self) -> &OutboundRequest;
    pub fn state_mut(&mut self) -> &mut S;
    pub const fn is_vacant(&self) -> bool;
    pub fn clear(&mut self);
}
```

`initialize` is valid only for a vacant admitted slot. `clear` is valid only after terminal result/cancellation disposition has been retained by the caller and acknowledged. Reuse occurs only after state drop in caller context.

The request-only static trait is frozen to:

```rust
pub trait PollClientBinding {
    type Compiler: BindingCompilerExtension;
    type RequestState;

    fn artifact_compatibility(&self) -> BindingArtifactCompatibility;
    fn request_state_layout(&self) -> BindingStateLayout;

    fn start_request(
        &mut self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<
            <Self::Compiler as BindingCompilerExtension>::Artifact,
        >,
        slot: &mut ClientRequestSlot<Self::RequestState>,
        budget: &mut WorkBudget,
    ) -> Result<
        StartStatus<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    >;

    fn poll_request(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut ClientRequestSlot<Self::RequestState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>>;

    fn start_cancel_request(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: CleanupPhaseContext,
        slot: &mut ClientRequestSlot<Self::RequestState>,
        budget: &mut WorkBudget,
    ) -> CoreResult<
        StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>,
    >;

    fn poll_cancel_request(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut ClientRequestSlot<Self::RequestState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>>;

    fn acknowledge_request(
        &mut self,
        slot: &mut ClientRequestSlot<Self::RequestState>,
    ) -> CoreResult<()>;
}
```

`start_request` transfers the request only when accepted. Rejection returns the exact request and leaves the slot vacant. A synchronous completion is terminal without hidden retained protocol work. A pending start owns the request and state in the slot until terminal acknowledgement. Cancellation uses the existing `BindingCallSettlement` / `CleanupPhaseContext` semantics. A zero `BindingPolls` budget performs no binding callback or slot mutation. No subscription associated state or method is admitted.

## Complete registration projection

The first Consumer proof extends the already-accepted Producer Property Read complete bundle into one dual-role Producer+Consumer Property Read bundle. Pure client-only bundle generalization is not claimed by this tranche.

`BindingRegistrationCapabilities` keeps all existing Producer constructors/semantics and adds only:

```rust
impl BindingRegistrationCapabilities {
    pub const fn producer_and_consumer_property_read() -> Self;
    pub const fn supports_consumer_property_read(self) -> bool;
}
```

For Host registration, `HostBindingRegistrationInput` gains a dual-role constructor that takes the existing server plus one target `Box<dyn clinkz_wot_core::binding::ClientBinding>`. Existing Producer-only `new(...)` remains source- and behavior-compatible. `HostBindingRegistration::new` validates that a dual-role input has both components, that compiler/server/client artifact compatibility equals the registration identity, and that the selected host execution/resource declarations are valid. It exposes:

```rust
impl HostBindingRegistration {
    pub fn client(&self) -> Option<&dyn clinkz_wot_core::binding::ClientBinding>;
}
```

For application-static registration, existing Producer-only `StaticBindingRegistrationInput<B>` and `StaticBindingRegistration<B>` source spelling remains valid. The dual-role path uses one typed server/client pair:

```rust
pub struct StaticBindingComponents<S, C> { /* private */ }

impl<S, C> StaticBindingComponents<S, C> {
    pub const fn new(server: S, client: C) -> Self;
    pub const fn server(&self) -> &S;
    pub const fn client(&self) -> &C;
    pub fn client_mut(&mut self) -> &mut C;
}
```

where `S: PollServerBinding`, `C: PollClientBinding<Compiler = S::Compiler>`. The pair delegates the Producer server contract unchanged and gives the complete registration one typed Consumer component with the same compiler/artifact universe. `StaticBindingRegistrationInput<StaticBindingComponents<S, C>>` gains a dual-role constructor, and `StaticBindingRegistration<StaticBindingComponents<S, C>>` gains a dual-role validating constructor. Existing Producer-only `StaticBindingRegistration<B>::new` is unchanged.

The dual-role validation requires Producer and Consumer Property Read capabilities, the existing selected static execution profile, one compiler identity, equal server/client artifact compatibility, valid existing resource/ingress declarations, and `client.request_state_layout()` fitting within the registration's admitted retained-resource ceiling. This tranche does not add client-only registration, subscription state, another compiler component, or an artifact side table.

## Ownership and cancellation rules

- Host and constrained constructors are nonblocking and side-effect free until the request is accepted into the returned call/typed slot.
- The artifact borrow is checked before acceptance and cannot be retained.
- Caller-interest drop never means call cleanup. The call/slot owner remains live until result, cancellation settlement, acknowledged transfer, or durable residual disposition.
- Pre-acceptance rejection returns the exact `OutboundRequest`; it never rebuilds a request and never re-enters Planning.
- Completion and cancellation race through the existing first-cause/settlement contract. A late successful output remains untrusted until `validate_untrusted_binding_output` succeeds.
- `BindingCallSettlement::Returned` is the only late-value branch. Cancellation cannot discard a late output or invent `NoSideEffect` after an unknown side effect.
- No target path obtains `Thing`, raw `Form`, `InteractionOptions`, credential provider, legacy `BindingRequest`, `supports`, `supports_with_thing`, or a candidate-selection callback.

## Explicit exclusions

This tranche does not implement or claim:

- `BIND-PROGRESS-001` or any subscription start/driver API;
- `AppliedSecurity`, credential-store/provider migration, or non-empty binding-carried security material;
- fallback, retry-to-another-candidate, binding health reselection, or capability probing;
- write/action/observe/subscribe/collection operation families;
- Servient consumed-plan publication, application facade, caller-interest ownership, or drain orchestration;
- Consumer architecture-gate registration or completion;
- concrete Zenoh/zenoh-pico behavior;
- broad WP-300 completion;
- removal of legacy `BindingRequest`, legacy outbound `ClientBinding`, or their legitimate unmigrated callers.

## Pre-implementation checks

The following must pass at the admitted revision before implementation starts:

```text
tools/check-design-artifacts.sh
cargo test --workspace --locked
cargo check --locked -p clinkz-wot-core --no-default-features
cargo check --locked -p clinkz-wot-core --no-default-features --features async
cargo check --locked -p clinkz-wot-core
```

## Completion evidence

The evidence at
`docs/evidence/WP-300-consumer-property-read-binding-execution.toml` is the
superseded historical checkpoint. Before this tranche can be readmitted, this
section and the affected Host/static execution projections must be amended from
an independently accepted result-sealing decision. Before the reopened tranche
becomes `complete`, replacement evidence must record the exact corrected
implementation checkpoint and passing evidence for all of the following:

- `OutboundRequest::property_read` exact positive construction plus rejection of non-property targets and non-`ConsumerCall` artifacts;
- request accessors derive binding/plan generations from one artifact ref and expose no TD/Form/options/provider/fallback surface;
- no-security-material baseline only; no new public security carrier appears;
- public `validate_untrusted_binding_output(&OutboundRequest, InteractionOutput)` delegates to the WP-100 kernel and rejects every existing identity/shape negative case while retaining opaque native status;
- external Host authoring through `clinkz_wot_core::binding::ClientBinding`, including exact artifact/compatibility check, exact-request rejection, constructor-before-side-effect ownership, declared lifetime footprint, successful terminal output, timeout/cancellation, late-result retention, and cleanup settlement;
- the installable Host complete registration derives private validation authority before request transfer and exposes only a sealed runtime call path; normal and late `Returned` successes are validated without changing their terminal classification, and accepted/rejected cleanup transfer preserves the call, seal, and accounting as one work object;
- external static authoring through `PollClientBinding` and `ClientRequestSlot`, including synchronous-ready, pending, rejected, zero-budget, cancellation-late, acknowledgement, clear, and generation-safe reuse behavior;
- the installable static complete registration captures validation authority before `start_request`, seals synchronous-ready output immediately, seals pending normal and late output against the live slot request, and rejects acknowledgement/clear until the terminal value is sealed or cancellation has terminally disposed of the call;
- valid success plus every binding id, binding generation, plan id, response selection, status, payload shape/role, and action-reference negative is exercised after real request transfer in both Host and static representations, including synchronous, pending, and cancellation-late exits;
- one dual-role Host complete registration and one dual-role static complete registration with equal compiler/server/client compatibility, no raw installed-client execution projection, and regression of the existing Producer-only registration APIs;
- a mismatched client compatibility or oversized static request state is rejected before publication or protocol work;
- the scoped artifact borrow cannot be retained by the returned `'static` Host call or static request state and no per-call copy of resolved static target/form data is introduced;
- target tests poison legacy `BindingRequest`, `supports`, `supports_with_thing`, and legacy async `ClientBinding::invoke`; the target path still succeeds and contains no conversion to those values;
- target Host/static traits expose no subscription member and no implementation/evidence path depends on inactive `BIND-PROGRESS-001`;
- public raw Host/static traits remain authoring SPIs, but WP-400 cannot obtain an unsealed call path from an installed complete registration;
- the validation seal and any static mediation state have explicit retained/transient resource and work accounting;
- `no-default`, `async-no-std`, and `std` cells pass; and
- normal mainline CI passes.

The evidence must not claim WP-400, the Consumer Property Read architecture gate, real Zenoh Consumer execution, subscription semantics, broad WP-300 completion, or legacy removal.
