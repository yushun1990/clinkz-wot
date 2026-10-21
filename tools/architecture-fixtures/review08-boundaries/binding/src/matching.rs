//! A bounded native topic matcher, not an MQTT client, broker or subscription SPI.
//! Filters/packet parsing and captures stay here, outside the Engine crate.
use review08_core::{AcceptPermit, Gate, MatchedCandidate};
use std::rc::{Rc, Weak};

const REGISTRATIONS: usize = 2;
const PACKETS: usize = 2;
const BYTES: usize = 32;
const LEVELS: usize = 4;

struct Registration {
    gate: Gate,
    levels: Vec<Box<str>>,
}

#[derive(Default)]
pub struct Matcher {
    registrations: Vec<Registration>,
    packets: Vec<Weak<Packet>>,
}

impl Matcher {
    /// Engine supplies the exact logical registration token. Identical native
    /// filters do not collapse independently authorized recipients.
    pub fn install(&mut self, gate: Gate, filter: &str) -> Result<(), &'static str> {
        if self.registrations.len() == REGISTRATIONS
            || self
                .registrations
                .iter()
                .any(|r| r.gate.same_registration(&gate))
            || filter.is_empty()
            || filter.len() > BYTES
            || filter.contains('\0')
        {
            return Err("registration capacity/identity/filter");
        }
        let levels: Vec<_> = filter.split('/').collect();
        if levels.len() > LEVELS
            || levels.iter().enumerate().any(|(i, l)| {
                (l.contains('+') && *l != "+")
                    || (l.contains('#') && (*l != "#" || i + 1 != levels.len()))
            })
        {
            return Err("unsupported filter");
        }
        self.registrations.push(Registration {
            gate,
            levels: levels.into_iter().map(Into::into).collect(),
        });
        Ok(())
    }
    /// Remove this logical participant, never every registration of its filter.
    pub fn remove(&mut self, gate: &Gate) {
        self.registrations
            .retain(|r| !r.gate.same_registration(gate));
    }
    /// Explicitly bounded stand-in for ONE decoded native packet, not wire I/O.
    /// Two packet backings let B progress while A retains one accepted event.
    pub fn candidates(
        &mut self,
        topic: &str,
        payload: &[u8],
    ) -> Result<Vec<Candidate>, &'static str> {
        self.packets.retain(|p| p.strong_count() != 0);
        if self.packets.len() == PACKETS {
            return Err("packet backpressure");
        }
        if topic.is_empty()
            || topic.len() > BYTES
            || payload.len() > BYTES
            || topic.contains(['\0', '+', '#'])
            || topic.split('/').count() > LEVELS
        {
            return Err("unsupported packet");
        }
        let packet = Rc::new(Packet {
            topic: topic.into(),
            payload: payload.into(),
        });
        self.packets.push(Rc::downgrade(&packet));
        let mut candidates = Vec::new();
        for registration in &self.registrations {
            if let Some(captures) = registration.captures(topic) {
                candidates.push(Candidate {
                    gate: registration.gate.clone(),
                    event: Event {
                        packet: packet.clone(),
                        captures,
                    },
                });
            }
        }
        Ok(candidates)
    }
}

impl Registration {
    fn captures(&self, topic: &str) -> Option<Vec<Box<str>>> {
        if topic.starts_with('$') && matches!(self.levels[0].as_ref(), "+" | "#") {
            return None;
        }
        let parts: Vec<_> = topic.split('/').collect();
        let mut captures = Vec::new();
        for (i, level) in self.levels.iter().enumerate() {
            match level.as_ref() {
                "#" => {
                    captures.push(parts.get(i..).unwrap_or_default().join("/").into());
                    return Some(captures);
                }
                "+" => captures.push((*parts.get(i)?).into()),
                literal if Some(&literal) != parts.get(i) => return None,
                _ => {}
            }
        }
        (parts.len() == self.levels.len()).then_some(captures)
    }
}

pub struct Packet {
    topic: Box<str>,
    payload: Box<[u8]>,
}
pub struct Event {
    packet: Rc<Packet>,
    captures: Vec<Box<str>>,
}
impl Event {
    pub fn captures(&self) -> &[Box<str>] {
        &self.captures
    }
    pub fn topic(&self) -> &str {
        &self.packet.topic
    }
    pub fn payload(&self) -> &[u8] {
        &self.packet.payload
    }
}
pub struct Candidate {
    gate: Gate,
    event: Event,
}
impl Candidate {
    pub fn event(&self) -> &Event {
        &self.event
    }
    pub fn shares_packet(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.event.packet, &other.event.packet)
    }
    pub fn packet_lease(&self) -> Weak<Packet> {
        Rc::downgrade(&self.event.packet)
    }
}
impl MatchedCandidate for Candidate {
    type Event = Event;
    fn registration(&self) -> &Gate {
        &self.gate
    }
    fn accept(self, permit: AcceptPermit<'_>) -> Result<Event, Self> {
        if !permit.authorizes(&self.gate) {
            return Err(self);
        }
        Ok(self.event)
    }
}
