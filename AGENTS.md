# Agent Guidance

This repository implements `clinkz-wot`, a Rust Web of Things engine for the Clinkz platform.

## Language Policy

- All technical specifications, Rust doc comments, inline comments, public API documentation, examples, and error messages must be written in English.
- Product discussions may happen in other languages, but committed technical artifacts should stay English-only.

## Session Preparation

- Before each task session, read the relevant documentation under `docs/`.
- If `PLAN.md` already instructs which `docs/` documents to read, follow `PLAN.md` and skip the separate default `docs/` reading step.

## Architecture Boundaries

- Keep the engine protocol-neutral.
- Do not add zenoh-specific logic to TD, TM, or core runtime crates.
- Treat zenoh as the first optional protocol binding, not as a required engine dependency.
- Keep W3C WoT vocabulary separate from Clinkz extensions.
- Use a Clinkz JSON-LD namespace, such as `cz:`, for Clinkz-specific binding, storage, compute, or platform metadata.

## no_std Policy

- TD, TM, and core runtime abstractions must support `no_std + alloc`.
- Avoid filesystem, sockets, threads, async runtimes, process APIs, and OS-only APIs in `no_std` crates.
- Put host/cloud runtime functionality behind `std` features or in separate `std` crates.
- Embedded support means TD/TM construction, serialization, validation, and local Thing dispatch with abstract transport adapters.

## W3C Compatibility

- Use W3C WoT TD 1.1 as the default compliance target.
- Keep TD 2.0 work behind an experimental feature such as `td2-preview`.
- Preserve unknown extension fields during deserialization and serialization.
- Preserve round-trip fidelity for TD/TM documents unless a validation mode explicitly rejects them.
- Support `base` plus relative form `href` values; binding implementations should resolve form targets through a shared helper instead of duplicating resolution logic.

## Implementation Style

- Prefer the existing crate and module patterns before adding new abstractions.
- When developing a new crate or module, treat gaps discovered in dependency
  crates as design feedback, not only as local implementation blockers. Early
  versions of dependency crates are expected to be incomplete; if the new work
  reveals a missing API, missing trait implementation, weak data model, or
  design error in a dependency crate, evaluate whether the missing capability
  belongs to that dependency crate's responsibility. If it does, fix the
  dependency crate first and cover the change with focused tests before relying
  on it from the new crate. If the capability would violate the dependency
  crate's responsibility boundary, do not force it into the dependency; keep the
  behavior in the appropriate crate and record the dependency gap or follow-up
  explicitly.
- Keep TD/TM crates focused on data models, builders, serialization, deserialization, and validation.
- Put protocol behavior in binding crates.
- Put Discovery and Servient/runtime behavior in dedicated crates.
- Avoid concentrating unrelated implementation logic in one large source file.
  As a crate grows beyond a small implementation, split code into cohesive
  modules by responsibility and keep crate roots such as `lib.rs` focused on
  module declarations, crate-level documentation, and intentional public
  re-exports.
- Preserve the crate-root public API when reorganizing internal modules unless
  an API change is explicitly part of the task.
- Follow idiomatic Rust module organization and API style. Keep visibility as
  narrow as practical, prefer clear module boundaries over catch-all utility
  modules, colocate implementation details with the types or traits they
  support, and expose stable public items through deliberate `pub use`
  surfaces.
- Do not use `mod.rs` files for Rust modules. Use module-name-matching files
  and directories instead, such as `foo.rs` or `foo/bar.rs`, following the
  modern Rust module layout.
- Separate integration-test scenarios from bulky test support code. Put fake
  handlers, fake bindings, fake transports, reusable fixtures, and helper
  registries in test support modules when they start to obscure the behavior
  under test.
- Do not split cohesive data model files solely because they are long. Prefer a
  module split when it clarifies ownership boundaries, reduces unrelated
  imports, or makes future changes safer without creating needless re-export
  churn.
- Follow the Rust API Guidelines checklist when designing public or
  semi-public APIs:
  - Naming: use standard Rust casing; use `as_`, `to_`, and `into_`
    consistently for ad-hoc conversions; follow Rust getter naming
    conventions; name iterator-producing collection methods `iter`,
    `iter_mut`, and `into_iter`; name iterator types after the methods that
    produce them; use meaningful feature names without placeholders; keep word
    order consistent across related names.
  - Interoperability: eagerly implement common traits when semantics allow,
    including `Copy`, `Clone`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Hash`,
    `Debug`, `Display`, and `Default`; use standard conversion traits such as
    `From`, `AsRef`, and `AsMut`; implement `FromIterator` and `Extend` for
    collection-like types; implement Serde `Serialize` and `Deserialize` for
    data structures when appropriate for the crate boundary; keep types `Send`
    and `Sync` where possible; make error types meaningful and well-behaved;
    provide `Hex`, `Octal`, and `Binary` formatting for binary number types;
    take generic readers and writers by value as `R: Read` and `W: Write`.
  - Macros: make macro input syntax resemble the generated output; allow macros
    to compose with attributes; ensure item macros work wherever items are
    allowed; support visibility specifiers in item macros; accept flexible type
    fragments.
  - Documentation: provide thorough crate-level docs with examples; give public
    items useful rustdoc examples when practical; write examples with `?`
    instead of `unwrap` or `try!`; document error, panic, and safety behavior;
    hyperlink relevant concepts and types; keep `Cargo.toml` metadata complete;
    document significant release changes; hide unhelpful implementation details
    from rustdoc.
  - Predictability: avoid inherent methods on smart pointer types; place
    conversions on the most specific involved type; make functions with a clear
    receiver into methods; avoid out-parameters; keep operator overloads
    unsurprising; implement `Deref` and `DerefMut` only for smart pointers;
    make constructors static inherent methods.
  - Flexibility: expose useful intermediate results to avoid duplicate work;
    let callers control allocation, copying, and placement where reasonable;
    use generics to avoid needless assumptions about parameter types; keep
    traits object-safe when trait objects are a plausible use case.
  - Generic bounds: avoid putting trait bounds on generic struct or enum
    definitions unless the bounds are required for field well-formedness or for
    an explicit type-level invariant. Put bounds on the `impl` blocks,
    methods, functions, or trait implementations that actually need the
    constrained behavior, and split `impl` blocks by their required bounds when
    that keeps the API more flexible.
  - Type safety: use newtypes to represent static distinctions; prefer
    meaningful custom types over ambiguous `bool` or `Option` arguments; use
    `bitflags` for sets of flags instead of enums; use builders for complex
    value construction.
  - Dependability: validate function arguments at API boundaries; never rely on
    failing destructors; provide explicit alternatives for destructor behavior
    that may block.
  - Debuggability: implement `Debug` for public types; ensure `Debug`
    representations are informative and never empty.
  - Future proofing: seal traits when downstream implementations would prevent
    future evolution; keep struct fields private unless direct field access is
    intentionally part of the stable API; use newtypes to encapsulate
    implementation details; avoid duplicating derived trait bounds on data
    structures.
  - Necessities: keep public dependencies of stable crates stable; use
    permissively licensed crates and dependencies.
- Follow the Rust Style Guide and `rustfmt` default style unless the repository
  explicitly configures otherwise: use spaces with 4-space indentation, keep
  lines within the default 100-column style, prefer block indentation, use
  trailing commas in multiline lists, avoid trailing whitespace, version-sort
  ordered items where Rust style expects sorting, keep `use` and `mod`
  declarations before other items, and format `Cargo.toml` according to the
  Rust style conventions.
- Follow Rust naming conventions consistently: types and enum variants use
  `UpperCamelCase`; fields, functions, methods, variables, modules, and macros
  use `snake_case`; constants and immutable statics use
  `SCREAMING_SNAKE_CASE`; use raw identifiers or a trailing underscore for
  reserved words instead of misspellings.
- Prefer line comments and line doc comments. Use `///` for item docs and `//!`
  for module or crate docs; put doc comments before attributes; keep comments
  concise, normally as complete English sentences.
- Use Rust's expression-oriented style where it improves clarity, without
  forcing expressions when statements are easier to read.
- Avoid `#[path]` module annotations unless there is a strong reason.
- Run Clippy where practical. Treat default Clippy groups (`correctness`,
  `suspicious`, `style`, `complexity`, and `perf`) as actionable feedback; use
  `pedantic`, `nursery`, and `cargo` lints selectively; never enable
  `restriction` as a whole, and only opt into individual restriction lints with
  a clear project-specific reason.
- Use feature flags to separate embedded, experimental, and host/runtime capabilities.

## Testing Expectations

- Add `no_std + alloc` compile checks for crates that claim embedded support.
- Keep round-trip fixture tests for TD/TM compatibility.
- Test protocol bindings separately from protocol-neutral core logic.
- Add fixtures with multiple forms per affordance to verify protocol-neutral selection behavior.
