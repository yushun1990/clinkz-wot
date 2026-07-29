# 0021 Host and Constrained Authoring Constructibility

Status: OPEN

Kind: owner-raised API-and-portability investigation

Priority: HIGH

Target: the accepted host-erased and `no_std + alloc` static compiler, cursor, artifact, and registration authoring contracts

## Scope and authority

This topic records a Project Owner concern about whether the accepted host and constrained representations are not only type-correct in repository fixtures but realistically constructible, understandable, and maintainable by independent Protocol Binding and application authors.

The concern is an investigation input. It does not reject the accepted D8 representation, does not select another representation, and does not prescribe API simplification or performance trade-offs. Codex owns the technical judgment from repository evidence and implementation feedback.

## Repository observations

The repository records that:

- `BindingCompilerExtension` owns associated cursor and artifact types;
- host applications use Core-owned safe erasure;
- constrained applications compose heterogeneous third-party compilers through application-closed static enums;
- cursor ownership must survive pending and failure paths;
- artifact envelopes carry identity and measured-footprint constraints;
- outputs must own their data without retaining TD or registration-snapshot lifetimes;
- paired host and constrained third-party authoring fixtures are part of the accepted contract evidence;
- product Rust implementation of the planned boundary is not yet present.

## Owner concern

The Project Owner is concerned that a representation can satisfy compile-schema tests while still imposing excessive type coupling, boilerplate, central enum editing, ownership complexity, allocation cost, or Core-internal knowledge on real third-party authors. The concern also includes whether host and constrained forms remain semantically equivalent when implemented rather than only described.

## Questions for investigation

1. What exact public types and operations must an independent Protocol Binding author implement in host and constrained profiles?
2. Which Core concepts must the author understand to compile one form into one artifact?
3. How much application-owned enum, cursor, artifact, dispatch, and error wiring is required when adding another compiler in a constrained build?
4. Can a third-party compiler be authored without access to in-repository private types or undocumented invariants?
5. Do host erasure mismatches return all owned values needed for deterministic recovery without hidden allocation or type loss?
6. Do pending and failure transitions preserve cursor ownership without forcing awkward or ambiguous error APIs?
7. Is artifact footprint measurement meaningful and consistently enforceable across host and constrained profiles?
8. Can host and static registrations produce semantically equivalent plans and artifact identities for the same TD form?
9. Are the paired authoring fixtures representative of an external crate and realistic application composition, rather than privileged repository adapters?
10. What public API friction is revealed by implementing the first real Zenoh and zenoh-pico compilers?
11. What evidence would prove that the accepted representation is realistically constructible and portable?
12. If the concern is supported, which accepted contract or authoritative owner must be reconsidered?

## Constraints

- Do not treat verbosity or generic complexity alone as proof of an API defect.
- Do not assume that host and constrained profiles must use identical implementation techniques.
- Do not weaken ownership, bounded progress, measured resources, safe mismatch recovery, TD-lifetime independence, or protocol neutrality.
- Do not prescribe trait erasure, enum generation, macros, allocation policy, or alternative API shapes before investigation.
- Do not reopen D8 solely because product implementation has not started.
- Preserve the AI-led model.

## Expected decision output

Codex should determine:

1. whether the accepted host and constrained contracts are independently constructible in realistic external authoring scenarios;
2. whether fixtures and implementation evidence expose any public API, ownership, resource, or portability defect;
3. whether host and constrained semantics remain equivalent where required;
4. whether any authoritative contract requires correction or reaffirmation;
5. the conditions for moving this topic through its workspace lifecycle.
