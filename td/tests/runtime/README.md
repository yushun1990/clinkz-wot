# Production TD allocator runtime witness

This binary executes the public production `ValidatedThingCursor`, proof and
Property Read lending APIs with `no_std + alloc`. It imports no admission
prototype, private TD source projection, Planning coordinator or Servient model.
The same scenarios run natively on Linux and as a linked
`thumbv7em-none-eabihf` ELF booted on QEMU's
[MPS2 AN386 Cortex-M4](https://www.qemu.org/docs/master/system/arm/mps2.html).
The ARM run executes the allocator, moves, URI program and cleanup; it is more
than a target compile. QEMU execution supplies no device cycle, stack-margin or
product RAM claim.

From the Git repository root, with Rust, the ARM Rust target, Python 3, `timeout`,
a native C linker and `qemu-system-arm` installed:

```sh
rustup target add thumbv7em-none-eabihf
td/tests/runtime/run.sh
td/tests/runtime/run.sh --features async
```

On Ubuntu, `sudo apt-get install --no-install-recommends qemu-system-arm`
supplies the emulator. `QEMU_SYSTEM_ARM` can select another installed executable.
The script prints tool versions, resolved normal dependency features, actual
owner/allocator Layouts, work debits and allocator traces. It builds into
`target/td-admission-runtime`. A panic, ARM exception or timeout fails the command.
Mainline CI runs both commands on its pinned Rust toolchain. Runtime dependencies
use the repository's locked versions and enable no Rust `std` feature. Linux
uses libc for startup, memory intrinsics, console/exit and the access oracle's
page permissions and fork/wait controls; ARM uses a small
reset vector and semihosting console/exit. Unexpected native unwind entry fails
the run; the binary's panic strategy is abort.

The two unchanged named policies exercise:

- Twenty logical resource categories use fixed public-input oracles with
  one-short, exact and one-spare limits (60 outcomes per policy). Document,
  text, extension, Number, JSON depth/member/item/node, affordance/Form,
  additional-response/URI-variable, schema node/depth/edge, URI source and
  effective security root/depth limits all check the exact first cause and
  terminal allocator release. Inputs are provisioned one at a time outside
  admission. Existing lifetime and six physical boundary sweeps complete the
  27-row TD operation catalog; derived URI/effective-document limits also have
  separate semantic-phase boundaries in production TD tests.
- Whole typed inspection and shared Basic followed by lending of non-first
  original Forms, an empty readable range, defaults, explicit-empty security,
  relative URI resolution, path pop/repair and Unicode scope/copy totals.
- Actual geometric frame allocations, old/new coexistence, current URI
  replacement, exact requested sizes/alignments, allocation count, simultaneous
  peak and largest request. Exact and one-byte-short boundaries cover all six
  TD physical controls before allocator entry: the attempted Layouts must be
  exactly the successful run's prefix before its first forbidden request, with
  allocator-observed live bytes at each entry and peak below the tested ceiling.
  Inline movement/return capacity is computed from the running production types
  and is not a fake heap request.
- Every inspection and semantic suspension in the corpus, unacknowledged Ready
  states and Done: cancellation and abandonment require zero extra work or
  allocations and leave zero TD live bytes. Every actual TD allocation request
  is injected with null. Semantic failures preserve the first cause.
- Every URI/output action in the corpus with repeated one-unit-short credit:
  no partial multi-class debit or allocation, followed by the exact complete
  facts, class-debit and allocator trace of the uninterrupted run. Every semantic
  position additionally runs with zero output credit and read-only URI storage,
  and with zero URI credit and inaccessible source/derived URI bytes, including
  positions whose reported debit is zero. Both URI class totals are derived
  from the fixed inputs, independently of production WorkBudget debits. The
  two-byte repair actions also get independently selected one-output-unit
  probes; their requirement is never learned from the reported copy debit.
- Ready retries run with actual URI bytes inaccessible. A separate small source
  isolates base/href bytes, scope strings and their String descriptors as well;
  repeated zero/short output, URI and cleanup credit cannot read these regions.
  Access is restored before the caller traverses the returned loans.
- Exact lifetime exhaustion and repeated rewind without replenishment; complete
  4-KiB static and 16-KiB gateway resolved targets under their named work/memory
  ceilings; 63/64/65 and 255/256/257 Number boundaries and zero-disabled Numbers.
  The exact lifetime test repeats complete semantic passes when needed to fit
  the gateway's supported atomic iterator envelope. No named structural or URI
  ceiling is reduced.

The fixed URI copy oracle requires 28 and 137 writes for the two main Forms
(165 total): emitted bytes still count when popped; the authority-less repair
also writes two padding bytes, shifts seven path bytes, and inserts `/.`.
The simple long-URI cases require exactly 4,096/16,384 writes.

The independent URI-meaning oracle counts the declared resolver actions for
these fixed inputs. Each classification costs 16 UriBytes, fix/extend/insertion
actions cost 2, and other transitions cost 1. This arithmetic does not call the
production resolver or inspect its debits. `N` is the long Form's href length:

| Stage | First main Form | Second main Form | Long Form |
| --- | ---: | ---: | ---: |
| Target setup | 1 | 1 | 1 |
| Merge and merge classification | 0 | 17 | 17 |
| Configure | 1 | 1 | 1 |
| Prefix bytes and four span advances | 8 | 8 | 12 |
| Path segments, pops, stream and span advances | 129 | 221 | N + 30 |
| Fix, extend, shift and insertion | 14 | 2 | 2 |
| Tail bytes and four span advances | 8 | 4 | 4 |
| UTF-8 completion | 1 | 1 | 1 |
| **UriBytes total** | **162** | **255** | **N + 68** |

For the first path, the six segment costs are 19, 21, 21, 19, 19 and 28,
followed by two path-span advances. The second path has root/a/b costs
19 + 21 + 21, one span advance, parent-pop cost 21, and a 128-byte streaming
segment plus ten transition/scan actions. The long path has root cost 19,
one span advance, and N + 10 streaming actions. These expectations require
417 main-pass UriBytes and 4,155/16,443 for the long cases. Both URI class
oracles are checked at each Form and feed the exact shared lifetime boundary.
The one-unit-short sweep retains multi-class checks. Both independent zero-class
sweeps cover all 287 positions without selecting them from reported debits.
Zero-debit Pending retries contribute no accepted step; actions belonging only
to other classes may progress and are recorded once. Each run then resumes and
must reproduce the complete facts, class totals, step count and allocator trace.
This also observes unpaid scalar state changes that touch no protected bytes.

Equal eventual totals cannot establish prepayment of an individual action: a
two-byte write could debit one unit and recover the missing unit later. The
fixed first Form therefore has an independent action-position oracle anchored
to its public Form Ready event, without consulting either URI class debit.
Insertion precedes Ready by ten actions: four `?x#f` bytes, four tail-span
advances, UTF-8 completion and Finish. Padding precedes insertion by nine:
seven shifted `//value` bytes, one shift-end transition and insertion itself.
At both positions, three one-output-unit retries run with URI storage read-only
and require Pending, unchanged credit in every class and an identical allocator
trace. Stores (including same-value stores) fault. Exactly two output units and
two URI units must then advance that action; full replay must preserve facts,
work, step count and allocator trace. These are the corpus's only multi-byte
copy actions; all positions still get the independent zero-output probe. A
change to the action schedule requires updating this fixed-fixture oracle.
Positive UriBytes shortages now use complete literal action schedules for both
fixed targets, anchored only to their public Form Ready events. The first has
71 actions from Resolve through Ready: setup/allocation, configure/prefix,
root/a/parent/parent/empty/value segments, span advances, repair, tail and UTF-8
completion. The second has 181 actions: setup/allocation, merge/classification,
configure/prefix, root/a/b, span advance, parent/pop, 128-byte stream, fix and
tail/UTF-8 completion. Classification requires 16 units; fix/padding/insertion
requires two; other URI actions require one; allocation and Finish require zero
UriBytes. These schedules independently reproduce the table totals and verify
every reported position before the shortage sweep. Each positive shortage
must preserve all budgets, allocator trace and continuation, then reproduce
the complete run. They are fixed corpus evidence, not an exhaustive URI grammar
or target instruction count.

The access oracle uses Linux [`mprotect`](https://man7.org/linux/man-pages/man2/mprotect.2.html)
or the Cortex-M4 MPU ([Arm register definitions](https://github.com/ARM-software/CMSIS_5/blob/develop/CMSIS/Core/Include/core_cm4.h)).
It observes real accesses, including stores of unchanged bytes, without TD hooks
or a copied resolver. Both backends first demonstrate a denied load and a denied
same-value store. Linux controls run in forked children and require SIGSEGV;
ARM controls require DACCVIOL and the expected MMFAR, then restore access and
retry only those deliberate control instructions. Interrupts remain masked, so
ARM MPU faults escalate through HardFault. Any fault during TD execution fails
the witness. A guard's catalog stays active throughout the step: new byte blocks
are registered and protected **before allocator return**, including initial
acquisition and replacement. The allocator restores access only within its
own fixed canary initialization/check, invokes no TD/caller code there, and
re-arms all ranges before handing back control. Released ranges remain guarded
until the step ends. This backend supports 4-KiB native pages and at most eight
protected power-of-two regions; unsupported hardware/page configuration fails
explicitly.

Both registered commands also run `check_mutations.py`. It copies the current
tracked worktree and runtime sources into a disposable directory, compiles each
counterexample, and executes native and booted ARM binaries. The baseline must
pass; a compile failure never counts as rejection. Retained cases cover the
reviewer's extra copy immediately after acquisition, copies conditional on zero
output credit at initial acquisition and replacement, ARM-only missing URI
charging, its zero-URI-credit-only variant, and undercharged two-byte repairs
compensated by later tail charges. The compensated mutation preserves final
totals and must fault at the independent one-credit padding probe. Acquisition
cases require actual access faults. A positive-URI-shortage-only mutation
undercharges classification only when offered 15 units; full-credit and zero
credit runs remain unchanged, while the independently selected positive probe
must reject it on both targets. Missing-charge cases require the independent
URI oracle or access fault. The unchanged native branches of ARM-only mutations
must pass. Logs/build artifacts are under `target/td-admission-mutations`; production worktree sources
remain untouched.

The allocator has fixed 256-KiB caller-source and 128-KiB TD regions, fixed
placement/observation metadata and eight-byte guards after each request. Source
capacity includes ordinary serde provisioning and isolated URI page padding;
the TD delegation and named production ceilings keep their original values.
Its complete aligned backing Layout is charged once as startup storage.
TD suballocations delegate that backing; their
requested bytes are not charged again as new physical backing. Arena span also
reports padding, guards and unreused holes. URI blocks and the selected Ready
source allocations own isolated page regions; all their unused page capacity
and the access-control metadata remain in the charged backing Layout.
Deallocation checks the original pointer/Layout and guards before matching the
release. The binary has one caller and no threads; ARM interrupts stay disabled.
This allocator is an evidence backend, not a proposed product allocator or
general concurrent API.
The child temporary ledger is capped by the delegated TD region. Inline
movement/return capacity is reserved separately before entry and released only
after physical children reach zero; the backing reservation stays live.

Caller input is provisioned through ordinary serde outside bounded admission,
including a boxed Thing root. Its actual retained allocator requests are
observed, not estimated from typed lengths. The upstream source account is
charged before lending and remains live through every TD outcome. The child
ledger receives the same source projection. Only actual source destruction
releases the original source account. A separate zero-source-charge run has the
identical TD allocation/work/fact trace. Startup backing remains charged after
source and TD children reach zero. This proves neither first-allocation bounded
ingestion nor simultaneous Servient parent/global allowance pairing. Test driver,
configuration and caller stack storage are outside the additional TD owner
formula; no whole-process stack/RSS measurement is claimed.

The registered [WP-100 contract](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md)
owns production completion. This runner contributes executable resource and
lifecycle evidence for its fixed corpus. It does not implement the external
Planning semantic join or independently accept completion. Its fixed corpus
joins the production field/Basic tests and external witness in WP-100's
registered completion evidence; it cannot alone establish complete typed
coverage. WP-200 owns variable output/compiler cleanup; WP-400 owns the
transaction, paired allowances, publication, leases and reclamation. Strict
bounded ingestion and real protocol/device characterization retain their
separate owners.
