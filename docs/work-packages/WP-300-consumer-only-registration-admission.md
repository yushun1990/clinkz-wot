# Historical / restricted Property Read record

Status: historical v5 authority; **no v6 implementation admission or architecture
acceptance**. The original record and exact prose are preserved below and at
[the predecessor revision](https://github.com/yushun1990/clinkz-wot/blob/ed09f059e383b7a1b9d22902ee29aa218b4471a6/docs/work-packages/WP-300-consumer-only-registration-admission.md).
[ADR-0025](../ADRs/0025-semantic-engine-and-binding-runtimes.org) supersedes its
general API, gate ordering and lifecycle projections. The [index](index.toml)
records individual tranche impact dispositions. Tests of retained Property Read
code remain useful at that narrow scope; neither a passed status nor this
record constrains the successor SPI. Exact schema/resource encodings still used
by retained adapters keep their original meaning. Any new source scope must use
the successor requirements and applicable review, not inherit this admission.

## Preserved predecessor record (non-authoritative for the general architecture)

# Consumer-only complete registration admission

[The index](index.toml) owns the state of
`WP-300-CONSUMER-ONLY-REGISTRATION`. This record defines its exact v5.1 ADR-0013
implementation and evidence envelope.

[Independent admission review of PR #152](https://github.com/yushun1990/clinkz-wot/pull/152#issuecomment-6120235799)
accepted base `5ed0ad7c1713794a2c7cacac2094f57d3d14b81c` to head
`ef0e9ec2c1621e10d5c265b9904ed49dd760f93e` with no blocking findings, subject
to successful required CI. This separate admission transition follows that
acceptance and satisfied CI condition. Admission covers this envelope only.

[Independent production review of PR #154](https://github.com/yushun1990/clinkz-wot/pull/154#issuecomment-6135946450)
accepted the implementation and completion evidence at exact head
`ca01012127e32fd1ade34e8ce73e6c8b1e59d526` with no blocking findings.
The [registered completion evidence](../evidence/WP-300-consumer-only-registration.toml)
retains the implementation reference, seven-obligation results, separate
static/Host measurements and scope exclusions. Completion covers this tranche
only; broader WP-300, WP-400 admission/completion, Consumer gate registration
and all gate/milestone transitions remain separate.

## Decision and scope

Permit one complete Consumer Property Read registration without a Producer
server. Completeness means all components for the advertised role: immutable
identity, selected representation, compiler, private client, retained/transient
resource declarations and status policy. No server, route/readiness/response
state, handler, activation authority or ingress owner is synthesized.

The [constructibility investigation](../../workspace/0079-consumer-runtime-entry-constructibility.md)
compares this additive correction with dummy servers, changing the existing
generic Producer bundle, a role framework and broader runtime/package redesign.
The selected boundary keeps existing Producer and dual-role public contracts
and the singular semantic validators. It does not select a Host runtime carrier.

Core defines `StaticConsumerBindingRegistrationInput<C: PollClientBinding>`
and `StaticConsumerBindingRegistration<C>`, and the `std`-only
`HostConsumerBindingRegistrationInput` and `HostConsumerBindingRegistration`.
Input constructors assemble without protocol work. `new(input)` validates
atomically and returns `BindingInputRejection<Input>` with the unchanged input
on rejection. Validated owners provide immutable identity/capabilities/execution,
compiler/resources/status observations and consuming `into_input`; they expose
no installed raw client or mutable compiler/configuration. Capability metadata
adds only `BindingRegistrationCapabilities::consumer_property_read()`.

Static input takes identity, execution, `StaticBindingCompilerRegistration<C::Compiler>`,
client, resources and status. Host input takes the corresponding erased compiler
and boxed target `binding::ClientBinding`. There is no ingress parameter. Static
validation checks selected cell, compiler/client artifact compatibility, declared
ceilings, actual complete `StaticConsumerPropertyReadSlot<C::RequestState>`
storage and binding transient delta. Host validation checks selected cell,
compiler/client compatibility and declarations. Generic native compiler validity
does not confer closed aggregate support.

Both complete owners consume themselves in `try_into_consumer_compiler`, with
unchanged-owner rejection. Core alone issues the existing
`ConsumerCompilerRegistration<Self>` for the actual closed `ResolvedTargetCompiler`
and immutable configuration, or its supported fallible Host adapter. All identity
fields survive capture. Same compatibility, associated types, declarations or an
arbitrary erased native compiler are insufficient. No new support issuer or
public attestation token is permitted.

Planning adds sealed `ConsumerPropertyReadRegistration` implementations for
those two checked owners. It delegates to the same paid aggregate, all-bounds
barrier, selector and source-free rollback. It gains no compiler knowledge,
raw-client path, TD interpretation or parent/provenance authority.

Static complete owners expose the existing signatures for
`start_consumer_property_read`, `poll_consumer_property_read`,
`start_cancel_consumer_property_read`, `poll_cancel_consumer_property_read` and
`acknowledge_consumer_property_read`, substituting `C::Compiler`/`C::RequestState`
for the dual-role types. Host complete owners expose the existing sealed
`start_consumer_property_read` signature. The exact private seal, normal and
cancellation-late validation, slot disposition, terminal acknowledgement and
Host decorated-call accounting are shared with the completed dual-role path.
Neither a publicly supplied expected-identity bag nor a second semantic kernel
is allowed. Static cancellation still has no transfer owner; Host cleanup
transfers the complete decorated owner. Request names/targets remain absent.

## Dependencies and source boundary

The completed/current binding execution, closed compiler support and aggregate
tranches are the direct dependencies. Their registered passed evidence owns
existing semantics. WP-000 is the completed package dependency. Broad WP-300,
WP-400, Discovery, ObserveProperty and real protocols are not prerequisites.

Permitted production paths:

- `core/src/binding.rs` and `core/src/lib.rs`, for additive complete bundles and
  private shared mediation/capture mechanics;
- `planning/src/consumer_property_read.rs`, only for the two sealed adapters.

Focused tests may extend `planning/tests/consumer_property_read_binding.rs`,
`planning/tests/consumer_compiler_support.rs` and the existing isolated compiler,
aggregate and runtime-entry fixtures. No production manifests, legacy paths,
Servient facade, resource schema, WorkClass, state machine or protocol code change.
If ordinary helper extraction cannot retain the existing semantics inside this
envelope, amend and independently review the scope before proceeding.

## Resource and evidence boundaries

Use existing resource kinds and profile applicability. Missing Producer owners
consume no route/ingress storage; existing unused declaration ceilings are not
physical charges. Count the complete static slot/seal and binding-private state,
Host call/decorator, compiler support/adapter, registration and temporary storage
once, reporting target-specific Layouts separately. Do not sum an aggregate
allowance into a contiguous allocator request or infer physical backing from a
ledger peak. Preserve rollback history and exactly-once object destruction.

This precursor does not repair or strengthen existing Host invocation allocation
or callback admission: `HostBindingCallBox::new` remains an infallible allocation.
Its completion cannot prove recoverable runtime OOM or WP-400 capacity admission.
It also does not change the local/non-Send Planning root, create global accounts,
publish output, grant call leases or make device-fit claims.

The [historical external fixture](https://github.com/yushun1990/clinkz-wot/blob/5091685/tools/architecture-fixtures/consumer-runtime-entry/README.md)
is pre-implementation construction evidence using actual raw SPI parts. Its
model cannot be installed, mint Core support or execute a sealed call. The Host
model has no access to private support/configuration checks. Its own constructor
success is not production eligibility. Public production positives below are
mandatory completion evidence, rather than a prerequisite requiring unadmitted
source. Existing dual-role evidence cannot substitute for them.

Topic 0076's role-construction question is resolved narrowly by this migrated
decision. Its unresolved deployment, Host runtime ownership, optional Discovery,
generated-image and hardware questions are disjoint from these three product
paths. They remain material WP-400 admission questions. No new ADR or governance
gate is needed to project existing role-complete registration authority.

## Executable pre-implementation checks

The exact commands are registered in `index.toml` and mainline. Use Rust 1.95.0:
authority/API/DAG/ADR/resource/performance validation, locked workspace tests,
the new fixture in std/no-default/async, two ARM compile cells, and portable
Core binding execution regressions. ARM results compile the normal library only;
portable runtime tests execute on the Host. There are no placeholder checks.

## Production completion and independent acceptance

Stable key: `consumer-only-registration`. Output:
`docs/evidence/WP-300-consumer-only-registration.toml`. Record implementation
revision, exact commands, feature cells, measured owners/Layouts and results.
Required public production evidence:

1. A downstream client implementing no server trait constructs complete static
   and Host Consumer-only bundles, with zero server/route/ingress owner and no
   protocol work during construction. Rejection preserves exact compiler/client,
   complete identities and existing reservation history.
2. Reject wrong compiler/client compatibility, unsupported cell, invalid resource
   declaration, complete static-slot limits minus one/exact/plus one, transient
   overrun and arithmetic overflow before execution. Expose no installed raw
   client or public seal issuance; add meaningful compile-contract negatives.
3. Capture real closed support in both forms, preserving nondefault binding id,
   generation, configuration, compatibility and diagnostic ordinal. Reject wrong
   actual configuration, open/same-compatibility impostors and generic Host
   erasure with unchanged-owner return. Reproduce supported Host allocation
   failure and retained-slot cleanup through the actual public surface.
4. Use both new checked owners through actual aggregate validation/preflight,
   all bounds before start, complete multi-property output and lookup. After
   source destruction, retain the separate complete registration and execute
   from concrete artifacts. Preserve empty ranges and original Form coordinates.
5. Invoke only new complete-owner methods; normal sync/pending and
   cancellation-late results cover the existing identity/shape negatives and
   remain in their correct terminal branch. Keep opaque native status, sealed
   acknowledgement, generation-safe reuse and exactly-once cleanup. Poison
   legacy invocation/selection and prove no bypass.
6. Measure separate static/Host resource deltas and zero terminal retained
   registration, artifact, request/slot and adapter bytes under explicit cleanup.
   Retain and drive the original Planning root/account; no implicit variable
   Drop or pretend source/account release may satisfy this.
7. Run std/no-default/async runtime cells, isolated normal dependency checks and
   both ARM compile cells, all existing Consumer compiler/aggregate regressions,
   all intersecting registered Producer gate commands, authority checks and
   required mainline CI. Record explicit reaffirmation or escalate a falsified
   existing gate through independent gate control; the author changes no status.

The fresh reviewer must challenge the additive public types and private sharing,
reconstruct exact base/head and executable evidence, and assess the real public
positives. Admission and later completion are independent decisions. This
admission claims no production completion and does not admit WP-400 or register
a Consumer gate.
