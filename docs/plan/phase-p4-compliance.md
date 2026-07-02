# Phase P4 — Compliance and Verification

> Baseline: `docs/baseline/engine-architecture-baseline.md` (v4.0) §0, §8, §9, §10.

## Goal

Lock the compliance surface and the verification baseline. Finalize
documentation alignment. P4 runs continuously but finalizes after P3.

## Entry Criteria

- P0–P3 are complete and the workspace compiles whole.

## Current State (being aligned)

- `scripts/check-no-std.sh` (15 lines): lists 7 crates + 2 async-flavor checks.
  Uses `clinkz-wot-codec-cbor` (note: crate name has `codec` singular).
- `scripts/check-m7.sh`: aggregate M7 entry point.
- `scripts/check-reserved-features.sh`: zenoh-pico feature compilation, fake
  platform tests, mutually-exclusive runtime backend diagnostics.
- `scripts/check-td2-preview.sh`: TD 2.0 feature gate.
- `docs/technical-spec.md`, `docs/wot-compliance.md`, `docs/no-std-embedded.md`,
  `docs/verification.md`: cross-cutting docs still describing the v3.x surface.

## Work Breakdown

### Step 4.1 — Scripting API conformance map tests

One focused test per row of v4.0 §10, in a new `servient/tests/scripting_api_conformance.rs`
(or split across crate test suites where the surface lives):

- **WoT facade:** `produce`/`consume`/`discover`/`fetch_td`.
- **Producer:** every `set_*_handler` (read/write/observe/unobserve/action
  invoke/query/cancel/event subscribe/unsubscribe); server-side
  `read_property`/`write_property`/`emit_event`/`emit_property_change`;
  `expose`/`destroy`.
- **Consumer:** `read_property`/`write_property`/`invoke_action`/
  `observe_property`/`unobserve_property`/`subscribe_event`/
  `unsubscribe_event`; the six bulk methods.
- **Discovery:** `ThingDiscoveryProcess` lazy session (`next`/`stop`/`error`),
  no `remaining()`.

Each test uses a fake binding (in a test-support module) so it runs in the
default workspace path without zenoh.

### Step 4.2 — Documented-deviation tests

Verify the v4.0 §9 deviations behave as declared:

- **Pull-queue subscription:** `subscribe_event` returns a `Subscription`;
  pushed samples are drained by `poll_next` (sync) and `Stream::next` (async,
  `async` feature); drop-oldest + overflow counter under saturation.
- **`Result` error model:** every interaction returns `Result<_, CoreError>`/
  `ServientError`; no panics on protocol failure.
- **`Discoverer` is a trait object:** confirm `Servient` holds
  `Arc<dyn Discoverer>` and a remote-capable implementation can be injected
  (fake in tests).

### Step 4.3 — Feature-matrix verification

Audit defect AD5: a "focused matrix" lets feature-interaction defects surface
late at edge combinations. So **build-check covers ALL valid feature
combinations per crate; tests cover a representative subset**:

- **Build-check (`cargo check`) — full combination matrix.** For each crate,
  enumerate every valid combination of `std`/`async`/`zenoh`/`zenoh-pico`/
  `td2-preview` (minus the mutually-exclusive `zenoh` ∩ `zenoh-pico` pair) and
  `cargo check` it. ~28 combinations across the workspace — catches every
  compile-time feature-interaction defect. Cheap enough for CI.
- **Test (`cargo test`) — representative subset:** default (std);
  `--no-default-features` (no_std + alloc); `--features async`; `--features
  zenoh`; `--features zenoh-pico` (fake platform); `--features td2-preview`.
- Mutually-exclusive `zenoh` ∩ `zenoh-pico` produces a clear diagnostic
  (update `check-reserved-features.sh`).
- `docs/verification.md` records **which combinations are build-checked vs
  test-covered**, leaving no blind combination.

### Step 4.4 — `scripts/check-no-std.sh` update

`check-no-std.sh` is a **compile-check** (`cargo check --no-default-features`),
not a runtime test — state this in the script header and in `verification.md`.
It asserts the crate roots compile `no_std + alloc`; it does NOT exercise the
no_std driving path at runtime (deferred with zenoh-pico). Update for v4.0:

- Replace any `--features multithread` lines (removed in P0) — the lock is now
  always thread-safe; no `multithread` feature.
- Confirm coverage: `td`, `core`, `protocol-bindings`,
  `protocol-bindings-zenoh`, `discovery`, `servient`, `codec-cbor`, plus the
  `async` no-std flavor for `core` and `servient`.
- Add `discovery` no-std check for the new crate-root surface (session/reader/
  publisher/discoverer), not just the old local module.

### Step 4.5 — TD 1.1 fixture coverage

Retain and extend fixture tests:

- Round-trip fidelity (unknown fields, JSON-LD contexts, `OneOrNormal` compact
  forms).
- Multi-form affordance selection (content type, subprotocol, operation).
- `base` + relative `href` resolution through the shared binding helper.
- Clinkz `cz:`/`cz-zenoh:` extension terms preserved and parsed.
- Full TD 1.1 `op` vocabulary always available in default builds
  (`cancelaction`/`subscribeallevents`/`unsubscribeallevents`).

### Step 4.6 — Clippy and formatting

- `cargo clippy --workspace --all-targets` clean (default groups:
  correctness/suspicious/style/complexity/perf).
- `cargo fmt --check` clean.

### Step 4.7 — Aggregate script

Rename/rewrite `scripts/check-m7.sh` → `scripts/check-baseline.sh` (or keep
`check-m7.sh` as an alias) as the v4.0 workspace baseline entry point:

```text
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets
scripts/check-no-std.sh
scripts/check-reserved-features.sh
scripts/check-td2-preview.sh
```

### Step 4.8 — Documentation alignment

Update cross-cutting docs to v4.0:

- `docs/technical-spec.md`: crate layout, feature policy (`WotLock`, removed
  `multithread`), validation levels, serialization policy, error policy.
- `docs/wot-compliance.md`: **reverse** the §Scripting API Boundary positioning
  to "Scripting API is a conformance target" (v4.0 §0); keep the
  subscription-deviation note as v4.0 §9.1; update the TD 2.0 gate note.
- `docs/no-std-embedded.md`: remove the `multithread` Layer-1 section (the lock
  is always thread-safe now); keep zenoh-pico (Layer 2) and embassy (Layer 3)
  boundaries; update supported capabilities for the async driving primitive.
- `docs/verification.md`: record the v4.0 regular verification path.
- `PLAN.md`: mark P0–P4 status as complete; archive the deprecated-docs index.

### Step 4.9 — Acceptance criteria sign-off

Walk the `PLAN.md` §Acceptance list and v4.0 §10 conformance map; confirm each
row is met by a test or documented deviation. Record any remaining gap as a
follow-up in `docs/deferred-design-followups.md` (pruned of items now
delivered).

## Deliverables

- A green workspace baseline (Step 4.7 script).
- Scripting API conformance map + deviation tests.
- All cross-cutting docs aligned with v4.0.

## Exit Criteria

- v4.0 acceptance criteria (`PLAN.md` §Acceptance) met.
- Scripting API conformance map (v4.0 §10) fully covered.
- The only documented Scripting API deviations are v4.0 §9.
- No SUPERSEDED doc is referenced as authoritative anywhere in the repo (grep
  for citations).
- `docs/deferred-design-followups.md` pruned of delivered items (#2 handler
  consolidation, #3 trait removal, #4 apply_security, #5 data_type split, #6/#7
  where done).

## Risks

- Feature-matrix scope split: **build-check (`cargo check`) covers ALL valid
  feature combinations per crate (~28); tests (`cargo test`) cover a
  representative subset** (default, no-default-features, async, zenoh,
  zenoh-pico, td2-preview). Do not run the full combination matrix under
  `cargo test` — record the split in `docs/verification.md` so the build-checked-vs-test-covered boundary is explicit.
- Reversing the Scripting API positioning in `wot-compliance.md` touches a
  load-bearing doc; ensure every backreference (PLAN, technical-spec, baseline)
  is consistent after the edit.
