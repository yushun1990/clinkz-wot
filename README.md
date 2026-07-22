# clinkz-wot

`clinkz-wot` is a protocol-neutral Rust Web of Things engine targeting the
W3C WoT Scripting API across both host and constrained environments.

The engine uses W3C WoT Thing Descriptions as semantic contracts and separates
protocol-neutral interaction behavior from protocol-specific transport code.
Zenoh is the first concrete Protocol Binding.

## Project status

The active target is the **v4.9 architecture-closure revision**.

The repository still contains implementation surfaces from earlier revisions.
Those surfaces are migration inputs and must not be treated as the target
architecture when they conflict with the registered v4.9 architecture,
specifications, ADRs, or work-package records.

The completed v4.9 implementation work currently consists primarily of the
admitted foundation refresh. Handler, Planning, Binding SPI, Servient, and
Zenoh migrations proceed only through independently reviewed implementation
tranches.

The project is under active architectural and implementation refactoring. It is
not yet a stable production release.

## Architectural direction

The v4.9 target follows these primary rules:

* W3C WoT TD 1.1 is the default compatibility target.
* TD and TM documents are lossless semantic data contracts, not runtime state.
* Interaction hot paths execute immutable admitted plans.
* TD parsing, defaulting, form selection, and capability matching happen before
  interaction execution.
* Servient owns application orchestration, plan-set lifetime, lifecycle
  transactions, scheduling policy, activation, and cleanup ownership.
* Protocol Bindings own protocol syntax, transport I/O, correlation,
  protocol-local flow control, and binding-local state.
* A Protocol Binding does not invoke application handlers directly and does not
  reinterpret a TD during the interaction hot path.
* Protocol Binding crates are linked through ordinary Cargo composition and
  explicitly registered by the application.
* Binding composition for one Servient instance is startup-only in v1.
* Runtime loading and unloading of Protocol Binding code is not a v1 feature.
* Every queue, cache, cursor set, retained operation, type-erased object, and
  external-input buffer has an explicit admitted bound.
* `no_std + alloc` uses caller-driven progress and bounded storage while
  preserving the same protocol-neutral semantics as host builds.
* Every operation that can outlive its caller has one generation-bearing owner,
  an explicit cancellation path, a bounded retained footprint, and a terminal
  cleanup disposition.
* User, provider, codec, and binding callbacks execute outside engine locks and
  constrained critical sections.

## Primary runtime model

```text
Thing Description or produced-Thing draft
                    |
                    v
       parse, preserve, and validate
                    |
                    v
        capture planning inputs
                    |
                    v
           shared logical planner
                    |
                    v
        immutable logical plan set
                    |
                    v
       Protocol Binding compilation
                    |
                    v
       immutable binding artifacts
                    |
                    v
      Servient preparation and activation
                    |
                    v
       engine-orchestrated progress
                    |
          +---------+---------+
          |                   |
          v                   v
 application handler     Protocol Binding I/O
          |                   |
          +---------+---------+
                    |
                    v
        completion and cleanup
```

A Protocol Binding translates between protocol-specific traffic and the
protocol-neutral runtime contract. Servient remains the orchestration authority
for route resolution, handler execution, activation, operation lifetime, and
cleanup.

## Documentation authority

Read active project material in this order:

1. [`docs/design.md`](docs/design.md) selects the active revision and indexes
   the normative sources.
2. [`docs/architecture/README.md`](docs/architecture/README.md) introduces the
   architecture backbone and its reading order.
3. Accepted decisions under [`docs/ADRs/`](docs/ADRs/) record architectural
   choices and rejected alternatives.
4. Registered domain specifications under [`docs/spec/`](docs/spec/) own
   detailed behavior and public contracts.
5. Machine-readable API, state, resource, requirement, performance, and
   work-package artifacts own their exact projections.
6. [`PLAN.md`](PLAN.md) reports navigation, admission status, and implementation
   progress.

Reviews, audits, evidence records, deprecated documents, and files under
[`workspace/`](workspace/) provide history and convergence support. They are not
architecture authorities unless their conclusions have been migrated into the
registered normative owners.

A conflict between normative sources is a gate failure. It must not be resolved
by selecting whichever document appears newer or more detailed.

## Architecture backbone

The concise architecture backbone is organized as:

1. [System goals and context](docs/architecture/00-system-goals-and-context.md)
2. [Primary data flows](docs/architecture/10-primary-data-flows.md)
3. [Module boundaries](docs/architecture/20-module-boundaries.md)
4. [Compiled-plan lifecycle](docs/architecture/30-compiled-plan-lifecycle.md)
5. [Protocol Binding SPI and deployment](docs/architecture/40-protocol-binding-spi-and-deployment.md)
6. [Servient runtime lifecycle](docs/architecture/50-servient-runtime-lifecycle.md)

Detailed specifications must project this backbone without silently redefining
its cross-module invariants.

## Workspace crates

The repository is organized as a Rust workspace. Important areas include:

| Area                                 | Role                                                                                                                       |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| `foundation/`                        | Bounded resources, profiles, work budgets, time and generation foundations, and shared low-level contracts.                |
| `td/`                                | Thing Description and Thing Model data structures, parsing, serialization, validation, and extension preservation.         |
| `core/`                              | Protocol-neutral interaction values, handler contracts, operation state, status, errors, and runtime-facing SPI semantics. |
| `planning/`                          | Logical planning, capability matching, immutable plan construction, and validation of untrusted binding outputs.           |
| `protocol-bindings/core/`            | Protocol Binding authoring contracts and shared protocol-binding support.                                                  |
| `protocol-bindings/protocols/zenoh/` | Zenoh-specific planning, transport, and runtime integration.                                                               |
| `servient/`                          | Application orchestration, plan-set ownership, lifecycle coordination, scheduling, activation, and cleanup.                |
| `discovery/`                         | WoT discovery and Directory-related client and provider capabilities.                                                      |
| `tools/`                             | Executable design checks, governance checks, fixtures, and conformance support.                                            |
| `docs/`                              | Architecture, specifications, ADRs, requirements, work packages, reviews, audits, and evidence.                            |
| `workspace/`                         | Non-authoritative design discussions and proposals awaiting decision or migration.                                         |

Some crate and API boundaries are still being migrated. The architecture
backbone, registered specifications, and work-package DAG define the target
ownership when current source layout differs from the v4.9 target.

## Implementation admission

The project does not migrate the entire runtime in one coordinated edit.

Implementation proceeds through scoped tranches:

```text
architecture backbone
        |
        v
exact tranche contract
        |
        v
independent entry review
        |
        v
implementation
        |
        v
completion evidence
        |
        v
downstream admission
```

A tranche must identify:

* its exact API and implementation paths;
* affected requirements and state machines;
* resource and workload impact;
* dependencies and downstream removals;
* pre-implementation contract checks;
* post-implementation completion evidence;
* treatment of relevant prior evidence.

Scoped admission does not close the global architecture or release gates.
Conversely, an unrelated open global finding does not automatically block a
tranche that has been proven disjoint.

See:

* [`docs/ADRs/0013-work-package-scoped-implementation-admission.org`](docs/ADRs/0013-work-package-scoped-implementation-admission.org)
* [`docs/work-packages/index.toml`](docs/work-packages/index.toml)
* [`PLAN.md`](PLAN.md)

## Current implementation sequence

The current high-level migration order is:

1. foundation resource, work, time, generation, and accounting contracts;
2. Core handler, security, codec, and lock-isolation contracts;
3. immutable logical and binding plans, capability indexes, and compiler
   migration;
4. client and server Binding SPI, routes, subscriptions, responses, and
   emissions;
5. Servient lifecycle, cleanup, application facades, and scheduling policy;
6. Discovery client cleanup;
7. Zenoh and zenoh-pico migration;
8. umbrella composition, obsolete API removal, and final convergence evidence.

The authoritative dependency graph and evidence keys are maintained in the
work-package index rather than in this README.

## Build and checks

Clone the repository and run the baseline workspace checks:

```sh
git clone https://github.com/yushun1990/clinkz-wot.git
cd clinkz-wot

cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

The repository also contains project-specific scripts and executable design
checks. Consult `PLAN.md`, the active work-package record, and `tools/` before
assuming that the generic Cargo commands constitute complete tranche or release
evidence.

Examples of additional checks may include:

```sh
scripts/check-baseline.sh
tools/check-design-requirements.sh
```

The exact required commands are owned by the applicable work package and
evidence record.

## Feature and platform posture

The target runtime supports:

* standard host environments;
* async host integration where admitted;
* `no_std + alloc`;
* caller-driven constrained progress;
* bounded static or application-provided storage profiles;
* Cargo-linked Protocol Binding crates.

Host and constrained implementations may use different scheduling and storage
strategies, but they must preserve the same protocol-neutral semantics and
ownership rules.

A successful host build is not sufficient evidence for the constrained target.
Affected tranches must provide the feature-cell, source, compile, resource, and
workload evidence required by their registered contracts.

## Protocol Bindings

A Protocol Binding is responsible for:

* recognizing and compiling supported protocol forms;
* protocol-specific addressing and representation conversion;
* transport session and connection handling;
* inbound and outbound protocol I/O;
* correlation and protocol-local flow control;
* converting protocol results into bounded protocol-neutral outputs;
* advancing binding-local operations when the engine grants progress.

A Protocol Binding is not responsible for:

* choosing a different TD form during the interaction hot path;
* directly invoking application handlers;
* owning application lifecycle;
* publishing serving state independently of Servient;
* hiding an unbounded worker or queue behind the SPI;
* redefining protocol-neutral cancellation, generation, cleanup, or resource
  semantics.

The detailed target contract is maintained in
[`docs/spec/binding-spi.md`](docs/spec/binding-spi.md).

## Development guidance

Before changing public APIs or runtime behavior:

1. identify the active work package or tranche;
2. read the architecture backbone and accepted ADRs affecting that scope;
3. locate the authoritative domain specification and machine-readable
   projections;
4. confirm that the tranche is admitted;
5. run its pre-implementation checks;
6. keep the change inside its declared paths and dependencies;
7. produce the registered completion evidence;
8. request an independent same-revision review.

For unresolved design questions, add a non-authoritative topic under
[`workspace/`](workspace/) and migrate the stable conclusion into its proper
owner after the discussion converges.

## Stability

No public API compatibility guarantee is currently provided.

Until the v4.9 migration and final convergence gates close, public types,
module ownership, feature combinations, crate boundaries, and integration
surfaces may change through admitted work packages.

Do not build a long-lived external integration against undocumented or legacy
runtime surfaces without pinning an exact commit and accepting migration work.

## License

See the repository license files for licensing terms.
