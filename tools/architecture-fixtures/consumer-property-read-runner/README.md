# Consumer Property Read Runtime application

This downstream package uses only public Consumer-only registrations and the
production Servient runtime. `application` consumes a real borrowed typed Thing,
destroys it, reads `a-read` Form 2 and `c-温度` Form 1 through actual compiled
artifacts, receives Core-sealed results and explicitly shuts down. Its normal
library, including the real associated-state slot and Runtime layout table,
compiles for ARM without `std` or an executor.

The companion binding prepares its eight variable results before installation.
Its normal constructor/poll/cancel/destructor paths allocate no payload, String,
map or stats backing. Host invocation attempts only the recoverable concrete
call box after Core has provisioned its envelope. The admitted profile has one
URI variable, 64 UTF-8 bytes and 64 bytes of actual String capacity per value,
64-byte property names and result bodies, and 32-byte media/coding capacities.
The narrowed 1,280-byte raw allowance covers the 376-byte native owner plus
these bounded variable owners; Core and request/result/transport envelopes have
additional Runtime allowances. The binding's broader reported 2,048-byte raw
footprint remains conservative. Callback hooks are finite test controls for
reentrancy and interleavings, not arbitrary callback qualification.

Host construction returns `(runtime, report)` from `try_new`; `report()` remains
usable after every runtime/handle/ticket is gone. Construction rejection returns
the original complete registration, checked policy, clock and parent owners.
Static construction returns the registration and policy on rejection and leaves
its borrowed clock, capacity bank and exclusively lent report storage unchanged.
Both representations keep the original committed Planning cleanup account.

Host call storage and complete static slot storage have independent byte limits
and shared parent accounts in addition to engine bytes. Host reserves the whole
call allowance before invocation and retains it through result/cleanup release.
Static reserves its complete resident slot, including bounded indirect backing,
at startup and retains that capacity while the runtime owns even a vacant slot.
The independent report also has to fit the cleanup per-item limit at startup.

Each progress/shutdown operation shares one configured cleanup work quantum
across cancellation, release and reclamation. Excess caller credit remains
unused. Startup rejects a quantum below an indivisible declared terminal
operation; this fixture's four-item registration retirement requires four units.
An acknowledged Host manual transfer changes its matching Pending report to
Complete before raw-call destruction, including result discard. An existing
residual report is preserved.

`consume(thing, final_work, |build| ...)` lends a local, source-bearing adapter.
Drive `build.step(work)` until `build.is_finished()`. The scope can wait for the
application's work/timer, but cannot export a source loan or a local Planning
root. Returning early abandons the entire continuation to the reserved manual
root. A finished adapter means source-bearing work ended; only `consume` Ready
or subsequent final installation exposes the published generation. Short final
credit leaves source-free Frozen state. Host `take_handle` then retrieves the
sole installed facade.

Property Read rejection returns `(options, optional_request)`. Before request
construction, options are unchanged. After Core rejection, selection controls
remain in options and the original URI map/strings remain in the returned
request. No request or selection-name copy is required. The fixed read/result
opportunity stays occupied through terminal observation and acknowledgement.

Use `step` after binding wakes or the reported `next_deadline`. Call-ticket loss
requests cancellation; handle loss closes the generation; root loss closes
admission while live tickets can still receive their sealed terminal value.
The first control step after cancellation samples fresh time for its independent
drain deadline; refilling work never extends it. `shutdown` drives retained work
to Ready and retires the complete registration outside structural locks. Facade
and report backing remain charged until their owners are dropped. Final-root abandonment
records incomplete shutdown before every controlled deallocation, performs no
protocol progress, and uses the original reclaimer with finite non-refilled
credits. It supplies local disposition rather than external cleanup success.

Tests and exact allocation observation live in
`servient/tests/consumer_property_read/`. The completion evidence record maps
them to the six admitted obligations and existing performance identities.
Neither these fixtures nor ARM compilation certify a real protocol, arbitrary
client callbacks, physical device fit, fleet scalability or a completed gate.

```
cargo test --locked --offline -p clinkz-wot-consumer-property-read-runtime-runner
cargo test --locked --offline -p clinkz-wot-consumer-property-read-runtime-runner --no-default-features
cargo test --locked --offline -p clinkz-wot-consumer-property-read-runtime-runner --no-default-features --features async
cargo check --locked --offline -p clinkz-wot-consumer-property-read-runtime-runner --no-default-features --target thumbv7em-none-eabihf
```
