# Consumer Host call construction boundary

Status: MIGRATED

## Question and evidence boundary

At fetched master `d22d5af6ae98211d56709b9ffebcf4fe8a86c601`, what is the
smallest sound Core/Binding prerequisite for WP-400 Consumer Property Read?
The selected narrow target and obligations are migrated to the
[ADR-0013 candidate](../docs/work-packages/WP-300-consumer-host-call-construction-admission.md),
Binding SPI, API ownership and WP-300/WP-400. Migration of the decision is not
implementation admission. Topic 0076 retains unrelated runtime/product questions.

## Independent production analysis

The actual target path is complete Consumer-only or dual-role Host registration
→ shared `start_host_consumer_property_read` → identity/capability checks → fixed
error and request-derived private seal → raw `ClientBinding::invoke` → raw
footprint callback → one of two Core wrappers. This is the target Binding SPI,
not legacy `outbound::ClientBinding::invoke`. The closed Host compiler's backing
and Planning output bridge are distinct allocations and owners.

Raw rejection returns the exact request. Raw acceptance returns an owned
`HostBindingCallBox` before protocol effects; its generic `new` calls `Box::pin`.
The normal wrapper `SealedHostConsumerCall` delegates result, deadline and cancel
progress and applies Core's validator to successful normal/late results. The
accepted-overrun wrapper `RejectedAcceptedHostConsumerCall` owns raw state and
unused seal, emits a fixed terminal resource error without polling raw state,
and reports saturated retained items/bytes on addition overflow. Both wrappers
are allocated **after raw acceptance** by `HostBindingCallBox::new`. Neither is
safe merely because the declaration is generous or the compiler slot is fallible.

The existing five-site downstream test aborts on every injected null allocation
in both branches; the final request is after acceptance. A focused client with
prebuilt fixed output isolates the raw concrete box and Core box: two allocator
requests, one after acceptance. These are actual installed Core paths and
allocator subprocesses, not a handwritten recovery model.

There is another independent boundary: `invoke`, footprint/deadline callbacks
and Drop have no WorkBudget enforcement, and budgeted raw polls are cooperative.
Registration validity and `ConsumerCompilerSupport` do not attest those behaviors.
Cleanup transfer carries the whole call. Accepted overrun reports a terminal
error but retains the raw owner until final release; resource error reporting
does not prove arbitrary raw destruction safe. A runtime must require concrete,
finite constructor/callback/storage/destructor evidence and keep ownership
through settlement. Neither catching panic nor adding a budget argument can
recover allocator abort or preempt arbitrary blocking code.

## Credible alternatives

| Alternative | Production consequence | Decision |
| --- | --- | --- |
| No Core change; declared limits/compiler support certify calls | Both post-acceptance infallible allocations remain; unsupported callbacks can block or allocate invisibly | Rejected by actual abort evidence and compiler-versus-call scope. |
| Fallible raw erasure only | Raw failure can return a concrete state/request, but either Core wrapper can still abort after acceptance | Useful additive primitive, insufficient alone. |
| Fallible wrapper allocation after acceptance | On null, the request is already accepted and needs a complete retained cleanup owner; dropping raw or fabricating input rejection violates ownership | Rejected without another pre-backed owner, which would itself be the real correction. |
| One complete Core envelope fallibly backed before `invoke` | Covers both existing mediators with one known maximum Layout; failure returns the original request before the raw callback; successful population/erasure allocates nothing | Selected. Same raw/installed SPI, generic carrier and static slots; one existing Core allocation moves earlier. Ordinary private helper extraction only. |
| Inline opaque installed Consumer owner | Reuses the mediators and removes Core heap backing; raw allocation can be fallible | Executable comparator. Changes both installed start return types and cleanup envelope type, and transports the accepted-error variant inline. Saves one allocation but grows the stored call carrier from 16 to 344 bytes. The Result stays 432 bytes because input rejection is already larger. That public/resource migration is unnecessary for this immediate safety prerequisite. |
| Inline policy inside generic `HostBindingCallBox<T, C>` | Keeps signatures but makes unrelated Producer/route call carriers carry Consumer seal/error/typed dispatch storage | Credible if broad measured need emerges; larger generic impact than a private Consumer envelope. |
| Provisioned call/wrapper pool or caller-owned slots | Can remove per-request allocations, but requires capacity ownership, refill, pin/projection, cancellation and reuse proofs | Credible runtime optimization; no realistic pool necessity has been established. Defer rather than make it a new Core framework. |
| Public fallible/budgeted invocation SPI or new call-support issuer | All implementers migrate; arbitrary pre-call allocation, blocking and destruction remain unproved | No necessity shown. Existing rejection shape and concrete runtime policy can express eligibility; a new signature/certificate would not supply evidence. |

The selected envelope includes tag/vacant storage and the larger accepted-error
variant, rather than assuming a normal-only preallocation can cover both cases.
It is a charged allocation, not capacity created by a declaration. The optional
raw constructor reuses the existing compiler `try_box` primitive and returns
`B` unchanged, enabling exact request recovery without a universal Box API or
new allocator framework. Other allocations performed while building `B` still
need concrete proof. One admitted correction owns both layers; splitting them
would leave an unsafe intermediate prerequisite.

## Admission and successor boundary

The reproducible experiments patch only a disposable Git snapshot. Existing Core
sealing, cancellation, Consumer-only/dual-role transfer and static tests are
reused. The public production source is untouched. The fixture README records
measurements and limitations; no gate/CI/current-task facts are cached here.

This candidate does not attest an actual protocol or install a supported runtime
policy. WP-400 must still select actual bindings and finite inputs, retain private
paired parent/global allowances, generation/plan/call leases and cleanup roots,
and prove publication races, delivery abandonment, drain and bounded fallback/
shutdown. Do not expand this construction correction into those decisions or
infer Consumer gate completion from its experiments.
