# Strict Number lexeme boundary probe

This is non-production evidence for the strict JSON Number portion of
[`WP-100-CONSUMER-VALIDATED-THING`](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md#evidence-required-before-separate-readmission)
pre-readmission item 6. The tranche remains `planned` / `candidate`.

## Reproduce

From the repository root:

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-strict-number-lexeme/Cargo.toml
cargo check --locked --target thumbv7em-none-eabihf --manifest-path tools/architecture-fixtures/validated-thing-strict-number-lexeme/Cargo.toml
cargo fmt --manifest-path tools/architecture-fixtures/validated-thing-strict-number-lexeme/Cargo.toml -- --check
```

The fixture uses the current workspace's serde_json 1.0.149 in its locked
Host grammar oracle. The lexer library is `no_std` and owns no heap storage;
the thumb command is a compile check, not a target runtime test.

## Boundary proved

`NumberLexeme` is called only after the outer JSON parser identifies a Number
start. Its scalar state recognizes JSON Number sign, integer, fraction, and
exponent syntax. The caller first debits `CodecInputBytes` from a step budget
and a non-resettable lifetime, then feeds one observed byte. Only `Consumed`
authorizes the caller to copy that byte. When the next syntactically possible
Number byte would make the token `L + 1` bytes, `feed` returns terminal
`Limit` with exactly `L` bytes consumed and copied. It does not inspect the
rest of the token, parse an f64, or complete the Number grammar. A delimiter
is observed under the caller's charge but left to the outer JSON parser.

Tests exercise `63/64/65`, `255/256/257`, zero-disabled first-byte rejection,
and an application-defined 257-byte ceiling. A 65,536-byte exponent with a
late invalid character stops after 65 observed bytes under `L = 64`, with 64
copied bytes. One-byte step budgets resume the same scalar state; a zero step
budget observes nothing, and exhausted lifetime work observes no unpaid byte.
Small valid/invalid grammar cases agree with the locked serde_json parser.

This fixture does not implement the outer JSON parser, typed TD field decoder,
the full opaque admission configuration, arena accounting, Basic validation,
or cancellation and rollback. Its byte debit is a test driver for this lexical
boundary, not a complete admission work trace. It cannot independently satisfy
pre-readmission item 6 or authorize production source changes.
