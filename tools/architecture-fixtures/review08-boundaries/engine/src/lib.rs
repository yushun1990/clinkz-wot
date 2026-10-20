//! Test-sized, single-thread Engine. Native selectors stay opaque here.
use review08_core::*;
use std::collections::{BTreeMap, BTreeSet};

struct Record<R> {
    route: R,
    gate: Gate,
    principal: u64,
    handler: fn() -> bool,
}
pub struct Engine<R> {
    authority: Authority,
    records: BTreeMap<RouteId, Record<R>>,
    endpoints: BTreeMap<Endpoint, RouteId>,
    generations: BTreeMap<u32, u32>,
    grants: BTreeSet<(u64, Endpoint)>,
    pub dispatched: Vec<RouteId>,
}
impl<R> Default for Engine<R> {
    fn default() -> Self {
        Self {
            authority: Authority::default(),
            records: BTreeMap::new(),
            endpoints: BTreeMap::new(),
            generations: BTreeMap::new(),
            grants: BTreeSet::new(),
            dispatched: Vec::new(),
        }
    }
}
impl<R: ReadRoute> Engine<R> {
    pub fn grant(&mut self, principal: u64, key: Endpoint) {
        self.grants.insert((principal, key));
    }
    pub fn revoke(&mut self, principal: u64, key: Endpoint) {
        self.grants.remove(&(principal, key));
    }
    /// Refusal returns the complete route, including any partial native owner.
    pub fn install(
        &mut self,
        slot: u32,
        principal: u64,
        mut route: R,
        handler: fn() -> bool,
    ) -> Result<RouteId, (&'static str, R)> {
        if self.records.len() == 2
            || self.records.keys().any(|id| id.slot == slot)
            || self.endpoints.contains_key(route.endpoint())
            || !self.grants.contains(&(principal, route.endpoint().clone()))
        {
            return Err(("engine admission/authorization", route));
        }
        let generation = self
            .generations
            .get(&slot)
            .copied()
            .unwrap_or(0)
            .checked_add(1);
        let Some(generation) = generation else {
            return Err(("generation exhausted", route));
        };
        let id = RouteId { slot, generation };
        let gate = self.authority.gate(id);
        if let Err(error) = route.install(gate.clone()) {
            return Err((error, route));
        }
        self.generations.insert(slot, generation);
        self.endpoints.insert(route.endpoint().clone(), id);
        self.records.insert(
            id,
            Record {
                route,
                gate,
                principal,
                handler,
            },
        );
        Ok(id)
    }
    /// No callback or yield between readiness checks and this local publication.
    pub fn publish(&self, ids: &[RouteId]) -> bool {
        if ids.iter().any(|id| !self.records.contains_key(id)) {
            return false;
        }
        for id in ids {
            self.authority.set_serving(&self.records[id].gate, true);
        }
        true
    }
    pub fn accept(&mut self, input: R::Observation) -> Option<Call<R::Delivery>> {
        let routed = self.route(input)?;
        self.dispatch(routed)
    }
    /// Capture the generation when Engine routing occurs; never retarget a queue.
    pub fn route(&self, input: R::Observation) -> Option<Routed<R::Observation>> {
        let id = *self.endpoints.get(input.endpoint())?;
        Some(Routed {
            id,
            gate: self.records.get(&id)?.gate.clone(),
            input,
        })
    }
    pub fn dispatch(&mut self, routed: Routed<R::Observation>) -> Option<Call<R::Delivery>> {
        let Routed { id, gate, input } = routed;
        let record = self.records.get_mut(&id)?;
        if !self
            .grants
            .contains(&(record.principal, input.endpoint().clone()))
        {
            return None;
        }
        let delivery = self
            .authority
            .with_permit(&gate, |permit| record.route.accept(permit, input))?
            .ok()?;
        // ONLY the Engine resolves and calls a handler. Binding receives no closure.
        self.dispatched.push(id);
        Some(Call {
            id,
            result: (record.handler)(),
            delivery: Some(delivery),
        })
    }
    pub fn stop(&mut self, id: RouteId) -> bool {
        let Some(mut record) = self.records.remove(&id) else {
            return false;
        };
        self.authority.set_serving(&record.gate, false);
        self.endpoints.remove(record.route.endpoint());
        record.route.close();
        true
    }
}

/// Privately minted routing decision; not yet accepted or dispatched.
pub struct Routed<I> {
    id: RouteId,
    gate: Gate,
    input: I,
}

pub struct Call<D> {
    pub id: RouteId,
    result: bool,
    delivery: Option<D>,
}
impl<D: ReadDelivery> Call<D> {
    pub fn poll_reply(&mut self) -> Result<bool, &'static str> {
        if let Some(delivery) = self.delivery.as_mut() {
            if !delivery.poll_reply(self.result)? {
                return Ok(false);
            }
            self.delivery.take();
        }
        Ok(true)
    }
}
