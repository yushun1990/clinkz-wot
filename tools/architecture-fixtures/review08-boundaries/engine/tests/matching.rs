use review08_core::{Authority, Gate, MatchedCandidate};
use review08_engine::matching::EventEngine;
use review08_third_party_binding::matching::{Candidate, Event, Matcher};

fn candidates(matcher: &mut Matcher) -> (Candidate, Candidate) {
    let mut batch = matcher
        .candidates("meters/m1/watts", b"42")
        .unwrap()
        .into_iter();
    let a = batch.next().unwrap();
    let b = batch.next().unwrap();
    assert!(batch.next().is_none());
    assert!(a.shares_packet(&b));
    (a, b)
}
fn register(
    engine: &mut EventEngine<Candidate>,
    matcher: &mut Matcher,
    filters: [&str; 2],
    handlers: [fn(&Event) -> bool; 2],
) -> [Gate; 2] {
    engine.grant(10, 1);
    engine.grant(20, 2);
    let a = engine.register(1, 10, handlers[0]);
    let b = engine.register(2, 20, handlers[1]);
    matcher.install(a.clone(), filters[0]).unwrap();
    matcher.install(b.clone(), filters[1]).unwrap();
    [a, b]
}

#[test]
fn overlapping_and_identical_filters_keep_independent_authority_and_stop() {
    for (filters, captures) in [
        (["meters/+/watts", "meters/m1/#"], ["m1", "watts"]),
        (["meters/+/watts", "meters/+/watts"], ["m1", "m1"]),
    ] {
        let mut matcher = Matcher::default();
        let mut engine = EventEngine::<Candidate>::default();
        let gates = register(
            &mut engine,
            &mut matcher,
            filters,
            [
                |_| panic!("a revoked or cancelled recipient must not invoke its handler"),
                |event| {
                    assert_eq!(event.payload(), b"42");
                    true
                },
            ],
        );
        assert!(matcher.candidates("other/topic", b"42").unwrap().is_empty());
        let (a, b) = candidates(&mut matcher);
        assert_eq!(a.registration().route_id(), gates[0].route_id());
        assert_eq!(b.registration().route_id(), gates[1].route_id());
        assert_eq!(a.event().captures()[0].as_ref(), captures[0]);
        assert_eq!(b.event().captures()[0].as_ref(), captures[1]);
        // Native matching is possible before publication; acceptance is not.
        assert!(engine.accept(a).is_err());
        assert!(engine.accept(b).is_err());
        engine.publish(&gates);

        let (a, b) = candidates(&mut matcher);
        let packet = a.packet_lease();
        engine.revoke(10, 1); // Check CURRENT authority after matching/queuing.
        drop(engine.accept(a).err().unwrap());
        let accepted_b = engine.accept(b).ok().unwrap();
        assert_eq!(accepted_b.event().captures()[0].as_ref(), captures[1]);
        assert!(packet.upgrade().is_some());
        assert!(accepted_b.run());
        assert!(packet.upgrade().is_none());

        engine.grant(10, 1);
        let (a, b) = candidates(&mut matcher);
        let packet = a.packet_lease();
        let accepted_a = engine.accept(a).ok().unwrap();
        let accepted_b = engine.accept(b).ok().unwrap();
        drop(accepted_a); // Cancels A alone, even for an identical filter.
        assert!(packet.upgrade().is_some());
        assert!(accepted_b.run());
        assert!(packet.upgrade().is_none());

        engine.stop(&gates[0]);
        matcher.remove(&gates[0]);
        let mut survivors = matcher.candidates("meters/m1/watts", b"42").unwrap();
        assert_eq!(survivors.len(), 1);
        let b = survivors.pop().unwrap();
        assert_eq!(b.registration().route_id(), gates[1].route_id());
        assert!(engine.accept(b).ok().unwrap().run());
        engine.stop(&gates[1]);
        matcher.remove(&gates[1]);
        assert!(
            matcher
                .candidates("meters/m1/watts", b"42")
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn fanout_bounds_delivery_per_recipient_and_pins_queued_generations() {
    let mut matcher = Matcher::default();
    let mut engine = EventEngine::<Candidate>::default();
    let gates = register(
        &mut engine,
        &mut matcher,
        ["meters/+/watts", "meters/m1/#"],
        [
            |event| event.captures()[0].as_ref() == "m1",
            |event| event.topic() == "meters/m1/watts",
        ],
    );
    engine.publish(&gates);
    assert!(
        matcher.install(gates[0].clone(), "meters/#").is_err(),
        "logical recipient bound"
    );

    let (a, b) = candidates(&mut matcher);
    let first = a.packet_lease();
    let slow_a = engine.accept(a).ok().unwrap();
    assert!(engine.accept(b).ok().unwrap().run());
    let (a, b) = candidates(&mut matcher);
    let second = a.packet_lease();
    drop(engine.accept(a).err().unwrap()); // A's one credit is still occupied.
    let fast_b = engine.accept(b).ok().unwrap();
    assert!(
        matcher.candidates("meters/m1/watts", b"42").is_err(),
        "physical backing bound"
    );
    assert!(fast_b.run(), "B progresses while A remains pending");
    assert!(second.upgrade().is_none());

    let (queued_a, queued_b) = candidates(&mut matcher);
    engine.stop(&gates[0]);
    matcher.remove(&gates[0]);
    let replacement = engine.register(1, 10, |_| false);
    assert_eq!(replacement.route_id().slot, gates[0].route_id().slot);
    assert!(replacement.route_id().generation > gates[0].route_id().generation);
    matcher
        .install(replacement.clone(), "meters/+/watts")
        .unwrap();
    engine.publish(std::slice::from_ref(&replacement));
    assert!(
        engine.accept(queued_a).is_err(),
        "never retarget an old matching result"
    );
    assert!(engine.accept(queued_b).ok().unwrap().run());
    assert!(
        slow_a.run(),
        "accepted work retains its OLD handler through stop"
    );
    assert!(first.upgrade().is_none());

    let foreign_root = Authority::default();
    let foreign_gate = foreign_root.gate(gates[1].route_id());
    foreign_root.set_serving(&foreign_gate, true);
    let mut foreign_matcher = Matcher::default();
    foreign_matcher.install(foreign_gate, "meters/#").unwrap();
    let forged = foreign_matcher
        .candidates("meters/m1/watts", b"42")
        .unwrap()
        .pop()
        .unwrap();
    assert!(
        engine.accept(forged).is_err(),
        "numeric identity cannot impersonate an issuer"
    );
    engine.stop(&replacement);
    matcher.remove(&replacement);
    engine.stop(&gates[1]);
    matcher.remove(&gates[1]);
}
