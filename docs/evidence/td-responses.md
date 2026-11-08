# TD-described finite response execution

This increment concretizes ADR-0025 and EXEC-DATA/OUTCOME-001 after master
`678395337e40bcd5559c6a9bb244854f1981b92a` (#272). It replaces provisional
primary-only finite restrictions with the [shared response contract](../spec/interaction-core.md#td-described-finite-response-alternatives)
and one [HTTP Property profile](../../protocol-bindings/protocols/http/README.md#td-described-finite-response-alternatives).
The [investigation](../../workspace/0119-td-response-execution.md) records the
independent standards/architecture reassessment and rejected alternatives.
W3C TD 1.1 is the dated Recommendation; its protocol examples are not a universal
native response-classification or effect-certainty contract.

## Executable capabilities

The real TCP Consumer/Producer tests in
[response_alternatives.rs](../../protocol-bindings/protocols/http/tests/support/response_alternatives.rs)
exercise primary Read/Write, additional success, declared application failure,
schemaDefinitions overrides, inherited ordinary schemas, media defaults,
and independent JSON request/UTF-8-parameter response media. Two Forms of one
Property use different error schemas and branch order; one behavior chooses
through its borrowed original contract. A preserved real-I/O counterexample
proves that sibling target/operation equality cannot certify the wrong Form's
response. The HTTP validator retains native attribution with the actual
exclusive exchange/reply. Prepared sources
and registries are dropped before actual exchanges. Native response identity
is HTTP-owned; TD success can differ from the native status class. The tests
retain original instances, actual services, reply opportunities and resource
accounts through settlement and final physical listener/socket release.

Negative qualification tests refuse an interaction when any response has an
unsupported media/assertion, ambiguous/missing/malformed native discriminator,
body/status incompatibility or unsupported continuing acceptance. Both Consumer
selection and complete Producer preparation exercise that refusal before I/O.
Producer final checks compare the exact retained response/native/schema meaning.
The older output-only Producer adapter refuses additional responses rather than
promise branches it cannot execute. Its new primary response owner is included
in retained bytes, largest allocation and disposal work.
Core tests prove untrusted additional indices cannot enlarge the original
request's admitted set, lie about success/data presence or require a native
numeric status. Independent TCP peers test malformed response values separately
from undeclared native responses. Rejected Consumer data retains its original
branch/body rather than becoming a declared application error.

Write tests deliberately apply the value before returning declared failure or
invalid output. Consumer/Producer preserve uncertain effects and never replay.
A Producer handler can report completion independently of delivery; a Consumer
additional success remains Unknown without separate native completion proof.
Cancellation of a blocked failure response preserves the first local cause,
original Producer application result, uncertain effect and undelivered reply
through funded retirement. Existing primary lifecycle, invalid input, timeout,
replacement, Security composition and resource tests remain regression evidence.

The shared contract, response projection and checks are alloc/no_std and do not
require pointer atomics. `ValueExecution` lends complete response meaning from
Host content. The existing caller-owned Read explicitly declines additional
responses before native start. Its real datagram and no-atomic build evidence
remain scoped to that representation's supported primary Read. Its actual native
datagram results now omit numeric status entirely, without a synthetic zero.

## Reproduction

Use the exact reviewed commit and report actual results in the PR/review. The
required automatic validation status checks only Git diff hygiene.

```sh
cargo test --workspace --locked -- --test-threads=1
cargo test --locked -p clinkz-wot-core --no-default-features --lib
cargo test --locked -p clinkz-wot-constrained-consumer-fixture --features host-meaning
cargo check --locked -p clinkz-wot-constrained-consumer-fixture --target thumbv6m-none-eabi
cargo test --locked --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml
tools/check-design-artifacts.sh
sh scripts/check-feature-matrix.sh
git diff --check
```

No benchmark determines this architecture. Existing allocation observations
check funding/disposal obligations; the design checker only verifies existing
performance fixture metadata. These checks do not constitute independent
architecture acceptance, milestone closure or production qualification.

## Limits

The increment executes finite leaf Property responses over the existing manually
driven Host HTTP/HTTPS implementation. The portable semantic values are shared;
TD-driven Planning and Host owners are not newly ported. Continuing Actions,
notifications and collections retain their existing narrower response support.
The HTTP discriminator uses unique declared status codes, not arbitrary header,
body or media-based branch matching. JSON and one explicit UTF-8 media parameter
are the only added representations; no non-JSON/image interoperability, broad
schema/charset support, automatic retry or remote cancellation certainty is
claimed. Named response constraints retain the existing structured subset.

Historical Property Read, collection, Security and portable evidence is
reaffirmed as regression evidence within its original scope; it does not certify
these alternatives by inheritance. No package, gate or milestone status changes.
