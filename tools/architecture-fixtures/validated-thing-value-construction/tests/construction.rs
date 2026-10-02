use clinkz_wot_foundation::WorkBudget;
use validated_thing_value_construction_probe::{Cause, Cursor, Kind, Limits, Phase, Progress};

#[path = "../../validated-thing-schema-kernel/tests/support/literal_value_reference.rs"]
mod literal_value_reference;

mod support;
use support::{budget, drive, equivalent};

#[test]
fn full_literal_construction_and_typed_path_agree_across_step_sizes() {
    let cases = [
        "{}",
        r#"{"x":null,"yes":true,"no":false,"array":[null,true,false,{},[],""],"empty":""}"#,
        r#"{"z":{"b":2,"a":1},"a":[{"x":false},"second",[3,4]],"utf8":"中文😀","escaped":"\u4e2d\u6587\ud83d\ude00\n\t\b\f\r\\\/\"\u0000"}"#,
        r#"{"x":{"$serde_json::private::Number":"1e309"},"title":{"$serde_json::private::RawValue":"17"},"const":{"$serde_json::private::RawValue":"[true,false]"},"nested":[{"$serde_json::private::RawValue":"true trailing"}],"literal":"[true,false]"}"#,
        r#"{"title":17,"ti\u0074le":"last","x":{"a":true,"\u0061":false},"type":"number","type":"integer","const":true,"const":null,"x":{"kept":"yes"}}"#,
        r#"{"x":{},"x":{"a":[1,2,3]},"x":true,"x":null,"x":{"final":[false,"ok"]}}"#,
        r#"{"negative":-0,"decimal":1.00,"number":1E0,"tiny":1e-999999,"large":18446744073709551616000,"opaque":1e309,"negative_exponent":-1E9}"#,
    ];
    for text in cases {
        let reference = literal_value_reference::literal(text).unwrap();
        let mut footprint = None;
        for size in [1, 2, 7, 64] {
            let strict =
                drive(Cursor::from_json(text.as_bytes(), Limits::default()), size).unwrap();
            let typed = drive(Cursor::from_value(&reference, Limits::default()), size).unwrap();
            equivalent(strict.view(), &reference);
            equivalent(typed.view(), &reference);
            assert_eq!(
                strict.footprint().retained_requested_bytes,
                typed.footprint().retained_requested_bytes
            );
            let current = strict.footprint().retained_requested_bytes;
            if let Some(previous) = footprint {
                assert_eq!(current, previous);
            } else {
                footprint = Some(current);
            }
            assert!(strict.trace().grow_copies > 0 || text == "{}");
            assert!(strict.trace().seal_copies > 0);
        }
    }
}

#[test]
fn source_order_is_erased_only_for_maps_and_discarded_occurrences() {
    let first = r#"{"z":{"y":[1,2],"a":"same"},"a":false}"#;
    let second = r#"{"a":false,"z":{"a":"same","y":[1,2]}}"#;
    let overwritten = r#"{"z":12345,"a":true,"z":{"y":null,"a":"same","y":[1,2]},"a":false}"#;
    let reference = literal_value_reference::literal(first).unwrap();
    let base = drive(Cursor::from_json(first.as_bytes(), Limits::default()), 1).unwrap();
    for text in [second, overwritten] {
        let result = drive(Cursor::from_json(text.as_bytes(), Limits::default()), 1).unwrap();
        equivalent(result.view(), &reference);
        assert_eq!(
            result.footprint().retained_requested_bytes,
            base.footprint().retained_requested_bytes
        );
        if text == overwritten {
            assert!(result.trace().discarded_nodes > 0);
        }
    }
    let reversed = drive(
        Cursor::from_json(
            br#"{"z":{"y":[2,1],"a":"same"},"a":false}"#,
            Limits::default(),
        ),
        1,
    )
    .unwrap();
    assert_eq!(
        reversed
            .view()
            .get("z")
            .unwrap()
            .get("y")
            .unwrap()
            .child(0)
            .unwrap()
            .text(),
        Some("2")
    );
    assert_eq!(
        base.view()
            .get("z")
            .unwrap()
            .get("y")
            .unwrap()
            .child(0)
            .unwrap()
            .text(),
        Some("1")
    );
}

#[test]
fn public_ap_number_content_is_differentially_constructed_without_owned_number_state() {
    let integers = [
        "0",
        "-0",
        "1",
        "-1",
        "18446744073709551615",
        "18446744073709551616",
        "-9223372036854775808",
        "-9223372036854775809",
    ];
    let fractions = ["", ".0", ".00", ".125", ".00000000000000001"];
    let exponents = [
        "",
        "e0",
        "E0",
        "e+0",
        "E-0",
        "e00001",
        "E+00001",
        "e-00001",
        "e309",
        "e-9999999999999999999999999999",
    ];
    for integer in integers {
        for fraction in fractions {
            for exponent in exponents {
                let token = format!("{integer}{fraction}{exponent}");
                let reference: serde_json::Number = serde_json::from_str(&token).unwrap();
                let text = format!(r#"{{"x":{token}}}"#);
                let output =
                    drive(Cursor::from_json(text.as_bytes(), Limits::default()), 2).unwrap();
                assert_eq!(
                    output.view().get("x").unwrap().text(),
                    Some(reference.as_str()),
                    "{token}"
                );
            }
        }
    }
}

#[test]
fn all_utf8_and_escape_bytes_are_resumable_and_invalid_units_are_rejected() {
    for value in [
        '\0',
        '\u{7f}',
        '\u{80}',
        '\u{7ff}',
        '\u{800}',
        '\u{d7ff}',
        '\u{e000}',
        '\u{ffff}',
        '\u{10000}',
        '\u{10ffff}',
        '\n',
        '"',
        '\\',
    ] {
        let text = format!(
            "{{\"x\":{}}}",
            serde_json::to_string(&value.to_string()).unwrap()
        );
        let result = drive(Cursor::from_json(text.as_bytes(), Limits::default()), 1).unwrap();
        assert_eq!(
            result.view().get("x").unwrap().text(),
            Some(value.to_string().as_str())
        );
    }
    for text in [
        r#"{"x":"\ud800"}"#,
        r#"{"x":"\udc00"}"#,
        r#"{"x":"\ud800\u1234"}"#,
        r#"{"x":"\x"}"#,
        r#"{"x":"\u12xx"}"#,
        r#"{"x":"line
break"}"#,
    ] {
        assert_eq!(
            drive(Cursor::from_json(text.as_bytes(), Limits::default()), 1)
                .err()
                .unwrap()
                .cause,
            Cause::Syntax
        );
    }
    for bytes in [
        &b"\xc0\xaf"[..],
        &b"\xe0\x80\x80"[..],
        &b"\xed\xa0\x80"[..],
        &b"\xf4\x90\x80\x80"[..],
        &b"\xf5\x80\x80\x80"[..],
        &b"\x80"[..],
    ] {
        let mut text = b"{\"x\":\"".to_vec();
        text.extend_from_slice(bytes);
        text.extend_from_slice(b"\"}");
        assert_eq!(
            drive(Cursor::from_json(&text, Limits::default()), 1)
                .err()
                .unwrap()
                .cause,
            Cause::Syntax
        );
    }
}

#[test]
fn complete_grammar_has_no_unpaid_finish_or_ignored_overwritten_syntax() {
    for text in [
        "",
        "[]",
        "null",
        "{",
        "{\"x\"}",
        "{\"x\":}",
        "{\"x\":true,}",
        "{\"x\":[1,]}",
        "{\"x\":01}",
        "{\"x\":1.}",
        "{\"x\":1e+}",
        "{\"x\":trueX}",
        "{\"x\":false false}",
        "{}X",
        "{}{}",
        "{\"x\":1e+,\"x\":0}",
        "{\"x\":\"\\ud800\",\"x\":true}",
    ] {
        assert!(literal_value_reference::literal(text).is_err() || text == "[]" || text == "null");
        assert_eq!(
            drive(Cursor::from_json(text.as_bytes(), Limits::default()), 1)
                .err()
                .unwrap()
                .cause,
            Cause::Syntax,
            "{text}"
        );
    }
    let valid = r#" { "x" : [ {}, [], -1.2e+3, "" ], "x":null } "#;
    equivalent(
        drive(Cursor::from_json(valid.as_bytes(), Limits::default()), 1)
            .unwrap()
            .view(),
        &literal_value_reference::literal(valid).unwrap(),
    );
}

#[test]
fn number_ceilings_apply_before_decoded_output_and_stop_without_tail_scan() {
    for limit in [0, 3, 64, 256, 257] {
        let limits = Limits {
            number: limit,
            ..Limits::default()
        };
        if limit == 0 {
            let error = drive(Cursor::from_json(br#"{"x":0}"#, limits), 1)
                .err()
                .unwrap();
            assert_eq!(error.cause, Cause::RawNumber);
            assert_eq!(error.offset, 6);
            continue;
        }
        for n in [limit - 1, limit, limit + 1] {
            let token = "1".repeat(n.max(1));
            let text = format!(r#"{{"x":{token}}}"#);
            let typed = literal_value_reference::literal(&text).unwrap();
            let strict = drive(Cursor::from_json(text.as_bytes(), limits), 1);
            let compat = drive(Cursor::from_value(&typed, limits), 1);
            if n <= limit {
                equivalent(strict.unwrap().view(), &typed);
                compat.unwrap();
            } else {
                let error = strict.err().unwrap();
                assert_eq!(error.cause, Cause::RawNumber);
                assert_eq!(error.offset, 5 + limit + 1);
                assert_eq!(compat.err().unwrap().cause, Cause::DecodedNumber);
            }
        }
    }
    let error = drive(
        Cursor::from_json(
            br#"{"x":1e0}"#,
            Limits {
                number: 3,
                ..Limits::default()
            },
        ),
        1,
    )
    .err()
    .unwrap();
    assert_eq!(error.cause, Cause::DecodedNumber);
    // Only the key byte was emitted; no Number content is materialized.
    assert_eq!(error.trace.work[2], 1);
    let text = format!(r#"{{"x":1e{}X}}"#, "0".repeat(65_536));
    let error = drive(
        Cursor::from_json(
            text.as_bytes(),
            Limits {
                number: 64,
                ..Limits::default()
            },
        ),
        1,
    )
    .err()
    .unwrap();
    assert_eq!(
        (error.cause, error.offset, error.trace.work[2]),
        (Cause::RawNumber, 70, 1)
    );
    assert_eq!(error.trace.wire_observed, 70);
}

#[test]
fn zero_budget_lifetime_and_each_phase_cancellation_preserve_terminal_release() {
    let text = br#"{"z":[1,"long text",{"a":true}],"a":null,"a":false}"#;
    let cursor = Cursor::from_json(text, Limits::default());
    let before = cursor.trace();
    let Progress::Pending(cursor) = cursor.step(&mut WorkBudget::new(), false) else {
        panic!()
    };
    assert_eq!(cursor.trace(), before);
    assert_eq!(cursor.live_bytes(), 0);
    let good = drive(cursor, 1).unwrap();
    let spent: u64 = good.trace().work.iter().sum();
    assert!(
        drive(
            Cursor::from_json(
                text,
                Limits {
                    lifetime: spent,
                    ..Limits::default()
                }
            ),
            1
        )
        .is_ok()
    );
    let short = drive(
        Cursor::from_json(
            text,
            Limits {
                lifetime: spent - 1,
                ..Limits::default()
            },
        ),
        1,
    )
    .err()
    .unwrap();
    assert_eq!(short.cause, Cause::Lifetime);
    assert_eq!(short.phase, Phase::Seal);
    let mut phases = [false; 6];
    'stops: for stop in 0..100_000 {
        let mut cursor = Cursor::from_json(text, Limits::default());
        for _ in 0..stop {
            match cursor.step(&mut budget(1), false) {
                Progress::Pending(next) => cursor = next,
                Progress::Complete(_) => {
                    break 'stops;
                }
                Progress::Failed(failure) => panic!("{failure:?}"),
            }
        }
        let phase = cursor.phase();
        phases[match phase {
            Phase::Input => 0,
            Phase::Sort => 1,
            Phase::Duplicates => 2,
            Phase::Reachability => 3,
            Phase::Compact => 4,
            Phase::Seal => 5,
            Phase::Complete => unreachable!(),
        }] = true;
        let Progress::Failed(error) = cursor.step(&mut WorkBudget::new(), true) else {
            panic!()
        };
        assert_eq!(error.cause, Cause::Cancelled);
        assert_eq!(error.phase, phase);
        assert_eq!(error.allocations, error.releases);
        assert_eq!(error.live_after_rollback, 0);
    }
    assert_eq!(phases, [true; 6]);
}

#[test]
fn deep_input_uses_the_frame_arena_and_never_an_engine_recursive_graph() {
    let text = format!("{{\"x\":{}0{}}}", "[".repeat(512), "]".repeat(512));
    let result = drive(Cursor::from_json(text.as_bytes(), Limits::default()), 1).unwrap();
    let mut view = result.view().get("x").unwrap();
    for _ in 0..512 {
        assert_eq!(view.kind(), Kind::Array);
        view = view.child(0).unwrap();
    }
    assert_eq!(view.text(), Some("0"));
    let error = drive(
        Cursor::from_json(
            text.as_bytes(),
            Limits {
                frames: 32,
                ..Limits::default()
            },
        ),
        1,
    )
    .err()
    .unwrap();
    assert_eq!(error.cause, Cause::Frames);
    assert!(error.offset < 40);
    let mut bad = text.clone();
    bad.pop();
    assert_eq!(
        drive(Cursor::from_json(bad.as_bytes(), Limits::default()), 1)
            .err()
            .unwrap()
            .cause,
        Cause::Syntax
    );
}

#[test]
fn actual_source_temporary_peak_and_contiguous_boundaries_are_independent() {
    let text = br#"{"long-key":[true,"longer string",12.3],"a":{"x":null},"a":false}"#;
    let result = drive(Cursor::from_json(text, Limits::default()), 1).unwrap();
    let footprint = result.footprint();
    assert!(footprint.conversion_peak_bytes >= footprint.retained_requested_bytes);
    for kind in 0..4 {
        let boundary = [
            footprint.retained_requested_bytes,
            footprint.temporary_peak_bytes,
            footprint.conversion_peak_bytes,
            footprint.largest_request_bytes,
        ][kind];
        for delta in [-1_i64, 0, 1] {
            let limit = (boundary as i64 + delta) as u64;
            let mut limits = Limits::default();
            match kind {
                0 => limits.source = limit,
                1 => limits.temporary = limit,
                2 => limits.peak = limit,
                3 => limits.contiguous = limit,
                _ => unreachable!(),
            }
            let result = drive(Cursor::from_json(text, limits), 1);
            if delta < 0 {
                assert_eq!(result.err().unwrap().cause, Cause::Memory);
            } else {
                assert!(result.is_ok());
            }
        }
    }
    for (limits, cause) in [
        (
            Limits {
                input_bytes: text.len() - 1,
                ..Limits::default()
            },
            Cause::InputBytes,
        ),
        (
            Limits {
                nodes: 1,
                ..Limits::default()
            },
            Cause::Nodes,
        ),
        (
            Limits {
                edges: 0,
                ..Limits::default()
            },
            Cause::Edges,
        ),
        (
            Limits {
                bytes: 0,
                ..Limits::default()
            },
            Cause::Bytes,
        ),
    ] {
        assert_eq!(
            drive(Cursor::from_json(text, limits), 1)
                .err()
                .unwrap()
                .cause,
            cause
        );
    }
}

#[test]
fn complete_owner_contains_no_borrow_of_wire_or_typed_source() {
    fn require_static<T: 'static>(_: &T) {}
    let value = {
        let text = String::from(r#"{"x":{"a":[true,"owned",1e309]}}"#);
        drive(Cursor::from_json(text.as_bytes(), Limits::default()), 1).unwrap()
    };
    require_static(&value);
    assert_eq!(
        value
            .view()
            .get("x")
            .unwrap()
            .get("a")
            .unwrap()
            .child(1)
            .unwrap()
            .text(),
        Some("owned")
    );
    let value = {
        let source = literal_value_reference::literal(r#"{"x":"typed owned"}"#).unwrap();
        drive(Cursor::from_value(&source, Limits::default()), 1).unwrap()
    };
    require_static(&value);
    assert_eq!(value.view().get("x").unwrap().text(), Some("typed owned"));
}

#[test]
fn grammar_mutations_differentially_check_the_constructed_arena_not_only_tokens() {
    let original = br#"{"alpha":[null,true,false,{"b":"\u4e2d\ud83d\ude00","a":1e0}],"alpha":{"x":false},"n":-0}"#;
    for position in 0..original.len() {
        for replacement in b"{}[]:,\\\"ntf09eE+-x \t\n\xc0" {
            let mut input = original.to_vec();
            input[position] = *replacement;
            let reference = core::str::from_utf8(&input)
                .ok()
                .and_then(|text| literal_value_reference::literal(text).ok())
                .filter(serde_json::Value::is_object);
            let output = drive(Cursor::from_json(&input, Limits::default()), 1);
            match reference {
                Some(reference) => equivalent(output.unwrap().view(), &reference),
                None => assert_eq!(
                    output.err().unwrap().cause,
                    Cause::Syntax,
                    "position={position}, byte={replacement}"
                ),
            }
        }
    }
}

#[test]
fn long_equal_key_prefixes_are_paid_and_duplicate_cleanup_preserves_associations() {
    let prefix = "λ".repeat(512);
    let text = format!(
        r#"{{"{prefix}z":null,"{prefix}a":["old",true],"{prefix}m":false,"{prefix}a":{{"final":1}}}}"#
    );
    let reference = literal_value_reference::literal(&text).unwrap();
    let value = drive(Cursor::from_json(text.as_bytes(), Limits::default()), 1).unwrap();
    equivalent(value.view(), &reference);
    assert!(value.trace().key_bytes > 2 * prefix.len());
    assert!(value.trace().discarded_nodes > 0);
    let error = drive(
        Cursor::from_json(
            text.as_bytes(),
            Limits {
                lifetime: 25_000,
                ..Limits::default()
            },
        ),
        1,
    )
    .err()
    .unwrap();
    assert_eq!(error.cause, Cause::Lifetime);
    assert!(error.trace.wire_observed <= text.len());
}
