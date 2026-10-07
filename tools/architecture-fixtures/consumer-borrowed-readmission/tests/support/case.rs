use clinkz_wot_foundation::{
    AdmissionLedger, Generation, ResourceKind as R, ResourceLimits, SlotIndex, WorkBudget,
    WorkClass as W,
};
use consumer_borrowed_readmission_probe::{td::*, thing::Thing};
pub fn budget(n: u64) -> WorkBudget {
    let mut b = WorkBudget::new();
    for c in W::ALL {
        b.set_remaining(c, n);
    }
    b
}
pub fn ledger(l: &ResourceLimits) -> AdmissionLedger {
    let heap = ValidatedThingAdmissionConfig::try_from_limits(l)
        .unwrap()
        .controlled_heap_envelope();
    AdmissionLedger::new(
        SlotIndex::new(0),
        Generation::INITIAL,
        0,
        l.get(R::AdmissionTemporaryBytesPerOperationMax)
            .unwrap()
            .min(heap),
        0,
        0,
        0,
        0,
    )
}
pub fn validate<'a>(
    t: &'a Thing,
    l: &ResourceLimits,
    q: u64,
) -> Result<ValidatedThing<'a>, ValidatedThingCause> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(l).unwrap();
    let mut c = ValidatedThingCursor::from_thing(t, &config, ledger(l));
    for _ in 0..1_000_000 {
        match c.step(&mut budget(q), false) {
            ValidatedThingProgress::Pending(n) => c = n,
            ValidatedThingProgress::Complete(v) => return Ok(v),
            ValidatedThingProgress::Failed(e) => return Err(e),
        }
    }
    panic!("stalled")
}
pub fn source() -> Thing {
    serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"probe","id":"urn:probe","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},"properties":{"p":{"type":"null","forms":[{"href":"target"}]}}}"#).unwrap()
}
pub fn normalization_source(base_segment: usize) -> Thing {
    use consumer_borrowed_readmission_probe::data_type::{BaseUri, FormHref};
    let mut t = source();
    t.base = Some(BaseUri::parse(&format!("https://h/{}/", "a".repeat(base_segment))).unwrap());
    t.properties
        .as_mut()
        .unwrap()
        .get_mut("p")
        .unwrap()
        ._interaction
        .forms[0]
        .href = FormHref::parse("bbbbbbbbbbbbbbbbbbbb/../../x").unwrap();
    t
}

pub fn drive(
    read: &mut ValidatedPropertyReadCursor<'_>,
    q: u64,
) -> Result<(), ValidatedThingCause> {
    for _ in 0..1_000_000 {
        match read.step(&mut budget(q), false)? {
            ValidatedPropertyReadStep::Ready(_) => read.acknowledge(),
            ValidatedPropertyReadStep::Pending => {}
            ValidatedPropertyReadStep::Done => return Ok(()),
        }
    }
    panic!("semantic stalled")
}
