# 0010 Complete the `docs/design.md` Decomposition

Status: OPEN
Kind: documentation-architecture improvement
Target revision: v4.9 normative-authority convergence

## Scope and authority

This topic discusses how to complete the ongoing decomposition of
`docs/design.md`.

It does not change architecture, public APIs, lifecycle semantics, implementation
admission, or existing normative authority.

Any stable conclusion must be migrated into the appropriate authoritative
artifact under `docs/`, `AGENTS.md`, or the project planning and governance
artifacts.

## Context

The project has already begun decomposing the former monolithic design document.

The current authority structure is approximately:

```text
docs/design.md
    active revision selector
    normative-source manifest
    residual owner for unmigrated requirements

docs/architecture/
    cross-module invariants and primary flows

docs/spec/
    single-owner detailed behavioral and API contracts
```

Planning and Protocol Binding behavior have already moved into dedicated domain
specifications.

Additional domain specifications are planned, including foundation, documents,
interaction core, subscriptions and emissions, Servient behavior, discovery,
and profiles and verification.

Therefore, the question is no longer whether `docs/design.md` should be split.

The question is how to finish the decomposition safely and clearly.

## Problem

`docs/design.md` currently has two roles:

1. a durable manifest and revision entry point;
2. a temporary residual owner for requirements that have not yet moved into a
   dedicated specification.

The second role is transitional, but there is no concise, visible completion
plan for retiring it.

This creates several risks:

* readers cannot easily tell which requirements still belong to
  `docs/design.md`;
* the same behavior may accidentally appear in both `docs/design.md` and a
  domain specification;
* mechanical movement of sections may be mistaken for a valid authority
  migration;
* planned specification files may be created before their contracts are
  complete;
* `docs/design.md` may remain indefinitely large despite the modular hierarchy.

## Proposal

Codex should review the current residual contents of `docs/design.md` and define
a concrete completion strategy for the decomposition.

The resulting strategy should answer:

1. What is the final long-term responsibility of `docs/design.md`?

2. Which remaining requirement families should move into each planned domain
   specification?

3. In what order should those domains be migrated?

4. What conditions must be satisfied before a requirement is removed from
   `docs/design.md`?

5. How should the project prove that a migration did not create duplicate,
   missing, or contradictory normative owners?

6. Which existing registry or checker should expose the remaining residual
   ownership?

7. Should any planned domain specifications be merged, renamed, reordered, or
   removed before further migration?

## Suggested direction

The expected end state is:

```text
docs/design.md
    active revision
    normative-language rules
    authority and change-control rules
    normative-source manifest
    concise revision record

docs/architecture/
    cross-domain architecture and invariants

docs/spec/
    complete single-owner domain contracts
```

`docs/design.md` should eventually stop owning detailed domain behavior.

Migration should be based on requirement ownership rather than current Markdown
headings or source-code layout.

A domain specification should not become active merely because an empty or
partial file has been created.

## Expected output

Codex should produce:

* a proposed final responsibility for `docs/design.md`;
* a reviewed domain ownership map;
* a migration order;
* completion criteria for each migrated domain;
* any necessary changes to documentation indexes, registries, checkers, ADRs,
  work packages, or project state.

This workspace topic should move to `DECIDED` after that direction converges.

It should move to `MIGRATED` only after the agreed policy and ownership changes
have been reflected in their authoritative repository artifacts.

## Open question

Should this work be handled as one documentation-convergence effort, or as
several independently reviewed domain migrations?
