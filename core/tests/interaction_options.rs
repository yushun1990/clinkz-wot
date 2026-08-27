use core::time::Duration;

use clinkz_wot_core::{InteractionOptions, Payload};

#[test]
fn new_target_methods_preserve_omission_and_explicit_selection() {
    let empty = InteractionOptions::new();
    assert!(empty.uri_variables().is_empty());
    assert_eq!(empty.form_index(), None);
    assert_eq!(empty.timeout(), None);

    let timeout = Duration::from_millis(750);
    let selected = InteractionOptions::new()
        .with_uri_variable("zone", "north")
        .with_form_index(3)
        .with_timeout(timeout);

    assert_eq!(
        selected.uri_variables().get("zone").map(String::as_str),
        Some("north")
    );
    assert_eq!(selected.form_index(), Some(3));
    assert_eq!(selected.timeout(), Some(timeout));
}

#[test]
fn legacy_data_constructor_still_compiles_without_joining_target_accessors() {
    let payload = Payload::new(b"legacy-write-value".to_vec(), "application/octet-stream");
    let options = InteractionOptions::with_data(payload.clone());

    assert_eq!(options.data.as_ref(), Some(&payload));
    assert!(options.uri_variables().is_empty());
    assert_eq!(options.form_index(), None);
    assert_eq!(options.timeout(), None);
}
