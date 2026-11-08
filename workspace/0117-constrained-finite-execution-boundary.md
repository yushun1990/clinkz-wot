# Finite execution without mandatory Host ownership

Status: MIGRATED

Basis: master `2c474cdb67f073ad0551ad5458edc49f786e0541`, after #270.
This investigation reaffirms ADR-0025 and SAFE-PROFILE-001. It corrects an
implementation boundary, not the selected architecture or release scope.

Property Read requires an addressed readable value, its prepared DataSchema,
an actually supported selected Binding mapping and fresh destination/instance
permission. A Binding supplies native response evidence; the Engine checks
original mapping identity, terminal response shape, media and value constraints.
The [WoT Architecture 1.1 Recommendation](https://www.w3.org/TR/2023/REC-wot-architecture11-20231205/#sec-interaction-model)
does not prescribe Rust ownership or scheduling. Cancellation, first cause,
late-result retention, resource funding and stale-reference fencing are retained
project obligations, not claims that W3C prescribes these Rust APIs.

Repository inspection found the following separation:

| Concern | Independent obligation | Existing implementation choice |
| --- | --- | --- |
| Immutable meaning | Exact coordinate, operation, value roles and security operands | Host name Arc; prepared schema Box/Vec for variable trees |
| Definition | Actual compatible program, codec, service support and native mapping | Host registration erasure and funded Arc owners |
| Authority | Original issuer/incarnation, passive currentness, no revival | Host Arc/AtomicBool; caller loan/Cell is also sufficient |
| Admitted work | Non-reused obligation identity, bounded progress, first cause and late output | Host Rc registry, boxed pinned calls and wake integration |
| Result | Original output and funding survive until disposal; views cannot escape | Host Arc payload versus inline array and caller lease |
| Native resources | Binding owns I/O, correlation and physical release | Sockets, mutexes, threads and concrete drivers are native choices |

The Core module gates concealed portable identity, errors, deadlines, metadata
and borrowed views. More materially, modern finite progress lived inside a
Servient owner whose definition and services required Host accounts and Arc.
Planning already prepares source-independent value constraints, but its modern
TD selection/compilation implementation is Host-only. Its output owner is not
a necessary execution contract. Historical static Property Read remains tied
to superseded PlanSet assumptions and cannot establish the modern boundary.

Alternatives considered:

- Keep current gates and cite historical static execution: no modern proof.
- Replace Arc with Rc: still mandates reference counting and allocated owners.
- Parameterize every payload, service, instance and runtime: spreads storage
  policy through unrelated capabilities and changes correct Host machinery.
- Select a narrow borrowed value loan and extract actual shared finite progress:
  preserves working Host owners while exposing the demonstrated common boundary.

Selected: Core owns `ValueExecution`, borrowed output/metadata and the finite
transition kernel. The Host finite Consumer uses that same kernel. Core's
caller-owned Read facet defines native calls, output loans and actual services
without a Servient dependency. Servient provides one exclusive caller-owned slot
over those contracts. Its authority uses lifetime-bound issuer identity and
non-reused per-incarnation sequences; a complete caller reservation moves with
work and settlement. Immutable dependencies stay caller-owned through safe loans.
This slice supports NoSec Read only; unsupported security/schema meaning refuses
before native construction. It introduces no replacement TD interpretation.

The external test Binding and numeric codec compile without std/atomics and
execute real datagrams with the same Core evaluator on the host. Tests exercise
pending work, short credit, actual policy callbacks, invalid and late responses,
revocation/replacement, cleanup failure/acknowledgement, socket disposal and lease
reuse/refund. Separate Host regressions cover its existing native representations.
No identical storage, allocation cost, wake trace, SDK, hardware, general static
DataSchema storage or broad profile maturity is claimed.

Migration owners: interaction-core (shared meaning/progress), binding-spi
(native facet), runtime-safety (authority/cleanup/storage), module boundaries,
API ownership and WP-100/400 scope. Executable commands and limitations live in
`docs/evidence/constrained-finite-execution.md`. Prior historical gate statuses
and evidence scopes remain unchanged. No new milestone or gate is introduced;
capability work returns to common Security composition and execution semantics.
