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

From the repository root, with Rust, the ARM Rust target, `timeout`, a native C
linker and `qemu-system-arm` installed:

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
uses libc only for startup, memory intrinsics and console/exit; ARM uses a small
reset vector and semihosting console/exit. Unexpected native unwind entry fails
the run; the binary's panic strategy is abort.

The two unchanged named policies exercise:

- Whole typed inspection and shared Basic followed by lending of non-first
  original Forms, an empty readable range, defaults, explicit-empty security,
  relative URI resolution, path pop/repair and Unicode scope/copy totals.
- Actual geometric frame allocations, old/new coexistence, current URI
  replacement, exact requested sizes/alignments, allocation count, simultaneous
  peak and largest request. Exact and one-byte-short boundaries cover all six
  TD physical controls before allocator entry. Inline movement/return capacity
  is computed from the running production types and is not a fake heap request.
- Every inspection and semantic suspension in the corpus, unacknowledged Ready
  states and Done: cancellation and abandonment require zero extra work or
  allocations and leave zero TD live bytes. Every actual TD allocation request
  is injected with null. Semantic failures preserve the first cause.
- Every URI/output action in the corpus with repeated one-unit-short credit:
  no partial multi-class debit or allocation, followed by the exact complete
  facts, class-debit and allocator trace of the uninterrupted run. Ready retries
  inspect the actual URI and scope loans at zero credit.
- Exact lifetime exhaustion and repeated rewind without replenishment; complete
  4-KiB static and 16-KiB gateway resolved targets under their named work/memory
  ceilings; 63/64/65 and 255/256/257 Number boundaries and zero-disabled Numbers.
  The exact lifetime test repeats complete semantic passes when needed to fit
  the gateway's supported atomic iterator envelope. No named structural or URI
  ceiling is reduced.

The allocator has two fixed 128-KiB regions, fixed placement/observation metadata
and eight-byte guards after each request. Its complete aligned backing Layout is
charged once as startup storage. TD suballocations delegate that backing; their
requested bytes are not charged again as new physical backing. Arena span also
reports padding, guards and unreused holes. Deallocation checks the original
pointer/Layout and guards before matching the release. The binary has one
caller and no threads; ARM interrupts stay disabled. This allocator is an
evidence backend, not a proposed product allocator or general concurrent API.
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
Planning semantic join, establish exhaustive field/resource coverage, remove
the orphaned Foundation reclassification API, register completion evidence or
accept a gate. WP-200 owns variable output/compiler cleanup; WP-400 owns the
transaction, paired allowances, publication, leases and reclamation. Strict
bounded ingestion and real protocol/device characterization retain their
separate owners.
