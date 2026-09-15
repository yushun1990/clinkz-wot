# Bounded atomic Number boundary and constrained workload

Non-production probe and declared workload for the
[Number amendment](../../../docs/amendments/WP-100-bounded-atomic-number-v1.md).
It supplies a narrow lexical-boundary model and a reusable corpus/measurement
entry. It does not implement `ValidatedThing`, a full JSON lexer, or the eight
pre-readmission evidence items. Host tests and thumb compilation are not
constrained runtime measurements.

## Reproduce the portable evidence

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/bounded-atomic-number/Cargo.toml
cargo build --release --locked --target thumbv7em-none-eabihf --manifest-path tools/architecture-fixtures/bounded-atomic-number/Cargo.toml
cargo fmt --manifest-path tools/architecture-fixtures/bounded-atomic-number/Cargo.toml -- --check
```

The standalone lockfile resolves public serde_json AP access. The probe is
`no_std + alloc`; no production TD feature or implementation is changed.
Tests cover every configured limit 0..256, `L - 1`/`L`/`L + 1` for nonempty
Numbers, first-excess-byte rejection without consuming an unbounded suffix,
and typed AP length checking before projection. They also exercise opaque
overflow preservation and resource-failure precedence over predicate failure.
The byte-observation model receives Number bytes from a future lexer; it is
not evidence of that lexer's syntax, budget, or cancellation implementation.

## Constrained acceptance workload

Workload identity: `WP100-ATOMIC-NUMBER-M4-v1`. This is a pre-readmission
architecture probe, not a completed PERF-CS workload or a product benchmark.
The amendment owns its mandatory cycle/stack acceptance limits. This section
owns the corpus and measurement procedure used to falsify that candidate.

Target: the STM32F407G-DISC1, Cortex-M4F r0p1, bare metal, fixed 168 MHz,
128 KiB SRAM plus 64 KiB CCM, flash with five wait states, and DWT CYCCNT
declared in `docs/performance/constrained.toml`. Record instruction/data
prefetch settings, flash/SRAM code placement, clock verification, allocator,
board revision, rustc `-Vv`, resolved Cargo features, lockfile digest, ELF
digest, and linker map. Use the release options from this fixture's manifest
in the board runner as well; a dependent crate's profile is not inherited.
Other supported compiler/dependency graphs must satisfy the same bounds.

### Cases and path coverage

Use `for_each_case` without pruning short or long inputs. It spans every
length 1..256, including 63/64/65 and 255/256, both signs, long significands,
long exponents with leading zeros, overflow/underflow, near-subnormal and
normal boundaries, the finite maximum, and exact halfway values with decimal
neighbors. Test 256 on this constrained target even though the named static
profile is 64: application-defined Consumer profiles can select 256.

The exact halfway spelling around 1 and its padded neighbors attack the
ambiguous truncated-significand decision. In the inspected rustc 1.95.0 source,
`core/src/num/dec2flt/mod.rs` can select `parse_long_mantissa`; its
`decimal_seq.rs` has a 768-digit buffer. A 256-byte source bound therefore
cannot be used as a stack bound. The M4F has no hardware binary64 arithmetic;
the linked software helpers are inside the measurement boundary.

Before recording uninstrumented measurements, collect a separate debugger or
source-coverage trace showing at least one case reaches the slow fallback in
the selected binary. Use the exact halfway family first. If a compiler changes
the algorithm, review its replacement paths and add adversarial cases that
exercise the most expensive path; a missing fallback hit is not silently
treated as success. Keep the trace and case identifier with the results.
These are implementation observations for workload coverage, not copied
parser semantics or a private API dependency in production.

### Measurement boundary

1. Iterate the corpus outside timing, construct each `serde_json::Number`
   through `serde_json::from_str`, verify `as_str()` preserves the input, and
   create `LexemeLimit::new(256)`. Admit/precharge the known input length before
   timing. No Number parsing, allocation, corpus generation, or output logging
   belongs inside `project`.
2. At the pre-projection cancellation checkpoint, read DWT CYCCNT; call the
   non-inlined `project` once; read CYCCNT immediately after its return at the
   post-projection checkpoint. Consume the returned result using `black_box`.
   Include the call/return, length guard, public projection, finite filter,
   timing overhead, and all software helpers. Do not subtract a baseline or
   cache the float. Reject unexpected allocation in this interval.
3. For each case collect 100 warmups, 1,000 measured samples, and a cold-first
   sample after resetting/invalidating relevant instruction/prefetch state.
   Record every sample and the maximum, not just percentiles. Use wrapping
   CYCCNT subtraction; bound the interval so multiple wraps are impossible.
4. Isolate the projection on a dedicated, aligned downward-growing stack with
   at least 8 KiB of painted unused space plus a protected guard. Record the
   baseline SP immediately before entry. Use two complementary stack paint
   patterns on separate runs, inspect the high-water mark after return, and
   include all callees. Check the linker map/disassembly for SP adjustments
   and untouched reserved frames: watermark writes alone can undercount
   reserved stack. Report the conservative maximum stack depth from both
   methods; guard damage or uncertain depth fails the workload.
5. Isolate unrelated interrupts for the parser-cost run and report that fact;
   do not claim application wall-clock cancellation latency from an IRQ-masked
   microbenchmark. Repeat with the declared application IRQ load and measure
   cancellation asserted immediately after entry, observing it at return.
   Record interrupt stack separately. External interrupt service time is an
   additional application schedulability obligation, not hidden parser work.

### Pass, fail, and interpretation

Every sample, including cold-first and slow-fallback cases, must meet the
amendment's 168,000-cycle / 4,096-byte additional-stack bounds. No outlier
removal, average-only success, inferred cycles from a Host, or compile-only
substitution is allowed. Report per case: family, exact lexeme, length,
projection bits or rejection, cycles, stack high-water/conservative depth,
allocation count, and build/runner fingerprints. Preserve raw results outside
the source tree until an evidence artifact records the exact reviewed build.

The 1 ms parser interval is an explicit engineering acceptance policy: it
leaves a cancellation checkpoint after each numeric projection instead of
allowing several projections to consume a longer indivisible step. The stack
allowance reserves at most 1/32 of the board's 128 KiB SRAM for this operation's
additional call depth; caller frames, interrupt frames, heap, and other live
state still require the complete item-7 resource proof. These budgets are not
derived from `n` work units. Charging `n` controls scheduling/accounting;
target measurements establish whether that charge can safely represent this
atomic operation.

Passing the declared workload supplies empirical evidence for retaining 256,
not an exhaustive mathematical WCET proof. Independent review must judge the
parser-path coverage and complete target resource evidence. If any bound fails,
select and remeasure a smaller hard ceiling or reopen the projection design;
do not relax the budget or call 256 justified merely because it is finite.
The migration declares this workload; target results and all eight accepted
pre-readmission items remain required before a separate admission transition.
