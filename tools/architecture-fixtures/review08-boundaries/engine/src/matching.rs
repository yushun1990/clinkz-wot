//! Companion matching/fan-out sketch. No protocol pattern or packet inspection.
use review08_core::{Authority, Gate, MatchedCandidate, RouteId};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

struct Record<E> {
    gate: Gate,
    principal: u64,
    handler: fn(&E) -> bool,
    busy: Rc<Cell<bool>>,
}
pub struct EventEngine<C: MatchedCandidate> {
    authority: Authority,
    records: BTreeMap<RouteId, Record<C::Event>>,
    generations: BTreeMap<u32, u32>,
    grants: BTreeSet<(u64, u32)>,
}
impl<C: MatchedCandidate> Default for EventEngine<C> {
    fn default() -> Self {
        Self {
            authority: Authority::default(),
            records: BTreeMap::new(),
            generations: BTreeMap::new(),
            grants: BTreeSet::new(),
        }
    }
}
impl<C: MatchedCandidate> EventEngine<C> {
    pub fn grant(&mut self, principal: u64, slot: u32) {
        self.grants.insert((principal, slot));
    }
    pub fn revoke(&mut self, principal: u64, slot: u32) {
        self.grants.remove(&(principal, slot));
    }
    /// Pure registration. The composition caller installs the returned read-only
    /// token and native pattern in its Binding before publishing this record.
    pub fn register(&mut self, slot: u32, principal: u64, handler: fn(&C::Event) -> bool) -> Gate {
        assert!(self.records.len() < 2 && !self.records.keys().any(|r| r.slot == slot));
        assert!(self.grants.contains(&(principal, slot)));
        let generation = self.generations.entry(slot).or_default();
        *generation = generation.checked_add(1).unwrap();
        let id = RouteId {
            slot,
            generation: *generation,
        };
        let gate = self.authority.gate(id);
        self.records.insert(
            id,
            Record {
                gate: gate.clone(),
                principal,
                handler,
                busy: Rc::new(Cell::new(false)),
            },
        );
        gate
    }
    pub fn publish(&self, gates: &[Gate]) {
        assert!(gates.iter().all(|g| {
            self.records
                .get(&g.route_id())
                .is_some_and(|r| r.gate.same_registration(g))
        }));
        for gate in gates {
            self.authority.set_serving(gate, true);
        }
    }
    /// One independent delivery credit per recipient. Refusal preserves its
    /// candidate; neither rejection nor cancellation consumes a sibling's data.
    pub fn accept(&self, candidate: C) -> Result<EventDelivery<C::Event>, C> {
        let id = candidate.registration().route_id();
        let Some(record) = self.records.get(&id) else {
            return Err(candidate);
        };
        if !record.gate.same_registration(candidate.registration())
            || record.busy.get()
            || !self.grants.contains(&(record.principal, id.slot))
        {
            return Err(candidate);
        }
        record.busy.set(true);
        let mut retained = Some(candidate);
        let accepted = self
            .authority
            .with_permit(&record.gate, |p| retained.take().unwrap().accept(p));
        match accepted {
            Some(Ok(event)) => Ok(EventDelivery {
                event,
                handler: record.handler,
                busy: record.busy.clone(),
            }),
            other => {
                record.busy.set(false);
                Err(match other {
                    Some(Err(candidate)) => candidate,
                    None => retained.unwrap(),
                    _ => unreachable!(),
                })
            }
        }
    }
    /// Revokes before the caller removes the exact native registration token.
    pub fn stop(&mut self, gate: &Gate) {
        let record = self.records.get(&gate.route_id()).unwrap();
        assert!(record.gate.same_registration(gate));
        self.authority.set_serving(gate, false);
        self.records.remove(&gate.route_id());
    }
}

/// Engine-selected handler and shared packet owner remain together after accept.
/// Drop cancels only this recipient; accepted sibling delivery can still run.
pub struct EventDelivery<E> {
    event: E,
    handler: fn(&E) -> bool,
    busy: Rc<Cell<bool>>,
}
impl<E> EventDelivery<E> {
    pub fn event(&self) -> &E {
        &self.event
    }
    pub fn run(self) -> bool {
        (self.handler)(&self.event)
    }
}
impl<E> Drop for EventDelivery<E> {
    fn drop(&mut self) {
        self.busy.set(false);
    }
}
