use core::time::Duration;

use clinkz_wot_core::InteractionOptions;

#[test]
fn options_preserve_omission_and_explicit_selection() {
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
