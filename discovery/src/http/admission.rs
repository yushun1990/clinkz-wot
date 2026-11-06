//! Wire bounds/ambiguity checks precede existing bounded TD semantic admission.
use super::*;
use clinkz_wot_foundation::*;
use clinkz_wot_td::{
    ValidatedThingAdmissionConfig, ValidatedThingCause, ValidatedThingCursor,
    ValidatedThingProgress,
};
use serde::{
    Deserialize,
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
};
use std::{collections::BTreeSet, fmt};

// This scanner checks JSON ambiguity and a finite allocation/traversal envelope;
// it does not interpret TD vocabulary. Ordinary typed TD decoding remains the
// only field decoder. Exact-number TD behavior is inherited from that decoder.
struct Scan<'a> {
    nodes: &'a mut usize,
    depth: usize,
}
impl<'de> DeserializeSeed<'de> for Scan<'_> {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> core::result::Result<(), D::Error> {
        if *self.nodes == 0 || self.depth > 32 {
            return Err(de::Error::custom("JSON limit"));
        }
        *self.nodes -= 1;
        d.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Scan<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded JSON")
    }
    fn visit_bool<X: de::Error>(self, _: bool) -> core::result::Result<(), X> {
        Ok(())
    }
    fn visit_unit<X: de::Error>(self) -> core::result::Result<(), X> {
        Ok(())
    }
    fn visit_i64<X: de::Error>(self, _: i64) -> core::result::Result<(), X> {
        Ok(())
    }
    fn visit_u64<X: de::Error>(self, _: u64) -> core::result::Result<(), X> {
        Ok(())
    }
    fn visit_f64<X: de::Error>(self, _: f64) -> core::result::Result<(), X> {
        Ok(())
    }
    fn visit_str<X: de::Error>(self, _: &str) -> core::result::Result<(), X> {
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> core::result::Result<(), A::Error> {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if keys.len() >= 1024 || !keys.insert(key) {
                return Err(de::Error::custom("duplicate or excess member"));
            }
            map.next_value_seed(Scan {
                nodes: self.nodes,
                depth: self.depth + 1,
            })?;
        }
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> core::result::Result<(), A::Error> {
        while seq
            .next_element_seed(Scan {
                nodes: self.nodes,
                depth: self.depth + 1,
            })?
            .is_some()
        {}
        Ok(())
    }
}
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, 1_000_000))
}
async fn admit(thing: &Thing, cancel: &DiscoveryCancellation) -> Result<()> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS)
        .map_err(|_| E::Configuration)?;
    let mut cursor = ValidatedThingCursor::from_thing(
        thing,
        &config,
        AdmissionLedger::new(
            SlotIndex::new(0),
            Generation::INITIAL,
            0,
            8 << 20,
            0,
            0,
            0,
            0,
        ),
    );
    // One nonrenewable work allowance per TD. The cursor also enforces its
    // inherited document-lifetime ceilings. Yield between paid bounded steps.
    let mut budget = work();
    loop {
        if cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        let before: u64 = WorkClass::ALL
            .into_iter()
            .map(|k| budget.remaining(k))
            .sum();
        cursor = match cursor.step(&mut budget, false) {
            ValidatedThingProgress::Complete(proof) => {
                drop(proof);
                return Ok(());
            }
            ValidatedThingProgress::Pending(next) => next,
            ValidatedThingProgress::Failed(cause) => {
                return Err(match cause {
                    ValidatedThingCause::Invalid(_) => E::InvalidThingDescription,
                    _ => E::AdmissionLimit,
                });
            }
        };
        if WorkClass::ALL
            .into_iter()
            .map(|k| budget.remaining(k))
            .sum::<u64>()
            == before
        {
            return Err(E::AdmissionLimit);
        }
        tokio::task::yield_now().await;
    }
}
pub(super) async fn page(
    body: &[u8],
    directory: bool,
    limits: DiscoveryLimits,
    cancel: &DiscoveryCancellation,
) -> Result<Vec<DiscoveredThing>> {
    let text = std::str::from_utf8(body).map_err(|_| E::InvalidJson)?;
    let mut de = serde_json::Deserializer::from_str(text);
    Scan {
        nodes: &mut 8192,
        depth: 0,
    }
    .deserialize(&mut de)
    .map_err(|_| E::InvalidJson)?;
    de.end().map_err(|_| E::InvalidJson)?;
    let raw: Vec<&serde_json::value::RawValue> = if directory {
        serde_json::from_str(text).map_err(|_| E::InvalidJson)?
    } else {
        vec![serde_json::from_str(text).map_err(|_| E::InvalidJson)?]
    };
    if raw.len() > limits.max_results {
        return Err(E::Limit);
    }
    let mut output = Vec::with_capacity(raw.len());
    for raw in raw {
        if cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        if raw.get().len() > limits.td_bytes {
            return Err(E::Limit);
        }
        let thing: Thing = Thing::deserialize(&mut serde_json::Deserializer::from_str(raw.get()))
            .map_err(|_| E::InvalidThingDescription)?;
        admit(&thing, cancel).await?;
        output.push(DiscoveredThing { thing });
    }
    Ok(output)
}
