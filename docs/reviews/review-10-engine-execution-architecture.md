# Architecture Review 10: Reusable Definitions and Composable Execution

- Date: 2026-11-09
- Status: non-normative target architecture and proposed engineering sequence
- Reviewed source: master
  [`f6773277a1a3f854465fe94f14b60765451e400c`](https://github.com/yushun1990/clinkz-wot/tree/f6773277a1a3f854465fe94f14b60765451e400c)
- Authority: [active v6 source map](../spec/authority.toml) and
  [ADR-0025](../ADRs/0025-semantic-engine-and-binding-runtimes.org)

**Recommendation:** separate persistently usable logical instance associations
from execution capacity. Share eligible prepared meaning and qualified mappings;
attach original authority, operands and funding to each admitted obligation.
Preserve family-specific semantics and Binding-owned native infrastructure,
without preserving current Runtime containers as the general architecture.

This proposal incorporates the [independent architectural discussion][discussion].
It changes no production code, normative contract, work-package admission or
milestone. [PLAN.md](../../PLAN.md) remains the roadmap; registered v6 owners
remain implementation authority.

**First-principles basis.** Revoking one instance must not destroy a sibling's
shared meaning or native service. Changing call operands must not change another
call. Ending an exchange must not dispose an unfinished invocation or unread
result. Retaining an idle Consumer association creates no inherent need for a
vacant execution slot. These independent obligations determine ownership before
allocation or scheduling choices.

A physical device, a described Thing and a local consumed/exposed incarnation
are different identities. One virtual Thing can represent several devices;
separate Consumers can consume the same remote Thing. TD id, endpoint and schema
equality confer no local authority. The
[WoT Architecture 1.1](https://www.w3.org/TR/2023/REC-wot-architecture11-20231205/)
and [TD 1.1](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/)
Recommendations establish interaction meaning, not Rust containers.
[Core identity](../spec/interaction-core.md),
[Planning reuse](../spec/planning.md),
[Runtime authority](../spec/runtime-safety.md) and
[the v6 lifetime contract](../architecture/30-compiled-plan-lifecycle.md)
anticipate these distinctions; independent-capacity composition and occurrence
policy remain capabilities to implement. Historical Plan Set assumptions in
[0081](../../workspace/0081-reusable-plan-and-thing-identity.md) and the open
topology investigation [0082](../../workspace/0082-logical-physical-subscription-decoupling.md)
select no successor hierarchy.

**Target ownership and cardinality.** These are independently reasoned identities
and lifetimes, not a mandatory type hierarchy or central graph object.

| Responsibility | Content, identity and lifetime | Permitted sharing and boundary |
| --- | --- | --- |
| Prepared semantic meaning | Source-independent operation/data/security/response/variable assertions; original funded backing until final use | Reuse across actual compatible mappings and instances; no current grant, credential, behavior or native resource |
| Qualified native mapping | Actual provider/configuration, specialized native instructions, service dependencies and captured admission bounds | Many instance associations may use one mapping when its actual operand/dependency contract permits; an endpoint-specialized artifact stays specialized |
| Logical instance associations | Independent incarnation/currentness, selected mappings, live policy/service references and quota associations; Producer behavior/publication where applicable | One authority across its facets/Bindings; many instances may share eligible definitions and services without sharing permission |
| Stable operands | Bounded values deliberately fixed or defaulted for an association, under its qualified mapping | May be borrowed/shared; no extra identity or copy of an already specialized endpoint is required. Changing a fixed association cannot retarget prior work |
| Per-interaction operands | Validated inputs, variable values, member selection, resolved destination and deadline; occurrence/security context where applicable | Original work retains combined operands by immutable loan or snapshot; no mutable global current target/source |
| Admitted work and results | Original instance/facet/mapping, operands, continuation/reply rights, effects, first cause, result and cleanup owner | One native input may yield zero/many logical candidates. Identity, permission checks and funded lifetime remain per obligation; retention grants no fresh acceptance |
| Execution capacity and quota domains | Actual funded storage/progress/cleanup capacity; independent admission limits and expressly promised future capacity | A capacity domain may serve many instances; an instance may use several domains. Shared storage does not imply shared grants or identical quotas |
| Binding-native infrastructure | Physical connections/listeners/interests/buffers/jobs, compatible participation/work leases and retirement policy | May serve many mappings, instances and obligations; final destruction follows actual final use, not Thing count |

There is no required equality among physical-source, Thing, local-incarnation,
definition, mapping, facet, slot, account, logical-conversation and native-resource
counts. One Thing may need several definitions/services; many Things may share
eligible ones. Specialization may limit sharing. Independent revocation/delivery
needs logical state, not repeated immutable preparation or idle work storage.
An independently retained/replaced object needs provenance, not a plan-set epoch
for every Thing.

**Persistent usability, operands and admission.** An idle Consumer association
retains original identity/currentness, qualified operation/policy/mapping references,
stable operands and actual record costs. It can attempt execution without repeating invariant
preparation or owning vacant call slots. Preparation-required, unavailable,
revoked and capacity-exhausted remain distinct. PREP-DEFER-001 permits bounded
first-use preparation, distinct from prepared readiness. Admission can refuse
before effects or wait under an explicit bounded policy; usability promises no
unfunded simultaneous success.

For example, 1,000 associations may share eligible meaning with capacity for four
finite obligations; a fifth refuses or waits. Active subscriptions/exposures still
need their continuing state with no traffic. No O(1) total state or throughput
claim follows.

Combine qualified mapping operands, stable association values and this call's
values under explicit rules. Reject overrides of fixed values; do not turn TD
DataSchema annotations into implicit defaults. Validate the complete operands,
let the Binding resolve/check the native destination, and apply fresh original
instance/destination permission before credential use or semantic effects.
Capture that destination and association with the work; recheck currentness and
dependencies after callbacks and at applicable delayed handoffs. URI expansion
provides no universal inverse ingress matcher. The
[variable contract](../spec/interaction-core.md#uri-variable-execution) and
[actual finite path][finite-admission] already supply the bounded outbound seam.

Capacity covers retained calls, continuations, queues/staging, unread/late results
and cleanup, not just active I/O. A scheduling slot may be reused only after a
complete funded transfer; retained result backing cannot be refunded then.
Physical backing retains one owner and original account through final use;
references, operands and records pay their separate costs. Instance admission
accounts are not inferred from a definition's preparation bank. Enforce aggregate
and requested per-instance quotas without presenting overlapping allowances as
additional physical RAM. One shared bank does not establish per-Thing isolation.
[Foundation](../spec/foundation.md) owns these resource distinctions.

Dedicated per-instance capacity remains an optional implementation/profile.
Today's prepaid inline slot guarantees that storage, not every later call's
result/native capacity or successful permission checks. Stronger guarantees must
name and fund all promised resources. A work owner must remain reachable for
cleanup after public handles disappear: qualified finite local disposal or an
acknowledged complete transfer, with cleanup capacity retained under saturation.
Call cancellation, facet close, instance revocation and authorized whole-service
shutdown have separate scopes; none silently disposes a sibling's work. Retention
grants no current permission after authority Drop and no native shutdown right.

**Cross-family execution responsibilities.** Share qualification, dependency
checks, retained backing and funding mechanisms where their obligations coincide.
Keep the following semantic lifetimes distinct.

| Execution | Required retained state and behavior |
| --- | --- |
| Finite | Original input, selected meaning/mapping, destination, result and native settlement. Preserve first cause and late evidence; complete funded handoff precedes slot reuse |
| Continuing Action | Invocation identity, opaque continuation and separately qualified invoke/query/cancel associations survive the initiating exchange. Acceptance, pending/completion, local cancellation and remote cancellation uncertainty remain distinct |
| Consumer notification | Independent conversation/cursor, permission, limits, loss/stop/control and per-occurrence context/response ownership. Property state and Event subscription/data/dataResponse/cancellation roles remain distinct; a schema creates no reply right or wire backpressure |
| Producer exposure and emission | One original logical authority/behavior/publication across native participants; complete final-TD qualification and closed readiness before one local publication commit. Accepted requests/invocations/emissions retain separate result and retirement owners |

Named Event occurrences, Property maps/member sets and Action-status inventories
retain their exact collection semantics. A native root interaction never becomes
N leaf calls/subscriptions. Preserve actual member reports and atomicity evidence
without inventing guarantees. Producer emission is not Consumer subscription;
remote broker subscribers need no Engine record/copy loop. Protocol ACK, local
publication, recipient delivery, cancellation, effect certainty, settlement and
physical retirement are different observations. Never replay an uncertain effect.

TD owns interpretation; Planning coordinates pure bounded support/selection and
shared semantic loans without runtime source callbacks. Reuse actual prepared
owners rather than recompile and intern per instance; new TD input still needs
bounded admission. Actual native/codec/schema/security pairing remains mandatory.
Consumer support covers each complete selected interaction; Producer qualification
covers every retained final Form/effective operation, behavior and required
readiness. Static and Host representations need not require Arc, Clone or pointer
atomics in shared semantic contracts.

Bindings own native matching, correlation, distribution, token/wire conversion
and retirement. Core carries candidates/opportunities/failure evidence; Servient
authorizes, dispatches and decides logical fencing. A generic dispatch callback
would preserve F1. Separate native readiness/gate/coverage facts from Servient
membership/authority checks, including `ClosedProducerParticipation`.

**Inbound source attribution and authorization.** Application data such as
`{ meterId, reading }`, a native source claim, verified security context, an
affordance coordinate and logical Thing authority are different facts. A Fleet
Event can legitimately carry meter labels without making every meter a Thing or
claiming authentication. Configured services establish verified claims at their
actual trust scope; a broker principal or topic string does not by itself prove
a meter's identity. No universal device identity is selected.

When routing, policy or advertised guarantees depend on occurrence-specific
source context, the target Engine supports a bounded original occurrence context
that the actual mapping/verification services qualify. It retains the required
logical coordinate, source/verified claims and dependency association with the
original input/reply/backing. Engine acceptance and applicable delayed delivery
or response checks receive that context, fresh policy and original authority.
Missing/unproven required claims refuse; raw claims never silently become verified
principals. Native proof may remain opaque/native; typed or opaque checked views
are representation choices. No universal captures map or Core protocol matcher
is required. Applications supply domain meaning/policy/services; they do not
reimplement Engine acceptance after application delivery.

The present [candidate and policy interfaces][notification-interface] lack this
dynamic expression: [delivery authorization][notification-delivery] receives the
conversation target, and the [composed policy][notification-policy] uses its fixed
endpoint/operation.
It cannot inspect the root occurrence's Event name or verified source claims.
Two meters producing the same Fleet Event/value cannot receive different fresh
source-dependent decisions through that callback. A global last-source side
channel loses queued-item ownership and fails reentry. This is a missing stronger
capability, not a demonstrated bypass of the current whole-conversation grant.
Where a qualified association fixes and enforces the relevant source, its mapping
and recipient may already suffice without extra metadata. Finite Producer ingress
already has a [verification-before-behavior seam][producer-verification].

Concrete protocol evidence remains indispensable: the Engine cannot manufacture
missing authentication. MQTT's [wildcard attribution][mqtt-attribution] yields a
declared Event name, not arbitrary meter identity. `SubscribeAllEvents` is not a
fleet-device enumeration. Its [recipient Interests and native Topics][mqtt-sharing]
already permit compatible equal-filter sharing; distinct overlapping filters
refuse. Broader coalescing/security support is a separate capability, not an
Engine subscription-pool requirement. Sharing must qualify actual trust/tenant/
credential/configuration and delivery compatibility, preserve independent
cursors/loss/stop, and never duplicate an exclusive reply right or exchange
broadcast for competing-consumer semantics. Neither one subscription per Thing
nor one fleet-wide wildcard is mandated.

**Existing mechanisms and container dispositions.** Retain safety mechanisms
because their obligations remain, not their enclosing structs or API names.

| Existing implementation | Evidence and disposition |
| --- | --- |
| Finite content/mapping/binding | [Actual separation][consumer-definitions] supports shared meaning, specialized mappings and separate policy/accounts. [Content reuse][finite-preparation] covers Property Read/Write and ReadAllProperties; exact qualified rebinding is broader. Retain the responsibility split, not Consumer-only assumptions in shared meaning |
| Finite `ConsumerRuntime` | [One authority, frozen bindings, one bank and 1–32 prepaid inline slots][finite-runtime] form a valid bounded Host option. Preserve `FiniteCallState`, original-work checkout/reentry, result and cleanup safety; remove this package as the required entry for persistently usable associations |
| Continuing Action carrier/runtime | [Bundled meaning/mappings/services/preparation bank][action-definitions] and [private invocation slots][action-runtime] lack independent checked bind-many/capacity composition. Replace those couplings; retain the original invocation/control/continuation obligations |
| Composed/raw notification roots | [One definition/authority in the composed root][notification-runtime]; the [raw root][raw-notification] has one authority/provider and configured slots, several cursors and independently shareable native infrastructure. Decouple persistent associations from unused capacity and add the required occurrence-policy expression; retain conversation/response ownership |
| Producer owner/participants | [RawProducerServient][producer-owner] already holds multiple instances and one Action store; a [real TCP fixture][shared-listener] uses one listener for two instances. Preserve one logical authority/publication boundary and native leases; fix F1 rather than assuming every container has Consumer cardinality |
| Historical plan-set/route-stage/single-read entry points | Replace as general APIs where they enforce obsolete coupling. Keep actual identity/funding/retirement evidence; no mandatory plan-set epoch without an independently retained/replaced owner |

[ConsumerInstance][consumer-authority] can already exist without any Runtime;
[dynamic HTTP execution][dynamic-execution] addresses different destinations and
independent Consumers with one qualified definition. These disprove a universal
device → Thing → Runtime → connection rule, but do not implement independent
capacity for persistent Host associations or arbitrary inbound attribution.
On-demand construction remains a baseline only with an explicit root/work
lifetime: [dropping the finite root][finite-drop] fences its accepted calls.
[Caller-owned Read][borrowed-read] has a different funding representation but
still contains one instance and an inline work option, not a general shared bank.

Consolidate schema preparation/evaluation and retained service owners. Preserve
admission-time dependency snapshots through binding and work: changed support/
bounds require refusal or explicit requalification/funding, not a new baseline.
Permission, availability and credentials still require fresh checks. Share actual
service backing once, importing its original owner. Provide heterogeneous
optional facets and startup assembly with those associations; neither an
application-only wrapper nor a new mega-registry repairs missing ownership.
`Any` storage is not inherently wrong. Notification selection's concrete `B`,
repeated Producer extraction/downcasting in [mixed assembly][mixed-assembly] and
the restricted public `Servient` alias are composition limits to replace where
they obstruct the target. HTTP's same-native-owner Action inventory remains a
declared mapping restriction, not an Engine rule. Unsupported combinations may
refuse explicitly; sharing cannot weaken required schema/security/response meaning.

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

**Proposed engineering sequence.** Increments establish ownership guarantees,
using concrete paths to falsify them; they do not promote existing family
containers into the target. Correct F1 before broader Producer authority claims
and F2/F3 before accepting affected service/binding handoffs. These bounded repairs
can proceed alongside disjoint capability work; none alone proves the target.

1. **Independent associations and funded work admission.** Deliver persistently
   usable associations with checked reusable meaning/mappings, independently
   chosen instance/work accounts and capacity supplied or acquired at admission.
   Use existing real HTTP finite execution for both one logical Thing addressing
   multiple permitted destinations and more independent Things than provisioned
   calls. Compare the existing on-demand-root baseline with the simplest separate
   work owner; no pool is prescribed. Demonstrate no repeated invariant preparation
   or mandatory idle slot per association, incompatible/fixed-operand refusal,
   aggregate/per-instance limits, saturation, callback reentry, independent
   revocation and funded normal/late result disposal after source/root retirement.
   A finite success establishes this admission seam, not incoming source trust,
   continuing lifetime or Producer readiness. Check the handoff against all
   cross-family obligations above before freezing its API.
2. **Original occurrence context and independent native participation.** Deliver
   the bounded mapping/verification-to-policy expression and retain it through
   queueing, delayed delivery, response and cleanup. Exercise one Fleet Event with
   distinct qualified sources and fresh source-dependent permission, plus
   independently authorized Things/conversations sharing compatible native
   infrastructure. Use MQTT's real shared-interest path for its supported claim;
   use an actual evidence-bearing notification mapping/verifier for the source
   claim. Plain NoSec MQTT cannot prove authenticated meters. Include forged,
   absent, ambiguous and stale context, policy change while queued, independent
   stop, loss, original reply rights and final release. This establishes the
   selected topology/trust contract, not general wildcard merging, every security
   scheme or fleet throughput. F2/F3 repair and checked notification bind-many
   belong in this handoff; a post-delivery application filter is not completion.
3. **Retained obligations across exchange and publication lifetimes.** Apply the
   same association/capacity boundary to continuing work and independently bound
   exposures. Real HTTP Actions must retain original invoke/control mappings,
   continuation, permission, accounts and late/unread outcomes after the initial
   exchange ends, including replacement and uncertain remote cancellation.
   Qualified Producer exposures must reuse eligible meaning with independent
   behaviors/policies/publication, compatible native leases and complete final-TD
   qualification. Extract F1 through native candidate/opportunity/failure facts;
   Servient alone dispatches and fences logical authority. Exercise two exposures,
   failure before/after publication, one owner's revocation and differing work/
   native retirement orders. Action evidence does not establish Producer coverage;
   exposure readiness does not establish recipient delivery or universal native
   sharing. Neither permits a universal session/call representation.

Startup composition and obsolete-path removal accompany each increment, not a
final facade around frozen containers. Mixed finite, notification, Action and
Producer participants must keep independent currentness, funded results/cleanup
and progress when an unrelated supported participant stalls. Preserve caller-owned
finite execution and portable prepared meaning; qualify only the profiles and
combinations actually exercised. These are scoped completion boundaries, not a
new global all-protocol gate or a change to roadmap priority.

**Alternatives, open choices and risks.** Dedicated per-instance roots remain a
valid bounded option; making them the sole persistent association representation
would retain the identified coupling. Application fan-out is legitimate for a
Fleet interaction, but cannot stand in for requested independent Engine grants
and cursors. Sharing only schema allocations leaves repeated qualification and
instance handoffs unresolved; a universal Runtime/scheduler/cache/matcher adds
unproved machinery. Prefer independently funded work and family-specific states
over either workaround.

Caller-supplied versus admission-allocated storage, optional bounded work banks,
typed versus opaque occurrence views, static versus Host erasure and native
grouping remain implementation choices. They must satisfy the target; allocation
measurements can choose a representation, not decide whether ownership and
authorization distinctions exist. Specialization, native trust limitations and
explicit unsupported combinations are valid when truthful; they are not permission
to drop source meaning or weaken required support. No universal authenticated
device identity, automatic cache, shared slot pool or mandatory new type hierarchy
is selected.

Principal risks are early refunds, stale dependency/context capture, authority
leakage, weakened Producer coverage, excessive shared-owner retention and stalled
cleanup/progress. Validate original ownership and refusal rollback at their actual
boundaries. Breaking carrier/constructor/adapter changes are preferable to pre-v1
compatibility layers that retain obsolete coupling. Future changes require the
applicable PREP, EXEC, SPI, SAFE, RES and MOD-DEPS authority/API/work-package
projections; this assessment changes none of them. Independent architectural
acceptance and broader interoperability/profile/release qualification remain
separate from passing scoped tests.

**Recorded validation and limits.** The original assessment recorded 133 passing
focused tests at the reviewed source: 59 HTTP reuse/Action/composed-Producer tests,
47 downstream integration/Action/notification tests and 27 MQTT qualification
tests. Exact commands:

```sh
cargo test -p clinkz-wot-binding-http --test reusable_execution --test composed_thing --test action_consumer
cargo test --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml --test action --test integration --test notification
cargo test -p clinkz-wot-binding-mqtt --test consumer_qualification --test qualification --test property_qualification --test collection_qualification --test event_collection_qualification
cargo tree -p clinkz-wot-binding-http --no-default-features --edges normal --depth 1
cargo tree -p clinkz-wot-binding-mqtt --no-default-features --edges normal --depth 1
```

That assessment's probe reproduced F2/F3 through public preparation/runtime APIs
without network progress. Its assertions preserve reviewed defects, not desired
general behavior; retire or invert the relevant assertions when corrections land. Native
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

Mosquitto was unavailable during the original assessment, so broker integration
tests were not rerun. This prose revision does not rerun or expand those runtime
claims. Independent-capacity composition, occurrence-context authorization,
cross-family bind-many, sustained mixed-workload progress and broader portable
execution remain target capabilities. Neither benchmarks nor passing tests
substitute for the ownership reasoning above. Required diff hygiene is not
whole-project acceptance.

[discussion]: https://github.com/yushun1990/clinkz-wot/pull/275#issuecomment-6742511702
[consumer-definitions]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/core/src/consumer.rs#L560-L1008
[consumer-authority]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/core/src/consumer.rs#L263-L313
[finite-runtime]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/consumer.rs#L600-L663
[finite-admission]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/consumer.rs#L988-L1017
[finite-drop]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/consumer.rs#L1192-L1199
[borrowed-read]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/borrowed_consumer.rs#L108-L187
[dynamic-execution]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/protocol-bindings/protocols/http/tests/uri_variables.rs#L359-L445
[notification-interface]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/core/src/notification.rs#L317-L410
[notification-delivery]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/notification.rs#L1121-L1170
[notification-policy]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/consumer_notification.rs#L15-L47
[raw-notification]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/notification.rs#L284-L387
[producer-verification]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/host.rs#L606-L636
[producer-owner]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/host.rs#L340-L383
[shared-listener]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/tests/native_ingress.rs#L287-L325
[mqtt-attribution]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/protocol-bindings/protocols/mqtt/src/consumer.rs#L519-L542
[mqtt-sharing]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/protocol-bindings/protocols/mqtt/src/consumer.rs#L624-L687
[action-definitions]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/core/src/consumer_action.rs
[finite-preparation]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/planning/src/open_consumer.rs
[action-preparation]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/planning/src/open_consumer/action.rs
[notification-preparation]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/planning/src/open_consumer/notification.rs
[action-runtime]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/consumer_action.rs
[notification-runtime]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/servient/finite/src/consumer_notification.rs
[http-action-runtime]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/protocol-bindings/protocols/http/src/action_producer.rs
[mqtt-native]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/protocol-bindings/protocols/mqtt/src/native.rs
[mixed-assembly]: https://github.com/yushun1990/clinkz-wot/blob/f6773277a1a3f854465fe94f14b60765451e400c/protocol-bindings/protocols/mqtt/tests/collection_coexistence.rs
