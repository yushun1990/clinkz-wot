# Architecture Review 10: Reusable Definitions and Composable Execution

- Date: 2026-11-09
- Status: non-normative assessment and proposed engineering sequence for Owner review
- Reviewed source: master
  [`f6773277a1a3f854465fe94f14b60765451e400c`](https://github.com/yushun1990/clinkz-wot/tree/f6773277a1a3f854465fe94f14b60765451e400c)
- Authority: [active v6 source map](../spec/authority.toml) and
  [ADR-0025](../ADRs/0025-semantic-engine-and-binding-runtimes.org)

**Recommendation:** retain v6's responsibility boundaries and replace the
inconsistent preparation-to-instance handoffs. Share immutable semantic meaning,
qualify actual native mappings, and bind independent logical instances around
separate execution capabilities. The current finite Consumer split is a useful
starting point; it should not become a universal runtime representation.

This assessment implements no redesign, changes no accepted contract or
work-package admission, and closes no milestone. It retains the requested
technical decision proposal, source evidence, executable counterexamples and
implementation alternatives. The engineering sequence is a proposal, not a
session checklist or a second roadmap. [PLAN.md](../../PLAN.md) remains the
roadmap owner. Future architecture-sensitive implementation requires migration
into the affected authoritative owners and independent acceptance under repository
governance.

Review 09's first finite reuse increment is now implemented. Its earlier
schema/security, callback and ownership diagnoses must not be inherited as
current findings. This review reconstructs the two related priorities from the
source above: reusable definitions independently bound to Things, and composable
finite, continuing Action, notification and Producer execution.

**First-principles responsibilities.** WoT permits different protocols to
implement different affordances within one Thing. It does not prescribe a Rust
runtime representation. TD 1.1 distinguishes synchronous Action results,
continuing execution, Event control/data roles and root collection operations.
The dated [WoT Architecture 1.1 Recommendation](https://www.w3.org/TR/2023/REC-wot-architecture11-20231205/)
and [TD 1.1 Recommendation](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/)
provide that semantic baseline. The
[Scripting API](https://www.w3.org/TR/2023/NOTE-wot-scripting-api-20231003/)
is a Group Note, not a requirement to reproduce its API. The
[Binding Templates retirement notice](https://www.w3.org/TR/2025/NOTE-wot-binding-templates-20251104/)
does not qualify a concrete mapping or replace TD 1.1 with a draft contract.

| Responsibility | Genuinely shared | Differences that must remain |
| --- | --- | --- |
| Description meaning | Applicability, DataSchema interpretation, security expressions, media/response meaning, URI-variable declarations and extension resolution | Each family retains its exact data roles and controls |
| Qualification | Actual implementation/configuration pairing, complete selected support, bounded construction and dependency checks | Consumer selection may omit unsupported interactions; Producer publication must qualify the entire retained final description |
| Instance binding | Independent identity/currentness, policy association, accounts and retained dependencies | Consumer destination/credential policy differs from Producer verification, behavior registration and publication |
| Execution safety | Capacity before acceptance, fresh permission, callback-time rechecks, original ownership, first cause and funded cleanup | Finite settlement, continuing invocation, subscription conversation and publication have different transitions |
| Physical ownership | Original backing remains funded through final use | Connection, listener, broker interest, packet, continuation and application work have different owners and retirement policies |

Consolidate the shared obligations where practical. Preserve the distinctions
in the final column. In particular:

- Finite exchanges terminate with their own input/result/native settlement.
  Action invocation lifetime can outlive the initiating exchange; acceptance,
  pending execution, completion and cancellation acknowledgement remain distinct.
- Property observation carries state. Events retain subscription, data,
  dataResponse and cancellation roles. A schema creates neither a native reply
  channel nor protocol backpressure; a protocol ACK is not application data.
- Root Property maps, selected member sets, named Event occurrences and Action
  status inventories retain their own semantics. A native collection operation
  cannot silently become N leaf interactions. Preserve supported member reports
  and actual atomicity evidence without inventing stronger guarantees.
- Consumer notifications and Producer emission are different owners. Native
  distribution does not require an Engine record or copy loop per remote
  subscriber. Local publication does not prove recipient delivery.
- Cancellation, effect certainty, delivery, settlement acknowledgement and
  physical retirement remain independent observations. No uncertain write or
  Action is implicitly replayed.

**Demonstrated defect F1: Binding code crosses the execution-authority boundary.**
[HttpActionRuntime::step][http-action-runtime] receives a mutable
`RawProducerServient`; its implementation invokes, queries, cancels, progresses
and acknowledges logical Action work. MQTT's [native publication runtime][mqtt-native]
also receives the logical owner and calls `owner.revoke`. Both Binding crates
have an unconditional normal dependency on `clinkz-wot-servient-finite`, including
with default features disabled. `cargo tree` confirms those edges.

This conflicts with [MOD-DEPS-001](../architecture/20-module-boundaries.md) and
the [native/ingress responsibility contracts](../spec/binding-spi.md).
Caller-driven execution does not remove the dependency or dispatch capability.
The direct HTTP finite ingress implementation already demonstrates the useful
alternative: Core contracts carry candidates and original opportunities, and
Servient dispatches behavior without the Binding receiving the logical owner.

Native code should return candidates, opportunities and failure evidence.
Servient should authorize, dispatch and decide logical revocation. Preserve
native matching, token encoding, correlation and physical resources in the
Binding. Do not erase the owner behind a dispatch trait object and call that
boundary extraction.

**Demonstrated defect F2: Action preparation repeatedly charges shared physical
service backing.** [Action selection][action-preparation] adds data/access
retained bytes to every selected operation's funding. Invoke, query and cancel
can charge the same aliased service three times. [Notification selection][notification-preparation]
uses the same per-definition adoption pattern. Neither path imports the retained
service owners already supported by finite preparation.

The [diagnostic probe](probes/review-10-execution-handoffs.rs) passes one actual
data service to the three Action mappings. Increasing that service's declared
backing by 60,000 bytes increases preparation charges by 180,000 bytes. The
same object remains shared through its original Arc. Per-operation references
and associations need separate funding; recharging physical service backing
does not establish additional ownership. This conflicts with the original-owner
and no-double-charge obligations in [Foundation](../spec/foundation.md).

The probe directly demonstrates the Action case. The analogous notification
accounting pattern is source evidence, not a claim that a second cross-root
notification measurement was executed.

**Demonstrated defect F3: runtime construction can refresh prepared declarations
without requalification.** Action runtime construction reads current service
policies/native bounds in `Selected::new`. Notification construction similarly
reads the current data policy. Their prepared carriers do not preserve the
corresponding complete admission snapshot. The later checks compare against
these newly captured values. [Action capture][action-runtime],
[notification capture][notification-runtime].

The probe prepares each definition with a data service declaring 32 retained
bytes, changes its declaration to 123,488 bytes, then successfully constructs
both runtimes. On the measured x86_64 build, Action preparation charged 2,257
bytes and construction added 2,880, retaining only 5,137 total charged bytes.
The notification service remains disconnected. No native I/O is needed to
expose this handoff mismatch.

This intentionally changes metadata, without allocating another 123,456 physical
bytes. It demonstrates the missing declaration fence, not memory corruption,
credential leakage or an unmeasured allocator result. The finite path already
retains and checks captured declarations; apply that obligation to other
handoffs without silently capturing a new baseline.

**Current capabilities and remaining limitations.** The finite Consumer split
is real: [ConsumerExecutionContent][consumer-definitions] excludes endpoints,
services, authority and accounts; `PreparedConsumerContent` retains original
backing; `ConsumerDefinition` retains the qualified native mapping and actual
data service; `ConsumerBinding` attaches access policy and instance accounts.
Its tests demonstrate independent instances, source retirement and retained
normal/late settlements. Keep this boundary.

Cross-mapping reuse through `prepare_with_content` remains narrower: Property
Read/Write and ReadAllProperties. Exact qualified definitions can be rebound
more broadly. Those are different capabilities. A static target/path/header
change requires actual native preparation; an already-qualified dynamic operand
can vary under its native contract and fresh destination authorization.
[Common-content comparison][finite-preparation] and the
[URI-variable contract](../spec/planning.md#uri-variable-preparation) own those
distinct checks. Repeating the original ReadAllProperties reuse demonstration
would not complete the requested cross-family capability.

Continuing Action and notification definition carriers still bundle immutable
meaning, mappings, services and the preparation bank. Their constructors consume
those bundles. Sharing schemas within one association, or through an internal
Arc, does not supply an independently funded checked bind-many handoff.
Action runtime construction requires definitions at the same original bank;
notification runtime construction funds through its definition's bank. This
coupling is a missing capability, separately from F2/F3's accounting defects.
[Action carrier][action-definitions], [notification carrier][consumer-definitions].

Preparation repeats coordination across families. Each Consumer family starts
its own Basic admission and Form traversal. Producer [visit_thing][producer-preparation]
resolves a Form separately for each effective operation. Concrete Producer
preparation constructs common schema meaning inside individual definitions,
including HTTP Action schemas and MQTT start/stop schemas. Existing shared schema
helpers prevent independent constraint interpretations; the duplication concerns
admission, construction, selection and ownership. It is not evidence that every
family implements a different schema language. [HTTP preparation][http-action-preparation],
[MQTT preparation][mqtt-preparation].

Heterogeneous composition works in important scopes, but application assembly
remains incomplete:

- Finite selection supports heterogeneous actual providers. Continuing Action
  controls can use different Bindings with explicit continuation compatibility.
- Consumer facets already share one independent authority. Producer participants
  already publish under one logical owner; this must not be replaced with
  per-Binding authority registries.
- Notification selection is parameterized by one concrete `B`. Heterogeneous
  selection needs caller-written adaptation or separate registries. This is a
  limitation of the provided composition API, not proof that an enum adapter
  could never implement it.
- Producer callers extract concrete definitions, construct participants and
  downcast them for progress. These steps recur in the actual
  [mixed HTTP/MQTT assembly][mixed-assembly]. `Any` storage itself is not a
  defect; rebuilding qualified associations in each application is the limitation.

The HTTP Producer's same-native-owner requirement for an Action's full
invoke/control inventory is a declared mapping restriction, not an Engine rule.
Explicit refusal of unsupported security, response, schema or URI combinations
is incomplete coverage, not silent weakening. Common NoSec-only family gates
currently prevent a capable Binding alone from adding broader security; move
qualification onto the shared meaning/service boundary without claiming that
every broader scheme is already implemented.

**Proposed target.** Preserve five independent ownership boundaries. They do
not mandate five new public Rust types.

| Boundary | Retains | Remains independent of |
| --- | --- | --- |
| Prepared meaning | Supported immutable assertions, exact family data roles, response/security/variable meaning and original backing | Instance authority, mutable policy, credentials and native resources |
| Qualified native mapping | Actual registration/configuration, native operands, captured service dependencies and compatibility evidence | Current logical permission |
| Instance binding or exposure | Original logical identity, policy/behavior associations, accounts and selected mappings | Shared definition lifetime |
| Accepted work | Original binding, inputs, continuation/opportunities, results, effects, first cause and cleanup funding | Replacement instances and preparation roots |
| Native infrastructure | Physical resources, compatible participation/work leases, native limits and retirement | Thing cardinality and application behavior ownership |

Retain `PreparedSchema`, `ValueSchemas`, collection schemas, security
requirements, finite responses and URI-variable meaning. Construct or compare
their supported assertions through one Planning implementation. Extend
`ValueExecution`'s borrowing approach to the immutable operands required by
Actions and notifications. Extract pure meaning from Host-gated carriers where
needed. Different storage representations must not require Arc, Clone, pointer
atomics or a universal program in the shared semantic interface.

An Action definition retains application input/output meaning and described
control requirements; selected invoke/query/cancel mappings remain separate
associations. Notification definitions retain Property values, Event roles and
collection attribution explicitly. Producer preparation should receive these
same common prepared operands rather than reconstruct a constraint tree for
each native operation. A Binding can specialize actual native instructions.

Reuse TD's existing traversal through a role-neutral preparation surface.
Planning should share admission and scoped semantic loans across requested
families and all effective operations of a Form. Consumer selection and complete
Producer coverage remain different policies over that workflow. This requires
neither a complete normalized TD copy nor eager preparation of every Consumer
operation. Bounded loans and selectively owned facts are sufficient. No source
document or preparation callback survives into execution.

Extend the existing retained-service-owner mechanism across Action, notification
and Producer composition. Adopt physical backing once at the composition root;
fund importing references and instance associations separately. Preserve
admission-time bounds and actual dependency identity through runtime construction.
Changed declarations must refuse or invalidate the association while returning
protected owners, rather than establish a new baseline. Mutable availability,
credentials and current permission still require fresh per-use checks.

Instance binding attaches independent authority, policy, behavior where relevant,
and accounts. It must not infer its bank from a definition's preparation bank.
Native compatibility remains the actual Binding's responsibility: equal names,
types, tags or hashes grant neither mapping compatibility nor permission.
Sharing immutable content neither retargets an exact artifact nor transfers its
original backing charge.

Keep the finite transition kernel, continuing invocation owner, notification
conversation owner and Producer publication owner. Consolidate dependency checks,
service retention and funding helpers where obligations coincide. Different
validation timing, input/output roles, selected sets and native opportunities
remain explicit family contracts; one output-validation callback is insufficient.

For Producer Actions, add the smallest Core-defined native ingress/control
contract needed to retain candidates and reply opportunities. Servient accepts
those candidates and invokes its own behavior store. HTTP retains token encoding,
matching, correlation and wire status conversion. MQTT progress reports native
availability/failure and retains physical cleanup; Servient applies logical
fencing. Preserve the existing local publication commit and complete final-TD
qualification.

`ClosedProducerParticipation` contains useful publication obligations, but its
Servient-dependent methods cannot simply move into Core unchanged. Separate
passive native readiness/gate/coverage facts from Servient-owned membership and
authority checks. An association must still identify the complete original
participant set through publication and retirement. Keep family-specific Action
and notification associations where their semantics differ.

Startup composition should accept heterogeneous optional capability facets.
Applications select an operation and drive its typed capability without repeatedly
reconstructing provider grouping, funding and publication associations. Host
erasure can support heterogeneous notification selection and storage; typed/static
composition remains available. Bindings decide native grouping, sharing,
collisions and infrastructure progress. No registration needs dummy symmetric
roles or every operation family. No universal task/session driver or new executor
is selected.

| Existing abstraction | Proposed disposition |
| --- | --- |
| TD semantic loans and common schema preparation/evaluation | Retain; reuse traversal and prepared operands more broadly |
| Finite content/mapping/binding separation | Retain its responsibility split; remove Consumer-only assumptions from genuinely shared meaning |
| Owned incremental compiler machinery | Retain bounded work, rollback and ownership; permit different qualified native preparation mechanics |
| ConsumerInstance and Producer logical authority | Retain independent authority across facets and role-specific permission/publication semantics |
| Action, notification and publication lifecycle owners | Retain; separate prepared content and instance associations |
| Bundled Action/notification definition carriers | Replace with independently retained meaning, qualified mappings and checked bindings |
| Binding code receiving RawProducerServient | Remove through native/Servient boundary extraction |
| Historical plan-set, route-stage and single-read Servient APIs | Retire as general entry points after supported paths migrate; retain applicable safety evidence |
| Mandatory plan-set epochs in newer finite envelopes | Simplify where no independently replaceable owner exists; preserve actual mapping and obligation provenance |

The public `Servient` alias still names the restricted Producer installation
runtime. It should not define the eventual composition API. This is a public
entry-point limitation, not a reason to erase the newer execution owners.

**Proposed engineering sequence.** Each increment must deliver an executable
capability, with proportionate acceptance evidence, rather than merely another
facade or isolated helper extraction.

1. **Extend prepared content and checked binding through continuing Actions.**
   Consolidate common operands and retained service ownership around the finite
   seam. Preserve captured declarations in Action and notification handoffs,
   correcting F2/F3. Deliver actual Action definition reuse across two
   independently authorized/funded Consumers alongside finite execution. Use
   real HTTP to prove incompatible-bind refusal, source retirement, independent
   revocation and retained late results. Do not repeat only the existing
   ReadAllProperties increment.
2. **Join notifications to that preparation and instance-binding composition.**
   Add reusable notification meaning, independent bindings/accounts and
   heterogeneous Host provider selection. Share admission and selection
   coordination across finite, Action and notification requests. Exercise MQTT
   plus an independent notification implementation: separate cursors/stop,
   original response opportunities, visible loss, shared native interests and
   final release. Unsupported required Event responses still refuse.
3. **Correct Producer execution authority and enable independently bound exposures.**
   Remove Binding-to-Servient dependencies and direct logical dispatch/revocation
   through narrow Core contracts. Reuse prepared meaning across two Producer
   instances with distinct behaviors, policies and publication state. Each
   exposure independently qualifies its entire final TD, exact native ownership,
   collisions and readiness before the local commit. Boundary extraction can
   proceed alongside increment 2 once shared operands settle; do not grant a
   broader Producer acceptance claim while F1 remains.
4. **Complete application composition and remove superseded paths.**
   Replace repeated extraction/downcasting with supported startup assembly and
   family capability access. Consolidate public entry points and delete migrated
   historical machinery. Combine finite calls, continuing Actions, notifications
   and Producer exposure over heterogeneous Bindings and independent instances.
   Include a stalled participant, callback reentry, replacement, unread results
   and different retirement orders. Preserve caller-owned finite execution and
   show that new immutable meaning has no mandatory Host dependency. This does
   not claim every constrained runtime family or hardware backend is complete.

Carrier ownership, registration inputs, preparation outputs, runtime constructors
and Producer adapters need breaking changes. Compatibility facades would preserve
the coupling being removed. Future implementation affects PREP-SELECT/DEFER/REUSE/
COVER, EXEC-ID/DATA/SECURITY/ACTION/NOTIFY/COLLECTION/OUTCOME, SPI-COMPOSE/NATIVE/
INGRESS/DELIVERY/RETIRE, SAFE-AUTH/PUBLISH/CANCEL/CLEANUP/CONCUR/PROFILE,
RES-OWNER/HANDOFF/WORK and MOD-DEPS, with API/work-package projections as applicable.
Most proposed changes fulfill existing v6 obligations; any changed meanings or
superseded contracts still require explicit migration. This report changes none
of those owners or admission states.

**Alternatives and risks.** Sharing whole definitions also shares their services
and funding; sharing only schema allocations leaves preparation and binding
fragmented. Independent family registries plus an application-only wrapper keep
repeated qualification and heterogeneous adapters. Forcing all families through
one compiler/call/session erases real controls and settlement distinctions. Reject
those as the target architecture. Keep typed facet APIs where useful, backed by
shared preparation and ownership mechanics rather than a mandatory mega-registry.

Caches, pools and global scheduling add separate ownership problems. Measure
preparation, shared content, per-instance records and active work before selecting
them. Existing multi-instance measurements identify repeated TD qualification and
record costs; they do not qualify a redesigned composition or prescribe a cache.

Principal migration risks are early refunds, stale dependency capture, weakened
Producer coverage, excessive retention through shared owners and starvation
introduced by composition. Validate differing drop orders, refusal rollback,
callback invalidation, original late results and stalled native participants.
Cross-Binding continuation/control compatibility remains mapping-specific. Trust
domains, native sharing and new representations need actual compatibility and
ownership evidence. Passing scoped tests is not universal protocol neutrality,
full profile support or release readiness.

**Validation and limits.** At the reviewed source tree, 133 focused existing
tests passed: 59 HTTP reuse/Action/composed-Producer tests, 47 downstream
integration/Action/notification tests and 27 MQTT qualification tests. Exact
commands:

```sh
cargo test -p clinkz-wot-binding-http --test reusable_execution --test composed_thing --test action_consumer
cargo test --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml --test action --test integration --test notification
cargo test -p clinkz-wot-binding-mqtt --test consumer_qualification --test qualification --test property_qualification --test collection_qualification --test event_collection_qualification
cargo tree -p clinkz-wot-binding-http --no-default-features --edges normal --depth 1
cargo tree -p clinkz-wot-binding-mqtt --no-default-features --edges normal --depth 1
```

The probe reproduced F2/F3 through public preparation/runtime APIs without
network progress. Its assertions preserve reviewed defects, not desired general
behavior; retire or invert the relevant assertions when corrections land. Native
cleanup, interoperability and physical allocation growth are outside its claim.
From the repository root, provision a temporary standalone Cargo manifest to
run the retained source without adding a workspace member:

```sh
python3 - <<'PY'
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path.cwd()
probe = Path(tempfile.mkdtemp(prefix="clinkz-review10-"))
source = root / "docs/reviews/probes/review-10-execution-handoffs.rs"
manifest = (
    '[package]\nname = "review10-execution-handoffs"\n'
    'version = "0.0.0"\nedition = "2024"\n[workspace]\n'
    '[[bin]]\nname = "review10"\npath = ' + json.dumps(str(source)) + '\n'
    '[dependencies]\n'
)
for name, relative in {
    "clinkz-wot-core": "core",
    "clinkz-wot-foundation": "foundation",
    "clinkz-wot-planning": "planning",
    "clinkz-wot-servient-finite": "servient/finite",
    "clinkz-wot-binding-http": "protocol-bindings/protocols/http",
    "clinkz-wot-binding-mqtt": "protocol-bindings/protocols/mqtt",
}.items():
    manifest += name + ' = { path = ' + json.dumps(str(root / relative)) + ' }\n'
manifest += 'serde_json = "1"\n'
(probe / "Cargo.toml").write_text(manifest)
shutil.copyfile(root / "Cargo.lock", probe / "Cargo.lock")
subprocess.run([
    "cargo", "run", "--offline", "--manifest-path", str(probe / "Cargo.toml"),
    "--target-dir", str(root / "target"), "--bin", "review10",
], check=True)
PY
```

`--offline` requires the repository's dependencies to have been fetched already.
The temporary lockfile starts from the repository lock; Cargo adds the standalone
probe package without modifying the repository lockfile. Numeric layout charges
above are observations, not portable ABI promises.

Mosquitto was unavailable during this assessment, so broker integration tests
were not rerun. Existing broker evidence was inspected as repository evidence.
The proposed heterogeneous assembly, cross-family bind-many behavior, sustained
mixed-workload progress and broader portable execution remain unverified target
capabilities. Required diff hygiene is not whole-project acceptance.

[consumer-definitions]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/core/src/consumer.rs
[action-definitions]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/core/src/consumer_action.rs
[finite-preparation]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/planning/src/open_consumer.rs
[action-preparation]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/planning/src/open_consumer/action.rs
[notification-preparation]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/planning/src/open_consumer/notification.rs
[action-runtime]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/consumer_action.rs
[notification-runtime]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/consumer_notification.rs
[producer-preparation]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/planning/src/producer_composition.rs
[http-action-runtime]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/protocol-bindings/protocols/http/src/action_producer.rs
[http-action-preparation]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/protocol-bindings/protocols/http/src/action_preparation.rs
[mqtt-preparation]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/protocol-bindings/protocols/mqtt/src/preparation.rs
[mqtt-native]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/protocol-bindings/protocols/mqtt/src/native.rs
[mixed-assembly]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/protocol-bindings/protocols/mqtt/tests/collection_coexistence.rs
