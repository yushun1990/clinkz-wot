#![allow(dead_code)]

use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingConfigurationDigest, BindingGeneration, BindingId,
    BindingRegistrationCapabilities, BindingRegistrationIdentity,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeProfile {
    Host,
    Static,
}

#[derive(Clone, Copy, Debug)]
struct CompleteRegistration {
    identity: BindingRegistrationIdentity,
    capabilities: BindingRegistrationCapabilities,
    host_execution: bool,
    static_execution: bool,
}

impl CompleteRegistration {
    fn eligible(self, profile: RuntimeProfile) -> bool {
        self.capabilities.supports_consumer_property_read()
            && match profile {
                RuntimeProfile::Host => self.host_execution,
                RuntimeProfile::Static => self.static_execution,
            }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectionError {
    NoEligibleRegistration,
    AmbiguousRegistration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SelectedRegistration {
    snapshot_ordinal: u32,
    identity: BindingRegistrationIdentity,
}

fn select_exactly_one(
    snapshot: &[CompleteRegistration],
    profile: RuntimeProfile,
) -> Result<SelectedRegistration, SelectionError> {
    let mut selected = None;

    for (ordinal, entry) in snapshot.iter().copied().enumerate() {
        if !entry.eligible(profile) {
            continue;
        }
        if selected.is_some() {
            return Err(SelectionError::AmbiguousRegistration);
        }
        selected = Some(SelectedRegistration {
            snapshot_ordinal: ordinal as u32,
            identity: entry.identity,
        });
    }

    selected.ok_or(SelectionError::NoEligibleRegistration)
}

fn identity(binding: u32, diagnostic_ordinal: u32) -> BindingRegistrationIdentity {
    BindingRegistrationIdentity::new(
        BindingId::new(binding),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([binding as u8; 32]),
        BindingArtifactCompatibility::new([binding as u8; 16]),
        diagnostic_ordinal,
    )
}

fn entry(
    binding: u32,
    diagnostic_ordinal: u32,
    consumer: bool,
    host: bool,
    static_execution: bool,
) -> CompleteRegistration {
    CompleteRegistration {
        identity: identity(binding, diagnostic_ordinal),
        capabilities: if consumer {
            BindingRegistrationCapabilities::producer_and_consumer_property_read()
        } else {
            BindingRegistrationCapabilities::producer_property_read()
        },
        host_execution: host,
        static_execution,
    }
}

#[test]
fn zero_eligible_registrations_is_rejected_without_order_fallback() {
    let snapshot = [entry(1, 17, false, true, true), entry(2, 3, true, false, true)];

    assert_eq!(
        select_exactly_one(&snapshot, RuntimeProfile::Host),
        Err(SelectionError::NoEligibleRegistration)
    );
}

#[test]
fn exactly_one_profile_eligible_registration_is_selected_by_snapshot_position() {
    let snapshot = [
        entry(1, 17, false, true, true),
        entry(2, 99, true, true, true),
        entry(3, 1, true, false, true),
    ];

    let selected = select_exactly_one(&snapshot, RuntimeProfile::Host).expect("one host entry");
    assert_eq!(selected.snapshot_ordinal, 1);
    assert_eq!(selected.identity.binding_id(), BindingId::new(2));
    assert_eq!(selected.identity.diagnostic_ordinal(), 99);
    assert_ne!(
        selected.snapshot_ordinal,
        selected.identity.diagnostic_ordinal(),
        "diagnostic ordinal is never the snapshot index"
    );
}

#[test]
fn multiple_eligible_registrations_are_ambiguous_even_if_the_first_would_work() {
    let snapshot = [entry(7, 4, true, true, true), entry(8, 5, true, true, true)];

    assert_eq!(
        select_exactly_one(&snapshot, RuntimeProfile::Host),
        Err(SelectionError::AmbiguousRegistration)
    );

    let reversed = [snapshot[1], snapshot[0]];
    assert_eq!(
        select_exactly_one(&reversed, RuntimeProfile::Host),
        Err(SelectionError::AmbiguousRegistration),
        "registration order must not resolve ambiguity"
    );
}

#[test]
fn profile_execution_half_is_part_of_eligibility() {
    let snapshot = [entry(1, 9, true, true, false), entry(2, 10, true, false, true)];

    let host = select_exactly_one(&snapshot, RuntimeProfile::Host).expect("one host entry");
    let static_selected =
        select_exactly_one(&snapshot, RuntimeProfile::Static).expect("one static entry");

    assert_eq!(host.identity.binding_id(), BindingId::new(1));
    assert_eq!(static_selected.identity.binding_id(), BindingId::new(2));
}
