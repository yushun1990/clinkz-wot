# Host HTTP Producer Property Read: implementation admission

Design revision 5.1; tranche `WP-600-HTTP-PRODUCER-PROPERTY-READ`.
[index.toml](index.toml) owns admission state, dependencies, paths, checks and
completion keys. **Implementation admission is accepted** by the
[independent PR #216 review](https://github.com/yushun1990/clinkz-wot/pull/216#pullrequestreview-5583560319)
at `90abeb1564b6da1e0d2001c5c2c3b587b48eb465`. Production implementation and
completion evidence remain outstanding. No gate, package or milestone is closed.

## Decision and alternatives

Deliver one real Host Property Read endpoint and the public Consumer-to-Producer
HTTP loop in one tranche, including its necessary shared corrections. Use
TD-resolved input, a small immutable Producer execution entry, a paired open
Producer compiler/executor and one retained endpoint driver. Keep Servient's
publication and borrowed accept permit; replace the Host phase-specific setup
and recovery families on this path. The exact contracts belong to
[Planning](../spec/planning.md#host-producer-property-read-entry) and
[Binding SPI](../spec/binding-spi.md#host-producer-endpoint-contract).

| Alternative | Disposition |
| --- | --- |
| Add HTTP behind today's first-route compiler | Rejected: method/security/response distinctions never reach it, and uncovered advertised behavior reaches preparation. |
| Reject risky declarations in an HTTP-specific Engine preflight | Reject as the shared solution: it duplicates vocabulary ownership and leaves the Producer compiler bottleneck. A generic whole-description coverage check is still necessary. |
| Resolved compilation plus today's four public setup phases | Feasible fallback; old fixtures show ownership, not necessity. The phases repeat owners, cancellation and recovery contracts for an HTTP listener whose native setup needs no such split. Prefer one driver. |
| Selected single-route entry and retained driver | Smallest useful replacement: no persistent logical-plan prerequisite, compiler whitelist, public native handshake taxonomy or session manager. Real HTTP is its completion boundary. |
| Whole Engine rewrite, universal protocol program or capability scoring | Deferred/rejected for this task: no dependency on general owned/resumable Consumer compilation, caching, embedded runtime replacement or reverted #212. |

This migrates only the Producer subset of
[0056](../../workspace/0056-target-spi-external-validation-and-cross-protocol-neutrality.md#producer-admission-subset).
#206 accepted a replacement direction, not an implemented SPI. #211's Python
HTTP/MQTT experiments are bounded semantic evidence, not production admission.
#214's resolved tree and public Consumer are reusable implementation, while its
`HostSemanticCompiler` adapter remains Consumer-only. #215 supplies engineering
principles, not a new gate. [#204](https://github.com/yushun1990/clinkz-wot/pull/204)
was closed without merging; its observations must be rechecked on current code.

## Evidence and blockers in the existing path

The reconstruction baseline is master `17f22b6170856a822fb77afe427c3532cf23e295`.
These are source defects to remove inside the admitted implementation, not
unimplemented predecessor contracts being counted as complete:

| Source evidence | Consequence / executable discriminator |
| --- | --- |
| `planning/src/property_read.rs::property_read_plan` returns from the first read-looking Form and uses `None => true`; TD's `effective_form_operations` handles read/write/write-only defaults. | Default write-only reaches a read handler; default read/write and extra Forms/properties are incompletely covered. |
| The same module constructs `BindingCompilerInput::new` without `with_semantics`; `core/src/binding_compiler.rs::SemanticCompilerAdapter::check_identity` accepts only `ConsumerCall`. | Current Producer compiler callbacks observe `resolved() == None`; #214 cannot simply be registered as a server compiler. |
| Static and Host `servient/src/property_read.rs::step_planning` check only `derived.target` against `handler_name` before freezing and preparing. | Twelve Basic-valid mutations produce equal plans and reach one preparation/handler/response in the public Host mock path. This demonstrates missing admission, not an HTTP authentication exploit. |
| `core/src/binding.rs` rejects non-`StrictV1` Producer declarations; Consumer's Host-owned policy is role-specific. | Do not present transitive nonallocating StrictV1 as qualified for Hyper/Tokio. The scoped Host Producer ownership contract below is required. |
| `servient/src/property_read.rs::HostRouteState` and the Host lifecycle calls in `core/src/binding.rs` repeat phase-specific recovery owners. | Simplification motivation, not a demonstrated native impossibility. Replace the families while preserving owner, permit, first cause and close obligations. |

The committed diagnostic in the existing architecture runner reproduces thirteen
cases: explicit POST, response media/status, coding, empty scopes, Form/root
Basic security, second Form/property, default read/write, default write-only
advertised Profile and an unhandled vendor instruction. It also verifies complete mock cleanup. The old static
and Host runner tests remain the reusable baseline for permit/late-result and
cleanup invariants. They do not prove a real HTTP Producer.

## Standards, guarantees and supported semantics

**W3C normative baseline:** [TD 1.1](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#default-value-definitions)
defines operation defaults; [Form security overrides](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#security-in-forms)
replace inherited requirements. [HTTP assertions](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#http-binding-assertions)
give GET for omitted read methods and honor explicit methods; one method cannot
be attached to a multi-operation Form. `profile` advertises mechanisms followed.
These rules do not prescribe Rust compilation stages, atomic publication or a
general requirement that implementations support every TD feature.
The [2024 Binding Templates](https://www.w3.org/TR/2024/NOTE-wot-binding-templates-20240528/)
is a Group Note with mapping guidance, not a Recommendation adding another
normative requirement here.

**Project engineering guarantees:** freeze one truthful advertised description;
require complete route, handler, security and mapping coverage before effects;
publish local dispatch atomically; dispatch only with an Engine-issued permit;
retain every accepted owner through failure and close. Requiring every retained
advertised route is a conservative project policy, not a universal WoT rule.

**First implementation limits:** Host/std, cleartext HTTP/1.1 loopback IP,
one synchronous `ReadPropertyHandler`, one Property with exactly one Form
whose effective operation set is exactly
`readproperty`; omitted op is supported when TD defaults produce that set.
Explicit read-only op need not require `readOnly:true`. Support default GET and
explicit GET (including supported local aliases), effective endpoint NoSec,
effective JSON, primary status 200 and relative href resolved against base.
Support source TD id absence and any TD-valid Property key, including empty;
local slot/generation, not source spelling, establishes authority.

The application supplies the concrete nonzero address/port before compilation;
the Binding verifies agreement with its immutable listen configuration. No port-0
TD rewrite, automatic Form contribution, Directory service or TD fetch endpoint.
Published description retrieval may retain an explicit separately charged typed
source owner; neither execution nor the compiled entry accesses it.

Reject the entire draft before native effects for extra properties/Forms,
Actions/Events/top-level Forms, any other effective operation, missing exact read
handler, ambiguous/unsupported registration, security/proxy, non-GET methods,
required custom HTTP headers, templates/URI variables, non-JSON/coding/scopes/
subprotocols/additional responses or unsupported primary response instructions.
Do not silently skip an unsupported sibling or rewrite its advertised semantics.
Zero/empty/malformed declarations remain distinct from absence. Pure private
compilation may fail; it never creates sockets or invokes handlers.

Standard TD descriptive metadata and parent `@type` class assertions remain
usable. The first Producer compiler conservatively declines other unhandled
extension predicates, including apparent annotations: it cannot promise a
published vendor instruction while silently treating it as descriptive. This
is a narrow availability limit, not a universal WoT critical-extension rule,
an application vocabulary inventory or a change to #214's Consumer policy.
Concrete Binding compilers retain the resolved content and can add qualified
mappings without Engine protocol cases. Known unsupported HTTP instructions
also refuse. Reject any advertised Profile in this initial Producer scope;
preserving a Profile URI alone does not establish the corresponding Thing's
behavior. This is an implementation availability limit, not W3C invalidity, and
does not select #211's experimental profile interpreter or #212's framework.
TD's bounded local-context/schema resolver restrictions stay explicit.

**Deferred:** multiple routes/properties, shared listeners/sessions, additional
methods/headers, authenticated security/HTTPS, DNS/non-loopback deployment,
templates, streaming, Profiles and embedded production. HTTPS/security remains
required before release. None is prohibited for WoT in general.

## Execution, ownership and limits

The HTTP crate adds `HttpProducerConfig::for_loopback(SocketAddr)` (rejecting
non-loopback or port zero) and a fallible `try_producer_registration` returning
a complete Producer-only registration or its original author input. Neither
constructor binds a socket or requires a dummy client. Configuration fixes the
listen address, limits and disposal identity before compilation.

Use `ServientBuilder::binding_registration` -> `produce_td` -> exact read-handler
registration -> `begin_expose` -> caller-driven `Servient::step` -> Serving.
After accepted replacement, these roots use the new Host Producer contract;
they must not retain the unsafe first-leaf path as a fallback. Existing public
Consumer registration/consume/select/read executes the opposite end of the loop.

The build privately validates the whole TD, enumerates advertised operations,
checks coverage and resolves/lower its sole supported coordinate. It reserves
entry, driver, ingress, response and cleanup capacity before installation.
The first driver poll may bind/listen, which is externally visible. Closed-gate
connections receive bounded backpressure or closure, never a semantic event or
handler call. Local readiness promises serviceability under the permit, not
remote reachability. Publication does no fallible Binding callback.

One exposure owns one listener, one accepted connection, one bounded request
slot and one response opportunity. Hyper parses/framing; its service callback
only bridges native request/reply state. Servient selects/invokes the handler
and applies the existing Core handler-result shape seal; media and byte bounds
are checked before delivery. Application handlers remain responsible for data
conforming to their TD schema; general schema compilation/validation is deferred,
as in the existing one-shot contract. Polling the HTTP driver must not synchronously wait for
Servient to dispatch. A response future/opportunity may remain Pending without
another connection or a detached task. Saturation rejects/backpressures before
another accepted Engine owner; queued kernel connections are deployment state.

First limits: target <=64 UTF-8 bytes, head <=16 fields/8192 raw bytes inclusive,
read <=4096 bytes per funded poll, bodyless request (absent/zero Content-Length,
no Transfer-Encoding), exact origin-form path/query and Host authority,
no pipelining/keepalive/upgrade/Expect, and validated JSON response <=4096 bytes.
Use `Connection: close`; unsupported requests close or receive bounded fixed
400/404/405/413/503 responses without handler dispatch. Error bodies contain no
peer data. Slow header/write progress has a caller-clock deadline. Charge
`BindingPolls`, `HandlerSteps` and `CleanupItems` before corresponding work;
zero credit cannot bind, accept, parse, dispatch or close-poll.

Engine-owned allocations use original local/parent accounts, actual generic
Layouts, peak overlap and prebacked cleanup/status destinations. Introduce the
role-specific `ProducerOwnedHostV2` ownership interpretation under ADR-0023:
controlled carriers and indirect retained Engine backing remain funded through
actual release; socket/parser/SDK backing has an explicit Binding policy and
measured growth/disposal envelope. The accepted Consumer schema-2 overlay/keys
remain Consumer-only; the Binding SPI owns the Producer field-family mapping
and fresh role/configuration identity. No transitive StrictV1 claim, new global
resource defaults or borrowing Consumer seals. Host process OOM/abort stays a
deployment failure under ADR-0023. Partial-install failure and last root loss
use the same driver close owner; no shared manager exists in this scope.

Close stops permits before closing listener/connection. Cancel/timeout preserves
the first cause; late requests, handler outcomes and delivery completions retain
their exact generation/response slot until disposed. A refused cleanup transfer
leaves the original owner live. Finite SDK-local disposal may allocate under the
declared Host policy; Engine status/transfer remains prebacked. The last root
may have no poller: the first scope requires finite synchronous local destruction of all remaining driver resources, with original Engine
accounts retained through that destruction. No network wait,
blocking join or unowned task tail. Socket retirement is a local certificate,
never proof that a peer observed nothing.

## Review 08: proposed architectural amendment

[Review 08](../reviews/review-08-integrated-architecture-reassessment.md#reassessment-of-the-accepted-http-producer-admission)
preserves the value of whole-description coverage, a source-free entry, one
retained setup/close owner and Servient publication/permits. It recommends
revising the generalization boundary before execution follows that direction:
owned programs/program references beyond `Copy`, independent preparation and
driver construction, shared native infrastructure leases, common Host/static
semantics and consistent optional-id/Property-key handling across roles.

The reworked candidate now demonstrates the
[definition/bind boundary](../reviews/review-08-integrated-architecture-reassessment.md#constructed-definition-and-instance-boundary)
and [shared Producer topology](../reviews/review-08-integrated-architecture-reassessment.md#constructed-shared-producer-boundary).
It places preparation contracts in Planning and resolved input views in TD,
with no TD dependency in execution-only Core. It requires one reusable program
owner independent of instance accounts, actual compiler/runtime policy checks,
and separately owned native progress/retirement. Multiple route leases preserve
Engine routing/publication/permits; a route's stop cannot close sibling service.
The shared-boundary correction permits Binding-owned native matching/captures
with zero/many independently checked logical recipients; equality remains an
exclusive-unicast adapter. Only final route participation release initiates ordinary
retirement, with polling unable to disable a live sibling. The UDP and separate
decoded-packet matching sketches neither execute this HTTP admission nor qualify
its SDK/disposal, broker sessions or a long-lived WoT capability.

These are **proposals**, not newly admitted Producer capabilities or exceptions.
The recorded accepted scope, dependencies, strict limits, six completion keys
and final-root obligations remain unchanged. The whole-description and actual
HTTP loop evidence are still outstanding. Independent architectural review must
settle the proposed scope/contract amendment before source work implements it.
This adds no package gate, status transition or
universal shared-session requirement to the current admission.

## Preconditions and impact

Completed/current `WP-600-HTTP-CONSUMER-PROPERTY-READ` and its indexed transitive
Consumer predecessors supply the real client and shared TD/resource machinery;
WP-000 is complete. The passed Producer gate supplies only its original
static/Host mock invariants. These are prechecks, not a claim that its flawed
whole-description coverage is accepted for HTTP. Broad WP-300 completion and
general Consumer compiler openness are not new predecessors.

All Producer semantic/entry/driver/Host-resource corrections are **inside this
tranche**. Implementing HTTP against imaginary completed corrections is forbidden.
The first coherent implementation checkpoint is TD-owned Producer lending and
generic coverage rejection before install, with the thirteen counterexamples
inverted plus supported positives. Then pair the actual HTTP compiler/driver,
retire the old Host first-leaf/stage path and run the real loop. Checkpoints do
not create separate admission units or waive the single completion boundary.

| Existing evidence | Impact disposition |
| --- | --- |
| Consumer TD/aggregate/Core/call/runtime/HTTP accepted scopes | Reuse source-specific acceptance; rerun indexed regressions. Renew changed shared Layouts, work and account peaks in this tranche's evidence; expand impact review for any changed Consumer semantics. No blanket reopening. |
| Producer Host registration, route/build/activation and response ownership | Replace Host phase-specific implementation evidence with single-driver ownership/permit/failure tests and actual HTTP. Preserve relevant negative oracles. Old gate results retain their original scope; no self-authored gate-state change. |
| Static/no-std Producer contracts and legacy unmatched capabilities | Remain separate existing paths; compile/runtime regressions required. No static HTTP or fixed-device claim. Shared semantic helpers cannot keep the known default/coverage defect where changed; widen scope before merging a newly intersecting change. |
| #206/0056 remaining compilation/reuse/other-protocol questions | Disjoint after the explicit Producer subset migration. No wholesale SPI migration or new proof framework. |

## Completion evidence and independent review

Write `docs/evidence/WP-600-http-producer-property-read.toml` only with real results,
exact source/dependency/configuration revisions and independent acceptance.

| Stable key | Acceptance |
| --- | --- |
| `producer-semantic-coverage` | TD default/security/alias/response fidelity, absent id/empty key positives; all thirteen old defects become supported truthful behavior or zero-effects refusal. Additional Form/property/action/event/top-level operation and missing handler/security fail before bind/start/handler/publication. Drop build source before runtime execution. |
| `producer-install-ownership` | External non-HTTP compiler/driver enters through the same paired public registration without Engine edits. Immediate/Pending readiness, occupied-port and partial-install failure, cancellation before/after bind/readiness/publication, rejected cleanup transfer, late result and stale generation preserve one owner and terminate. No phase-shaped adapters. |
| `http-producer-property-read` | Real raw independent TCP peer sees correct GET/path/Host, 200/JSON, at most one handler and one response; explicit/default GET and aliases work. Before Serving no handler/event/success response. Wrong method/target/authority, body/framing, exact/one-over/unfinished headers, slow/disconnected request and write refusal never bypass limits. Actual post-publication failure closes authority. |
| `http-consumer-producer-loop` | Existing public Consumer talks to production Producer over loopback; two distinct sequential generations/targets/results and >=100 repeated reads after source Drop, no legacy/raw-client shortcut. Cancel/deadline/client/root loss then fresh successful read/exposure. No unbounded retained growth. |
| `producer-resource-retirement` | Actual allocations/Layout/indirect backing/peak overlap, SDK request-parser read-ahead/growth and finite disposal observations; allocation/FD/ingress/response pressure, unfinished request, unread response and last root loss. All listener/socket/future/opportunity owners settle exactly once; original accounts return to baseline. Separate enforced limits, observations and deployment assumptions. |
| `producer-regression-conformance` | Listed prechecks, portable compile cells and existing Consumer/Producer resource/permit regressions pass. Renew affected measurements, inspect normal dependency graph (no HTTP/Tokio in neutral crates), run locked workspace checks and existing performance oracles. No new network-latency/embedded claim. |

The independent reviewer must challenge information sufficiency, full-description
refusal, artifact/executor pairing, the one-driver failure/permit/response bridge,
role-specific Host accounting and source-free public composition. Native-only
prechecks cannot accept those production claims. A discovered missing boundary
returns to this same ADR-0013 record before production changes continue.
