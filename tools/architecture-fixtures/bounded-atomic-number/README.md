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

## STM32F407G-DISC1 runner

`board-runner/` is the bare-metal measurement implementation for the declared
workload. It is a non-production workspace member and depends on this probe's
unchanged `project` entry and corpus. It configures the STM32F407VGT6 from the
board's 8 MHz HSE to 168 MHz, enables five flash wait states plus prefetch and
the instruction/data caches, and records the resulting RCC/FLASH registers.
Code executes from flash; allocation state and the measured stacks are in the
128 KiB SRAM; the runner's ordinary stack is in CCM.

The uninstrumented parser-cost sequence masks interrupts and records a
cold-first sample after resetting both flash caches, 100 warmups, every one of
1,000 measured DWT samples, and an extra complementary stack-paint invocation.
The measured projection runs on an aligned PSP stack with 8 KiB of paint and a
256-byte no-access MPU guard. Patterns `0xa55a6996` and `0x5aa59669` are applied
before separate invocations. Allocation calls, projection identity, baseline
PSP, both watermarks, the guard, and every cycle sample are recorded.

The declared application IRQ load is one TIM2 update interrupt per projection.
TIM2 runs at 84 MHz with `ARR=32`; its ISR asserts cancellation and timestamps
entry/exit. The loaded sequence repeats 100 warmups and 1,000 samples, records
the assertion delay and post-assertion return latency for every sample, and
uses a separate painted/guarded MSP stack so interrupt depth is not hidden in
the parser watermark. This synthetic load is only the declaration required by
this probe. It does not establish a particular application's schedulability.

The target streams newline-delimited raw JSON through semihosting. There are
12,844 corpus cases and two 1,000-sample sequences per case, so expect a long
run and a large raw file. Do not interrupt a measurement run; a stream without
the exact terminal record is invalid.

### Prepare, cover, and measure

Prerequisites are the installed `thumbv7em-none-eabihf` target, `probe-rs`,
Python 3, and an ARM-capable disassembler. The runner prefers
`llvm-tools-preview`, then `arm-none-eabi-objdump`, and finally GDB:

```sh
rustup component add llvm-tools-preview # recommended when available
```

Run from the repository root. Use a new or empty artifact directory outside
the source tree, and replace `MB997-REVISION` with the exact revision printed
on the board:

```sh
runner=tools/architecture-fixtures/bounded-atomic-number/board-runner/run-board.sh
artifacts=/tmp/WP100-ATOMIC-NUMBER-M4-v1

"$runner" prepare "$artifacts" MB997-REVISION
```

The same measured ELF has a USER-button-selected coverage mode. Hold the blue
USER button before running the next command and keep it held until profiling
has started. The runner repeatedly projects the 256-byte exact-halfway case so
the profiler can show `parse_long_mantissa` or a `dec2flt/slow.rs` frame/line:

```sh
"$runner" coverage "$artifacts"
```

If the fallback is not sampled in the default ten seconds, repeat coverage as
`WP100_COVERAGE_SECONDS=30 "$runner" coverage "$artifacts"`.

Release the USER button, then run the complete measurement. If more than one
debug probe is attached, prefix these commands with the selector reported by
`probe-rs list`, for example `WP100_PROBE=0483:374b:SERIAL`.

```sh
"$runner" measure "$artifacts"
"$runner" bundle "$artifacts"
```

`measure` returns status 2 when complete evidence falsifies a bound or coverage
condition, but preserves all output. Run `bundle` in either case. The bundle
contains the raw JSONL, validation summary, same-ELF slow-path profile, ELF,
linker map, ARM disassembly, DWARF frame dump, resolved features,
toolchain/probe versions, board revision, Git head/status, commands, digests,
and probe logs. Return the `.tar.gz` and adjacent `.sha256` for independent
review. Watermarks are not accepted alone: the ELF/map/disassembly/frame data
must be reviewed for reserved but untouched frames before treating the maximum
as conservative.

Neither a locally valid bundle nor a passing measurement admits WP-100. It is
raw candidate evidence for independent review of the existing pre-readmission
set.
