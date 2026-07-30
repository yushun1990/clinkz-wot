# 0036 Host and Constrained Semantic Parity

Status: OPEN

Kind: owner-raised portability and duplicate-semantics investigation

Priority: HIGH

Target: equivalent Binding and Servient lifecycle semantics across erased `std` execution and caller-owned `no_std + alloc` manual progress

## Scope and authority

This topic records a Project Owner concern that host and constrained profiles deliberately use different representations and may drift into two subtly different implementations of one semantic contract. It does not prescribe one shared runtime representation. Codex owns the portability decision.

## Repository observations

- Host execution may use trait objects, boxes, owned guards, reactors, and wake integration.
- Constrained execution uses associated state, caller-owned typed slots, static tables, manual polling, and linear `WorkBudget`.
- The design requires identical ownership, terminal retention, cancellation, cleanup, generation, and response semantics.
- Existing fixtures prove public constructibility, while the Property Read gate gives only limited runtime coverage.
- The async-no-std cell is currently compile-only for the narrow gate.

## Questions for investigation

1. Which lifecycle logic can be implemented once independently of storage representation?
2. Which behavior must be tested as a cross-profile equivalence oracle rather than in separate tests?
3. How are slot initialization, terminal acknowledgement, clear, and late-result behavior compared with host guard ownership?
4. Can host convenience adapters accidentally weaken zero-budget, cancellation, or cleanup semantics?
5. Can constrained generic and enum composition create unacceptable code-size or usability costs?
6. When is compile-only evidence insufficient for the async-no-std surface?
7. What differences are legitimate representation choices and what differences constitute semantic drift?

## Constraints

- Do not impose `Arc`, atomics, threads, or an executor on constrained profiles.
- Do not make the host path the normative behavior merely because it is easier to run.
- Preserve one semantic state and outcome model.
- Use shared deterministic traces where possible rather than duplicated prose assertions.

## Expected decision output

Codex should identify shared semantic machinery, cross-profile trace/equivalence tests, runtime evidence required beyond compilation, acceptable representation-specific differences, and any work-package changes needed to prevent drift.