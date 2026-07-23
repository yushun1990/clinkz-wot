# Subscription Receiver Ownership and Clone Semantics Question

Status: OPEN

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
