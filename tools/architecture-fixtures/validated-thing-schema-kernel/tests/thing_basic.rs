//! The unchanged production crate is the independent acceptance/ordering oracle.
// The same corpus must compile against two distinct TD Rust type universes.
#![allow(clippy::duplicate_mod)]
use clinkz_wot_td as original;
use original::validate::Validate as _;
use td_crate::validate::{Validate, ValidationLevel};
use validated_thing_schema_kernel_probe as td_crate;
#[path = "support/original_basic_corpus.rs"]
#[allow(dead_code)]
mod baseline;
#[path = "../../../../td/tests/support/basic_corpus_shared.rs"]
#[allow(dead_code)]
mod corpus;

#[test]
fn complete_public_basic_component_taxonomy_and_first_error_match_current_source() {
    let before = baseline::corpus::cases();
    let after = corpus::cases();
    assert_eq!(before.len(), after.len());
    assert!(after.len() > 170);
    for (before, after) in before.iter().zip(&after) {
        assert_eq!(before.label, after.label);
        for level in [
            ValidationLevel::Minimal,
            ValidationLevel::Basic,
            ValidationLevel::Profile,
            ValidationLevel::Full,
        ] {
            let old_level = match level {
                ValidationLevel::Minimal => original::validate::ValidationLevel::Minimal,
                ValidationLevel::Basic => original::validate::ValidationLevel::Basic,
                ValidationLevel::Profile => original::validate::ValidationLevel::Profile,
                ValidationLevel::Full => original::validate::ValidationLevel::Full,
            };
            let old = before.thing.validate_with_level(old_level);
            let new = after.thing.validate_with_level(level);
            // Debug pins taxonomy and all payloads; Display pins existing text.
            assert_eq!(
                old.as_ref().map_err(|err| format!("{err:?}")),
                new.as_ref().map_err(|err| format!("{err:?}")),
                "{} {level:?}",
                after.label
            );
            if level == ValidationLevel::Basic {
                assert_eq!(new.is_ok(), after.valid, "{}", after.label);
            }
            assert_eq!(
                old.map_err(|err| err.to_string()),
                new.map_err(|err| err.to_string()),
                "{} {level:?}",
                after.label
            );
            for ((name, old), (new_name, new)) in before
                .thing
                .security_definitions
                .iter()
                .zip(&after.thing.security_definitions)
            {
                assert_eq!(name, new_name);
                assert_eq!(
                    old.validate_with_level(old_level)
                        .map_err(|err| format!("{err:?}")),
                    new.validate_with_level(level)
                        .map_err(|err| format!("{err:?}")),
                    "{} security {name}",
                    after.label
                );
            }
            for (old, new) in before
                .thing
                .properties
                .iter()
                .flat_map(|m| m.values())
                .zip(after.thing.properties.iter().flat_map(|m| m.values()))
            {
                assert_eq!(
                    old.validate_with_level(old_level)
                        .map_err(|err| format!("{err:?}")),
                    new.validate_with_level(level)
                        .map_err(|err| format!("{err:?}")),
                    "{} Property",
                    after.label
                );
            }
            for (old, new) in before
                .thing
                .actions
                .iter()
                .flat_map(|m| m.values())
                .zip(after.thing.actions.iter().flat_map(|m| m.values()))
            {
                assert_eq!(
                    old.validate_with_level(old_level)
                        .map_err(|err| format!("{err:?}")),
                    new.validate_with_level(level)
                        .map_err(|err| format!("{err:?}")),
                    "{} Action",
                    after.label
                );
            }
            for (old, new) in before
                .thing
                .events
                .iter()
                .flat_map(|m| m.values())
                .zip(after.thing.events.iter().flat_map(|m| m.values()))
            {
                assert_eq!(
                    old.validate_with_level(old_level)
                        .map_err(|err| format!("{err:?}")),
                    new.validate_with_level(level)
                        .map_err(|err| format!("{err:?}")),
                    "{} Event",
                    after.label
                );
            }
        }
    }
}

#[test]
fn public_default_security_seam_preserves_flags_and_explicit_empty_overrides() {
    let before = baseline::corpus::cases();
    let after = corpus::cases();
    for (before, after) in before.iter().zip(&after) {
        for (old, new) in before
            .thing
            .properties
            .iter()
            .flat_map(|m| m.values())
            .zip(after.thing.properties.iter().flat_map(|m| m.values()))
        {
            for (old_form, new_form) in old._interaction.forms.iter().zip(&new._interaction.forms) {
                let old_ops = original::td_defaults::effective_form_operations(
                    original::td_defaults::FormContext::Property(old),
                    old_form,
                );
                let new_ops = td_crate::td_defaults::effective_form_operations(
                    td_crate::td_defaults::FormContext::Property(new),
                    new_form,
                );
                assert_eq!(
                    old_ops.iter().map(|op| op.as_str()).collect::<Vec<_>>(),
                    new_ops.iter().map(|op| op.as_str()).collect::<Vec<_>>(),
                    "{}",
                    after.label
                );
                assert_eq!(
                    original::td_defaults::effective_form_security(&before.thing, old_form),
                    td_crate::td_defaults::effective_form_security(&after.thing, new_form),
                    "{}",
                    after.label
                );
            }
        }
    }
}
