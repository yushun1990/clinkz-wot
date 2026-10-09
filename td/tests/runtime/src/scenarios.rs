use alloc::{boxed::Box, format, string::ToString};
use clinkz_wot_foundation::{
    AdmissionLedger, BenchmarkStaticReferenceV1, GatewayDefaultV1, Generation, ResourceAccount,
    ResourceKind as R, ResourceLimits, SlotIndex, StaticResourceProfile, WorkBudget,
    WorkClass as W,
};
use clinkz_wot_td::{
    ValidatedPropertyReadCursor as Read, ValidatedPropertyReadEvent as Event,
    ValidatedPropertyReadStep as Step, ValidatedThing, ValidatedThingAdmissionConfig as Config,
    ValidatedThingCause as Cause, ValidatedThingCursor as Inspect, ValidatedThingFailureKind,
    ValidatedThingPhase as Phase, ValidatedThingProgress as Progress, data_type::FormHref,
    thing::Thing, validate::Validate,
};
use core::mem::size_of;

use crate::{
    access::{Guard, Permission},
    heap, report,
};

const CREDIT: u64 = 256;
const POLLS: usize = 300_000;
const SEMANTIC_STEPS: usize = 1024;
const CLASSES: usize = W::ALL.len();

fn inline_bytes() -> u64 {
    2 * size_of::<Progress<'_>>().max(size_of::<Read<'_>>()) as u64
}

fn budget() -> WorkBudget {
    W::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, CREDIT))
}
fn spent(budget: &WorkBudget) -> [u64; CLASSES] {
    core::array::from_fn(|i| CREDIT - budget.remaining(W::ALL[i]))
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Cost {
    uri: u64,
    output: u64,
    ready: bool,
    form_ready: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct Run {
    inspect: usize,
    semantics: usize,
    properties: usize,
    forms: usize,
    digest: u64,
    work: [u64; CLASSES],
    inspection_work: u64,
    uri_writes: u64,
    uri_work: u64,
}
impl Run {
    fn new() -> Self {
        Self {
            inspect: 0,
            semantics: 0,
            properties: 0,
            forms: 0,
            digest: 0,
            work: [0; CLASSES],
            inspection_work: 0,
            uri_writes: 0,
            uri_work: 0,
        }
    }
    fn debit(&mut self, budget: &WorkBudget) {
        for (total, debit) in self.work.iter_mut().zip(spent(budget)) {
            *total += debit;
        }
    }
}
fn input() -> Box<Thing> {
    let mut t: Thing = serde_json::from_str(r#"{
      "@context":["https://www.w3.org/2022/wot/td/v1.1",{"private":{"deep":[true,null,1e309]}}],
      "title":"runtime", "id":"urn:runtime", "base":"foo:/a/b/",
      "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
      "properties":{
        "empty":{"type":"null","forms":[{"href":"ignored","op":[]}]},
        "zeta":{"type":"object","properties":{"nested":{"type":"array","items":{"type":"string"}}},
          "forms":[{"href":"write","op":["writeproperty"]},
            {"href":"/a/../..//value?x#f","contentCoding":"gzip","scopes":["one","二",""]},
            {"href":"../last","security":[]}]}
      },
      "actions":{"other":{"input":{"oneOf":[{"type":"string"},{"type":"integer"}]},"forms":[{"href":"action"}]}},
      "events":{"other":{"data":{"type":"string"},"forms":[{"href":"event"}]}},
      "opaque":{"array":[1,{"more":[true,null,"original source"]}]}
    }"#).unwrap();
    // The later Form forces current URI capacity replacement without copying
    // the previous target. Its original index remains two.
    t.base = Some(heap::isolated_source(|| {
        clinkz_wot_td::data_type::BaseUri::parse("foo:/a/b/").unwrap()
    }));
    let forms = &mut t
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms;
    forms[1].href = heap::isolated_source(|| FormHref::parse("/a/../..//value?x#f").unwrap());
    let last = format!("../{}", "v".repeat(128));
    forms[2].href = heap::isolated_source(|| FormHref::parse(&last).unwrap());
    drop(last);
    assert!(t.validate().is_ok());
    Box::new(t)
}
fn ledger(limits: &ResourceLimits, source: usize) -> AdmissionLedger {
    let mut ledger = AdmissionLedger::new(
        SlotIndex::new(1),
        Generation::INITIAL,
        source as u64,
        limits
            .get(R::AdmissionTemporaryBytesPerOperationMax)
            .unwrap()
            .min(heap::REGION_BYTES as u64),
        0,
        0,
        0,
        0,
    );
    // Projection of the still-live upstream source. TD must neither release it
    // nor count it as additional TD temporary capacity.
    ledger
        .try_reserve_source(R::RetainedSourceBytesPerOwnerMax, source as u64)
        .unwrap()
        .commit();
    ledger
}
fn cursor<'a>(t: &'a Thing, limits: &ResourceLimits, source: usize) -> Inspect<'a> {
    let before = heap::trace();
    let checked = Config::try_from_limits(limits).unwrap();
    let c = Inspect::from_thing(t, &checked, ledger(limits, source));
    assert_eq!(
        heap::trace(),
        before,
        "configuration/constructor must not allocate"
    );
    c
}
fn proof<'a>(mut cursor: Inspect<'a>, run: &mut Run) -> Result<ValidatedThing<'a>, Cause> {
    for _ in 0..POLLS {
        let mut b = budget();
        let next = cursor.step(&mut b, false);
        run.inspect += 1;
        run.debit(&b);
        match next {
            Progress::Pending(c) => cursor = c,
            Progress::Complete(p) => {
                run.inspection_work = run.work.iter().sum();
                return Ok(p);
            }
            Progress::Failed(cause) => return Err(cause),
        }
    }
    panic!("production inspection failed to progress");
}
fn hash(mut h: u64, text: &str) -> u64 {
    for byte in text.bytes() {
        h = h.wrapping_mul(16777619) ^ byte as u64;
    }
    h
}
type Facts = (bool, u64, u64, u64);
enum Observation {
    Pending,
    Done,
    Ready(Facts),
}
fn observe(step: Step<'_>) -> Observation {
    match step {
        Step::Pending => Observation::Pending,
        Step::Done => Observation::Done,
        Step::Ready(event) => Observation::Ready(fact(event)),
    }
}
fn fact(event: Event<'_>) -> Facts {
    match event {
        Event::Property { ordinal, name } => (true, hash(ordinal as u64, name), 0, 0),
        Event::Form(f) => {
            assert!(f.readable());
            if f.property_name() == "p" {
                // The full-envelope URI case also drives the physical limits
                // with URI storage, rather than frame growth, as the peak.
                let prefix = "http://a/";
                assert_eq!((f.property_ordinal(), f.original_index()), (0, 0));
                assert!(f.resolved_href().starts_with(prefix));
                assert_eq!(&f.resolved_href()[prefix.len()..], f.href());
                assert!(f.href().bytes().all(|b| b == b'a'));
                assert_eq!(f.scopes().len(), 0);
                assert_eq!(f.security_scheme(), Some("nosec"));
                assert_eq!(
                    f.copy_bytes(),
                    (1 + f.href().len() + f.resolved_href().len() + f.content_type().len()) as u64
                );
                // This fixture has no dot removal or repair: every prefix and
                // href byte is written exactly once. No production debit is
                // an input to the expected copy charge.
                return (
                    false,
                    hash(0, f.resolved_href()),
                    (prefix.len() + f.href().len()) as u64,
                    // Fixed-input stage counts: setup/merge/configure/prefix,
                    // one root segment, streamed href, fix/tail/validate.
                    // See the README table; never learn this from work debits.
                    (1 + 17 + 1 + 12 + (f.href().len() + 30) + 2 + 4 + 1) as u64,
                );
            }
            assert_eq!((f.property_ordinal(), f.property_name()), (1, "zeta"));
            assert!(matches!(f.original_index(), 1 | 2));
            assert_eq!(f.content_type(), "application/json");
            let mut h = hash(f.original_index() as u64, f.resolved_href());
            h = hash(h, f.href());
            let scopes = f.scopes();
            let mut bytes = 0;
            for s in scopes.iter() {
                bytes += s.len() as u64;
                h = hash(h, s);
            }
            assert_eq!(bytes, scopes.byte_len());
            let copy = "urn:runtime".len()
                + f.property_name().len()
                + f.href().len()
                + f.resolved_href().len()
                + f.content_type().len()
                + f.content_coding().map_or(0, str::len)
                + f.subprotocol().map_or(0, str::len)
                + bytes as usize;
            assert_eq!(f.copy_bytes(), copy as u64);
            let writes = if f.original_index() == 1 {
                assert_eq!(f.resolved_href(), "foo:/.//value?x#f");
                assert_eq!(f.security_count(), 1);
                assert_eq!(f.security_name(), Some("none"));
                assert_eq!(f.security_scheme(), Some("nosec"));
                assert_eq!((scopes.len(), scopes.byte_len()), (3, 6));
                // Hand-derived writes for this fixed URI: prefix; /a/ then
                // /value (popped bytes still cost); two repair padding bytes;
                // seven shifted path bytes; '/.' insertion; ?x#f tail.
                "foo:".len() + "/a/".len() + "/value".len() + 2 + "//value".len() + 2 + "?x#f".len()
            } else {
                assert_eq!(f.security_count(), 0); // explicit empty overrides root
                assert!(f.security_name().is_none() && f.security_scheme().is_none());
                assert_eq!(f.resolved_href().len(), "foo:/a/".len() + 128);
                // /b/ is popped after emission; its writes remain charged.
                "foo:".len() + "/a/b/".len() + 128
            };
            // Independent fixed-input transition counts (README table).
            // Classify/MergeClassify cost 16; repair costs 2; other actions 1.
            let uri_work = if f.original_index() == 1 {
                1 + 0 + 1 + 8 + 129 + 14 + 8 + 1
            } else {
                1 + 17 + 1 + 8 + 221 + 2 + 4 + 1
            };
            (false, h, writes as u64, uri_work)
        }
    }
}
fn semantic_poll(c: &mut Read<'_>, run: &mut Run) -> Result<(bool, Cost), Cause> {
    let mut b = budget();
    let result = observe(c.step(&mut b, false)?);
    record_poll(c, run, spent(&b), result)
}
fn record_poll(
    c: &mut Read<'_>,
    run: &mut Run,
    debit: [u64; CLASSES],
    result: Observation,
) -> Result<(bool, Cost), Cause> {
    run.semantics += 1;
    for (total, debit) in run.work.iter_mut().zip(debit) {
        *total += debit;
    }
    let cost = Cost {
        uri: debit[W::UriBytes as usize],
        output: debit[W::CodecOutputBytes as usize],
        ready: matches!(&result, Observation::Ready(_)),
        form_ready: matches!(&result, Observation::Ready((false, ..))),
    };
    match result {
        Observation::Pending => Ok((false, cost)),
        Observation::Done => Ok((true, cost)),
        Observation::Ready(expected) => {
            let before = heap::trace();
            // Re-lend and inspect actual facts/URI/scopes at zero credit. No
            // allocation, copy, sizing scan or new TD work is available.
            for _ in 0..3 {
                let event = {
                    // Actual URI bytes are inaccessible during production's
                    // Ready step. Restore access before the caller reads them.
                    let _guard = Guard::new(heap::byte_regions(false), Permission::None);
                    let Step::Ready(event) = c.step(&mut WorkBudget::new(), false)? else {
                        panic!("Ready must persist until acknowledgement");
                    };
                    event
                };
                assert_eq!(fact(event), expected);
                assert_eq!(heap::trace(), before);
            }
            if expected.0 {
                run.properties += 1;
            } else {
                run.forms += 1;
                run.uri_writes += expected.2;
                run.uri_work += expected.3;
                assert_eq!(
                    run.work[W::CodecOutputBytes as usize],
                    run.uri_writes,
                    "mandatory URI writes must debit CodecOutputBytes and the shared lifetime"
                );
                assert_eq!(
                    run.work[W::UriBytes as usize],
                    run.uri_work,
                    "mandatory URI meaning must debit UriBytes and the shared lifetime"
                );
            }
            run.digest = run.digest.wrapping_mul(16777619) ^ expected.1;
            c.acknowledge();
            Ok((false, cost))
        }
    }
}
fn finish(c: &mut Read<'_>, run: &mut Run) -> Result<(), Cause> {
    for _ in 0..POLLS {
        if semantic_poll(c, run)?.0 {
            return Ok(());
        }
    }
    panic!("production semantics failed to progress");
}
fn complete(t: &Thing, limits: &ResourceLimits, source: usize) -> Result<Run, Cause> {
    let mut run = Run::new();
    let mut c = proof(cursor(t, limits, source), &mut run)?.into_property_read();
    if let Err(cause) = finish(&mut c, &mut run) {
        let before = heap::trace();
        assert_eq!(c.step(&mut budget(), true).err(), Some(cause));
        c.acknowledge();
        c = c.rewind();
        assert_eq!(c.step(&mut WorkBudget::new(), false).err(), Some(cause));
        assert_eq!(heap::trace(), before);
        return Err(cause);
    }
    Ok(run)
}
fn inspect_prefix<'a>(
    t: &'a Thing,
    limits: &ResourceLimits,
    source: usize,
    polls: usize,
) -> Inspect<'a> {
    let mut c = cursor(t, limits, source);
    for _ in 0..polls {
        c = match c.step(&mut budget(), false) {
            Progress::Pending(c) => c,
            _ => panic!("prefix exceeded inspection suspension boundary"),
        };
    }
    c
}
fn semantic_prefix<'a>(
    t: &'a Thing,
    limits: &ResourceLimits,
    source: usize,
    polls: usize,
) -> (Read<'a>, Run) {
    let mut run = Run::new();
    let mut c = proof(cursor(t, limits, source), &mut run)
        .unwrap()
        .into_property_read();
    for _ in 0..polls {
        semantic_poll(&mut c, &mut run).unwrap();
    }
    (c, run)
}
fn physical_boundaries(t: &Thing, limits: &ResourceLimits, source: usize, baseline: heap::Trace) {
    let inline = inline_bytes();
    for (kind, observed) in [
        (
            R::AdmissionTemporaryBytesPerOperationMax,
            baseline.peak as u64 + inline,
        ),
        (
            R::AdmissionTemporaryBytesGlobalMax,
            baseline.peak as u64 + inline,
        ),
        (
            R::PeakLiveBytesPerAdmissionMax,
            baseline.peak as u64 + inline,
        ),
        (
            R::AdmissionPeakLiveBytesGlobalMax,
            source as u64 + baseline.peak as u64 + inline,
        ),
        (
            R::EngineLiveBytesGlobalMax,
            source as u64 + baseline.peak as u64 + inline,
        ),
        (
            R::LargestContiguousAllocationBytesMax,
            baseline.largest as u64,
        ),
    ] {
        for ceiling in [observed - 1, observed] {
            let policy = limits.clone().with_limit(kind, Some(ceiling));
            heap::begin(0);
            let result = complete(t, &policy, source);
            let trace = heap::end();
            if ceiling == observed {
                assert!(
                    result.is_ok(),
                    "exact physical threshold {kind:?}: {result:?}"
                );
                assert_eq!(trace, baseline);
            } else {
                let Err(Cause::Limit(limit)) = result else {
                    panic!("missing {kind:?} rejection");
                };
                assert_eq!(
                    (limit.kind(), limit.configured(), limit.observed()),
                    (kind, ceiling, observed)
                );
                let requested_total = |request: &heap::Request| -> u64 {
                    if kind == R::LargestContiguousAllocationBytesMax {
                        request.bytes as u64
                    } else {
                        let retained = if matches!(
                            kind,
                            R::AdmissionPeakLiveBytesGlobalMax | R::EngineLiveBytesGlobalMax
                        ) {
                            source as u64
                        } else {
                            0
                        };
                        retained + inline + request.live_before as u64 + request.bytes as u64
                    }
                };
                let prefix = baseline.requests[..baseline.attempts]
                    .iter()
                    .take_while(|request| requested_total(request) <= ceiling)
                    .count();
                assert_eq!(
                    trace.attempts, prefix,
                    "reject before the first forbidden allocator entry: {kind:?}"
                );
                assert_eq!(
                    &trace.requests[..trace.attempts],
                    &baseline.requests[..prefix]
                );
                assert!(
                    trace.requests[..trace.attempts]
                        .iter()
                        .all(|r| requested_total(r) <= ceiling)
                );
                let actual = if kind == R::LargestContiguousAllocationBytesMax {
                    trace.largest as u64
                } else {
                    inline
                        + trace.peak as u64
                        + if matches!(
                            kind,
                            R::AdmissionPeakLiveBytesGlobalMax | R::EngineLiveBytesGlobalMax
                        ) {
                            source as u64
                        } else {
                            0
                        }
                };
                assert!(
                    actual <= ceiling,
                    "allocator-observed peak/request exceeded {kind:?}"
                );
            }
        }
    }
    report(format_args!(
        "physical: inline={inline} source={source} requests={} heap_peak={} largest={} arena_span={}",
        baseline.attempts, baseline.peak, baseline.largest, baseline.span
    ));
    for request in &baseline.requests[..baseline.attempts] {
        report(format_args!(
            "  Layout bytes={} align={} live_before={}",
            request.bytes, request.align, request.live_before
        ));
    }
}
fn two_byte_copy_positions(costs: &[Cost]) -> [(usize, &'static str); 2] {
    // Anchor to the public first Form event, never to an output/URI debit.
    // Its fixed suffix is: insertion; four tail bytes and four span advances;
    // UTF-8 completion; Finish (which returns Ready). Before insertion there
    // are seven shifted "//value" bytes and one shift-end transition. Padding
    // precedes those eight actions. See the independent fixture arithmetic in
    // the README. A changed action schedule must update this fixture oracle.
    let ready = costs.iter().position(|cost| cost.form_ready).unwrap();
    let insertion = ready.checked_sub("?x#f".len() + 4 + 1 + 1).unwrap();
    let padding = insertion.checked_sub("//value".len() + 1 + 1).unwrap();
    [(padding, "padding"), (insertion, "insertion")]
}
fn sweep(
    t: &Thing,
    limits: &ResourceLimits,
    source: usize,
    expected: &Run,
    baseline: heap::Trace,
    costs: &[Cost],
) {
    let two_byte_copies = two_byte_copy_positions(costs);
    for position in 0..expected.inspect {
        for cancel in [false, true] {
            heap::begin(0);
            let mut c = inspect_prefix(t, limits, source, position);
            let before = heap::trace();
            for _ in 0..2 {
                c = match c.step(&mut WorkBudget::new(), false) {
                    Progress::Pending(c) => c,
                    _ => panic!("zero credit advanced inspection"),
                };
                assert_eq!(heap::trace(), before);
            }
            if cancel {
                assert!(matches!(
                    c.step(&mut WorkBudget::new(), true),
                    Progress::Failed(Cause::Cancelled { .. })
                ));
            } else {
                drop(c);
            }
            let released = heap::end();
            assert_eq!(released.attempts, before.attempts);
            assert_eq!(heap::source_live(), source);
        }
    }
    let mut shortages = 0;
    for position in 0..=expected.semantics {
        for cancel in [false, true] {
            heap::begin(0);
            let (mut c, _) = semantic_prefix(t, limits, source, position);
            let before = heap::trace();
            if cancel && position < expected.semantics {
                let cause = c.step(&mut WorkBudget::new(), true).err().unwrap();
                assert_eq!(
                    cause,
                    Cause::Cancelled {
                        phase: Phase::Semantics
                    }
                );
                assert_eq!(c.step(&mut budget(), false).err(), Some(cause));
                c.acknowledge();
                c = c.rewind();
                assert_eq!(c.step(&mut budget(), true).err(), Some(cause));
            } else if position == expected.semantics {
                assert!(matches!(
                    c.step(&mut WorkBudget::new(), true),
                    Ok(Step::Done)
                ));
            }
            drop(c);
            assert_eq!(heap::end().attempts, before.attempts);
            assert_eq!(heap::source_live(), source);
        }
        if position == expected.semantics {
            continue;
        }
        if costs[position].ready {
            // Include the state with a Ready event still unacknowledged, in
            // addition to the ordinary driver's post-acknowledgement states.
            for cancel in [false, true] {
                heap::begin(0);
                let (mut c, _) = semantic_prefix(t, limits, source, position);
                assert!(matches!(c.step(&mut budget(), false), Ok(Step::Ready(_))));
                let before = heap::trace();
                if cancel {
                    assert_eq!(
                        c.step(&mut WorkBudget::new(), true).err(),
                        Some(Cause::Cancelled {
                            phase: Phase::Semantics
                        })
                    );
                }
                drop(c);
                assert_eq!(heap::end().attempts, before.attempts);
                assert_eq!(heap::source_live(), source);
            }
        }
        // Select every position independently of observed debits. Even a
        // target-specific missing/zero charge must encounter read-only URI
        // storage at zero output credit, or inaccessible source/derived bytes
        // at zero URI credit. New blocks join the active guard before return.
        for class in [W::CodecOutputBytes, W::UriBytes] {
            heap::begin(0);
            let (mut c, mut run) = semantic_prefix(t, limits, source, position);
            let mut done = false;
            for _ in 0..3 {
                let mut zero = budget().with_remaining(class, 0);
                let initial = W::ALL.map(|w| zero.remaining(w));
                let before = heap::trace();
                let result = {
                    let uri = class == W::UriBytes;
                    let _guard = Guard::new(
                        heap::byte_regions(uri),
                        if uri {
                            Permission::None
                        } else {
                            Permission::ReadOnly
                        },
                    );
                    c.step(&mut zero, false).unwrap()
                };
                assert_eq!(zero.remaining(class), 0);
                let debit = core::array::from_fn(|i| initial[i] - zero.remaining(W::ALL[i]));
                let result = observe(result); // caller bytes after protection ends
                if debit == [0; CLASSES] {
                    assert!(matches!(result, Observation::Pending));
                    assert_eq!(heap::trace(), before);
                } else {
                    // Actions belonging only to other classes may progress.
                    // Record them once; rejected zero-debit polls contribute
                    // nothing. Full replay detects unpaid scalar state changes.
                    done = record_poll(&mut c, &mut run, debit, result).unwrap().0;
                    break;
                }
            }
            if !done {
                finish(&mut c, &mut run).unwrap();
            }
            drop(c);
            assert_eq!(&run, expected);
            assert_eq!(heap::end(), baseline);
        }
        let copy_repair = two_byte_copies
            .iter()
            .find(|(index, _)| *index == position)
            .map(|(_, name)| *name);
        for (class, required) in [
            (W::UriBytes, costs[position].uri),
            (
                W::CodecOutputBytes,
                if copy_repair.is_some() {
                    2 // fixture-owned requirement, even if production reports 0/1
                } else {
                    costs[position].output
                },
            ),
        ] {
            if required == 0 {
                continue;
            }
            heap::begin(0);
            let (mut c, mut run) = semantic_prefix(t, limits, source, position);
            let before = heap::trace();
            let repair = copy_repair.filter(|_| class == W::CodecOutputBytes);
            if let Some(name) = repair {
                report(format_args!(
                    "two-byte copy predebit: action={name} position={position} output_credit=1"
                ));
            }
            for _ in 0..3 {
                let mut short = budget().with_remaining(class, required - 1);
                let unchanged = W::ALL.map(|w| short.remaining(w));
                {
                    let _guard = Guard::new(heap::byte_regions(false), Permission::ReadOnly);
                    assert!(matches!(c.step(&mut short, false), Ok(Step::Pending)));
                }
                assert_eq!(W::ALL.map(|w| short.remaining(w)), unchanged); // no partial multiclass debit
                assert_eq!(heap::trace(), before);
            }
            if repair.is_some() {
                let mut exact = budget().with_remaining(W::CodecOutputBytes, 2);
                let initial = W::ALL.map(|w| exact.remaining(w));
                let result = observe(c.step(&mut exact, false).unwrap());
                assert!(matches!(result, Observation::Pending));
                let debit = core::array::from_fn(|i| initial[i] - exact.remaining(W::ALL[i]));
                assert_eq!(debit[W::CodecOutputBytes as usize], 2);
                assert_eq!(debit[W::UriBytes as usize], 2);
                assert!(!record_poll(&mut c, &mut run, debit, result).unwrap().0);
            }
            finish(&mut c, &mut run).unwrap();
            drop(c);
            assert_eq!(&run, expected); // full remaining semantics and work trace, not only counters
            assert_eq!(heap::end(), baseline);
            shortages += 1;
        }
    }
    failures(t, limits, source, baseline);
    report(format_args!(
        "lifecycle: inspect_positions={} semantic_positions={} ready_positions={} short_actions={} independent_zero_output_and_uri_probes={} independent_two_byte_copy_actions={} failures={} terminal_td_bytes=0",
        expected.inspect,
        expected.semantics + 1,
        costs.iter().filter(|cost| cost.ready).count(),
        shortages,
        expected.semantics * 2,
        two_byte_copies.len(),
        baseline.attempts
    ));
}
fn ready_accesses(limits: &ResourceLimits) {
    let mut t: Thing = serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"guard","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},"properties":{"guard":{"type":"null","forms":[{"href":"/p"}]}}}"#).unwrap();
    // Isolate source bytes and scope String descriptors, not the Form itself:
    // constructing its fixed loan is allowed; scanning bytes or scope entries
    // is forbidden. The allocator owns all page padding in its charged backing.
    t.base = Some(heap::isolated_source(|| {
        clinkz_wot_td::data_type::BaseUri::parse("foo:/a/b/").unwrap()
    }));
    let f = &mut t
        .properties
        .as_mut()
        .unwrap()
        .get_mut("guard")
        .unwrap()
        ._interaction
        .forms[0];
    f.href = heap::isolated_source(|| FormHref::parse("../ready").unwrap());
    f.scopes = Some(heap::isolated_source(|| {
        alloc::vec!["one".to_string(), "二".to_string()]
    }));
    heap::begin(0);
    let mut c = proof(cursor(&t, limits, 0), &mut Run::new())
        .unwrap()
        .into_property_read();
    let mut found = false;
    for _ in 0..POLLS {
        match c.step(&mut budget(), false).unwrap() {
            Step::Pending => {}
            Step::Ready(Event::Property { .. }) => c.acknowledge(),
            Step::Ready(Event::Form(_)) => {
                found = true;
                break;
            }
            Step::Done => panic!("missing guarded Form"),
        }
    }
    assert!(found);
    assert_eq!(heap::byte_regions(true).count(), 6);
    let before = heap::trace();
    let mut retries = 0;
    for _ in 0..3 {
        for mut b in [
            WorkBudget::new(),
            budget().with_remaining(W::UriBytes, 0),
            budget().with_remaining(W::CodecOutputBytes, 1),
            budget().with_remaining(W::CleanupItems, 0),
        ] {
            let remaining = W::ALL.map(|w| b.remaining(w));
            let event = {
                let _guard = Guard::new(heap::byte_regions(true), Permission::None);
                let Step::Ready(event) = c.step(&mut b, false).unwrap() else {
                    panic!("protected Ready changed phase");
                };
                event
            };
            let Event::Form(f) = event else {
                panic!("protected Form changed event");
            };
            // The caller traverses actual bytes only after access is restored.
            assert_eq!(f.resolved_href(), "foo:/a/ready");
            assert_eq!(f.href(), "../ready");
            let scopes = f.scopes();
            let mut iter = scopes.iter();
            assert_eq!(iter.next(), Some("one"));
            assert_eq!(iter.next(), Some("二"));
            assert_eq!(iter.next(), None);
            assert_eq!(scopes.byte_len(), 6);
            assert_eq!(W::ALL.map(|w| b.remaining(w)), remaining);
            assert_eq!(heap::trace(), before);
            retries += 1;
        }
    }
    c.acknowledge();
    drop(c);
    heap::end();
    drop(t);
    heap::assert_empty();
    report(format_args!(
        "ready access oracle: retries={retries} source_uri_scope_regions=5 derived_uri_regions=1 forbidden_reads=0"
    ));
}
fn failures(t: &Thing, limits: &ResourceLimits, source: usize, baseline: heap::Trace) {
    let live_source = heap::source_live();
    for failure in 1..=baseline.attempts {
        heap::begin(failure);
        let result = complete(t, limits, source);
        let trace = heap::end();
        let Err(Cause::Failed(cause)) = result else {
            panic!("hidden allocation failure");
        };
        assert_eq!(cause.kind(), ValidatedThingFailureKind::AllocationFailed);
        assert_eq!(
            cause.requested_bytes(),
            baseline.requests[failure - 1].bytes as u64
        );
        assert_eq!(trace.attempts, failure);
        assert_eq!(&trace.requests[..failure], &baseline.requests[..failure]);
        assert_eq!(heap::source_live(), live_source);
    }
}
fn lifetime(t: &Thing, limits: &ResourceLimits, source: usize, expected: &Run) {
    let total = expected.work.iter().sum::<u64>()
        - expected.work[W::CodecOutputBytes as usize]
        - expected.work[W::UriBytes as usize]
        + expected.uri_writes
        + expected.uri_work;
    let pass_work = total - expected.inspection_work;
    // The gateway's admitted atomic iterator envelope can exceed this small
    // input's first-pass cost. Reach an exact boundary with additional complete
    // semantic passes, preserving all named structural/URI/Number ceilings.
    let mut exact_passes = 1;
    let exact_total = loop {
        let n = expected.inspection_work + pass_work * exact_passes;
        let policy = limits
            .clone()
            .with_limit(R::DocumentValidationWorkUnitsMax, Some(n - 1));
        if Config::try_from_limits(&policy).is_ok() {
            break n;
        }
        exact_passes += 1;
        assert!(exact_passes < 100);
    };
    for ceiling in [exact_total - 1, exact_total] {
        let policy = limits
            .clone()
            .with_limit(R::DocumentValidationWorkUnitsMax, Some(ceiling));
        heap::begin(0);
        let mut run = Run::new();
        let mut c = proof(cursor(t, &policy, source), &mut run)
            .unwrap()
            .into_property_read();
        let mut result = Ok(());
        for pass in 0..exact_passes {
            if pass != 0 {
                c = c.rewind();
            }
            result = finish(&mut c, &mut run);
            if result.is_err() {
                break;
            }
        }
        drop(c);
        heap::end();
        if ceiling == exact_total {
            assert_eq!(result, Ok(()));
            assert_eq!(run.work.iter().sum::<u64>(), exact_total);
            assert_eq!(run.forms as u64, expected.forms as u64 * exact_passes);
        } else {
            assert!(
                matches!(result, Err(Cause::Limit(l)) if l.kind() == R::DocumentValidationWorkUnitsMax)
            );
        }
    }
    heap::begin(0);
    let (mut c, mut run) = semantic_prefix(t, limits, source, expected.semantics);
    let mut passes = 1;
    loop {
        c = c.rewind();
        match finish(&mut c, &mut run) {
            Ok(()) => passes += 1,
            Err(Cause::Limit(l)) if l.kind() == R::DocumentValidationWorkUnitsMax => {
                let cause = Cause::Limit(l);
                assert_eq!(c.step(&mut WorkBudget::new(), true).err(), Some(cause));
                break;
            }
            other => panic!("unexpected rewind result {other:?}"),
        }
        assert!(passes < 10_000, "rewind replenished lifetime allowance");
    }
    drop(c);
    heap::end();
    report(format_args!(
        "work: first_pass={total} classes={:?} independent_uri_writes={} independent_uri_work={} exact_passes={exact_passes} exact_total={exact_total} completed_passes_before_limit={passes}",
        expected.work, expected.uri_writes, expected.uri_work
    ));
}
fn named_uri(limits: &ResourceLimits) {
    let mut t: Thing = serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"long","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},"properties":{"p":{"type":"null","forms":[{"href":"/p"}]}}}"#).unwrap();
    let prefix = "http://a/";
    let maximum = limits.get(R::UriTemplateSourceBytesMax).unwrap() as usize;
    t.base = Some(heap::isolated_source(|| {
        clinkz_wot_td::data_type::BaseUri::parse(prefix).unwrap()
    }));
    let href = "a".repeat(maximum - prefix.len());
    t.properties
        .as_mut()
        .unwrap()
        .get_mut("p")
        .unwrap()
        ._interaction
        .forms[0]
        .href = heap::isolated_source(|| FormHref::parse(&href).unwrap());
    drop(href);
    heap::begin(0);
    let run = complete(&t, limits, 0).unwrap();
    assert_eq!((run.properties, run.forms), (1, 1));
    let trace = heap::end();
    physical_boundaries(&t, limits, 0, trace);
    failures(&t, limits, 0, trace);
    report(format_args!(
        "named_uri: bytes={maximum} independent_uri_writes={} independent_uri_work={} heap_peak={} largest={} terminal_td_bytes=0",
        run.uri_writes, run.uri_work, trace.peak, trace.largest
    ));
}
fn number_boundaries(limits: &ResourceLimits) {
    let maximum = limits.get(R::NumberLexemeBytesMax).unwrap() as usize;
    let mut t: Thing = serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"number","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}}}"#).unwrap();
    for n in [maximum - 1, maximum, maximum + 1] {
        let number: serde_json::Number = format!("1{}", "0".repeat(n - 1)).parse().unwrap();
        t._extra_fields
            .insert("opaque-number".to_string(), number.into());
        heap::begin(0);
        let mut run = Run::new();
        let result = proof(cursor(&t, limits, 0), &mut run);
        if n <= maximum {
            drop(result.unwrap());
        } else {
            assert!(matches!(result, Err(Cause::Limit(l)) if l.kind() == R::NumberLexemeBytesMax));
        }
        heap::end();
    }
    let zero = limits.clone().with_limit(R::NumberLexemeBytesMax, Some(0));
    heap::begin(0);
    assert!(matches!(proof(cursor(&t, &zero, 0), &mut Run::new()),
        Err(Cause::Limit(l)) if l.kind() == R::NumberLexemeBytesMax));
    heap::end();
    report(format_args!(
        "number: lexical_thresholds={}/{}/{} zero_disabled=true",
        maximum - 1,
        maximum,
        maximum + 1
    ));
}
pub fn run() {
    report(format_args!(
        "production TD runtime: pointer_bits={} async={}",
        usize::BITS,
        cfg!(feature = "async")
    ));
    report(format_args!(
        "owner Layouts: inspect={} progress={} proof={} lending={} allocator_backing={:?}",
        size_of::<Inspect<'_>>(),
        size_of::<Progress<'_>>(),
        size_of::<ValidatedThing<'_>>(),
        size_of::<Read<'_>>(),
        heap::owner_layout()
    ));
    // All fixed allocator backing, slots, observer capacity, padding and guards
    // are startup storage paid once. Delegated child requests are not added to
    // this physical backing a second time. This is not a Servient transaction.
    let mut startup = ResourceAccount::new(
        SlotIndex::new(0),
        Generation::INITIAL,
        R::EngineLiveBytesGlobalMax,
        BenchmarkStaticReferenceV1::LIMITS
            .get(R::EngineLiveBytesGlobalMax)
            .unwrap(),
    );
    startup
        .try_reserve(heap::owner_layout().size() as u64)
        .unwrap()
        .commit();
    // Heap child capacity delegates the already charged fixed TD region;
    // movement/return slots require their own additional inline provision.
    startup.try_reserve(inline_bytes()).unwrap().commit();
    crate::access::self_test();
    for limits in [BenchmarkStaticReferenceV1::LIMITS, GatewayDefaultV1::LIMITS] {
        let t = input();
        let source = heap::source_live();
        let mut upstream = ResourceAccount::new(
            SlotIndex::new(2),
            Generation::INITIAL,
            R::RetainedSourceBytesPerOwnerMax,
            source as u64,
        );
        upstream.try_reserve(source as u64).unwrap().commit();
        heap::begin(0);
        let mut expected = Run::new();
        let mut c = proof(cursor(&t, limits, source), &mut expected)
            .unwrap()
            .into_property_read();
        let mut costs = [Cost::default(); SEMANTIC_STEPS];
        loop {
            let index = expected.semantics;
            assert!(index < costs.len());
            let (done, cost) = semantic_poll(&mut c, &mut expected).unwrap();
            costs[index] = cost;
            if done {
                break;
            }
        }
        drop(c);
        let baseline = heap::end();
        assert_eq!((expected.properties, expected.forms), (2, 2));
        assert!(baseline.attempts >= 5 && baseline.peak > baseline.largest);
        physical_boundaries(&t, limits, source, baseline);
        // A no-upstream baseline makes no fictitious source reservation and
        // yields exactly the same controlled heap/work/facts as retained input.
        heap::begin(0);
        assert_eq!(complete(&t, limits, 0), Ok(expected.clone()));
        assert_eq!(heap::end(), baseline);
        sweep(
            &t,
            limits,
            source,
            &expected,
            baseline,
            &costs[..expected.semantics],
        );
        lifetime(&t, limits, source, &expected);
        assert_eq!(upstream.used(), source as u64);
        assert_eq!(heap::source_live(), source);
        drop(t); // physical source destruction belongs to its original owner
        assert_eq!(heap::source_live(), 0);
        assert!(upstream.release_committed(source as u64));
        assert_eq!(upstream.used(), 0);
        named_uri(limits);
        ready_accesses(limits);
        number_boundaries(limits);
        heap::assert_empty();
    }
    heap::assert_empty();
    assert!(startup.release_committed(inline_bytes()));
    assert_eq!(startup.used(), heap::owner_layout().size() as u64);
    report(format_args!(
        "PASS: production TD runtime; td_live=0 source_live=0 startup_backing_charge={}",
        startup.used()
    ));
}
