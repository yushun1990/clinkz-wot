# Subscription Receiver Ownership and Clone Semantics Question

Status: MIGRATED

## Context

The subscription flow defines:

> The application facade owns one receive cursor; cloning does not
> create competing consumers.

This establishes an important invariant:

-   one subscription execution has one receive cursor;
-   cloning must not accidentally create competing consumers.

However, the current wording leaves ambiguity around the API ownership
model.

## Question

What is the intended semantic meaning of `Clone` in the subscription
API?

The current statement does not clarify:

-   Which object is cloneable?
-   Does cloning duplicate a subscription consumer?
-   Does cloning only duplicate a control handle?
-   Who owns the receive cursor?
-   Can multiple tasks consume from cloned values?

## Ambiguity Example

Consider:

``` rust
let a = subscription.clone();
let b = subscription.clone();

a.next().await;
b.next().await;
```

The current wording does not clearly define whether this means:

1.  two competing consumers sharing one cursor;
2.  two independent consumers receiving duplicated events;
3.  invalid usage that should not be supported.

## Design Scope Question

Should the subscription abstraction distinguish between:

-   lifecycle/control ownership;
-   event consumption ownership.

Possible API shapes include separate concepts such as:

-   a cloneable control handle;
-   a uniquely owned receiver/cursor.

However, the exact API model and decision should be determined through
architecture review.

## Requested Decision

Clarify:

1.  The ownership model of subscription state.
2.  The ownership model of the receive cursor.
3.  The semantic meaning of `Clone`.
4.  Whether the subscription model supports:
    -   single consumer;
    -   multiple independent consumers;
    -   competing consumers;
    -   broadcast semantics.

## Related Documents

-   `docs/architecture/10-primary-data-flows.md`
-   Subscription flow section

## Decision

The v1 subscription abstraction has one linear application receive capability,
not a cloneable receiver/control pair.

Ownership is split as follows:

-   Core owns protocol-neutral identity, items, lifecycle values, limits,
    terminal/loss semantics, portable slots, and the binding driver SPI.
-   One binding driver owns one protocol resource, one receive cursor,
    binding-local flow control, and binding cleanup state.
-   Servient's private `SubscriptionRecord`, keyed by `SubscriptionId`, owns the
    installed driver lifecycle and coordinates explicit stop, remote terminal,
    facade drop, consumed-handle drop, and drain.
-   The host `Subscription` facade owns exclusive application access to that
    one registry cursor. `StaticSubscription` owns the unique
    generation-bearing caller slot identity used with `StaticServient`.

`Subscription`, `StaticSubscription`, and the owned host driver are non-`Clone`.
There is no separately cloneable public receiver or per-subscription control
handle in v1. Passive metadata or status values may implement `Clone` or `Copy`
under their own contracts, but cloning them conveys no receive, stop, or cleanup
authority. Internal shared registry state likewise does not create a second
public cursor.

The supported consumption models are:

-   one subscription execution has one receive cursor;
-   separately admitted subscriptions with distinct `SubscriptionId`s are
    independent logical consumers, including multiple subscriptions for one
    target only where the selected Rust extension mode permits them;
-   moving a `Send` facade between tasks, or externally serializing it, still
    polls the same cursor and defines no competing-consumer scheduling policy;
-   one subscription does not provide broadcast delivery to cloned values; an
    application may explicitly fan out received items under its own bounded
    storage and scheduling contract.

Protocol-side multiplexing or one native collection driver may combine sources
behind a single logical subscription, but it never creates additional
application cursors.

## Repository evidence and rejected alternatives

ADR-0003 already selected the non-`Clone` Servient facade, exact binding-driver
ownership, and absence of a cloneable receive/control view. The residual
subscription contract in `docs/design.md`, API ownership rows, and WP-300/WP-400
package split agree. The ambiguous architecture sentence was an incomplete
summary, not a competing technical direction.

A cloneable combined facade was rejected because `poll_event` and stop/drop
would race for one cursor and teardown owner. A cloneable control plus unique
receiver split was rejected for v1 because consumed-handle drain already
coordinates lifecycle through the private registry; a second public authority
would add cancellation/join semantics without a demonstrated use case.
Per-clone broadcast was rejected because it multiplies retention and
backpressure accounting. Competing-consumer semantics were rejected because
delivery distribution and fairness would become observable but were neither
specified nor required.

The current `core::Subscription` remains a nonconforming migration source: it
is `Clone`, shares one `VecDeque` cursor across clones, supplies a local-only
stop, and implements local merge/fan-out. API ownership and WP-400 already
require its removal/relocation. D4 does not authorize patching or retaining that
implementation.

## Migration projection and remaining scope

The decision is projected into:

-   `docs/architecture/10-primary-data-flows.md` for the canonical flow;
-   the active residual subscription contract in `docs/design.md`;
-   ADR-0003 and `docs/spec/binding-spi.md`, which already own driver semantics;
-   `docs/api-ownership.csv`, whose existing frozen rows place the public
    facades in Servient and the driver in Core;
-   `docs/work-packages/WP-300-bindings.md` and
    `docs/work-packages/WP-400-servient.md` for implementation ownership and
    negative compile evidence;
-   `tools/check-architecture-adrs.sh` for executable projection checks;
-   `PLAN.md` for D4 status; and
-   `PROJECT_STATE.md` for durable continuation.

D4 resolves only receiver/control ownership and clone semantics. It does not
close other AR3 subscription findings, admit WP-300 or WP-400 implementation,
or prove the exact stop/cleanup API, constrained slot implementation, storage,
performance, or lifecycle evidence. Those remain governed by their existing
gates and work packages. When the D3 subscriptions-and-emissions target becomes
dependency-ready, this decision migrates with the stable requirement ids into
that single-owner specification.
